//! Run, epoch, and source-sequence producer logic.
//!
//! [`EpochProducer`] assigns per-source sequence numbers and emits the system records
//! that bracket lifecycle changes: a cold start (new `RunId`, sequences reset) versus a
//! warm definition change (same `RunId`, sequences continue, new `epochId`). It also
//! emits source high-water checkpoints. [`EpochResolver`] replays transition records
//! to map retained records back to their definition epoch by absolute byte position.

use std::collections::BTreeMap;

use crate::control::ControlBlockSnapshot;
use crate::registry::{
    EVENT_DEFINITION_CHANGED, EVENT_LOGGER_STARTED, EVENT_LOGGER_STOPPED, EVENT_SOURCE_HIGH_WATER,
    KEY_COLD_START, KEY_DEF_HASH_NEW, KEY_DEF_HASH_OLD, KEY_EPOCH_ID, KEY_SOURCE_HIGH_WATER,
    SYSTEM_SOURCE_ID, TY_BOOL, TY_BYTES, TY_ULINT,
};
use crate::ring::ReadRecord;
use crate::ring::{DEFAULT_BUFFER_ID, RingBuffer};
use crate::wire::{Record, Slot};

/// Drives a [`RingBuffer`], assigning sequence numbers and emitting lifecycle records.
#[derive(Debug, Clone)]
pub struct EpochProducer {
    ring: RingBuffer,
    run_id: u64,
    epoch_id: u64,
    epoch_first_abs: u64,
    system_seq: u64,
    source_seq: BTreeMap<u32, u64>,
    source_time: u64,
    definition_hash: [u8; 8],
    prev_definition_hash: [u8; 8],
    transition: Option<PendingTransition>,
}

#[derive(Debug, Clone)]
struct PendingTransition {
    def_hash_old: [u8; 8],
    def_hash_new: [u8; 8],
    epoch_id: u64,
    definition_changed_emitted: bool,
}

impl EpochProducer {
    /// Creates a producer over a fresh ring of `capacity` bytes at `run_id`/`epoch_id`.
    pub fn new(capacity: usize, run_id: u64, epoch_id: u64) -> Self {
        Self {
            ring: RingBuffer::new(capacity),
            run_id,
            epoch_id,
            epoch_first_abs: 0,
            system_seq: 0,
            source_seq: BTreeMap::new(),
            source_time: 1_780_000_000_000_000_000,
            definition_hash: [0; 8],
            prev_definition_hash: [0; 8],
            transition: None,
        }
    }

    /// The underlying ring, for reading back what was produced.
    pub fn ring(&self) -> &RingBuffer {
        &self.ring
    }

    /// Current run id.
    pub fn run_id(&self) -> u64 {
        self.run_id
    }

    /// Current epoch id.
    pub fn epoch_id(&self) -> u64 {
        self.epoch_id
    }

    /// Absolute offset of the `LoggerStarted` record that opened the current epoch.
    pub fn epoch_first_abs(&self) -> u64 {
        self.epoch_first_abs
    }

    /// Current control-block snapshot for a shared-memory consumer.
    pub fn control_snapshot(&self) -> ControlBlockSnapshot {
        ControlBlockSnapshot {
            version: 2,
            caps: 0,
            buffer_id: DEFAULT_BUFFER_ID,
            buffer_bytes: self.ring.capacity() as u32,
            head_abs: self.ring.head_abs(),
            oldest_abs: self.ring.oldest_abs(),
            lost_count: self.ring.lost_count(),
            run_id: self.run_id,
            epoch_id: self.epoch_id,
            epoch_first_abs: self.epoch_first_abs,
            definition_hash: self.definition_hash,
            prev_definition_hash: self.prev_definition_hash,
        }
    }

    /// Emits one data record for `source_id`, assigning the next per-source sequence.
    ///
    /// Errors with [`EpochError::EmissionDuringTransition`] if a transition is open.
    pub fn emit_data(&mut self, source_id: u32, event_type_id: u32) -> Result<(), EpochError> {
        if self.transition.is_some() {
            return Err(EpochError::EmissionDuringTransition);
        }
        let seq = self.next_source_seq(source_id);
        let record = self.record(source_id, seq, event_type_id);
        self.ring.write_record(&record)?;
        Ok(())
    }

