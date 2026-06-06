//! Reusable conformance reports and stale-record oracles for OpenOT shared-memory runs.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use open_ot_carriage::concurrent::{ConcurrentRawConsumer, ConcurrentStore};
use open_ot_carriage::consumer::LossAccountingConsumer;
use open_ot_carriage::registry::TY_ULINT;
use open_ot_carriage::ring::{ReadBatch, ReadRecord};
use open_ot_carriage::wire::Slot;
use open_ot_shm::{FenceMode, SharedConcurrentStore};

/// Vendor-range field key used by the synthetic live harness stale oracle.
pub const KEY_EXPECTED_RECORD_START_ABS: u16 = 0x8001;

/// A conformance helper error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceError {
    message: String,
}

impl ConformanceError {
    /// Creates an error from displayable text.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ConformanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ConformanceError {}

/// Stable violation category for report output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    /// A decoded record was found at a different absolute offset than expected.
    ExpectedAbsMismatch,
    /// The embedded absolute-offset oracle slot was missing.
    MissingExpectedAbsSlot,
    /// The embedded absolute-offset oracle slot had the wrong type or width.
    InvalidExpectedAbsSlot,
    /// A decoded record was not present in the sidecar expected stream.
    UnexpectedRecord,
    /// A source-local sequence moved backward or repeated.
    DuplicateOrBackwardSeq,
    /// Delivered records moved backward relative to the sidecar stream order.
    EventOrderMismatch,
}

impl ViolationKind {
    /// Stable report token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExpectedAbsMismatch => "expected_abs_mismatch",
            Self::MissingExpectedAbsSlot => "missing_expected_abs_slot",
            Self::InvalidExpectedAbsSlot => "invalid_expected_abs_slot",
            Self::UnexpectedRecord => "unexpected_record",
            Self::DuplicateOrBackwardSeq => "duplicate_or_backward_seq",
            Self::EventOrderMismatch => "event_order_mismatch",
        }
    }

    fn parse(value: &str) -> Result<Self, ConformanceError> {
        match value {
            "expected_abs_mismatch" => Ok(Self::ExpectedAbsMismatch),
            "missing_expected_abs_slot" => Ok(Self::MissingExpectedAbsSlot),
            "invalid_expected_abs_slot" => Ok(Self::InvalidExpectedAbsSlot),
            "unexpected_record" => Ok(Self::UnexpectedRecord),
            "duplicate_or_backward_seq" => Ok(Self::DuplicateOrBackwardSeq),
            "event_order_mismatch" => Ok(Self::EventOrderMismatch),
            _ => Err(ConformanceError::new(format!(
                "invalid violation kind {value}"
            ))),
        }
    }
}

/// One stale-oracle violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleViolation {
    /// Violation category.
    pub kind: ViolationKind,
    /// Record source id.
    pub source_id: u32,
    /// Record source-local sequence.
    pub seq: u64,
    /// Expected absolute byte start, or 0 when no absolute expectation applies.
    pub expected_abs: u64,
    /// Actual absolute byte start observed by the raw consumer.
    pub actual_abs: u64,
    /// Whether the record bytes passed CRC validation before this violation was detected.
    pub crc_passed: bool,
}

impl StaleViolation {
    fn new(kind: ViolationKind, read: &ReadRecord, expected_abs: u64, actual_abs: u64) -> Self {
        Self {
            kind,
            source_id: read.record.source_id,
            seq: read.record.seq,
            expected_abs,
            actual_abs,
            crc_passed: true,
        }
    }
}

/// Pluggable stale-record oracle.
pub trait StaleOracle {
    /// Observes one decoded, CRC-valid record returned by the raw consumer.
    fn observe(&mut self, read: &ReadRecord) -> Result<Vec<StaleViolation>, ConformanceError>;
}

/// Existing live-harness oracle using the synthetic expected absolute offset slot.
#[derive(Debug, Default)]
pub struct EmbeddedAbsOracle;

