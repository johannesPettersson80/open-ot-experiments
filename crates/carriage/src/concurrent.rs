//! Concurrent publish/read protocol for a shared-memory ring.
//!
//! Models the case where a consumer observes controller memory asynchronously (a
//! separate core or comms processor on weakly-ordered hardware). The producer advances
//! `oldest_abs`, executes a `Release` fence, then clobbers reclaimed bytes; the consumer
//! reads candidate bytes, executes an `Acquire` fence, then re-reads `oldest_abs` before
//! delivering. Those fences are what make the overwrite re-check sound on weak memory;
//! the loom tests (under `--cfg loom`) exercise the interleavings.

#[cfg(all(test, loom))]
mod sync {
    pub use loom::sync::Arc;
    pub use loom::sync::atomic::{AtomicU8, AtomicU64, Ordering, fence};
}

#[cfg(not(all(test, loom)))]
mod sync {
    pub use std::sync::Arc;
    pub use std::sync::atomic::{AtomicU8, AtomicU64, Ordering, fence};
}

use sync::{Arc, AtomicU8, AtomicU64, Ordering, fence};

use crate::ring::{ReadBatch, ReadRecord, RingError};
use crate::wire::{HEADER_LEN, Record, SYNC, WireError, decode};

/// A ring whose bytes and control fields are atomics, so a consumer on another core or
/// processor can read it while the producer writes.
#[derive(Debug)]
pub struct ConcurrentRing {
    bytes: Box<[AtomicU8]>,
    capacity: usize,
    head_abs: AtomicU64,
    oldest_abs: AtomicU64,
    lost_count: AtomicU64,
}

impl ConcurrentRing {
    /// Creates a shared ring of `capacity` bytes. Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Arc<Self> {
        assert!(capacity > 0, "ring capacity must be non-zero");
        Arc::new(Self {
            bytes: (0..capacity).map(|_| AtomicU8::new(0)).collect(),
            capacity,
            head_abs: AtomicU64::new(0),
            oldest_abs: AtomicU64::new(0),
            lost_count: AtomicU64::new(0),
        })
    }

    /// Byte capacity of the ring.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Acquire-loads the publish head (the committed write position).
    pub fn head_abs(&self) -> u64 {
        self.head_abs.load(Ordering::Acquire)
    }

    /// Acquire-loads the oldest retained absolute offset.
    pub fn oldest_abs(&self) -> u64 {
        self.oldest_abs.load(Ordering::Acquire)
    }

    /// Acquire-loads the count of records evicted so far.
    pub fn lost_count(&self) -> u64 {
        self.lost_count.load(Ordering::Acquire)
    }

    fn physical_offset(&self, abs: u64) -> usize {
        (abs % self.capacity as u64) as usize
    }

    fn load_byte(&self, phys: usize) -> u8 {
        self.bytes[phys].load(Ordering::Relaxed)
    }

    fn store_byte(&self, phys: usize, byte: u8) {
        self.bytes[phys].store(byte, Ordering::Relaxed);
    }

    fn read_raw_from(&self, cursor_abs: u64) -> Result<ReadBatch, RingError> {
        let first_oldest = self.oldest_abs.load(Ordering::Acquire);
        let head_abs = self.head_abs.load(Ordering::Acquire);
        let lapped = cursor_abs < first_oldest;
        let mut abs = if lapped { first_oldest } else { cursor_abs };
        let mut records = Vec::new();

        while abs < head_abs {
            let oldest_before = self.oldest_abs.load(Ordering::Acquire);
            if oldest_before > abs {
                return Err(RingError::OverwrittenMidRead {
                    read_abs: abs,
                    oldest_abs: oldest_before,
                });
            }

            let phys = self.physical_offset(abs);
            let first = self.load_byte(phys);
            if first == 0 {
                if phys == 0 {
                    return Err(RingError::UnexpectedWrapMarker { abs });
                }
                abs += (self.capacity - phys) as u64;
                continue;
            }

            if phys + HEADER_LEN > self.capacity {
                return Err(RingError::RecordCrossesBoundary { abs });
            }
            let sync = [
                self.load_byte(phys),
                self.load_byte(phys + 1),
                self.load_byte(phys + 2),
                self.load_byte(phys + 3),
            ];
            if sync != SYNC {
                return Err(WireError::WrongSync.into());
            }

            let total_len =
                u16::from_le_bytes([self.load_byte(phys + 4), self.load_byte(phys + 5)]) as usize;
            if total_len < HEADER_LEN {
                return Err(WireError::InvalidLength {
                    total_len,
                    available: self.capacity - phys,
                }
                .into());
            }
            if phys + total_len > self.capacity {
                return Err(RingError::RecordCrossesBoundary { abs });
            }

            let mut bytes = Vec::with_capacity(total_len);
            for i in 0..total_len {
                bytes.push(self.load_byte(phys + i));
            }

            // Pairs with the producer's release fence before clobbering reclaimed bytes.
            fence(Ordering::Acquire);
            let oldest_after = self.oldest_abs.load(Ordering::Relaxed);
            if oldest_after > abs {
                return Err(RingError::OverwrittenMidRead {
                    read_abs: abs,
                    oldest_abs: oldest_after,
                });
            }

            let decoded = decode(&bytes)?;

            records.push(ReadRecord {
                start_abs: abs,
                end_abs: abs + decoded.consumed as u64,
                record: decoded.record,
            });
            abs += total_len as u64;
        }

        let final_oldest = self.oldest_abs.load(Ordering::Acquire);
        if final_oldest != first_oldest && final_oldest > cursor_abs {
            return Err(RingError::OverwrittenMidRead {
                read_abs: cursor_abs,
                oldest_abs: final_oldest,
            });
        }

        Ok(ReadBatch {
            records,
            next_abs: head_abs,
            lapped,
        })
    }
}

