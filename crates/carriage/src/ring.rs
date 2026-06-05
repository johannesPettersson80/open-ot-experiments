//! Single-buffer byte-pool ring storage.
//!
//! [`RingBuffer`] is the controller-side store: a fixed byte pool addressed by
//! ever-increasing *absolute* offsets. `head_abs` is the next write position and
//! `oldest_abs` is the oldest byte still retained; a physical index is `abs % capacity`.
//! Overwrite is detected by absolute byte range, not by sequence number, because
//! physical reclamation is about bytes, not logical records.
//!
//! Writing past capacity reclaims the oldest bytes (drop-oldest). Evicted records are
//! remembered as [`LossRange`]s so the producer can later emit an authoritative
//! `RecordsDropped` record — see [`crate::loss`].

use std::collections::VecDeque;

use crate::control::{ControlBlockError, ControlBlockSnapshot};
use crate::wire::{HEADER_LEN, Record, SYNC, WireError, decode};

/// Default logical buffer identifier used when a single ring is in play.
pub const DEFAULT_BUFFER_ID: u32 = 1;

/// One record returned by a read, tagged with its absolute byte span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadRecord {
    /// Absolute offset of the record's first byte.
    pub start_abs: u64,
    /// Absolute offset one past the record's last byte.
    pub end_abs: u64,
    /// The decoded record.
    pub record: Record,
}

/// The result of reading from a cursor: the records found plus where to resume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadBatch {
    /// Records delivered in absolute-offset order.
    pub records: Vec<ReadRecord>,
    /// Absolute offset to resume from next time (the head observed at read time).
    pub next_abs: u64,
    /// True if the cursor had fallen behind `oldest_abs` and was fast-forwarded.
    pub lapped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordIndex {
    start_abs: u64,
    end_abs: u64,
    run_id: u64,
    source_id: u32,
    seq: u64,
}

/// A contiguous run of records the producer dropped (evicted) for one source.
///
/// Produced by [`RingBuffer::take_producer_loss_ranges`] and turned into an
/// authoritative `RecordsDropped` record by [`crate::loss::records_dropped_record`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossRange {
    /// Run the dropped records belonged to.
    pub run_id: u64,
    /// Source the dropped records belonged to.
    pub source_id: u32,
    /// First dropped sequence number (inclusive).
    pub first_seq: u64,
    /// Last dropped sequence number (inclusive).
    pub last_seq: u64,
}

impl LossRange {
    /// Number of records in the inclusive `[first_seq, last_seq]` range.
    pub fn count(&self) -> u64 {
        self.last_seq - self.first_seq + 1
    }
}

/// A single-writer ring buffer over a fixed byte pool.
///
/// Reads come in two flavours. [`read_from`](RingBuffer::read_from) uses the in-memory
/// index and is for in-process consumers. [`read_raw_from`](RingBuffer::read_raw_from)
/// walks the raw bytes the way an external translator must: following wrap markers,
/// checking `Sync`/length, validating CRC, and re-checking `oldest_abs` to reject a
/// record that was overwritten mid-read.
#[derive(Debug, Clone)]
pub struct RingBuffer {
    bytes: Vec<u8>,
    head_abs: u64,
    oldest_abs: u64,
    lost_count: u64,
    wraps: u64,
    index: VecDeque<RecordIndex>,
    producer_loss_ranges: Vec<LossRange>,
}