impl EmbeddedAbsOracle {
    fn expected_record_start_abs(read: &ReadRecord) -> Result<u64, StaleViolation> {
        let Some(slot) = read
            .record
            .slots
            .iter()
            .find(|slot| slot.key == KEY_EXPECTED_RECORD_START_ABS)
        else {
            return Err(StaleViolation::new(
                ViolationKind::MissingExpectedAbsSlot,
                read,
                0,
                read.start_abs,
            ));
        };
        if slot.ty != TY_ULINT || slot.payload.len() != 8 {
            return Err(StaleViolation::new(
                ViolationKind::InvalidExpectedAbsSlot,
                read,
                0,
                read.start_abs,
            ));
        }
        Ok(u64::from_le_bytes(
            slot.payload
                .as_slice()
                .try_into()
                .expect("slot width checked above"),
        ))
    }
}

impl StaleOracle for EmbeddedAbsOracle {
    fn observe(&mut self, read: &ReadRecord) -> Result<Vec<StaleViolation>, ConformanceError> {
        match Self::expected_record_start_abs(read) {
            Ok(expected_abs) if read.start_abs == expected_abs => Ok(Vec::new()),
            Ok(expected_abs) => Ok(vec![StaleViolation::new(
                ViolationKind::ExpectedAbsMismatch,
                read,
                expected_abs,
                read.start_abs,
            )]),
            Err(violation) => Ok(vec![violation]),
        }
    }
}

/// One expected record in producer write order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedRecord {
    /// Run id.
    pub run_id: u64,
    /// Source id.
    pub source_id: u32,
    /// Source-local sequence.
    pub seq: u64,
    /// Event type id.
    pub event_type_id: u32,
    /// Encoded record length in bytes.
    pub encoded_len: usize,
}