/// Single writer for a [`ConcurrentRing`]: evicts (with a release fence) before clobber,
/// then publishes the new head with `Release`.
#[derive(Debug)]
pub struct ConcurrentProducer {
    ring: Arc<ConcurrentRing>,
    spans: std::collections::VecDeque<(u64, u64)>,
    head_abs: u64,
}

impl ConcurrentProducer {
    /// Creates a producer bound to `ring`.
    pub fn new(ring: Arc<ConcurrentRing>) -> Self {
        Self {
            ring,
            spans: std::collections::VecDeque::new(),
            head_abs: 0,
        }
    }

    /// Encodes and publishes `record`, reclaiming the oldest bytes if needed.
    ///
    /// Ordering: advance `oldest_abs` and release-fence before clobbering reclaimed
    /// bytes, write the record bytes, then `Release`-store the new `head_abs`.
    pub fn write_record(&mut self, record: &Record) -> Result<(), RingError> {
        let encoded = record.encode(true)?;
        if encoded.len() > self.ring.capacity {
            return Err(RingError::RecordExceedsCapacity {
                record_len: encoded.len(),
                capacity: self.ring.capacity,
            });
        }

        let start_phys = self.ring.physical_offset(self.head_abs);
        let record_start_abs =
            if start_phys == 0 || start_phys + encoded.len() <= self.ring.capacity {
                self.head_abs
            } else {
                self.head_abs + (self.ring.capacity - start_phys) as u64
            };
        let final_head = record_start_abs + encoded.len() as u64;

        self.evict_before_clobber(final_head, record_start_abs);

        if record_start_abs != self.head_abs {
            self.ring.store_byte(start_phys, 0);
        }

        let record_start = self.ring.physical_offset(record_start_abs);
        for (i, byte) in encoded.iter().enumerate() {
            self.ring.store_byte(record_start + i, *byte);
        }

        self.spans.push_back((record_start_abs, final_head));
        self.head_abs = final_head;
        self.ring.head_abs.store(final_head, Ordering::Release);
        Ok(())
    }

