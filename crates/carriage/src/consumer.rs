//! Reading consumers: read a [`RingBuffer`] and feed loss accounting.
//!
//! [`LossAccountingConsumer`] uses the in-memory index ([`RingBuffer::read_from`]);
//! [`RawByteConsumer`] walks raw bytes ([`RingBuffer::read_raw_from`]) the way an external
//! translator must. Both keep an absolute read cursor and a [`crate::loss`] tracker, and
//! account loss as records are read.
//!
//! [`RingBuffer`]: crate::ring::RingBuffer
//! [`RingBuffer::read_from`]: crate::ring::RingBuffer::read_from
//! [`RingBuffer::read_raw_from`]: crate::ring::RingBuffer::read_raw_from

use crate::control::ControlBlockSnapshot;
use crate::loss::{LossEvent, LossTracker};
use crate::ring::{DEFAULT_BUFFER_ID, ReadBatch, RingBuffer, RingError};

/// Reads through the in-memory index ([`RingBuffer::read_from`]) while accounting loss.
///
/// [`RingBuffer::read_from`]: crate::ring::RingBuffer::read_from
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossAccountingConsumer {
    cursor_abs: u64,
    tracker: LossTracker,
}

impl Default for LossAccountingConsumer {
    fn default() -> Self {
        Self::new()
    }
}

impl LossAccountingConsumer {
    /// Creates a consumer for the [`DEFAULT_BUFFER_ID`] buffer.
    pub fn new() -> Self {
        Self::with_buffer_id(DEFAULT_BUFFER_ID)
    }

    /// Creates a consumer that tags its loss with `buffer_id`.
    pub fn with_buffer_id(buffer_id: u32) -> Self {
        Self {
            cursor_abs: 0,
            tracker: LossTracker::new(buffer_id),
        }
    }

    /// Reads new records from `ring`, accounts them, and advances the cursor.
    pub fn poll(&mut self, ring: &RingBuffer) -> Result<ReadBatch, RingError> {
        let batch = ring.read_from(self.cursor_abs)?;
        self.account_batch(&batch);
        Ok(batch)
    }

    /// Accounts an already-read batch and advances the cursor to its `next_abs`.
    pub fn account_batch(&mut self, batch: &ReadBatch) {
        for read in &batch.records {
            self.tracker.account(&read.record);
        }
        self.cursor_abs = batch.next_abs;
    }

    /// Delivered count for `source_id` in run 1 (convenience for single-run tests).
    pub fn delivered(&self, source_id: u32) -> u64 {
        self.delivered_in_run(1, source_id)
    }

    /// Delivered count for `(run_id, source_id)`.
    pub fn delivered_in_run(&self, run_id: u64, source_id: u32) -> u64 {
        self.tracker.delivered(run_id, source_id)
    }

    /// Lost count for `source_id` in run 1 (convenience for single-run tests).
    pub fn lost(&self, source_id: u32) -> u64 {
        self.lost_in_run(1, source_id)
    }

    /// Lost count for `(run_id, source_id)`.
    pub fn lost_in_run(&self, run_id: u64, source_id: u32) -> u64 {
        self.tracker.lost(run_id, source_id)
    }

    /// All reconciled loss intervals, sorted.
    pub fn loss_events(&self) -> Vec<LossEvent> {
        self.tracker.loss_events()
    }

    /// Current absolute read cursor.
    pub fn cursor_abs(&self) -> u64 {
        self.cursor_abs
    }
}

/// Reads by walking raw bytes ([`RingBuffer::read_raw_from`]), as an external translator
/// must, while accounting loss.
///
/// [`RingBuffer::read_raw_from`]: crate::ring::RingBuffer::read_raw_from
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawByteConsumer {
    cursor_abs: u64,
    tracker: LossTracker,
}

impl Default for RawByteConsumer {
    fn default() -> Self {
        Self::new()
    }
}

impl RawByteConsumer {
    /// Creates a consumer for the [`DEFAULT_BUFFER_ID`] buffer.
    pub fn new() -> Self {
        Self::with_buffer_id(DEFAULT_BUFFER_ID)
    }