impl ExpectedRecord {
    /// Creates an expected record descriptor.
    #[must_use]
    pub const fn new(
        run_id: u64,
        source_id: u32,
        seq: u64,
        event_type_id: u32,
        encoded_len: usize,
    ) -> Self {
        Self {
            run_id,
            source_id,
            seq,
            event_type_id,
            encoded_len,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RecordKey {
    run_id: u64,
    source_id: u32,
    seq: u64,
    event_type_id: u32,
}

impl From<&ExpectedRecord> for RecordKey {
    fn from(record: &ExpectedRecord) -> Self {
        Self {
            run_id: record.run_id,
            source_id: record.source_id,
            seq: record.seq,
            event_type_id: record.event_type_id,
        }
    }
}

impl From<&ReadRecord> for RecordKey {
    fn from(read: &ReadRecord) -> Self {
        Self {
            run_id: read.record.run_id,
            source_id: read.record.source_id,
            seq: read.record.seq,
            event_type_id: read.record.event_type_id,
        }
    }
}

/// Sidecar oracle for spec-conformant streams with no embedded position slot.
#[derive(Debug, Clone)]
pub struct SidecarExpectedAbsOracle {
    expected_by_key: BTreeMap<RecordKey, u64>,
    last_expected_abs: Option<u64>,
}

impl SidecarExpectedAbsOracle {
    /// Builds the sidecar oracle from expected records in producer write order.
    pub fn new(
        capacity: usize,
        records: impl IntoIterator<Item = ExpectedRecord>,
    ) -> Result<Self, ConformanceError> {
        if capacity == 0 {
            return Err(ConformanceError::new("capacity must be non-zero"));
        }
        let mut expected_by_key = BTreeMap::new();
        let mut head_abs = 0u64;

        for record in records {
            if record.encoded_len == 0 {
                return Err(ConformanceError::new(
                    "expected record length must be non-zero",
                ));
            }
            if record.encoded_len > capacity {
                return Err(ConformanceError::new(format!(
                    "expected record length {} exceeds capacity {capacity}",
                    record.encoded_len
                )));
            }
            let expected_abs = next_start_abs(capacity, head_abs, record.encoded_len);
            let key = RecordKey::from(&record);
            if expected_by_key.insert(key, expected_abs).is_some() {
                return Err(ConformanceError::new(format!(
                    "duplicate expected record run={} source={} seq={} event={}",
                    record.run_id, record.source_id, record.seq, record.event_type_id
                )));
            }
            head_abs = expected_abs + record.encoded_len as u64;
        }

        Ok(Self {
            expected_by_key,
            last_expected_abs: None,
        })
    }
}

impl StaleOracle for SidecarExpectedAbsOracle {
    fn observe(&mut self, read: &ReadRecord) -> Result<Vec<StaleViolation>, ConformanceError> {
        let key = RecordKey::from(read);
        let Some(expected_abs) = self.expected_by_key.get(&key).copied() else {
            return Ok(vec![StaleViolation::new(
                ViolationKind::UnexpectedRecord,
                read,
                0,
                read.start_abs,
            )]);
        };

        let mut violations = Vec::new();
        if read.start_abs != expected_abs {
            violations.push(StaleViolation::new(
                ViolationKind::ExpectedAbsMismatch,
                read,
                expected_abs,
                read.start_abs,
            ));
        }
        if self
            .last_expected_abs
            .is_some_and(|last| expected_abs < last)
        {
            violations.push(StaleViolation::new(
                ViolationKind::EventOrderMismatch,
                read,
                expected_abs,
                read.start_abs,
            ));
        }
        self.last_expected_abs = Some(expected_abs);
        Ok(violations)
    }
}

fn next_start_abs(capacity: usize, head_abs: u64, encoded_len: usize) -> u64 {
    let start_phys = (head_abs % capacity as u64) as usize;
    if start_phys == 0 || start_phys + encoded_len <= capacity {
        head_abs
    } else {
        head_abs + (capacity - start_phys) as u64
    }
}

/// Observes batches with a stale oracle plus source-local duplicate/backward checks.
#[derive(Debug)]
pub struct BatchObserver<O> {
    oracle: O,
    next_seq_by_source: BTreeMap<(u64, u32), u64>,
    seen_records: BTreeSet<RecordKey>,
    violations: Vec<StaleViolation>,
}

impl<O> BatchObserver<O>
where
    O: StaleOracle,
{
    /// Creates a batch observer.
    #[must_use]
    pub fn new(oracle: O) -> Self {
        Self {
            oracle,
            next_seq_by_source: BTreeMap::new(),
            seen_records: BTreeSet::new(),
            violations: Vec::new(),
        }
    }

    /// Observes all records in one decoded batch.
    pub fn observe_batch(&mut self, batch: &ReadBatch) -> Result<(), ConformanceError> {
        for read in &batch.records {
            self.violations.extend(self.oracle.observe(read)?);
            self.observe_sequence(read);
        }
        Ok(())
    }

    /// Returns accumulated violations.
    #[must_use]
    pub fn into_violations(self) -> Vec<StaleViolation> {
        self.violations
    }

    fn observe_sequence(&mut self, read: &ReadRecord) {
        let key = RecordKey::from(read);
        if !self.seen_records.insert(key) {
            self.violations.push(StaleViolation::new(
                ViolationKind::DuplicateOrBackwardSeq,
                read,
                0,
                read.start_abs,
            ));
            return;
        }

        let source_key = (read.record.run_id, read.record.source_id);
        let expected = self
            .next_seq_by_source
            .get(&source_key)
            .copied()
            .unwrap_or(0);
        if read.record.seq < expected {
            self.violations.push(StaleViolation::new(
                ViolationKind::DuplicateOrBackwardSeq,
                read,
                0,
                read.start_abs,
            ));
        } else {
            self.next_seq_by_source
                .insert(source_key, read.record.seq + 1);
        }
    }
}

/// Expected reconciliation total for one source stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedSource {
    /// Run id.
    pub run_id: u64,
    /// Source id.
    pub source_id: u32,
    /// Expected delivered + lost count.
    pub expected_total: u64,
}

impl ExpectedSource {
    /// Creates an expected source total.
    #[must_use]
    pub const fn new(run_id: u64, source_id: u32, expected_total: u64) -> Self {
        Self {
            run_id,
            source_id,
            expected_total,
        }
    }
}

/// Observed reconciliation for one source stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceObserved {
    /// Run id.
    pub run_id: u64,
    /// Source id.
    pub source_id: u32,
    /// Expected delivered + lost count.
    pub expected_total: u64,
    /// Delivered records.
    pub delivered: u64,
    /// Reconciled lost records.
    pub lost: u64,
}

/// Report labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationMetadata {
    /// Run mode label.
    pub mode: String,
    /// Append mode label.
    pub append_mode: String,
    /// Fence mode.
    pub fence_mode: FenceMode,
}

