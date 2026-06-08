use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use open_ot_carriage::concurrent::{ConcurrentProducer, ConcurrentRawConsumer};
use open_ot_carriage::consumer::LossAccountingConsumer;
use open_ot_carriage::registry::{
    EVENT_MESSAGE, EVENT_SOURCE_HIGH_WATER, KEY_SOURCE_HIGH_WATER, TY_ULINT,
};
use open_ot_carriage::ring::ReadRecord;
use open_ot_carriage::wire::{Record, Slot};
use open_ot_conformance::{
    BatchObserver, ExpectedRecord, ExpectedSource, ObservationMetadata, ObservedReport,
    ReportInputs, SidecarExpectedAbsOracle,
};
use open_ot_shm::{FenceMode, SharedConcurrentStore};

const RUN_ID: u64 = 1;
const SOURCE_COUNT: u32 = 16;
const SOURCE_BASE: u32 = 100;
const SOURCE_TIME_BASE: u64 = 1_780_000_000_000_000_000;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn steady_state_multi_source_reconciles_without_loss() -> TestResult<()> {
    let capacity = 128 * 1024;
    let sources = source_ids();
    let records_per_source = 64u64;
    let (store, path) = create_store("steady", capacity)?;
    let mut producer = ConcurrentProducer::with_store(store.clone());
    let mut expected_records = Vec::new();

    for seq in 0..records_per_source {
        for source_id in &sources {
            append_record(
                &mut producer,
                &mut expected_records,
                message_record(*source_id, seq),
            )?;
        }
    }

    let observation = observe_store(
        &store,
        "multi-source-steady",
        expected_records,
        expected_sources(&sources, records_per_source),
    )?;

    assert_eq!(observation.report.lapped_batches, 0);
    assert_eq!(observation.report.lost_count, 0);
    assert_eq!(observation.report.rejected_records, 0);
    assert_eq!(observation.report.overwritten_retries, 0);
    assert!(observation.report.stale_violations.is_empty());
    assert!(observation.accounting.loss_events().is_empty());
    for source_id in &sources {
        assert_eq!(
            observation.accounting.delivered_in_run(RUN_ID, *source_id),
            records_per_source,
            "source {source_id} delivered count"
        );
        assert_eq!(
            observation.accounting.lost_in_run(RUN_ID, *source_id),
            0,
            "source {source_id} lost count"
        );
    }

    cleanup(path);
    Ok(())
}

#[test]
fn lapped_multi_source_reconciles_seq_gap_loss() -> TestResult<()> {
    let capacity = 4 * 1024;
    let sources = source_ids();
    let records_per_source = 128u64;
    let (store, path) = create_store("lapped", capacity)?;
    let mut producer = ConcurrentProducer::with_store(store.clone());
    let mut expected_records = Vec::new();

    for seq in 0..records_per_source {
        for source_id in &sources {
            append_record(
                &mut producer,
                &mut expected_records,
                message_record(*source_id, seq),
            )?;
        }
    }

    let observation = observe_store(
        &store,
        "multi-source-lapped",
        expected_records,
        expected_sources(&sources, records_per_source),
    )?;

    assert!(
        observation.report.lapped_batches > 0,
        "consumer must observe at least one physical lap"
    );
    assert!(
        observation.report.lost_count > 0,
        "shared store must report aggregate retention pressure"
    );
    assert_eq!(observation.report.rejected_records, 0);
    assert!(observation.report.stale_violations.is_empty());
    assert!(
        !observation.accounting.loss_events().is_empty(),
        "seq-gap loss accounting should produce per-source loss events"
    );

    let min_retained_seq = min_retained_seq_by_source(&observation.records);
    for source_id in &sources {
        let delivered = observation.accounting.delivered_in_run(RUN_ID, *source_id);
        let lost = observation.accounting.lost_in_run(RUN_ID, *source_id);
        assert!(
            delivered > 0,
            "source {source_id} needs retained post-gap records"
        );
        assert!(lost > 0, "source {source_id} should lose leading records");
        assert_eq!(
            delivered + lost,
            records_per_source,
            "source {source_id} must reconcile to produced count"
        );
        assert!(
            min_retained_seq.get(source_id).is_some_and(|seq| *seq > 0),
            "source {source_id} first retained record should be after a seq gap"
        );
    }

    cleanup(path);
    Ok(())
}