impl RingBuffer {
    /// Creates a ring with a fixed byte `capacity`. Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "ring capacity must be non-zero");
        Self {
            bytes: vec![0; capacity],
            head_abs: 0,
            oldest_abs: 0,
            lost_count: 0,
            wraps: 0,
            index: VecDeque::new(),
            producer_loss_ranges: Vec::new(),
        }
    }

    /// Builds a read-only ring view from a captured physical byte image and control fields.
    ///
    /// The in-memory producer index is intentionally empty. Consumers that validate a
    /// captured image must use [`read_raw_from`](Self::read_raw_from) or
    /// [`read_raw_from_snapshot`](Self::read_raw_from_snapshot), which walk the raw bytes
    /// and do not depend on the producer's private index.
    pub fn from_captured(
        bytes: Vec<u8>,
        head_abs: u64,
        oldest_abs: u64,
        lost_count: u64,
    ) -> Result<Self, RingError> {
        let capacity = bytes.len();
        if capacity == 0 {
            return Err(RingError::CapturedCapacityZero);
        }
        if oldest_abs > head_abs {
            return Err(RingError::InvalidCapturedWindow {
                head_abs,
                oldest_abs,
                capacity,
            });
        }
        if head_abs - oldest_abs > capacity as u64 {
            return Err(RingError::InvalidCapturedWindow {
                head_abs,
                oldest_abs,
                capacity,
            });
        }

        Ok(Self {
            bytes,
            head_abs,
            oldest_abs,
            lost_count,
            wraps: 0,
            index: VecDeque::new(),
            producer_loss_ranges: Vec::new(),
        })
    }

    /// Byte capacity of the ring.
    pub fn capacity(&self) -> usize {
        self.bytes.len()
    }

    /// Absolute offset of the next write (total bytes ever written, wrap markers included).
    pub fn head_abs(&self) -> u64 {
        self.head_abs
    }

    /// Absolute offset of the oldest byte still retained.
    pub fn oldest_abs(&self) -> u64 {
        self.oldest_abs
    }

    /// Count of records evicted from this ring.
    pub fn lost_count(&self) -> u64 {
        self.lost_count
    }

    /// Number of times the writer has emitted a wrap marker at a capacity boundary.
    pub fn wraps(&self) -> u64 {
        self.wraps
    }

    /// The raw physical byte pool, for fixtures and inspection.
    pub fn physical_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Drains and returns the producer-side record of everything evicted so far.
    ///
    /// This is the authoritative loss source for a fully-evicted, then-silent source:
    /// feed each range to [`crate::loss::records_dropped_record`].
    pub fn take_producer_loss_ranges(&mut self) -> Vec<LossRange> {
        std::mem::take(&mut self.producer_loss_ranges)
    }

    /// Encodes and appends `record`, reclaiming the oldest bytes if needed.
    ///
    /// Returns the record's `(start_abs, end_abs)`. Errors if the encoded record is
    /// larger than the whole ring.
    pub fn write_record(&mut self, record: &Record) -> Result<(u64, u64), RingError> {
        let encoded = record.encode(true)?;
        if encoded.len() > self.capacity() {
            return Err(RingError::RecordExceedsCapacity {
                record_len: encoded.len(),
                capacity: self.capacity(),
            });
        }

        let start_phys = self.physical_offset(self.head_abs);
        let start_abs = if start_phys == 0 || start_phys + encoded.len() <= self.capacity() {
            self.head_abs
        } else {
            self.head_abs + (self.capacity() - start_phys) as u64
        };
        let end_abs = start_abs + encoded.len() as u64;

        self.evict_before_clobber(end_abs, start_abs);

        if start_abs != self.head_abs {
            self.bytes[start_phys] = 0;
            self.wraps += 1;
        }
        let start = self.physical_offset(start_abs);
        let end = start + encoded.len();
        debug_assert!(end <= self.capacity());
        self.bytes[start..end].copy_from_slice(&encoded);
        self.head_abs = end_abs;

        self.index.push_back(RecordIndex {
            start_abs,
            end_abs,
            run_id: record.run_id,
            source_id: record.source_id,
            seq: record.seq,
        });
        Ok((start_abs, end_abs))
    }

    /// Reads records at or after `cursor_abs` using the in-memory index.
    ///
    /// A cursor older than `oldest_abs` is fast-forwarded and the batch is flagged
    /// `lapped`.
    pub fn read_from(&self, cursor_abs: u64) -> Result<ReadBatch, RingError> {
        let lapped = cursor_abs < self.oldest_abs;
        let effective_cursor = if lapped { self.oldest_abs } else { cursor_abs };

        let mut records = Vec::new();
        for idx in self
            .index
            .iter()
            .filter(|idx| idx.start_abs >= effective_cursor)
        {
            let start = self.physical_offset(idx.start_abs);
            let len = (idx.end_abs - idx.start_abs) as usize;
            let decoded = decode(&self.bytes[start..start + len])?;
            debug_assert_eq!(decoded.record.run_id, idx.run_id);
            debug_assert_eq!(decoded.record.source_id, idx.source_id);
            debug_assert_eq!(decoded.record.seq, idx.seq);
            records.push(ReadRecord {
                start_abs: idx.start_abs,
                end_abs: idx.end_abs,
                record: decoded.record,
            });
        }

        Ok(ReadBatch {
            records,
            next_abs: self.head_abs,
            lapped,
        })
    }

    /// Reads records at or after `cursor_abs` by walking raw bytes, as a translator must.
    ///
    /// Follows wrap markers (a single `0x00` where a `Sync` byte would be), validates
    /// `Sync`, `TotalRecordLength`, and CRC, and re-checks `oldest_abs` so a record
    /// reclaimed during the walk is rejected as [`RingError::OverwrittenMidRead`].
    pub fn read_raw_from(&self, cursor_abs: u64) -> Result<ReadBatch, RingError> {
        self.read_raw_window(cursor_abs, self.head_abs, self.oldest_abs, || {
            self.oldest_abs
        })
    }

    /// Reads raw bytes using `snapshot` as the source of `HeadAbs`/`OldestAbs`.
    ///
    /// This is the shared-memory consumer shape: the parser is identical to
    /// [`read_raw_from`](Self::read_raw_from), but the visible range comes from a
    /// coherent control-block snapshot instead of the in-process ring fields.
    pub fn read_raw_from_snapshot(
        &self,
        cursor_abs: u64,
        snapshot: &ControlBlockSnapshot,
    ) -> Result<ReadBatch, RingError> {
        let snapshot_bytes = usize::try_from(snapshot.buffer_bytes).unwrap_or(usize::MAX);
        if snapshot_bytes != self.capacity() {
            return Err(RingError::ControlBlockCapacityMismatch {
                snapshot_bytes: snapshot.buffer_bytes,
                capacity: self.capacity(),
            });
        }

        self.read_raw_window(cursor_abs, snapshot.head_abs, snapshot.oldest_abs, || {
            snapshot.oldest_abs
        })
    }

    fn read_raw_window<F>(
        &self,
        cursor_abs: u64,
        head_abs: u64,
        first_oldest: u64,
        mut oldest_after_read: F,
    ) -> Result<ReadBatch, RingError>
    where
        F: FnMut() -> u64,
    {
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

            let phys = self.physical_offset(abs);
            let first = self.bytes[phys];
            if first == 0 {
                if phys == 0 {
                    return Err(RingError::UnexpectedWrapMarker { abs });
                }
                abs += (self.capacity() - phys) as u64;
                continue;
            }

            if phys + HEADER_LEN > self.capacity() {
                return Err(RingError::RecordCrossesBoundary { abs });
            }
            if self.bytes[phys..phys + 4] != SYNC {
                return Err(WireError::WrongSync.into());
            }

            let total_len = read_le_u16(&self.bytes, phys + 4) as usize;
            if total_len < HEADER_LEN {
                return Err(WireError::InvalidLength {
                    total_len,
                    available: self.capacity() - phys,
                }
                .into());
            }
            if phys + total_len > self.capacity() {
                return Err(RingError::RecordCrossesBoundary { abs });
            }

            let decoded = decode(&self.bytes[phys..phys + total_len])?;
            let oldest_after = oldest_after_read();
            if oldest_after > abs {
                return Err(RingError::OverwrittenMidRead {
                    read_abs: abs,
                    oldest_abs: oldest_after,
                });
            }

            records.push(ReadRecord {
                start_abs: abs,
                end_abs: abs + decoded.consumed as u64,
                record: decoded.record,
            });
            abs += total_len as u64;
        }

        let final_oldest = oldest_after_read();
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

    fn evict_before_clobber(&mut self, final_head: u64, next_record_start: u64) {
        let retention_floor = final_head.saturating_sub(self.capacity() as u64);
        let mut evicted_count = 0u64;
        while self
            .index
            .front()
            .is_some_and(|idx| idx.start_abs < retention_floor)
        {
            let evicted = self.index.pop_front().expect("front checked above");
            self.record_producer_loss(evicted.run_id, evicted.source_id, evicted.seq);
            evicted_count += 1;
        }
        self.lost_count += evicted_count;
        self.oldest_abs = self
            .index
            .front()
            .map_or(next_record_start, |idx| idx.start_abs);
    }

    fn record_producer_loss(&mut self, run_id: u64, source_id: u32, seq: u64) {
        if let Some(last) = self.producer_loss_ranges.last_mut()
            && last.run_id == run_id
            && last.source_id == source_id
            && last.last_seq + 1 == seq
        {
            last.last_seq = seq;
            return;
        }
        self.producer_loss_ranges.push(LossRange {
            run_id,
            source_id,
            first_seq: seq,
            last_seq: seq,
        });
    }

    fn physical_offset(&self, abs: u64) -> usize {
        (abs % self.capacity() as u64) as usize
    }
}