impl ObservationMetadata {
    /// Creates report labels.
    #[must_use]
    pub fn new(
        mode: impl Into<String>,
        append_mode: impl Into<String>,
        fence_mode: FenceMode,
    ) -> Self {
        Self {
            mode: mode.into(),
            append_mode: append_mode.into(),
            fence_mode,
        }
    }
}

/// Inputs used to build an observed report.
pub struct ReportInputs<'a> {
    /// Report labels.
    pub metadata: ObservationMetadata,
    /// Expected source totals.
    pub expected_sources: Vec<ExpectedSource>,
    /// Raw consumer counters.
    pub raw: &'a ConcurrentRawConsumer<SharedConcurrentStore>,
    /// Loss accounting state.
    pub accounting: &'a LossAccountingConsumer,
    /// Shared store counters.
    pub store: &'a SharedConcurrentStore,
    /// Consumer poll errors captured as unfenced evidence instead of hard failures.
    pub poll_errors: u64,
    /// Oracle violations.
    pub stale_violations: Vec<StaleViolation>,
}

/// Observed conformance report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedReport {
    /// Run mode label.
    pub mode: String,
    /// Fence mode.
    pub fence_mode: FenceMode,
    /// Append mode label.
    pub append_mode: String,
    /// Ring byte capacity.
    pub cap: usize,
    /// Published absolute head.
    pub head_abs: u64,
    /// Producer retention-pressure counter.
    pub lost_count: u64,
    /// Number of batches that lapped to `oldest_abs`.
    pub lapped_batches: u64,
    /// Number of mid-read overwrite/control retries.
    pub overwritten_retries: u64,
    /// Number of post-overtake wire/CRC rejections.
    pub rejected_records: u64,
    /// Number of consumer poll errors captured by an experimental unfenced run.
    pub poll_errors: u64,
    /// Delivered total across expected sources.
    pub delivered_total: u64,
    /// Reconciled lost total across expected sources.
    pub lost_total: u64,
    /// Stale-oracle violations.
    pub stale_violations: Vec<StaleViolation>,
    /// Per-source reconciliation rows.
    pub sources: Vec<SourceObserved>,
}

impl ObservedReport {
    /// Builds a report from consumer/accounting state.
    #[must_use]
    pub fn from_consumer(inputs: ReportInputs<'_>) -> Self {
        let mut observed_sources = Vec::new();
        for expected in inputs.expected_sources {
            let delivered = inputs
                .accounting
                .delivered_in_run(expected.run_id, expected.source_id);
            let lost = inputs
                .accounting
                .lost_in_run(expected.run_id, expected.source_id);
            observed_sources.push(SourceObserved {
                run_id: expected.run_id,
                source_id: expected.source_id,
                expected_total: expected.expected_total,
                delivered,
                lost,
            });
        }
        let delivered_total = observed_sources.iter().map(|source| source.delivered).sum();
        let lost_total = observed_sources.iter().map(|source| source.lost).sum();
        Self {
            mode: inputs.metadata.mode,
            fence_mode: inputs.metadata.fence_mode,
            append_mode: inputs.metadata.append_mode,
            cap: inputs.store.capacity(),
            head_abs: inputs.store.load_head_acquire(),
            lost_count: inputs.store.load_lost_acquire(),
            lapped_batches: inputs.raw.lapped_batches(),
            overwritten_retries: inputs.raw.overwritten_retries(),
            rejected_records: inputs.raw.rejected_records(),
            poll_errors: inputs.poll_errors,
            delivered_total,
            lost_total,
            stale_violations: inputs.stale_violations,
            sources: observed_sources,
        }
    }

