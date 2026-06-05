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

use std::collections::VecDeque;

use crate::control::{ControlBlockError, ControlBlockSnapshot};
use crate::ring::{DEFAULT_BUFFER_ID, ReadBatch, ReadRecord, RingError};
use crate::wire::{FLAG_HAS_CRC, HEADER_LEN, Record, SYNC, WireError, decode, validate_record};

/// Operation-based storage contract for the concurrent publish/read protocol.
///
/// The protocol is intentionally expressed in terms of loads, stores, and fences
/// rather than borrowed atomic references. That keeps the implementation usable over
/// both the owned in-process store and a future mmap-backed store whose atomics are
/// constructed from checked shared-memory offsets.
pub trait ConcurrentStore: Clone + Send + Sync + 'static {
    /// Byte capacity of the ring storage.
    fn capacity(&self) -> usize;

    /// Relaxed-loads one physical byte.
    fn load_byte_relaxed(&self, phys: usize) -> u8;

    /// Relaxed-stores one physical byte.
    fn store_byte_relaxed(&self, phys: usize, value: u8);

    /// Acquire-loads the committed head.
    fn load_head_acquire(&self) -> u64;

    /// Release-stores the committed head.
    fn store_head_release(&self, value: u64);

    /// Acquire-loads the oldest retained absolute byte.
    fn load_oldest_acquire(&self) -> u64;

    /// Relaxed-loads the oldest retained absolute byte after an acquire fence.
    fn load_oldest_relaxed(&self) -> u64;

    /// Relaxed-stores the oldest retained absolute byte before the release fence.
    fn store_oldest_relaxed(&self, value: u64);

    /// Release-adds producer-known lost records.
    fn fetch_add_lost_release(&self, value: u64);

    /// Acquire-loads producer-known lost records.
    fn load_lost_acquire(&self) -> u64;

    /// Marks the start of a control snapshot update.
    ///
    /// Seqlock-backed stores set the version odd and apply the writer-entry barrier
    /// before any snapshot field is updated.
    fn begin_control_update(&self) {}

    /// Release-publishes the end of a control snapshot update.
    fn end_control_update_release(&self) {}

    /// Reads a coherent control snapshot when the store has one.
    fn read_control_snapshot(&self) -> Result<ControlBlockSnapshot, ControlBlockError>;

    /// Reads `OldestAbs` for the post-record overwrite check.
    fn read_oldest_after_record(&self) -> Result<u64, ControlBlockError> {
        Ok(self.load_oldest_relaxed())
    }

    /// Release-fences after advancing `OldestAbs`, before clobbering reclaimed bytes.
    fn release_before_clobber(&self) {
        fence(Ordering::Release);
    }

    /// Acquire-fences after reading candidate bytes, before re-checking `OldestAbs`.
    fn acquire_before_recheck(&self) {
        fence(Ordering::Acquire);
    }
}

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
}

/// Cloneable adapter that lets the concurrent protocol run over an owned in-process ring.
#[derive(Debug, Clone)]
pub struct OwnedConcurrentStore {
    ring: Arc<ConcurrentRing>,
}

impl OwnedConcurrentStore {
    /// Creates an owned-store adapter for `ring`.
    pub fn new(ring: Arc<ConcurrentRing>) -> Self {
        Self { ring }
    }

    /// Returns the underlying ring handle.
    pub fn ring(&self) -> Arc<ConcurrentRing> {
        Arc::clone(&self.ring)
    }
}

impl ConcurrentStore for OwnedConcurrentStore {
    fn capacity(&self) -> usize {
        self.ring.capacity
    }

    fn load_byte_relaxed(&self, phys: usize) -> u8 {
        self.ring.bytes[phys].load(Ordering::Relaxed)
    }

    fn store_byte_relaxed(&self, phys: usize, value: u8) {
        self.ring.bytes[phys].store(value, Ordering::Relaxed);
    }

    fn load_head_acquire(&self) -> u64 {
        self.ring.head_abs.load(Ordering::Acquire)
    }