/// Errors from ring reads and writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RingError {
    /// A record at `read_abs` was reclaimed before the read completed.
    OverwrittenMidRead {
        /// Absolute offset the consumer was reading.
        read_abs: u64,
        /// Absolute oldest retained offset observed after the read; it had passed `read_abs`.
        oldest_abs: u64,
    },
    /// A record's bytes would cross the physical capacity boundary at `abs`.
    RecordCrossesBoundary {
        /// Absolute offset at which the record would straddle the end of the byte pool.
        abs: u64,
    },
    /// The encoded record is larger than the entire ring.
    RecordExceedsCapacity {
        /// Encoded record length in bytes.
        record_len: usize,
        /// Total ring capacity in bytes.
        capacity: usize,
    },
    /// The control-block snapshot describes a different ring capacity.
    ControlBlockCapacityMismatch {
        /// Capacity advertised by the control-block snapshot.
        snapshot_bytes: u32,
        /// Capacity of the ring being read.
        capacity: usize,
    },
    /// A captured ring image had zero capacity.
    CapturedCapacityZero,
    /// A captured ring image advertised an impossible absolute live window.
    InvalidCapturedWindow {
        /// Published absolute head from the capture.
        head_abs: u64,
        /// Published oldest retained absolute byte from the capture.
        oldest_abs: u64,
        /// Captured physical byte capacity.
        capacity: usize,
    },
    /// A coherent control-block snapshot could not be read.
    ControlBlock(ControlBlockError),
    /// A `0x00` wrap marker appeared at physical offset 0, which is never valid.
    UnexpectedWrapMarker {
        /// Absolute offset at which the stray wrap marker was found.
        abs: u64,
    },
    /// The bytes failed wire-level decoding.
    Wire(WireError),
}