    /// Writes a stable text report.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        let mut out = String::new();
        out.push_str("report_version 2\n");
        out.push_str(&format!("mode {}\n", self.mode));
        out.push_str(&format!("fence {}\n", self.fence_mode.as_str()));
        out.push_str(&format!("append_mode {}\n", self.append_mode));
        out.push_str(&format!("cap {}\n", self.cap));
        out.push_str(&format!("head_abs {}\n", self.head_abs));
        out.push_str(&format!("lost_count {}\n", self.lost_count));
        out.push_str(&format!("lapped_batches {}\n", self.lapped_batches));
        out.push_str(&format!(
            "overwritten_retries {}\n",
            self.overwritten_retries
        ));
        out.push_str(&format!("rejected_records {}\n", self.rejected_records));
        out.push_str(&format!("poll_errors {}\n", self.poll_errors));
        out.push_str(&format!("delivered_total {}\n", self.delivered_total));
        out.push_str(&format!("lost_total {}\n", self.lost_total));
        out.push_str(&format!(
            "stale_violations {}\n",
            self.stale_violations.len()
        ));
        for stale in &self.stale_violations {
            out.push_str(&format!(
                "stale {} {} {} {} {} {}\n",
                stale.kind.as_str(),
                stale.source_id,
                stale.seq,
                stale.expected_abs,
                stale.actual_abs,
                stale.crc_passed
            ));
        }
        for source in &self.sources {
            out.push_str(&format!(
                "source {} {} {} {} {}\n",
                source.run_id,
                source.source_id,
                source.expected_total,
                source.delivered,
                source.lost
            ));
        }
        fs::write(path, out)
    }

    /// Reads a stable text report.
    pub fn read(path: &Path) -> Result<Self, ConformanceError> {
        let content = fs::read_to_string(path).map_err(|err| {
            ConformanceError::new(format!("failed to read report {}: {err}", path.display()))
        })?;
        let mut mode = None;
        let mut fence_mode = None;
        let mut append_mode = None;
        let mut cap = None;
        let mut head_abs = None;
        let mut lost_count = None;
        let mut lapped_batches = None;
        let mut overwritten_retries = None;
        let mut rejected_records = None;
        let mut poll_errors = Some(0);
        let mut delivered_total = None;
        let mut lost_total = None;
        let mut stale_count = None;
        let mut stale_violations = Vec::new();
        let mut sources = Vec::new();

        for line in content.lines() {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            match parts.as_slice() {
                ["report_version", "2"] => {}
                ["mode", value] => mode = Some((*value).to_string()),
                ["fence", value] => {
                    fence_mode = Some(FenceMode::parse(value).map_err(ConformanceError::new)?);
                }
                ["append_mode", value] => append_mode = Some((*value).to_string()),
                ["cap", value] => cap = Some(parse_value(value, "cap")?),
                ["head_abs", value] => head_abs = Some(parse_value(value, "head_abs")?),
                ["lost_count", value] => lost_count = Some(parse_value(value, "lost_count")?),
                ["lapped_batches", value] => {
                    lapped_batches = Some(parse_value(value, "lapped_batches")?);
                }
                ["overwritten_retries", value] => {
                    overwritten_retries = Some(parse_value(value, "overwritten_retries")?);
                }
                ["rejected_records", value] => {
                    rejected_records = Some(parse_value(value, "rejected_records")?);
                }
                ["poll_errors", value] => {
                    poll_errors = Some(parse_value(value, "poll_errors")?);
                }
                ["delivered_total", value] => {
                    delivered_total = Some(parse_value(value, "delivered_total")?);
                }
                ["lost_total", value] => lost_total = Some(parse_value(value, "lost_total")?),
                ["stale_violations", value] => {
                    stale_count = Some(parse_value(value, "stale_violations")?);
                }
                [
                    "stale",
                    kind,
                    source_id,
                    seq,
                    expected_abs,
                    actual_abs,
                    crc_passed,
                ] => {
                    stale_violations.push(StaleViolation {
                        kind: ViolationKind::parse(kind)?,
                        source_id: parse_value(source_id, "stale source_id")?,
                        seq: parse_value(seq, "stale seq")?,
                        expected_abs: parse_value(expected_abs, "stale expected_abs")?,
                        actual_abs: parse_value(actual_abs, "stale actual_abs")?,
                        crc_passed: parse_value(crc_passed, "stale crc_passed")?,
                    });
                }
                [
                    "stale",
                    source_id,
                    seq,
                    expected_abs,
                    actual_abs,
                    crc_passed,
                ] => {
                    stale_violations.push(StaleViolation {
                        kind: ViolationKind::ExpectedAbsMismatch,
                        source_id: parse_value(source_id, "stale source_id")?,
                        seq: parse_value(seq, "stale seq")?,
                        expected_abs: parse_value(expected_abs, "stale expected_abs")?,
                        actual_abs: parse_value(actual_abs, "stale actual_abs")?,
                        crc_passed: parse_value(crc_passed, "stale crc_passed")?,
                    });
                }
                ["source", run_id, source_id, expected_total, delivered, lost] => {
                    sources.push(SourceObserved {
                        run_id: parse_value(run_id, "source run_id")?,
                        source_id: parse_value(source_id, "source source_id")?,
                        expected_total: parse_value(expected_total, "source expected_total")?,
                        delivered: parse_value(delivered, "source delivered")?,
                        lost: parse_value(lost, "source lost")?,
                    });
                }
                ["source", source_id, expected_total, delivered, lost] => {
                    sources.push(SourceObserved {
                        run_id: 0,
                        source_id: parse_value(source_id, "source source_id")?,
                        expected_total: parse_value(expected_total, "source expected_total")?,
                        delivered: parse_value(delivered, "source delivered")?,
                        lost: parse_value(lost, "source lost")?,
                    });
                }
                _ => {
                    return Err(ConformanceError::new(format!(
                        "invalid observed line: {line}"
                    )));
                }
            }
        }

        if stale_count != Some(stale_violations.len()) {
            return Err(ConformanceError::new("stale_violations count mismatch"));
        }

        Ok(Self {
            mode: mode.ok_or_else(|| ConformanceError::new("missing mode"))?,
            fence_mode: fence_mode.ok_or_else(|| ConformanceError::new("missing fence"))?,
            append_mode: append_mode.ok_or_else(|| ConformanceError::new("missing append_mode"))?,
            cap: cap.ok_or_else(|| ConformanceError::new("missing cap"))?,
            head_abs: head_abs.ok_or_else(|| ConformanceError::new("missing head_abs"))?,
            lost_count: lost_count.ok_or_else(|| ConformanceError::new("missing lost_count"))?,
            lapped_batches: lapped_batches
                .ok_or_else(|| ConformanceError::new("missing lapped_batches"))?,
            overwritten_retries: overwritten_retries
                .ok_or_else(|| ConformanceError::new("missing overwritten_retries"))?,
            rejected_records: rejected_records
                .ok_or_else(|| ConformanceError::new("missing rejected_records"))?,
            poll_errors: poll_errors.ok_or_else(|| ConformanceError::new("missing poll_errors"))?,
            delivered_total: delivered_total
                .ok_or_else(|| ConformanceError::new("missing delivered_total"))?,
            lost_total: lost_total.ok_or_else(|| ConformanceError::new("missing lost_total"))?,
            stale_violations,
            sources,
        })
    }

    /// One-line human summary.
    #[must_use]
    pub fn summary_line(&self) -> String {
        format!(
            "summary: mode={} fence={} append_mode={} cap={} head_abs={} lost_count={} delivered={} lost={} lapped={} retries={} rejected={} poll_errors={} stale={}",
            self.mode,
            self.fence_mode.as_str(),
            self.append_mode,
            self.cap,
            self.head_abs,
            self.lost_count,
            self.delivered_total,
            self.lost_total,
            self.lapped_batches,
            self.overwritten_retries,
            self.rejected_records,
            self.poll_errors,
            self.stale_violations.len()
        )
    }

    /// Checks fenced-run invariants common to conformance harnesses.
    pub fn assert_fenced(
        &self,
        min_head_abs: Option<u64>,
        require_retention_pressure: bool,
    ) -> Result<(), ConformanceError> {
        if self.delivered_total == 0 {
            return Err(ConformanceError::new("consumer made no progress"));
        }
        if let Some(min_head_abs) = min_head_abs
            && self.head_abs <= min_head_abs
        {
            return Err(ConformanceError::new(format!(
                "insufficient absolute progress: head_abs={} min={min_head_abs}",
                self.head_abs
            )));
        }
        if require_retention_pressure && self.lost_count == 0 {
            return Err(ConformanceError::new(
                "producer did not report ring evictions",
            ));
        }
        if self.rejected_records != 0 {
            return Err(ConformanceError::new(format!(
                "fenced run rejected {} records",
                self.rejected_records
            )));
        }
        if self.poll_errors != 0 {
            return Err(ConformanceError::new(format!(
                "fenced run captured {} poll errors",
                self.poll_errors
            )));
        }
        if !self.stale_violations.is_empty() {
            return Err(ConformanceError::new(format!(
                "fenced run accepted stale records: {}",
                self.stale_violations.len()
            )));
        }
        for source in &self.sources {
            if source.delivered + source.lost != source.expected_total {
                return Err(ConformanceError::new(format!(
                    "source {} run {} reconciliation failed: delivered={} lost={} expected_total={}",
                    source.source_id,
                    source.run_id,
                    source.delivered,
                    source.lost,
                    source.expected_total
                )));
            }
        }
        Ok(())
    }

    /// Summarizes an unfenced run without treating non-reproduction as success or failure.
    #[must_use]
    pub fn unfenced_evidence(&self) -> UnfencedEvidence {
        let stale_violations = self.stale_violations.len();
        let hazard_observed =
            stale_violations > 0 || self.rejected_records > 0 || self.poll_errors > 0;
        UnfencedEvidence {
            fence_mode: self.fence_mode,
            hazard_observed,
            stale_violations,
            rejected_records: self.rejected_records,
            poll_errors: self.poll_errors,
            overwritten_retries: self.overwritten_retries,
            lapped_batches: self.lapped_batches,
            delivered_total: self.delivered_total,
            lost_total: self.lost_total,
            lost_count: self.lost_count,
        }
    }
}

