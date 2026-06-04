//! Loss accounting: the model and the three-mechanism reconciliation.
//!
//! Completeness rests on three complementary signals, none sufficient alone:
//!
//! 1. **Seq gaps** — the tracker infers loss from a jump in per-source sequence numbers.
//!    Only fires once a *later* record from that source arrives, so it cannot see a
//!    source that was dropped and then went silent.
//! 2. **Authoritative `RecordsDropped`** — the producer reports known evictions
//!    ([`records_dropped_record`]); the consumer parses them back into [`LossEvent`]s.
//! 3. **Source high-water** — a source-local checkpoint closes the silent tail inline
//!    when the checkpoint record is read.
//!
//! All three feed one interval-union so overlapping ranges merge without double counting.
//! The reading consumers that drive this accounting live in [`crate::consumer`].

use std::collections::BTreeMap;

use crate::registry::{
    EVENT_SOURCE_HIGH_WATER, KEY_DROPPED_COUNT, KEY_FIRST_LOST_SEQ, KEY_LAST_LOST_SEQ,
    KEY_SOURCE_HIGH_WATER, TY_UDINT, TY_ULINT,
};
use crate::ring::LossRange;
use crate::wire::{FLAG_SYNTHETIC, Record, Slot};

pub use crate::registry::EVENT_RECORDS_DROPPED;

/// A reconciled loss interval for one `(buffer, run, source)` stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossEvent {
    /// Buffer the loss belongs to.
    pub buffer_id: u32,
    /// Run the loss belongs to.
    pub run_id: u64,
    /// Source the loss belongs to.
    pub source_id: u32,
    /// First lost sequence number (inclusive).
    pub first_seq: u64,
    /// Last lost sequence number (inclusive).
    pub last_seq: u64,
    /// Number of lost records (`last_seq - first_seq + 1`).
    pub count: u64,
    /// `true` if inferred only from a seq gap; `false` if backed by an authoritative
    /// `RecordsDropped` record or a high-water checkpoint.
    pub synthetic: bool,
}

/// Accumulates per-source delivery counts, high-water marks, and merged loss intervals.
///
/// Crate-internal; the public surface is the consumers in [`crate::consumer`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LossTracker {
    buffer_id: u32,
    next_seq_by_source: BTreeMap<(u64, u32), u64>,
    delivered_by_source: BTreeMap<(u64, u32), u64>,
    loss_events: Vec<LossEvent>,
}

impl LossTracker {
    pub(crate) fn new(buffer_id: u32) -> Self {
        Self {
            buffer_id,
            next_seq_by_source: BTreeMap::new(),
            delivered_by_source: BTreeMap::new(),
            loss_events: Vec::new(),
        }
    }

    pub(crate) fn account(&mut self, record: &Record) {
        let source_key = (record.run_id, record.source_id);
        let expected = self
            .next_seq_by_source
            .get(&source_key)
            .copied()
            .unwrap_or(0);
        if let Some(produced_count) = parse_source_high_water(record)
            && produced_count > expected
        {
            self.insert_loss_event(LossEvent {
                buffer_id: self.buffer_id,
                run_id: record.run_id,
                source_id: record.source_id,
                first_seq: expected,
                last_seq: produced_count - 1,
                count: produced_count - expected,
                synthetic: false,
            });
        }
        if record.seq > expected {
            let first = expected;
            let last = record.seq - 1;
            self.insert_loss_event(LossEvent {
                buffer_id: self.buffer_id,
                run_id: record.run_id,
                source_id: record.source_id,
                first_seq: first,
                last_seq: last,
                count: last - first + 1,
                synthetic: true,
            });
        }
        if record.seq >= expected {
            *self.delivered_by_source.entry(source_key).or_insert(0) += 1;
            self.next_seq_by_source.insert(source_key, record.seq + 1);
        }

        if let Some(authoritative) = parse_records_dropped(record, self.buffer_id) {
            self.insert_loss_event(authoritative);
        }
    }