    /// Opens a transition: emits `LoggerStopped`, a high-water checkpoint, and snapshots
    /// each source's first sequence for the new epoch.
    pub fn begin_epoch_transition(
        &mut self,
        def_hash_old: [u8; 8],
        def_hash_new: [u8; 8],
        epoch_id: u64,
    ) -> Result<(), EpochError> {
        if self.transition.is_some() {
            return Err(EpochError::TransitionAlreadyOpen);
        }
        self.emit_system(EVENT_LOGGER_STOPPED, Vec::new())?;
        self.checkpoint_high_water()?;
        self.transition = Some(PendingTransition {
            def_hash_old,
            def_hash_new,
            epoch_id,
            definition_changed_emitted: false,
        });
        Ok(())
    }

    /// Emits one `SourceHighWater` record for each active source.
    ///
    /// Each checkpoint rides under the affected source id and consumes that source's
    /// next sequence. Its scalar `producedCount` slot equals its own envelope `Seq`.
    pub fn checkpoint_high_water(&mut self) -> Result<(), EpochError> {
        if self.transition.is_some() {
            return Err(EpochError::EmissionDuringTransition);
        }
        let high_water = self
            .source_seq
            .iter()
            .map(|(source_id, produced_count)| (*source_id, *produced_count))
            .collect::<Vec<_>>();

        for (source_id, produced_count) in high_water {
            let mut record = self.record(source_id, produced_count, EVENT_SOURCE_HIGH_WATER);
            record.slots.push(Slot::new(
                KEY_SOURCE_HIGH_WATER,
                TY_ULINT,
                produced_count.to_le_bytes(),
            ));
            self.ring.write_record(&record)?;
            self.source_seq.insert(source_id, produced_count + 1);
        }

        Ok(())
    }

    /// Emits the `DefinitionChanged` record carrying the hashes, new epoch id, and each
    /// source's first sequence for the new epoch. Must follow [`begin_epoch_transition`].
    ///
    /// [`begin_epoch_transition`]: EpochProducer::begin_epoch_transition
    pub fn emit_definition_changed(&mut self) -> Result<(), EpochError> {
        let mut pending = self.transition.take().ok_or(EpochError::NoTransitionOpen)?;
        if pending.definition_changed_emitted {
            self.transition = Some(pending);
            return Err(EpochError::DefinitionAlreadyEmitted);
        }
        let slots = vec![
            Slot::new(KEY_DEF_HASH_OLD, TY_BYTES, pending.def_hash_old),
            Slot::new(KEY_DEF_HASH_NEW, TY_BYTES, pending.def_hash_new),
            Slot::new(KEY_EPOCH_ID, TY_ULINT, pending.epoch_id.to_le_bytes()),
        ];
        self.emit_system(EVENT_DEFINITION_CHANGED, slots)?;
        pending.definition_changed_emitted = true;
        self.transition = Some(pending);
        Ok(())
    }

    /// Closes a warm transition: keeps `RunId` and sequences, emits `LoggerStarted` with
    /// `coldStart = false`, and adopts the new epoch.
    pub fn finish_definition_change(&mut self) -> Result<(), EpochError> {
        let pending = self.transition.take().ok_or(EpochError::NoTransitionOpen)?;
        if !pending.definition_changed_emitted {
            self.transition = Some(pending);
            return Err(EpochError::DefinitionNotEmitted);
        }

        self.epoch_id = pending.epoch_id;
        let (start_abs, _) = self.emit_system(
            EVENT_LOGGER_STARTED,
            vec![Slot::new(KEY_COLD_START, TY_BOOL, [0])],
        )?;
        self.epoch_first_abs = start_abs;
        self.prev_definition_hash = pending.def_hash_old;
        self.definition_hash = pending.def_hash_new;
        Ok(())
    }

    /// Closes a cold start: increments `RunId`, resets sequences, and emits
    /// `LoggerStarted` with `coldStart = true`.
    pub fn finish_cold_start(&mut self) -> Result<(), EpochError> {
        let pending = self.transition.take().ok_or(EpochError::NoTransitionOpen)?;
        if !pending.definition_changed_emitted {
            self.transition = Some(pending);
            return Err(EpochError::DefinitionNotEmitted);
        }

        self.run_id += 1;
        self.epoch_id = pending.epoch_id;
        self.system_seq = 0;
        self.source_seq.clear();
        let (start_abs, _) = self.emit_system(
            EVENT_LOGGER_STARTED,
            vec![Slot::new(KEY_COLD_START, TY_BOOL, [1])],
        )?;
        self.epoch_first_abs = start_abs;
        self.prev_definition_hash = pending.def_hash_old;
        self.definition_hash = pending.def_hash_new;
        Ok(())
    }