impl From<WireError> for RingError {
    fn from(value: WireError) -> Self {
        Self::Wire(value)
    }
}

fn read_le_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consumer::{LossAccountingConsumer, RawByteConsumer};
    use crate::control::ControlBlockSnapshot;
    use crate::wire::Slot;

    const EVENT_MESSAGE: u32 = 0x0003;
    const TY_UDINT: u8 = 0x05;

    #[test]
    fn ring_keepup_reads_in_order_zero_loss_across_wraps() {
        let mut ring = RingBuffer::new(128);
        let mut consumer = LossAccountingConsumer::new();
        let source = 7;

        let mut delivered_seq = Vec::new();
        for seq in 0..24u64 {
            ring.write_record(&minimal_record(source, seq)).unwrap();
            let batch = consumer.poll(&ring).unwrap();
            delivered_seq.extend(batch.records.into_iter().map(|r| r.record.seq));
        }

        assert!(
            ring.wraps() >= 3,
            "test did not force repeated physical wrap"
        );
        assert!(ring.head_abs() > ring.capacity() as u64 * 3);
        assert_eq!(delivered_seq, (0..24).collect::<Vec<_>>());
        assert_eq!(consumer.delivered(source), 24);
        assert_eq!(consumer.lost(source), 0);
    }

    #[test]
    fn forced_wrap_at_exact_boundary_starts_next_record_at_zero() {
        let mut ring = RingBuffer::new(88);
        let mut raw = RawByteConsumer::new();
        let source = 71;

        let (first_start, first_end) = ring.write_record(&minimal_record(source, 0)).unwrap();
        assert_eq!((first_start, first_end), (0, 44));
        raw.poll(&ring).unwrap();

        let (second_start, second_end) = ring.write_record(&minimal_record(source, 1)).unwrap();
        assert_eq!((second_start, second_end), (44, 88));
        assert_eq!(ring.head_abs() % ring.capacity() as u64, 0);
        raw.poll(&ring).unwrap();

        let (third_start, third_end) = ring.write_record(&minimal_record(source, 2)).unwrap();
        assert_eq!((third_start, third_end), (88, 132));
        assert_eq!(&ring.physical_bytes()[0..4], &SYNC);

        let batch = raw.poll(&ring).unwrap();
        assert!(!batch.lapped);
        assert_eq!(
            batch
                .records
                .iter()
                .map(|read| read.record.seq)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(raw.delivered_in_run(1, source), 3);
        assert_eq!(raw.lost_in_run(1, source), 0);
    }

    #[test]
    fn reconnect_after_overwrite_laps_to_oldest_and_accounts_exact_loss() {
        let mut ring = RingBuffer::new(256);
        let mut raw = RawByteConsumer::new();
        let source = 72;
        let produced = 80u64;

        for seq in 0..produced {
            ring.write_record(&minimal_record(source, seq)).unwrap();
        }

        let batch = raw.poll(&ring).unwrap();
        let retained = batch.records.len() as u64;

        assert!(batch.lapped, "stale reconnect must report physical lap");
        assert!(retained > 0, "test needs retained post-gap records");
        assert!(retained < produced, "test did not overwrite enough records");
        assert_eq!(raw.cursor_abs(), ring.head_abs());
        assert_eq!(raw.delivered_in_run(1, source), retained);
        assert_eq!(
            raw.delivered_in_run(1, source) + raw.lost_in_run(1, source),
            produced
        );

        let first_retained = batch.records.first().unwrap().record.seq;
        assert_eq!(raw.lost_in_run(1, source), first_retained);
        assert_eq!(
            batch
                .records
                .iter()
                .map(|read| read.record.seq)
                .collect::<Vec<_>>(),
            (first_retained..produced).collect::<Vec<_>>()
        );
    }

    #[test]
    fn raw_read_uses_control_snapshot_bounds() {
        let mut ring = RingBuffer::new(256);
        let source = 73;
        let (_, first_end) = ring.write_record(&minimal_record(source, 0)).unwrap();
        let (_, second_end) = ring.write_record(&minimal_record(source, 1)).unwrap();
        ring.write_record(&minimal_record(source, 2)).unwrap();

        let mut snapshot = snapshot_for_ring(&ring);
        snapshot.head_abs = second_end;
        let batch = ring.read_raw_from_snapshot(0, &snapshot).unwrap();
        assert_eq!(batch.next_abs, second_end);
        assert_eq!(
            batch
                .records
                .iter()
                .map(|read| read.record.seq)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );

        snapshot.oldest_abs = first_end;
        let batch = ring.read_raw_from_snapshot(0, &snapshot).unwrap();
        assert!(batch.lapped);
        assert_eq!(
            batch
                .records
                .iter()
                .map(|read| read.record.seq)
                .collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn raw_read_rejects_mismatched_control_snapshot_capacity() {
        let ring = RingBuffer::new(256);
        let mut snapshot = snapshot_for_ring(&ring);
        snapshot.buffer_bytes = 128;
        assert_eq!(
            ring.read_raw_from_snapshot(0, &snapshot),
            Err(RingError::ControlBlockCapacityMismatch {
                snapshot_bytes: 128,
                capacity: 256,
            })
        );
    }

    #[test]
    fn captured_ring_snapshot_walks_raw_bytes_without_index() {
        let mut produced = RingBuffer::new(256);
        produced.write_record(&minimal_record(75, 0)).unwrap();
        produced.write_record(&minimal_record(75, 1)).unwrap();
        let snapshot = snapshot_for_ring(&produced);
        let captured = RingBuffer::from_captured(
            produced.physical_bytes().to_vec(),
            produced.head_abs(),
            produced.oldest_abs(),
            produced.lost_count(),
        )
        .unwrap();

        assert!(
            captured.read_from(0).unwrap().records.is_empty(),
            "captured rings deliberately do not synthesize the producer index"
        );

        let batch = captured.read_raw_from_snapshot(0, &snapshot).unwrap();
        assert_eq!(
            batch
                .records
                .iter()
                .map(|read| read.record.seq)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    #[test]
    fn captured_ring_rejects_invalid_windows() {
        assert_eq!(
            RingBuffer::from_captured(Vec::new(), 0, 0, 0).unwrap_err(),
            RingError::CapturedCapacityZero
        );
        assert_eq!(
            RingBuffer::from_captured(vec![0; 16], 7, 8, 0).unwrap_err(),
            RingError::InvalidCapturedWindow {
                head_abs: 7,
                oldest_abs: 8,
                capacity: 16,
            }
        );
        assert_eq!(
            RingBuffer::from_captured(vec![0; 16], 32, 0, 0).unwrap_err(),
            RingError::InvalidCapturedWindow {
                head_abs: 32,
                oldest_abs: 0,
                capacity: 16,
            }
        );
    }

    #[test]
    fn torn_record_with_valid_sync_and_length_is_rejected_by_crc() {
        let mut ring = RingBuffer::new(128);
        let mut torn_record = minimal_record(73, 0);
        torn_record
            .slots
            .push(Slot::new(0x1000, TY_UDINT, 123u32.to_le_bytes()));
        let bytes = torn_record.encode(true).unwrap();
        assert_eq!(&bytes[0..4], &SYNC);
        assert_eq!(read_le_u16(&bytes, 4) as usize, bytes.len());

        let copied = HEADER_LEN + 4;
        ring.bytes[..copied].copy_from_slice(&bytes[..copied]);
        ring.bytes[copied..bytes.len()].fill(0xA5);
        ring.head_abs = bytes.len() as u64;
        ring.oldest_abs = 0;

        let mut raw = RawByteConsumer::new();
        let error = raw.poll(&ring).unwrap_err();

        assert!(matches!(
            error,
            RingError::Wire(WireError::CrcMismatch { .. })
        ));
        assert_eq!(raw.delivered_in_run(1, 73), 0);
        assert_eq!(raw.lost_in_run(1, 73), 0);
        assert_eq!(raw.cursor_abs(), 0);
    }

    #[test]
    fn clock_step_backward_does_not_reorder_or_drop_seq_stream() {
        let mut ring = RingBuffer::new(256);
        let mut raw = RawByteConsumer::new();
        let source = 74;

        ring.write_record(&record_with_time(source, 0, 1_000))
            .unwrap();
        ring.write_record(&record_with_time(source, 1, 900))
            .unwrap();
        ring.write_record(&record_with_time(source, 2, 950))
            .unwrap();

        let batch = raw.poll(&ring).unwrap();
        assert!(!batch.lapped);
        assert_eq!(
            batch
                .records
                .iter()
                .map(|read| (read.record.seq, read.record.source_time))
                .collect::<Vec<_>>(),
            vec![(0, 1_000), (1, 900), (2, 950)]
        );
        assert_eq!(raw.delivered_in_run(1, source), 3);
        assert_eq!(raw.lost_in_run(1, source), 0);
    }

    fn minimal_record(source_id: u32, seq: u64) -> Record {
        record_with_time(source_id, seq, 1_780_000_000_000_000_000 + seq)
    }

    fn snapshot_for_ring(ring: &RingBuffer) -> ControlBlockSnapshot {
        ControlBlockSnapshot {
            version: 2,
            caps: 0,
            buffer_id: DEFAULT_BUFFER_ID,
            buffer_bytes: ring.capacity() as u32,
            head_abs: ring.head_abs(),
            oldest_abs: ring.oldest_abs(),
            lost_count: ring.lost_count(),
            run_id: 1,
            epoch_id: 1,
            epoch_first_abs: 0,
            definition_hash: [0; 8],
            prev_definition_hash: [0; 8],
        }
    }

    fn record_with_time(source_id: u32, seq: u64, source_time: u64) -> Record {
        Record::new(source_time, 1, seq, source_id, EVENT_MESSAGE)
    }
}