    fn evict_before_clobber(&mut self, final_head: u64, next_record_start: u64) {
        let retention_floor = final_head.saturating_sub(self.ring.capacity as u64);
        let mut evicted = 0u64;
        while self
            .spans
            .front()
            .is_some_and(|(start_abs, _)| *start_abs < retention_floor)
        {
            self.spans.pop_front();
            evicted += 1;
        }
        if evicted > 0 {
            self.ring.lost_count.fetch_add(evicted, Ordering::Release);
        }
        let new_oldest = self
            .spans
            .front()
            .map_or(next_record_start, |(start_abs, _)| *start_abs);
        self.ring.oldest_abs.store(new_oldest, Ordering::Relaxed);
        fence(Ordering::Release);
    }
}

/// Reads a [`ConcurrentRing`] by walking raw bytes with the acquire-fenced overwrite
/// re-check, recovering by lapping to `oldest_abs` when it falls behind.
#[derive(Debug)]
pub struct ConcurrentRawConsumer {
    ring: Arc<ConcurrentRing>,
    cursor_abs: u64,
    lapped_batches: u64,
    overwritten_retries: u64,
    rejected_records: u64,
}

impl ConcurrentRawConsumer {
    /// Creates a consumer bound to `ring`.
    pub fn new(ring: Arc<ConcurrentRing>) -> Self {
        Self {
            ring,
            cursor_abs: 0,
            lapped_batches: 0,
            overwritten_retries: 0,
            rejected_records: 0,
        }
    }

    /// Reads available records, recovering from overwrite and decode faults by lapping
    /// the cursor to `oldest_abs` and counting the event.
    pub fn poll(&mut self) -> Result<ReadBatch, RingError> {
        match self.ring.read_raw_from(self.cursor_abs) {
            Ok(batch) => {
                if batch.lapped {
                    self.lapped_batches += 1;
                }
                self.cursor_abs = batch.next_abs;
                Ok(batch)
            }
            Err(RingError::OverwrittenMidRead { oldest_abs, .. }) => {
                self.cursor_abs = oldest_abs;
                self.overwritten_retries += 1;
                Ok(ReadBatch {
                    records: Vec::new(),
                    next_abs: oldest_abs,
                    lapped: true,
                })
            }
            Err(error @ RingError::Wire(_)) => {
                let oldest = self.ring.oldest_abs();
                if oldest > self.cursor_abs {
                    self.cursor_abs = oldest;
                    self.rejected_records += 1;
                    Ok(ReadBatch {
                        records: Vec::new(),
                        next_abs: oldest,
                        lapped: true,
                    })
                } else {
                    Err(error)
                }
            }
            Err(error) => Err(error),
        }
    }

    /// Current absolute read cursor.
    pub fn cursor_abs(&self) -> u64 {
        self.cursor_abs
    }

    /// Number of batches that observed a lap (cursor behind `oldest_abs`).
    pub fn lapped_batches(&self) -> u64 {
        self.lapped_batches
    }

    /// Number of mid-read overwrite retries.
    pub fn overwritten_retries(&self) -> u64 {
        self.overwritten_retries
    }