    fn store_head_release(&self, value: u64) {
        self.ring.head_abs.store(value, Ordering::Release);
    }

    fn load_oldest_acquire(&self) -> u64 {
        self.ring.oldest_abs.load(Ordering::Acquire)
    }

    fn load_oldest_relaxed(&self) -> u64 {
        self.ring.oldest_abs.load(Ordering::Relaxed)
    }

    fn store_oldest_relaxed(&self, value: u64) {
        self.ring.oldest_abs.store(value, Ordering::Relaxed);
    }

    fn fetch_add_lost_release(&self, value: u64) {
        self.ring.lost_count.fetch_add(value, Ordering::Release);
    }

    fn load_lost_acquire(&self) -> u64 {
        self.ring.lost_count.load(Ordering::Acquire)
    }

    fn read_control_snapshot(&self) -> Result<ControlBlockSnapshot, ControlBlockError> {
        Ok(ControlBlockSnapshot {
            version: 2,
            caps: 0,
            buffer_id: DEFAULT_BUFFER_ID,
            buffer_bytes: u32::try_from(self.capacity()).unwrap_or(u32::MAX),
            head_abs: self.load_head_acquire(),
            oldest_abs: self.load_oldest_acquire(),
            lost_count: self.load_lost_acquire(),
            run_id: 0,
            epoch_id: 0,
            epoch_first_abs: 0,
            definition_hash: [0; 8],
            prev_definition_hash: [0; 8],
        })
    }
}

fn physical_offset<S: ConcurrentStore>(store: &S, abs: u64) -> usize {
    (abs % store.capacity() as u64) as usize
}