    /// Creates a consumer that tags its loss with `buffer_id`.
    pub fn with_buffer_id(buffer_id: u32) -> Self {
        Self {
            cursor_abs: 0,
            tracker: LossTracker::new(buffer_id),
        }
    }

    /// Walks new bytes from `ring`, accounts the decoded records, and advances the cursor.
    pub fn poll(&mut self, ring: &RingBuffer) -> Result<ReadBatch, RingError> {
        let batch = ring.read_raw_from(self.cursor_abs)?;
        self.account_batch(&batch);
        Ok(batch)
    }

    /// Walks new bytes using a coherent control-block snapshot for `HeadAbs`/`OldestAbs`.
    pub fn poll_snapshot(
        &mut self,
        ring: &RingBuffer,
        snapshot: &ControlBlockSnapshot,
    ) -> Result<ReadBatch, RingError> {
        let batch = ring.read_raw_from_snapshot(self.cursor_abs, snapshot)?;
        self.account_batch(&batch);
        Ok(batch)
    }

    fn account_batch(&mut self, batch: &ReadBatch) {
        for read in &batch.records {
            self.tracker.account(&read.record);
        }
        self.cursor_abs = batch.next_abs;
    }

    /// Delivered count for `(run_id, source_id)`.
    pub fn delivered_in_run(&self, run_id: u64, source_id: u32) -> u64 {
        self.tracker.delivered(run_id, source_id)
    }

    /// Lost count for `(run_id, source_id)`.
    pub fn lost_in_run(&self, run_id: u64, source_id: u32) -> u64 {
        self.tracker.lost(run_id, source_id)
    }

    /// All reconciled loss intervals, sorted.
    pub fn loss_events(&self) -> Vec<LossEvent> {
        self.tracker.loss_events()
    }