    pub(crate) fn delivered(&self, run_id: u64, source_id: u32) -> u64 {
        self.delivered_by_source
            .get(&(run_id, source_id))
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn lost(&self, run_id: u64, source_id: u32) -> u64 {
        self.loss_events
            .iter()
            .filter(|event| event.run_id == run_id && event.source_id == source_id)
            .map(|event| event.count)
            .sum()
    }

    pub(crate) fn loss_events(&self) -> Vec<LossEvent> {
        let mut events = self.loss_events.clone();
        events.sort_by_key(|event| {
            (
                event.buffer_id,
                event.run_id,
                event.source_id,
                event.first_seq,
                event.last_seq,
            )
        });
        events
    }

    fn insert_loss_event(&mut self, event: LossEvent) {
        let mut intervals = Vec::new();
        self.loss_events.retain(|existing| {
            if same_loss_stream(existing, &event) && ranges_touch_or_overlap(existing, &event) {
                intervals.push(existing.clone());
                false
            } else {
                true
            }
        });
        intervals.push(event);
        intervals.sort_by_key(|event| event.first_seq);

        let mut merged = Vec::<LossEvent>::new();
        for interval in intervals {
            if let Some(last) = merged.last_mut()
                && ranges_touch_or_overlap(last, &interval)
            {
                last.first_seq = last.first_seq.min(interval.first_seq);
                last.last_seq = last.last_seq.max(interval.last_seq);
                last.count = last.last_seq - last.first_seq + 1;
                last.synthetic = last.synthetic && interval.synthetic;
                continue;
            }
            merged.push(interval);
        }

        self.loss_events.extend(merged);
    }
}

fn same_loss_stream(left: &LossEvent, right: &LossEvent) -> bool {
    left.buffer_id == right.buffer_id
        && left.run_id == right.run_id
        && left.source_id == right.source_id
}

fn ranges_touch_or_overlap(left: &LossEvent, right: &LossEvent) -> bool {
    right.first_seq <= left.last_seq.saturating_add(1)
        && left.first_seq <= right.last_seq.saturating_add(1)
}

/// Builds an authoritative `RecordsDropped` record from a producer [`LossRange`].
pub fn records_dropped_record(seq: u64, range: &LossRange) -> Record {
    let mut record = Record::new(
        1_780_000_000_000_000_000 + seq,
        range.run_id,
        seq,
        range.source_id,
        EVENT_RECORDS_DROPPED,
    );
    record.slots.push(Slot::new(
        KEY_DROPPED_COUNT,
        TY_UDINT,
        (range.count() as u32).to_le_bytes(),
    ));
    record.slots.push(Slot::new(
        KEY_FIRST_LOST_SEQ,
        TY_ULINT,
        range.first_seq.to_le_bytes(),
    ));
    record.slots.push(Slot::new(
        KEY_LAST_LOST_SEQ,
        TY_ULINT,
        range.last_seq.to_le_bytes(),
    ));
    record
}

fn parse_records_dropped(record: &Record, buffer_id: u32) -> Option<LossEvent> {
    if record.event_type_id != EVENT_RECORDS_DROPPED {
        return None;
    }

    let mut count = None;
    let mut first = None;
    let mut last = None;
    for slot in &record.slots {
        match slot.key {
            KEY_DROPPED_COUNT if slot.payload.len() == 4 => {
                count = Some(u32::from_le_bytes(slot.payload.as_slice().try_into().ok()?) as u64);
            }
            KEY_FIRST_LOST_SEQ if slot.payload.len() == 8 => {
                first = Some(u64::from_le_bytes(slot.payload.as_slice().try_into().ok()?));
            }
            KEY_LAST_LOST_SEQ if slot.payload.len() == 8 => {
                last = Some(u64::from_le_bytes(slot.payload.as_slice().try_into().ok()?));
            }
            _ => {}
        }
    }

    let first = first?;
    let last = last?;
    let count = count?;
    if last < first || count != last - first + 1 {
        return None;
    }

    Some(LossEvent {
        buffer_id,
        run_id: record.run_id,
        source_id: record.source_id,
        first_seq: first,
        last_seq: last,
        count,
        synthetic: record.flags & FLAG_SYNTHETIC != 0,
    })
}

fn parse_source_high_water(record: &Record) -> Option<u64> {
    if record.event_type_id != EVENT_SOURCE_HIGH_WATER {
        return None;
    }

    record.slots.iter().find_map(|slot| {
        if slot.key != KEY_SOURCE_HIGH_WATER || slot.payload.len() != 8 {
            return None;
        }
        let produced_count = u64::from_le_bytes(slot.payload.as_slice().try_into().ok()?);
        (produced_count == record.seq).then_some(produced_count)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ring::DEFAULT_BUFFER_ID;

    #[test]
    fn overlapping_synthetic_and_authoritative_loss_ranges_union_without_double_count() {
        let mut tracker = LossTracker::new(DEFAULT_BUFFER_ID);
        tracker.insert_loss_event(LossEvent {
            buffer_id: DEFAULT_BUFFER_ID,
            run_id: 1,
            source_id: 55,
            first_seq: 5,
            last_seq: 14,
            count: 10,
            synthetic: true,
        });
        tracker.insert_loss_event(LossEvent {
            buffer_id: DEFAULT_BUFFER_ID,
            run_id: 1,
            source_id: 55,
            first_seq: 0,
            last_seq: 9,
            count: 10,
            synthetic: false,
        });

        assert_eq!(tracker.lost(1, 55), 15);
        let events = tracker.loss_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].first_seq, 0);
        assert_eq!(events[0].last_seq, 14);
        assert_eq!(events[0].count, 15);
        assert!(!events[0].synthetic);
    }
}