/// Non-flaky evidence view for diagnostic unfenced runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnfencedEvidence {
    /// Fence mode used by the run.
    pub fence_mode: FenceMode,
    /// True when stale/duplicate/misordered records, rejected wire/CRC records, or poll errors appeared.
    pub hazard_observed: bool,
    /// Number of stale-oracle or ordering violations.
    pub stale_violations: usize,
    /// Number of post-overtake wire/CRC rejections.
    pub rejected_records: u64,
    /// Number of captured consumer poll errors.
    pub poll_errors: u64,
    /// Number of mid-read overwrite/control retries.
    pub overwritten_retries: u64,
    /// Number of lapped batches.
    pub lapped_batches: u64,
    /// Delivered total across expected sources.
    pub delivered_total: u64,
    /// Reconciled lost total across expected sources.
    pub lost_total: u64,
    /// Producer retention-pressure counter.
    pub lost_count: u64,
}

impl UnfencedEvidence {
    /// Stable outcome label.
    #[must_use]
    pub const fn outcome(&self) -> &'static str {
        if self.hazard_observed {
            "hazard-observed"
        } else {
            "non-reproduction"
        }
    }

    /// Standard disclaimer for a clean unfenced run.
    #[must_use]
    pub const fn non_reproduction_note() -> &'static str {
        "non-reproduction, NOT proof of safety"
    }

    /// One-line human summary.
    #[must_use]
    pub fn summary_line(&self) -> String {
        format!(
            "unfenced_evidence: outcome={} fence={} stale={} rejected={} poll_errors={} retries={} lapped={} delivered={} lost={} lost_count={}",
            self.outcome(),
            self.fence_mode.as_str(),
            self.stale_violations,
            self.rejected_records,
            self.poll_errors,
            self.overwritten_retries,
            self.lapped_batches,
            self.delivered_total,
            self.lost_total,
            self.lost_count
        )
    }
}