fn read_raw_from_store<S: ConcurrentStore>(
    store: &S,
    cursor_abs: u64,
) -> Result<ReadBatch, RingError> {
    let snapshot = store
        .read_control_snapshot()
        .map_err(RingError::ControlBlock)?;
    let snapshot_bytes = usize::try_from(snapshot.buffer_bytes).unwrap_or(usize::MAX);
    if snapshot_bytes != store.capacity() {
        return Err(RingError::ControlBlockCapacityMismatch {
            snapshot_bytes: snapshot.buffer_bytes,
            capacity: store.capacity(),
        });
    }

    let first_oldest = snapshot.oldest_abs;
    let head_abs = snapshot.head_abs;
    let lapped = cursor_abs < first_oldest;
    let mut abs = if lapped { first_oldest } else { cursor_abs };
    let mut records = Vec::new();

    while abs < head_abs {
        if first_oldest > abs {
            return Err(RingError::OverwrittenMidRead {
                read_abs: abs,
                oldest_abs: first_oldest,
            });
        }

        let phys = physical_offset(store, abs);
        let first = store.load_byte_relaxed(phys);
        if first == 0 {
            if phys == 0 {
                return Err(RingError::UnexpectedWrapMarker { abs });
            }
            abs += (store.capacity() - phys) as u64;
            continue;
        }

        if phys + HEADER_LEN > store.capacity() {
            return Err(RingError::RecordCrossesBoundary { abs });
        }
        let sync = [
            store.load_byte_relaxed(phys),
            store.load_byte_relaxed(phys + 1),
            store.load_byte_relaxed(phys + 2),
            store.load_byte_relaxed(phys + 3),
        ];
        if sync != SYNC {
            return Err(WireError::WrongSync.into());
        }

        let total_len = u16::from_le_bytes([
            store.load_byte_relaxed(phys + 4),
            store.load_byte_relaxed(phys + 5),
        ]) as usize;
        if total_len < HEADER_LEN {
            return Err(WireError::InvalidLength {
                total_len,
                available: store.capacity() - phys,
            }
            .into());
        }
        if phys + total_len > store.capacity() {
            return Err(RingError::RecordCrossesBoundary { abs });
        }

        let mut bytes = Vec::with_capacity(total_len);
        for i in 0..total_len {
            bytes.push(store.load_byte_relaxed(phys + i));
        }

        // Pairs with the producer's release fence before clobbering reclaimed bytes.
        store.acquire_before_recheck();
        let oldest_after = store
            .read_oldest_after_record()
            .map_err(RingError::ControlBlock)?;
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

    let final_oldest = store.load_oldest_acquire();
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

/// Single writer for a concurrent store: evicts (with a release fence) before clobber,
/// then publishes the new head with `Release`.
#[derive(Debug)]
pub struct ConcurrentProducer<S = OwnedConcurrentStore>
where
    S: ConcurrentStore,
{
    store: S,
    spans: VecDeque<(u64, u64)>,
    head_abs: u64,
}

impl ConcurrentProducer<OwnedConcurrentStore> {
    /// Creates a producer bound to `ring`.
    pub fn new(ring: Arc<ConcurrentRing>) -> Self {
        Self::with_store(OwnedConcurrentStore::new(ring))
    }
}

impl<S> ConcurrentProducer<S>
where
    S: ConcurrentStore,
{
    /// Creates a producer bound to any concurrent store.
    pub fn with_store(store: S) -> Self {
        Self {
            store,
            spans: VecDeque::new(),
            head_abs: 0,
        }
    }

    /// Returns the underlying store.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Computes the absolute start offset the next encoded record of `encoded_len` bytes will use.
    ///
    /// This is the same wrap-boundary calculation used by [`write_record`](Self::write_record).
    /// Callers that need to embed the physical/absolute write position in a record should query this
    /// after encoding a fixed-size placeholder record and before calling `write_record`.
    pub fn next_start_abs(&self, encoded_len: usize) -> u64 {
        let capacity = self.store.capacity();
        let start_phys = physical_offset(&self.store, self.head_abs);
        if start_phys == 0 || start_phys + encoded_len <= capacity {
            self.head_abs
        } else {
            self.head_abs + (capacity - start_phys) as u64
        }
    }

    /// Encodes and publishes `record`, reclaiming the oldest bytes if needed.
    ///
    /// Ordering: advance `oldest_abs` and release-fence before clobbering reclaimed
    /// bytes, write the record bytes, then `Release`-store the new `head_abs`.
    pub fn write_record(&mut self, record: &Record) -> Result<(), RingError> {
        let encoded = record.encode(true)?;
        let capacity = self.store.capacity();
        if encoded.len() > capacity {
            return Err(RingError::RecordExceedsCapacity {
                record_len: encoded.len(),
                capacity,
            });
        }
        self.append_bytes_core(&encoded);
        Ok(())
    }

    /// Publishes one already-encoded OpenOT record, reclaiming the oldest bytes if needed.
    ///
    /// The encoded slice must contain exactly one CRC-protected record. All validation
    /// completes before the shared append core mutates the ring or control block.
    pub fn append_encoded(&mut self, bytes: &[u8]) -> Result<(), RingError> {
        if bytes.len() < HEADER_LEN {
            return match validate_record(bytes) {
                Err(error) => Err(error.into()),
                Ok(_) => unreachable!("validate_record accepted bytes shorter than HEADER_LEN"),
            };
        }

        let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
        if flags & FLAG_HAS_CRC == 0 {
            return Err(RingError::EncodedRecordMissingCrc { flags });
        }

        let consumed = validate_record(bytes)?;
        if consumed != bytes.len() {
            return Err(RingError::EncodedRecordLengthMismatch {
                declared_len: consumed,
                actual_len: bytes.len(),
            });
        }

        let capacity = self.store.capacity();
        if bytes.len() > capacity {
            return Err(RingError::RecordExceedsCapacity {
                record_len: bytes.len(),
                capacity,
            });
        }

        self.append_bytes_core(bytes);
        Ok(())
    }

    fn append_bytes_core(&mut self, bytes: &[u8]) {
        debug_assert!(bytes.len() <= self.store.capacity());
        let start_phys = physical_offset(&self.store, self.head_abs);
        let record_start_abs = self.next_start_abs(bytes.len());
        let final_head = record_start_abs + bytes.len() as u64;

        self.store.begin_control_update();
        self.evict_before_clobber(final_head, record_start_abs);

        if record_start_abs != self.head_abs {
            self.store.store_byte_relaxed(start_phys, 0);
        }

        let record_start = physical_offset(&self.store, record_start_abs);
        for (i, byte) in bytes.iter().enumerate() {
            self.store.store_byte_relaxed(record_start + i, *byte);
        }

        self.spans.push_back((record_start_abs, final_head));
        self.head_abs = final_head;
        self.store.store_head_release(final_head);
        self.store.end_control_update_release();
    }

    fn evict_before_clobber(&mut self, final_head: u64, next_record_start: u64) {
        let retention_floor = final_head.saturating_sub(self.store.capacity() as u64);
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
            self.store.fetch_add_lost_release(evicted);
        }
        let new_oldest = self
            .spans
            .front()
            .map_or(next_record_start, |(start_abs, _)| *start_abs);
        self.store.store_oldest_relaxed(new_oldest);
        self.store.release_before_clobber();
    }
}

/// Reads a concurrent store by walking raw bytes with the acquire-fenced overwrite
/// re-check, recovering by lapping to `oldest_abs` when it falls behind.
#[derive(Debug)]
pub struct ConcurrentRawConsumer<S = OwnedConcurrentStore>
where
    S: ConcurrentStore,
{
    store: S,
    cursor_abs: u64,
    lapped_batches: u64,
    overwritten_retries: u64,
    rejected_records: u64,
}

impl ConcurrentRawConsumer<OwnedConcurrentStore> {
    /// Creates a consumer bound to `ring`.
    pub fn new(ring: Arc<ConcurrentRing>) -> Self {
        Self::with_store(OwnedConcurrentStore::new(ring))
    }
}

impl<S> ConcurrentRawConsumer<S>
where
    S: ConcurrentStore,
{
    /// Creates a consumer bound to any concurrent store.
    pub fn with_store(store: S) -> Self {
        Self {
            store,
            cursor_abs: 0,
            lapped_batches: 0,
            overwritten_retries: 0,
            rejected_records: 0,
        }
    }

    /// Returns the underlying store.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Reads available records, recovering from overwrite and decode faults by lapping
    /// the cursor to `oldest_abs` and counting the event.
    pub fn poll(&mut self) -> Result<ReadBatch, RingError> {
        match read_raw_from_store(&self.store, self.cursor_abs) {
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
            Err(RingError::ControlBlock(_)) => {
                self.overwritten_retries += 1;
                Ok(ReadBatch {
                    records: Vec::new(),
                    next_abs: self.cursor_abs,
                    lapped: false,
                })
            }
            Err(error @ RingError::Wire(_)) => {
                let oldest = self.store.load_oldest_acquire();
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

    /// Acquire-loads the committed head from the underlying store.
    pub fn head_abs(&self) -> u64 {
        self.store.load_head_acquire()
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
    use std::sync::atomic::{
        AtomicBool, AtomicU8 as StdAtomicU8, AtomicU64 as StdAtomicU64, Ordering, fence,
    };
    use std::thread;

    use super::*;
    use crate::consumer::LossAccountingConsumer;
    use crate::registry::TY_UINT;
    use crate::wire::Slot;

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
                    && consumer.cursor_abs() == consumer.head_abs()
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

    #[test]
    fn owned_store_protocol_uses_fence_hooks() {
        let ring = ConcurrentRing::new(256);
        let store = InstrumentedStore::new(OwnedConcurrentStore::new(Arc::clone(&ring)));
        let mut producer = ConcurrentProducer::with_store(store.clone());
        let mut consumer = ConcurrentRawConsumer::with_store(store.clone());

        producer.write_record(&minimal_record(41, 0)).unwrap();
        let batch = consumer.poll().unwrap();

        assert_eq!(batch.records.len(), 1);
        assert_eq!(batch.records[0].record.source_id, 41);
        assert_eq!(store.release_fences(), 1);
        assert_eq!(store.acquire_fences(), 1);
        assert_eq!(ring.head_abs(), consumer.cursor_abs());
    }

    #[test]
    fn next_start_abs_matches_written_record_start_across_wraps() {
        let ring = ConcurrentRing::new(100);
        let mut producer = ConcurrentProducer::new(Arc::clone(&ring));
        let mut consumer = ConcurrentRawConsumer::new(Arc::clone(&ring));
        let mut starts = Vec::new();

        for seq in 0..6 {
            let record = minimal_record(42, seq);
            let encoded_len = record.encode(true).unwrap().len();
            let expected_start = producer.next_start_abs(encoded_len);
            producer.write_record(&record).unwrap();

            let batch = consumer.poll().unwrap();
            let delivered = batch
                .records
                .iter()
                .find(|read| read.record.seq == seq)
                .expect("new record delivered");
            assert_eq!(delivered.start_abs, expected_start);
            starts.push(delivered.start_abs);
        }

        assert_eq!(starts, vec![0, 44, 100, 144, 200, 244]);
    }

    #[test]
    fn append_encoded_matches_write_record_across_wrap() {
        let write_ring = ConcurrentRing::new(120);
        let encoded_ring = ConcurrentRing::new(120);
        let mut write_producer = ConcurrentProducer::new(Arc::clone(&write_ring));
        let mut encoded_producer = ConcurrentProducer::new(Arc::clone(&encoded_ring));
        let records = [
            minimal_record(61, 0),
            minimal_record(61, 1),
            record_with_uint_slot(61, 2),
            minimal_record(61, 3),
        ];

        for record in &records {
            write_producer.write_record(record).unwrap();
            let encoded = record.encode(true).unwrap();
            encoded_producer.append_encoded(&encoded).unwrap();
        }

        assert_eq!(ring_bytes(&write_ring), ring_bytes(&encoded_ring));
        assert_eq!(write_ring.head_abs(), encoded_ring.head_abs());
        assert_eq!(write_ring.oldest_abs(), encoded_ring.oldest_abs());
        assert_eq!(write_ring.lost_count(), encoded_ring.lost_count());
        assert!(write_ring.head_abs() > write_ring.capacity() as u64);
    }

    #[test]
    fn append_encoded_rejects_malformed_inputs_without_mutating_ring() {
        let valid = minimal_record(62, 0).encode(true).unwrap();

        let missing_crc = minimal_record(62, 1).encode(false).unwrap();
        assert_rejects_without_mutation(
            &missing_crc,
            128,
            |error| matches!(error, RingError::EncodedRecordMissingCrc { flags } if flags & FLAG_HAS_CRC == 0),
        );

        let mut trailing = valid.clone();
        trailing.push(0);
        assert_rejects_without_mutation(&trailing, 128, |error| {
            matches!(
                error,
                RingError::EncodedRecordLengthMismatch {
                    declared_len,
                    actual_len,
                } if *declared_len == valid.len() && *actual_len == valid.len() + 1
            )
        });

        let mut bad_sync = valid.clone();
        bad_sync[0] = b'X';
        assert_rejects_without_mutation(&bad_sync, 128, |error| {
            matches!(error, RingError::Wire(WireError::WrongSync))
        });

        let truncated = valid[..valid.len() - 1].to_vec();
        assert_rejects_without_mutation(&truncated, 128, |error| {
            matches!(error, RingError::Wire(WireError::Truncated { .. }))
        });

        let mut bad_crc = valid.clone();
        bad_crc[HEADER_LEN] ^= 0x55;
        assert_rejects_without_mutation(&bad_crc, 128, |error| {
            matches!(error, RingError::Wire(WireError::CrcMismatch { .. }))
        });

        let oversized = record_with_large_slot(62, 2).encode(true).unwrap();
        assert!(oversized.len() > 64);
        assert_rejects_without_mutation(&oversized, 64, |error| {
            matches!(
                error,
                RingError::RecordExceedsCapacity {
                    record_len,
                    capacity: 64,
                } if *record_len == oversized.len()
            )
        });
    }

    #[test]
    fn consumer_uses_snapshot_head_and_fresh_oldest_recheck() {
        let record = minimal_record(51, 0);
        let encoded = record.encode(true).unwrap();
        let store = SnapshotOnlyStore::new(128, &encoded, encoded.len() as u64);
        let mut consumer = ConcurrentRawConsumer::with_store(store.clone());

        let batch = consumer.poll().unwrap();

        assert!(batch.records.is_empty());
        assert_eq!(consumer.cursor_abs(), encoded.len() as u64);
        assert_eq!(store.snapshot_reads(), 1);
        assert_eq!(store.head_loads(), 0);
        assert_eq!(store.post_record_oldest_reads(), 1);
    }

    fn assert_rejects_without_mutation(
        bytes: &[u8],
        capacity: usize,
        matches_expected: impl FnOnce(&RingError) -> bool,
    ) {
        let ring = ConcurrentRing::new(capacity);
        let mut producer = ConcurrentProducer::new(Arc::clone(&ring));
        producer.write_record(&minimal_record(90, 0)).unwrap();
        let before_bytes = ring_bytes(&ring);
        let before_head = ring.head_abs();
        let before_oldest = ring.oldest_abs();
        let before_lost = ring.lost_count();

        let error = producer
            .append_encoded(bytes)
            .expect_err("malformed append must fail");

        assert!(matches_expected(&error), "unexpected error: {error:?}");
        assert_eq!(ring_bytes(&ring), before_bytes);
        assert_eq!(ring.head_abs(), before_head);
        assert_eq!(ring.oldest_abs(), before_oldest);
        assert_eq!(ring.lost_count(), before_lost);
    }

    fn ring_bytes(ring: &ConcurrentRing) -> Vec<u8> {
        (0..ring.capacity())
            .map(|phys| ring.bytes[phys].load(Ordering::Acquire))
            .collect()
    }

    fn record_with_uint_slot(source_id: u32, seq: u64) -> Record {
        let mut record = minimal_record(source_id, seq);
        record
            .slots
            .push(Slot::new(0x1000, TY_UINT, 0x2222u16.to_le_bytes()));
        record
    }

    fn record_with_large_slot(source_id: u32, seq: u64) -> Record {
        let mut record = minimal_record(source_id, seq);
        record.slots.push(Slot::new(0x1000, TY_UINT, [0x55; 24]));
        record
    }

    #[derive(Debug, Clone)]
    struct InstrumentedStore {
        inner: OwnedConcurrentStore,
        release_fences: Arc<StdAtomicU64>,
        acquire_fences: Arc<StdAtomicU64>,
    }

    impl InstrumentedStore {
        fn new(inner: OwnedConcurrentStore) -> Self {
            Self {
                inner,
                release_fences: Arc::new(StdAtomicU64::new(0)),
                acquire_fences: Arc::new(StdAtomicU64::new(0)),
            }
        }

        fn release_fences(&self) -> u64 {
            self.release_fences.load(Ordering::Acquire)
        }

        fn acquire_fences(&self) -> u64 {
            self.acquire_fences.load(Ordering::Acquire)
        }
    }

    impl ConcurrentStore for InstrumentedStore {
        fn capacity(&self) -> usize {
            self.inner.capacity()
        }

        fn load_byte_relaxed(&self, phys: usize) -> u8 {
            self.inner.load_byte_relaxed(phys)
        }

        fn store_byte_relaxed(&self, phys: usize, value: u8) {
            self.inner.store_byte_relaxed(phys, value);
        }

        fn load_head_acquire(&self) -> u64 {
            self.inner.load_head_acquire()
        }

        fn store_head_release(&self, value: u64) {
            self.inner.store_head_release(value);
        }

        fn load_oldest_acquire(&self) -> u64 {
            self.inner.load_oldest_acquire()
        }

        fn load_oldest_relaxed(&self) -> u64 {
            self.inner.load_oldest_relaxed()
        }

        fn store_oldest_relaxed(&self, value: u64) {
            self.inner.store_oldest_relaxed(value);
        }

        fn fetch_add_lost_release(&self, value: u64) {
            self.inner.fetch_add_lost_release(value);
        }

        fn load_lost_acquire(&self) -> u64 {
            self.inner.load_lost_acquire()
        }

        fn read_control_snapshot(&self) -> Result<ControlBlockSnapshot, ControlBlockError> {
            self.inner.read_control_snapshot()
        }

        fn read_oldest_after_record(&self) -> Result<u64, ControlBlockError> {
            self.inner.read_oldest_after_record()
        }

        fn release_before_clobber(&self) {
            self.release_fences.fetch_add(1, Ordering::Release);
            fence(Ordering::Release);
        }

        fn acquire_before_recheck(&self) {
            fence(Ordering::Acquire);
            self.acquire_fences.fetch_add(1, Ordering::Release);
        }
    }

    #[derive(Debug, Clone)]
    struct SnapshotOnlyStore {
        bytes: Arc<Vec<StdAtomicU8>>,
        capacity: usize,
        snapshot_head: u64,
        post_record_oldest: u64,
        snapshot_reads: Arc<StdAtomicU64>,
        head_loads: Arc<StdAtomicU64>,
        post_record_oldest_reads: Arc<StdAtomicU64>,
    }

    impl SnapshotOnlyStore {
        fn new(capacity: usize, encoded: &[u8], post_record_oldest: u64) -> Self {
            let bytes = (0..capacity)
                .map(|index| StdAtomicU8::new(encoded.get(index).copied().unwrap_or(0)))
                .collect();
            Self {
                bytes: Arc::new(bytes),
                capacity,
                snapshot_head: encoded.len() as u64,
                post_record_oldest,
                snapshot_reads: Arc::new(StdAtomicU64::new(0)),
                head_loads: Arc::new(StdAtomicU64::new(0)),
                post_record_oldest_reads: Arc::new(StdAtomicU64::new(0)),
            }
        }

        fn snapshot_reads(&self) -> u64 {
            self.snapshot_reads.load(Ordering::Acquire)
        }

        fn head_loads(&self) -> u64 {
            self.head_loads.load(Ordering::Acquire)
        }

        fn post_record_oldest_reads(&self) -> u64 {
            self.post_record_oldest_reads.load(Ordering::Acquire)
        }
    }

    impl ConcurrentStore for SnapshotOnlyStore {
        fn capacity(&self) -> usize {
            self.capacity
        }

        fn load_byte_relaxed(&self, phys: usize) -> u8 {
            self.bytes[phys].load(Ordering::Relaxed)
        }

        fn store_byte_relaxed(&self, phys: usize, value: u8) {
            self.bytes[phys].store(value, Ordering::Relaxed);
        }

        fn load_head_acquire(&self) -> u64 {
            self.head_loads.fetch_add(1, Ordering::Release);
            0
        }

        fn store_head_release(&self, _value: u64) {}

        fn load_oldest_acquire(&self) -> u64 {
            0
        }

        fn load_oldest_relaxed(&self) -> u64 {
            0
        }

        fn store_oldest_relaxed(&self, _value: u64) {}

        fn fetch_add_lost_release(&self, _value: u64) {}

        fn load_lost_acquire(&self) -> u64 {
            0
        }

        fn read_control_snapshot(&self) -> Result<ControlBlockSnapshot, ControlBlockError> {
            self.snapshot_reads.fetch_add(1, Ordering::Release);
            Ok(ControlBlockSnapshot {
                version: 2,
                caps: 0,
                buffer_id: DEFAULT_BUFFER_ID,
                buffer_bytes: u32::try_from(self.capacity).unwrap(),
                head_abs: self.snapshot_head,
                oldest_abs: 0,
                lost_count: 0,
                run_id: 0,
                epoch_id: 0,
                epoch_first_abs: 0,
                definition_hash: [0; 8],
                prev_definition_hash: [0; 8],
            })
        }

        fn read_oldest_after_record(&self) -> Result<u64, ControlBlockError> {
            self.post_record_oldest_reads
                .fetch_add(1, Ordering::Release);
            Ok(self.post_record_oldest)
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