    /// Number of records rejected by wire validation (e.g. CRC).
    pub fn rejected_records(&self) -> u64 {
        self.rejected_records
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;

    use super::*;
    use crate::consumer::LossAccountingConsumer;

    const EVENT_MESSAGE: u32 = 0x0003;

    #[test]
    fn real_thread_stress_accepts_only_crc_valid_non_overwritten_records() {
        let ring = ConcurrentRing::new(2048);
        let producer_ring = Arc::clone(&ring);
        let done = Arc::new(AtomicBool::new(false));
        let producer_done = Arc::clone(&done);
        let records_per_source = 5_000u64;
        let sources = [31u32, 32u32];

        let producer = thread::spawn(move || {
            let mut producer = ConcurrentProducer::new(producer_ring);
            for seq in 0..records_per_source {
                for source in sources {
                    producer.write_record(&minimal_record(source, seq)).unwrap();
                }
            }
            producer_done.store(true, Ordering::Release);
        });

        let consumer_ring = Arc::clone(&ring);
        let consumer_done = Arc::clone(&done);
        let consumer = thread::spawn(move || {
            let mut consumer = ConcurrentRawConsumer::new(consumer_ring);
            let mut accounting = LossAccountingConsumer::new();
            let mut delivered = 0u64;

            loop {
                let batch = consumer.poll().unwrap();
                delivered += batch.records.len() as u64;
                accounting.account_batch(&batch);

                if consumer_done.load(Ordering::Acquire)
                    && consumer.cursor_abs() == consumer.ring.head_abs()
                {
                    break;
                }
                thread::yield_now();
            }

            (consumer, accounting, delivered)
        });

        producer.join().unwrap();
        let (consumer, accounting, delivered) = consumer.join().unwrap();

        assert!(
            delivered > 20,
            "consumer made too little progress: {delivered}"
        );
        assert!(ring.head_abs() > ring.capacity() as u64 * 100);
        assert!(ring.lost_count() > 0);
        assert!(
            consumer.lapped_batches() > 0
                || consumer.overwritten_retries() > 0
                || consumer.rejected_records() > 0,
            "stress did not exercise overwrite pressure"
        );
        for source in sources {
            assert_eq!(
                accounting.delivered(source) + accounting.lost(source),
                records_per_source,
                "loss reconciliation failed for source {source}"
            );
        }
    }

    fn minimal_record(source_id: u32, seq: u64) -> Record {
        Record::new(
            1_780_000_000_000_000_000 + seq,
            1,
            seq,
            source_id,
            EVENT_MESSAGE,
        )
    }
}

#[cfg(all(test, loom))]
mod loom_tests {
    use loom::sync::Arc;
    use loom::sync::atomic::{AtomicU8, AtomicU64, Ordering, fence};
    use loom::thread;

    const CAP: usize = 4;
    const OLD: [u8; CAP] = [0xA5, 4, 0x10, 0xB1];
    const NEW: [u8; CAP] = [0xA5, 4, 0x20, 0x81];

    #[test]
    fn loom_no_contention_accepts_complete_old_record() {
        loom::model(|| {
            let ring = ModelRing::new();
            ring.init_old_record();
            assert_eq!(ring.try_read_old_cursor_fenced(), Some(OLD));
        });
    }

    #[test]
    fn loom_rejects_mid_write_overwrite_or_reads_old_complete_record() {
        loom::model(|| {
            let ring = Arc::new(ModelRing::new());
            ring.init_old_record();

            let writer = Arc::clone(&ring);
            let producer = thread::spawn(move || {
                writer.oldest_abs.store(4, Ordering::Relaxed);
                fence(Ordering::Release);
                for (i, byte) in NEW.iter().enumerate() {
                    writer.bytes[i].store(*byte, Ordering::Relaxed);
                    thread::yield_now();
                }
                writer.head_abs.store(8, Ordering::Release);
            });

            let reader = Arc::clone(&ring);
            let consumer = thread::spawn(move || reader.try_read_old_cursor_fenced());

            producer.join().unwrap();
            let accepted = consumer.join().unwrap();
            if let Some(bytes) = accepted {
                assert_eq!(bytes, OLD, "consumer accepted torn or pre-publish bytes");
            }
        });
    }