fn parse_value<T>(value: &str, field: &str) -> Result<T, ConformanceError>
where
    T: std::str::FromStr,
{
    value
        .parse::<T>()
        .map_err(|_| ConformanceError::new(format!("invalid value for {field}: {value}")))
}

/// Creates the embedded expected-absolute-offset slot used by the synthetic harness.
#[must_use]
pub fn expected_abs_slot(expected_abs: u64) -> Slot {
    Slot::new(
        KEY_EXPECTED_RECORD_START_ABS,
        TY_ULINT,
        expected_abs.to_le_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use open_ot_carriage::registry::{EVENT_MESSAGE, EVENT_SOURCE_HIGH_WATER};
    use open_ot_carriage::ring::ReadRecord;
    use open_ot_carriage::wire::Record;

    #[test]
    fn embedded_abs_oracle_detects_mismatch() {
        let mut record = Record::new(1, 1, 0, 10, EVENT_MESSAGE);
        record.slots.push(expected_abs_slot(44));
        let read = ReadRecord {
            start_abs: 0,
            end_abs: 44,
            record,
        };

        let violations = EmbeddedAbsOracle.observe(&read).expect("observe record");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].kind, ViolationKind::ExpectedAbsMismatch);
        assert_eq!(violations[0].expected_abs, 44);
        assert_eq!(violations[0].actual_abs, 0);
    }

    #[test]
    fn sidecar_oracle_computes_wrap_offsets() {
        let mut oracle = SidecarExpectedAbsOracle::new(
            100,
            [
                ExpectedRecord::new(1, 10, 0, EVENT_MESSAGE, 44),
                ExpectedRecord::new(1, 10, 1, EVENT_MESSAGE, 44),
                ExpectedRecord::new(1, 10, 2, EVENT_SOURCE_HIGH_WATER, 56),
            ],
        )
        .expect("build sidecar oracle");
        let read = ReadRecord {
            start_abs: 100,
            end_abs: 156,
            record: Record::new(1, 1, 2, 10, EVENT_SOURCE_HIGH_WATER),
        };

        assert!(
            oracle
                .observe(&read)
                .expect("observe sidecar record")
                .is_empty()
        );
    }

    #[test]
    fn batch_observer_detects_duplicate_key() {
        let record = Record::new(1, 1, 0, 10, EVENT_MESSAGE);
        let batch = ReadBatch {
            records: vec![
                ReadRecord {
                    start_abs: 0,
                    end_abs: 44,
                    record: record.clone(),
                },
                ReadRecord {
                    start_abs: 44,
                    end_abs: 88,
                    record,
                },
            ],
            next_abs: 88,
            lapped: false,
        };
        let mut observer = BatchObserver::new(
            SidecarExpectedAbsOracle::new(128, [ExpectedRecord::new(1, 10, 0, EVENT_MESSAGE, 44)])
                .expect("build sidecar oracle"),
        );

        observer.observe_batch(&batch).expect("observe batch");
        let violations = observer.into_violations();

        assert!(
            violations
                .iter()
                .any(|violation| violation.kind == ViolationKind::DuplicateOrBackwardSeq)
        );
    }
}