#[test]
fn source_high_water_reconciles_fully_evicted_silent_tail() -> TestResult<()> {
    let capacity = 512;
    let silent_source = 900u32;
    let noisy_source = 901u32;
    let silent_data_records = 16u64;
    let noisy_records = 96u64;
    let (store, path) = create_store("high-water-tail", capacity)?;
    let mut producer = ConcurrentProducer::with_store(store.clone());
    let mut expected_records = Vec::new();

    for seq in 0..silent_data_records {
        append_record(
            &mut producer,
            &mut expected_records,
            message_record(silent_source, seq),
        )?;
    }
    for seq in 0..noisy_records {
        append_record(
            &mut producer,
            &mut expected_records,
            message_record(noisy_source, seq),
        )?;
    }
    append_record(
        &mut producer,
        &mut expected_records,
        source_high_water_record(silent_source, silent_data_records),
    )?;

    let observation = observe_store(
        &store,
        "multi-source-high-water-tail",
        expected_records,
        vec![
            ExpectedSource::new(RUN_ID, silent_source, silent_data_records + 1),
            ExpectedSource::new(RUN_ID, noisy_source, noisy_records),
        ],
    )?;

    assert!(
        observation.report.lapped_batches > 0,
        "test must physically lap before reading"
    );
    assert!(observation.report.lost_count > 0);
    assert_eq!(observation.report.rejected_records, 0);
    assert!(observation.report.stale_violations.is_empty());
    assert_eq!(
        observation
            .accounting
            .delivered_in_run(RUN_ID, silent_source),
        1,
        "only the retained high-water checkpoint should be delivered for the silent source"
    );
    assert_eq!(
        observation.accounting.lost_in_run(RUN_ID, silent_source),
        silent_data_records,
        "high-water should close the silent source tail"
    );
    let silent_records = observation
        .records
        .iter()
        .filter(|read| read.record.source_id == silent_source)
        .collect::<Vec<_>>();
    assert_eq!(silent_records.len(), 1);
    assert_eq!(
        silent_records[0].record.event_type_id,
        EVENT_SOURCE_HIGH_WATER
    );
    assert_eq!(silent_records[0].record.seq, silent_data_records);
    assert_eq!(
        observation
            .accounting
            .delivered_in_run(RUN_ID, silent_source)
            + observation.accounting.lost_in_run(RUN_ID, silent_source),
        silent_data_records + 1,
        "D data records plus one checkpoint should reconcile to D + 1"
    );
    let silent_loss = observation
        .accounting
        .loss_events()
        .into_iter()
        .find(|event| event.source_id == silent_source)
        .expect("silent source high-water loss event");
    assert!(!silent_loss.synthetic, "high-water loss is authoritative");
    assert_eq!(silent_loss.first_seq, 0);
    assert_eq!(silent_loss.last_seq, silent_data_records - 1);
    assert_eq!(silent_loss.count, silent_data_records);

    cleanup(path);
    Ok(())
}

struct Observation {
    report: ObservedReport,
    accounting: LossAccountingConsumer,
    records: Vec<ReadRecord>,
}

type TestResult<T> = Result<T, String>;

fn observe_store(
    store: &SharedConcurrentStore,
    mode: &str,
    expected_records: Vec<ExpectedRecord>,
    expected_sources: Vec<ExpectedSource>,
) -> TestResult<Observation> {
    let mut raw = ConcurrentRawConsumer::with_store(store.clone());
    let mut accounting = LossAccountingConsumer::new();
    let oracle = SidecarExpectedAbsOracle::new(store.capacity(), expected_records)
        .map_err(|error| error.to_string())?;
    let mut observer = BatchObserver::new(oracle);
    let mut records = Vec::new();

    for _ in 0..8 {
        let batch = raw.poll().map_err(|error| format!("{error:?}"))?;
        observer
            .observe_batch(&batch)
            .map_err(|error| error.to_string())?;
        accounting.account_batch(&batch);
        records.extend(batch.records.iter().cloned());
        if raw.cursor_abs() == raw.head_abs() {
            break;
        }
    }
    assert_eq!(
        raw.cursor_abs(),
        raw.head_abs(),
        "deterministic conformance read did not drain the store"
    );

    let report = ObservedReport::from_consumer(ReportInputs {
        metadata: ObservationMetadata::new(mode, "single-writer-interleave", FenceMode::Fenced),
        expected_sources,
        raw: &raw,
        accounting: &accounting,
        store,
        poll_errors: 0,
        stale_violations: observer.into_violations(),
    });

    Ok(Observation {
        report,
        accounting,
        records,
    })
}

fn append_record(
    producer: &mut ConcurrentProducer<SharedConcurrentStore>,
    expected_records: &mut Vec<ExpectedRecord>,
    record: Record,
) -> TestResult<()> {
    let encoded_len = record
        .encode(true)
        .map_err(|error| format!("{error:?}"))?
        .len();
    expected_records.push(ExpectedRecord::new(
        record.run_id,
        record.source_id,
        record.seq,
        record.event_type_id,
        encoded_len,
    ));
    producer
        .write_record(&record)
        .map_err(|error| format!("{error:?}"))?;
    Ok(())
}

fn source_ids() -> Vec<u32> {
    (0..SOURCE_COUNT)
        .map(|offset| SOURCE_BASE + offset)
        .collect()
}

fn expected_sources(sources: &[u32], expected_total: u64) -> Vec<ExpectedSource> {
    sources
        .iter()
        .map(|source_id| ExpectedSource::new(RUN_ID, *source_id, expected_total))
        .collect()
}

fn message_record(source_id: u32, seq: u64) -> Record {
    Record::new(
        SOURCE_TIME_BASE + seq,
        RUN_ID,
        seq,
        source_id,
        EVENT_MESSAGE,
    )
}

fn source_high_water_record(source_id: u32, produced_count: u64) -> Record {
    let mut record = Record::new(
        SOURCE_TIME_BASE + produced_count,
        RUN_ID,
        produced_count,
        source_id,
        EVENT_SOURCE_HIGH_WATER,
    );
    record.slots.push(Slot::new(
        KEY_SOURCE_HIGH_WATER,
        TY_ULINT,
        produced_count.to_le_bytes(),
    ));
    record
}

fn min_retained_seq_by_source(records: &[ReadRecord]) -> BTreeMap<u32, u64> {
    let mut min_by_source = BTreeMap::new();
    for read in records {
        min_by_source
            .entry(read.record.source_id)
            .and_modify(|seq: &mut u64| *seq = (*seq).min(read.record.seq))
            .or_insert(read.record.seq);
    }
    min_by_source
}

fn create_store(case_name: &str, capacity: usize) -> TestResult<(SharedConcurrentStore, PathBuf)> {
    let path = temp_path(case_name);
    let _ = fs::remove_file(&path);
    let store =
        SharedConcurrentStore::create(&path, capacity).map_err(|error| error.to_string())?;
    Ok((store, path))
}

fn temp_path(case_name: &str) -> PathBuf {
    let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "open-ot-conformance-{case_name}-{}-{unique}.shm",
        std::process::id()
    ))
}

fn cleanup(path: PathBuf) {
    let _ = fs::remove_file(path);
}