    fn emit_system(
        &mut self,
        event_type_id: u32,
        slots: Vec<Slot>,
    ) -> Result<(u64, u64), EpochError> {
        let seq = self.system_seq;
        self.system_seq += 1;
        let mut record = self.record(SYSTEM_SOURCE_ID, seq, event_type_id);
        record.slots = slots;
        Ok(self.ring.write_record(&record)?)
    }

    fn next_source_seq(&mut self, source_id: u32) -> u64 {
        let next = self.source_seq.entry(source_id).or_insert(0);
        let seq = *next;
        *next += 1;
        seq
    }

    fn record(&mut self, source_id: u32, seq: u64, event_type_id: u32) -> Record {
        let record = Record::new(self.source_time, self.run_id, seq, source_id, event_type_id);
        self.source_time += 1;
        record
    }
}

/// Errors from the epoch producer's transition state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpochError {
    /// `DefinitionChanged` was already emitted for the open transition.
    DefinitionAlreadyEmitted,
    /// A transition was finished before `DefinitionChanged` was emitted.
    DefinitionNotEmitted,
    /// Data or a checkpoint was emitted while a transition was open.
    EmissionDuringTransition,
    /// A transition-only call was made with no transition open.
    NoTransitionOpen,
    /// The underlying ring rejected a write.
    Ring(crate::ring::RingError),
    /// A transition was begun while one was already open.
    TransitionAlreadyOpen,
}

impl From<crate::ring::RingError> for EpochError {
    fn from(value: crate::ring::RingError) -> Self {
        Self::Ring(value)
    }
}

/// Replays system records to resolve each `(run, source, seq)` to its definition epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochResolver {
    buffer_id: u32,
    run_default_epoch: BTreeMap<u64, u64>,
    epoch_boundaries: BTreeMap<u64, Vec<(u64, u64)>>,
    pending_epoch: Option<u64>,
}

impl EpochResolver {
    /// Creates a resolver seeded with the initial run's default epoch.
    pub fn new(buffer_id: u32, initial_run_id: u64, initial_epoch_id: u64) -> Self {
        let mut run_default_epoch = BTreeMap::new();
        run_default_epoch.insert(initial_run_id, initial_epoch_id);
        Self {
            buffer_id,
            run_default_epoch,
            epoch_boundaries: BTreeMap::new(),
            pending_epoch: None,
        }
    }

    /// Feeds one retained record to the resolver, learning epoch boundaries from control records.
    pub fn observe(&mut self, buffer_id: u32, read: &ReadRecord) {
        if buffer_id != self.buffer_id {
            return;
        }
        let record = &read.record;

        if record.event_type_id == EVENT_DEFINITION_CHANGED {
            self.pending_epoch = definition_changed_epoch(record);
            return;
        }

        if record.event_type_id == EVENT_LOGGER_STARTED
            && let Some(pending) = self.pending_epoch.take()
        {
            if logger_started_cold_start(record).unwrap_or(false) {
                self.run_default_epoch.insert(record.run_id, pending);
                self.epoch_boundaries
                    .entry(record.run_id)
                    .or_default()
                    .push((read.start_abs, pending));
                return;
            }

            self.epoch_boundaries
                .entry(record.run_id)
                .or_default()
                .push((read.start_abs, pending));
            for boundaries in self.epoch_boundaries.values_mut() {
                boundaries.sort_by_key(|(start_abs, _)| *start_abs);
            }
        }
    }

    /// Resolves the definition epoch for a record absolute offset, or the run default.
    pub fn resolve(&self, buffer_id: u32, run_id: u64, start_abs: u64) -> Option<u64> {
        if buffer_id != self.buffer_id {
            return None;
        }
        self.epoch_boundaries
            .get(&run_id)
            .and_then(|boundaries| {
                boundaries
                    .iter()
                    .rev()
                    .find(|(epoch_first_abs, _)| start_abs >= *epoch_first_abs)
                    .map(|(_, epoch_id)| *epoch_id)
            })
            .or_else(|| self.run_default_epoch.get(&run_id).copied())
    }
}

fn definition_changed_epoch(record: &Record) -> Option<u64> {
    let mut epoch_id = None;
    for slot in &record.slots {
        if slot.key == KEY_EPOCH_ID && slot.payload.len() == 8 {
            epoch_id = Some(u64::from_le_bytes(slot.payload.as_slice().try_into().ok()?));
        }
    }
    epoch_id
}