    /// Current absolute read cursor.
    pub fn cursor_abs(&self) -> u64 {
        self.cursor_abs
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::loss::records_dropped_record;
    use crate::ring::ReadRecord;
    use crate::wire::Record;

    const EVENT_MESSAGE: u32 = 0x0003;

    #[test]
    fn multi_source_lapped_consumer_accounts_loss_from_seq_gaps() {
        let (ring, produced, sources) = lapped_multi_source_ring();
        let total_produced: u64 = produced.values().sum();
        let mut consumer = LossAccountingConsumer::new();
        let batch = consumer.poll(&ring).unwrap();
        let total_delivered = batch.records.len() as u64;

        assert!(batch.lapped, "consumer must be physically lapped");
        assert!(
            ring.head_abs() > ring.capacity() as u64 * 100,
            "test did not force enough absolute-offset progress"
        );
        assert!(ring.wraps() > 100, "test did not physically wrap enough");
        assert!(
            total_delivered * 20 < total_produced,
            "test retained too much data: delivered={total_delivered}, produced={total_produced}"
        );

        for source in sources {
            let delivered = consumer.delivered(source);
            let lost = consumer.lost(source);
            let produced = produced[&source];

            assert!(
                delivered > 0,
                "seq-gap accounting needs a retained post-gap record for source {source}"
            );
            assert_eq!(
                delivered + lost,
                produced,
                "loss attribution failed for source {source}: delivered={delivered}, lost={lost}, produced={produced}"
            );
        }
    }

    #[test]
    fn raw_byte_walker_matches_index_consumer_when_keeping_up() {
        let mut ring = RingBuffer::new(128);
        let mut index_consumer = LossAccountingConsumer::new();
        let mut raw_consumer = RawByteConsumer::new();
        let source = 7;

        for seq in 0..24u64 {
            ring.write_record(&minimal_record(source, seq)).unwrap();
            let index_batch = index_consumer.poll(&ring).unwrap();
            let raw_batch = raw_consumer.poll(&ring).unwrap();
            assert_same_delivery(&index_batch, &raw_batch);
        }

        assert!(ring.wraps() >= 3);
        assert_eq!(raw_consumer.delivered_in_run(1, source), 24);
        assert_eq!(raw_consumer.lost_in_run(1, source), 0);
    }

    #[test]
    fn raw_byte_walker_matches_index_consumer_after_lap() {
        let (ring, produced, sources) = lapped_multi_source_ring();
        let mut index_consumer = LossAccountingConsumer::new();
        let mut raw_consumer = RawByteConsumer::new();

        let index_batch = index_consumer.poll(&ring).unwrap();
        let raw_batch = raw_consumer.poll(&ring).unwrap();

        assert!(raw_batch.lapped);
        assert_same_delivery(&index_batch, &raw_batch);
        for source in sources {
            assert_eq!(
                raw_consumer.delivered_in_run(1, source),
                index_consumer.delivered(source)
            );
            assert_eq!(
                raw_consumer.lost_in_run(1, source),
                index_consumer.lost(source)
            );
            assert_eq!(
                raw_consumer.delivered_in_run(1, source) + raw_consumer.lost_in_run(1, source),
                produced[&source]
            );
        }
    }

    #[test]
    fn fully_evicted_source_needs_authoritative_records_dropped() {
        let silent_source = 88;
        let noisy_source = 99;

        let mut without_authority = RingBuffer::new(256);
        for seq in 0..5 {
            without_authority
                .write_record(&minimal_record(silent_source, seq))
                .unwrap();
        }
        for seq in 0..80 {
            without_authority
                .write_record(&minimal_record(noisy_source, seq))
                .unwrap();
        }

        let mut raw = RawByteConsumer::new();
        let batch = raw.poll(&without_authority).unwrap();
        assert!(batch.lapped);
        assert_eq!(raw.delivered_in_run(1, silent_source), 0);
        assert_eq!(raw.lost_in_run(1, silent_source), 0);

        let mut with_authority = RingBuffer::new(256);
        for seq in 0..5 {
            with_authority
                .write_record(&minimal_record(silent_source, seq))
                .unwrap();
        }
        for seq in 0..80 {
            with_authority
                .write_record(&minimal_record(noisy_source, seq))
                .unwrap();
        }
        let loss = with_authority
            .take_producer_loss_ranges()
            .into_iter()
            .find(|range| range.source_id == silent_source)
            .expect("producer must retain loss metadata for fully evicted source");
        assert_eq!(loss.first_seq, 0);
        assert_eq!(loss.last_seq, 4);

        with_authority
            .write_record(&records_dropped_record(5, &loss))
            .unwrap();

        let mut raw = RawByteConsumer::new();
        raw.poll(&with_authority).unwrap();
        assert_eq!(raw.delivered_in_run(1, silent_source), 1);
        assert_eq!(raw.lost_in_run(1, silent_source), 5);
        let events = raw.loss_events();
        assert_eq!(
            events.len(),
            2,
            "silent source loss + noisy-source leading loss"
        );
        let silent_events = events
            .iter()
            .filter(|event| event.source_id == silent_source)
            .collect::<Vec<_>>();
        assert_eq!(silent_events.len(), 1);
        assert!(!silent_events[0].synthetic);
        assert_eq!(silent_events[0].first_seq, 0);
        assert_eq!(silent_events[0].last_seq, 4);
        assert_eq!(silent_events[0].count, 5);
    }

    fn lapped_multi_source_ring() -> (RingBuffer, BTreeMap<u32, u64>, [u32; 4]) {
        let mut ring = RingBuffer::new(256);
        let mut produced = BTreeMap::<u32, u64>::new();
        let sources = [11u32, 12, 13, 14];
        let per_source = 300u64;

        for seq in 0..per_source {
            for source in sources {
                ring.write_record(&minimal_record(source, seq)).unwrap();
                *produced.entry(source).or_insert(0) += 1;
            }
        }

        (ring, produced, sources)
    }

    fn assert_same_delivery(left: &ReadBatch, right: &ReadBatch) {
        let left_records = left
            .records
            .iter()
            .map(record_fingerprint)
            .collect::<Vec<_>>();
        let right_records = right
            .records
            .iter()
            .map(record_fingerprint)
            .collect::<Vec<_>>();
        assert_eq!(left.lapped, right.lapped);
        assert_eq!(left.next_abs, right.next_abs);
        assert_eq!(left_records, right_records);
    }

    fn record_fingerprint(read: &ReadRecord) -> (u64, u64, u64, u32, u64, u32) {
        (
            read.start_abs,
            read.end_abs,
            read.record.run_id,
            read.record.source_id,
            read.record.seq,
            read.record.event_type_id,
        )
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