    #[test]
    fn loom_control_unfenced_model_does_not_expose_weak_memory_hole() {
        let detected = std::panic::catch_unwind(|| {
            loom::model(|| {
                let ring = Arc::new(ModelRing::new());
                ring.init_old_record();

                let writer = Arc::clone(&ring);
                let producer = thread::spawn(move || {
                    writer.oldest_abs.store(4, Ordering::Release);
                    for (i, byte) in NEW.iter().enumerate() {
                        writer.bytes[i].store(*byte, Ordering::Relaxed);
                        thread::yield_now();
                    }
                    writer.head_abs.store(8, Ordering::Release);
                });

                let reader = Arc::clone(&ring);
                let consumer = thread::spawn(move || reader.try_read_old_cursor_unfenced());

                producer.join().unwrap();
                let accepted = consumer.join().unwrap();
                if let Some(bytes) = accepted {
                    assert_ne!(bytes, NEW, "control accepted overwritten bytes");
                    assert_eq!(bytes, OLD, "control accepted torn bytes");
                }
            });
        })
        .is_err();

        assert!(
            !detected,
            "loom started detecting the deliberately unfenced overwrite model"
        );

        // This is intentionally a passing documentation test: the broken, unfenced
        // overwrite model runs to completion under loom. Correctness rests on the
        // section 4.3 release/acquire fences, not on loom detecting this relaxed-reordering gap.
        loom::model(|| {
            let ring = Arc::new(ModelRing::new());
            ring.init_old_record();

            let writer = Arc::clone(&ring);
            let producer = thread::spawn(move || {
                writer.oldest_abs.store(4, Ordering::Release);
                for (i, byte) in NEW.iter().enumerate() {
                    writer.bytes[i].store(*byte, Ordering::Relaxed);
                    thread::yield_now();
                }
                writer.head_abs.store(8, Ordering::Release);
            });

            let reader = Arc::clone(&ring);
            let consumer = thread::spawn(move || reader.try_read_old_cursor_unfenced());

            producer.join().unwrap();
            let _ = consumer.join().unwrap();
        });
    }

    struct ModelRing {
        bytes: [AtomicU8; CAP],
        head_abs: AtomicU64,
        oldest_abs: AtomicU64,
    }

    impl ModelRing {
        fn new() -> Self {
            Self {
                bytes: [
                    AtomicU8::new(0),
                    AtomicU8::new(0),
                    AtomicU8::new(0),
                    AtomicU8::new(0),
                ],
                head_abs: AtomicU64::new(0),
                oldest_abs: AtomicU64::new(0),
            }
        }

        fn init_old_record(&self) {
            for (i, byte) in OLD.iter().enumerate() {
                self.bytes[i].store(*byte, Ordering::Relaxed);
            }
            self.head_abs.store(4, Ordering::Release);
        }

        fn try_read_old_cursor_fenced(&self) -> Option<[u8; CAP]> {
            let cursor = 0;
            let oldest_before = self.oldest_abs.load(Ordering::Acquire);
            let head = self.head_abs.load(Ordering::Acquire);
            if cursor < oldest_before || cursor >= head {
                return None;
            }

            let mut bytes = [0; CAP];
            for (i, byte) in bytes.iter_mut().enumerate() {
                *byte = self.bytes[i].load(Ordering::Relaxed);
                thread::yield_now();
            }

            let valid =
                bytes[0] == 0xA5 && bytes[1] == 4 && bytes[3] == (bytes[0] ^ bytes[1] ^ bytes[2]);
            fence(Ordering::Acquire);
            let oldest_after = self.oldest_abs.load(Ordering::Relaxed);
            if oldest_after > cursor || !valid {
                return None;
            }

            Some(bytes)
        }

        fn try_read_old_cursor_unfenced(&self) -> Option<[u8; CAP]> {
            let cursor = 0;
            let oldest_before = self.oldest_abs.load(Ordering::Acquire);
            let head = self.head_abs.load(Ordering::Acquire);
            if cursor < oldest_before || cursor >= head {
                return None;
            }

            let mut bytes = [0; CAP];
            for (i, byte) in bytes.iter_mut().enumerate() {
                *byte = self.bytes[i].load(Ordering::Relaxed);
                thread::yield_now();
            }

            let valid =
                bytes[0] == 0xA5 && bytes[1] == 4 && bytes[3] == (bytes[0] ^ bytes[1] ^ bytes[2]);
            let oldest_after = self.oldest_abs.load(Ordering::Acquire);
            if oldest_after > cursor || !valid {
                return None;
            }

            Some(bytes)
        }
    }
}