fn logger_started_cold_start(record: &Record) -> Option<bool> {
    record
        .slots
        .iter()
        .find(|slot| slot.key == KEY_COLD_START && slot.payload.len() == 1)
        .map(|slot| slot.payload[0] != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consumer::RawByteConsumer;
    use crate::registry::{EVENT_MESSAGE, EVENT_SOURCE_HIGH_WATER};
    use crate::ring::DEFAULT_BUFFER_ID;

    #[test]
    fn warm_definition_change_keeps_run_and_seq_but_changes_epoch() {
        let mut producer = EpochProducer::new(1024, 1, 1);
        producer.emit_data(21, EVENT_MESSAGE).unwrap();
        producer.emit_data(21, EVENT_MESSAGE).unwrap();

        producer
            .begin_epoch_transition(*b"oldhash1", *b"newhash2", 2)
            .unwrap();
        assert_eq!(
            producer.emit_data(21, EVENT_MESSAGE),
            Err(EpochError::EmissionDuringTransition)
        );
        producer.emit_definition_changed().unwrap();
        producer.finish_definition_change().unwrap();
        producer.emit_data(21, EVENT_MESSAGE).unwrap();

        let mut raw = RawByteConsumer::new();
        let batch = raw.poll(producer.ring()).unwrap();
        assert_stop_definition_start_order(&batch);

        assert_eq!(data_records(&batch, 21), vec![(1, 0), (1, 1), (1, 3)]);
        assert_eq!(
            control_records(&batch, 21),
            vec![(EVENT_SOURCE_HIGH_WATER, 1, 2)]
        );
        assert_eq!(producer.run_id(), 1);
        assert_eq!(producer.epoch_id(), 2);

        let mut resolver = EpochResolver::new(DEFAULT_BUFFER_ID, 1, 1);
        for read in &batch.records {
            resolver.observe(DEFAULT_BUFFER_ID, read);
        }
        let resolved = batch
            .records
            .iter()
            .filter(|read| {
                read.record.source_id == 21 && read.record.event_type_id == EVENT_MESSAGE
            })
            .map(|read| {
                (
                    read.record.run_id,
                    read.record.seq,
                    resolver.resolve(DEFAULT_BUFFER_ID, read.record.run_id, read.start_abs),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            resolved,
            vec![(1, 0, Some(1)), (1, 1, Some(1)), (1, 3, Some(2))]
        );
    }

    #[test]
    fn producer_control_snapshot_tracks_epoch_and_raw_read_bounds() {
        let mut producer = EpochProducer::new(1024, 1, 1);
        producer.emit_data(21, EVENT_MESSAGE).unwrap();
        producer
            .begin_epoch_transition(*b"oldhash1", *b"newhash2", 2)
            .unwrap();
        producer.emit_definition_changed().unwrap();
        producer.finish_definition_change().unwrap();

        let snapshot = producer.control_snapshot();
        assert_eq!(snapshot.buffer_id, DEFAULT_BUFFER_ID);
        assert_eq!(snapshot.buffer_bytes, producer.ring().capacity() as u32);
        assert_eq!(snapshot.head_abs, producer.ring().head_abs());
        assert_eq!(snapshot.oldest_abs, producer.ring().oldest_abs());
        assert_eq!(snapshot.lost_count, producer.ring().lost_count());
        assert_eq!(snapshot.run_id, 1);
        assert_eq!(snapshot.epoch_id, 2);
        assert_eq!(snapshot.epoch_first_abs, producer.epoch_first_abs());
        assert_eq!(snapshot.definition_hash, *b"newhash2");
        assert_eq!(snapshot.prev_definition_hash, *b"oldhash1");

        let mut consumer = RawByteConsumer::new();
        let batch = consumer
            .poll_snapshot(producer.ring(), &snapshot)
            .expect("snapshot-backed raw read succeeds");
        assert_eq!(batch.next_abs, snapshot.head_abs);
        assert_eq!(consumer.cursor_abs(), snapshot.head_abs);
        assert_stop_definition_start_order(&batch);
    }

    #[test]
    fn cold_start_increments_run_and_resets_source_seq() {
        let mut producer = EpochProducer::new(1024, 1, 1);
        producer.emit_data(21, EVENT_MESSAGE).unwrap();
        producer
            .begin_epoch_transition(*b"oldhash1", *b"newhash2", 2)
            .unwrap();
        producer.emit_definition_changed().unwrap();
        producer.finish_cold_start().unwrap();
        producer.emit_data(21, EVENT_MESSAGE).unwrap();

        let mut raw = RawByteConsumer::new();
        let batch = raw.poll(producer.ring()).unwrap();
        assert_stop_definition_start_order(&batch);
        assert_eq!(data_records(&batch, 21), vec![(1, 0), (2, 0)]);
        assert_eq!(producer.run_id(), 2);
        assert_eq!(producer.epoch_id(), 2);

        let mut resolver = EpochResolver::new(DEFAULT_BUFFER_ID, 1, 1);
        for read in &batch.records {
            resolver.observe(DEFAULT_BUFFER_ID, read);
        }
        let resolved = batch
            .records
            .iter()
            .filter(|read| {
                read.record.source_id == 21 && read.record.event_type_id == EVENT_MESSAGE
            })
            .map(|read| {
                (
                    read.record.run_id,
                    read.record.seq,
                    resolver.resolve(DEFAULT_BUFFER_ID, read.record.run_id, read.start_abs),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(resolved, vec![(1, 0, Some(1)), (2, 0, Some(2))]);
    }

    #[test]
    fn silent_source_tail_loss_reconciled_by_high_water() {
        let silent_source = 88;
        let noisy_source = 99;
        let noisy_produced = 80u64;
        let mut producer = EpochProducer::new(256, 1, 1);

        for _ in 0..5 {
            producer.emit_data(silent_source, EVENT_MESSAGE).unwrap();
        }
        for _ in 0..noisy_produced {
            producer.emit_data(noisy_source, EVENT_MESSAGE).unwrap();
        }
        producer.checkpoint_high_water().unwrap();

        let mut consumer = RawByteConsumer::new();
        let batch = consumer.poll(producer.ring()).unwrap();

        assert!(batch.lapped);
        assert_eq!(consumer.cursor_abs(), producer.ring().head_abs());
        assert_eq!(consumer.delivered_in_run(1, silent_source), 1);
        assert_eq!(consumer.lost_in_run(1, silent_source), 5);
        let silent_events = consumer
            .loss_events()
            .into_iter()
            .filter(|event| event.source_id == silent_source)
            .collect::<Vec<_>>();
        assert_eq!(silent_events.len(), 1);
        assert_eq!(silent_events[0].first_seq, 0);
        assert_eq!(silent_events[0].last_seq, 4);
        assert_eq!(silent_events[0].count, 5);
        assert!(!silent_events[0].synthetic);
        assert_eq!(
            consumer.delivered_in_run(1, silent_source) + consumer.lost_in_run(1, silent_source),
            6
        );
        assert_eq!(
            consumer.delivered_in_run(1, noisy_source) + consumer.lost_in_run(1, noisy_source),
            noisy_produced + 1
        );
    }

    fn assert_stop_definition_start_order(batch: &crate::ring::ReadBatch) {
        let events = batch
            .records
            .iter()
            .map(|read| read.record.event_type_id)
            .collect::<Vec<_>>();
        let stop_pos = events
            .iter()
            .position(|event| *event == EVENT_LOGGER_STOPPED)
            .expect("LoggerStopped present");
        let start_pos = events
            .iter()
            .position(|event| *event == EVENT_LOGGER_STARTED)
            .expect("LoggerStarted present");
        assert!(stop_pos < start_pos);
        assert_eq!(
            &events[stop_pos..=start_pos],
            &[
                EVENT_LOGGER_STOPPED,
                EVENT_SOURCE_HIGH_WATER,
                EVENT_DEFINITION_CHANGED,
                EVENT_LOGGER_STARTED,
            ]
        );
    }

    fn data_records(batch: &crate::ring::ReadBatch, source_id: u32) -> Vec<(u64, u64)> {
        batch
            .records
            .iter()
            .filter(|read| {
                read.record.source_id == source_id && read.record.event_type_id == EVENT_MESSAGE
            })
            .map(|read| (read.record.run_id, read.record.seq))
            .collect()
    }

    fn control_records(batch: &crate::ring::ReadBatch, source_id: u32) -> Vec<(u32, u64, u64)> {
        batch
            .records
            .iter()
            .filter(|read| {
                read.record.source_id == source_id && read.record.event_type_id != EVENT_MESSAGE
            })
            .map(|read| {
                (
                    read.record.event_type_id,
                    read.record.run_id,
                    read.record.seq,
                )
            })
            .collect()
    }
}
