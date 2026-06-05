//! Cross-language capture validation for vendor-neutral ST producer fixtures.
//!
//! The validator regenerates a Rust reference image for a fixed scenario, compares a
//! captured ST ring image byte-for-byte, then reads the captured bytes with the raw
//! consumer path and checks loss accounting plus epoch resolution.

use crate::consumer::RawByteConsumer;
use crate::control::ControlBlockSnapshot;
use crate::epoch::{EpochError, EpochProducer, EpochResolver};
use crate::loss::LossEvent;
use crate::registry::{
    EVENT_DEFINITION_CHANGED, EVENT_LOGGER_STARTED, EVENT_MESSAGE, EVENT_SOURCE_HIGH_WATER,
    SYSTEM_SOURCE_ID,
};
use crate::ring::{DEFAULT_BUFFER_ID, ReadBatch, RingBuffer, RingError};

/// Capacity of the S4a captured ST producer scenarios.
pub const S4A_CAPTURE_CAPACITY: usize = 256;

const OLD_HASH: [u8; 8] = *b"oldhash1";
const NEW_HASH: [u8; 8] = *b"newhash2";
const TRANSITION_EPOCH: u64 = 2;

/// Fixed S4a capture scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureScenario {
    /// Heavy wrap/eviction scenario with two data sources and a warm transition.
    RichWrap,
    /// Small warm-transition scenario that retains high-water and lifecycle records.
    LifecycleSurvival,
}

impl CaptureScenario {
    /// Stable fixture directory name for this scenario.
    pub fn fixture_name(self) -> &'static str {
        match self {
            Self::RichWrap => "s4a-rich-wrap",
            Self::LifecycleSurvival => "s4a-lifecycle-survival",
        }
    }
}

/// Dynamic control fields captured from the producer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedControlFields {
    /// Published absolute head.
    pub head_abs: u64,
    /// Oldest retained absolute byte.
    pub oldest_abs: u64,
    /// Producer lost-record counter.
    pub lost_count: u64,
    /// Current run id.
    pub run_id: u64,
    /// Current epoch id.
    pub epoch_id: u64,
    /// Absolute start of the current epoch's `LoggerStarted` record.
    pub epoch_first_abs: u64,
    /// Current definition hash prefix.
    pub definition_hash: [u8; 8],
    /// Previous definition hash prefix.
    pub prev_definition_hash: [u8; 8],
}

impl CapturedControlFields {
    /// Builds dynamic capture fields from a full control snapshot.
    pub fn from_snapshot(snapshot: &ControlBlockSnapshot) -> Self {
        Self {
            head_abs: snapshot.head_abs,
            oldest_abs: snapshot.oldest_abs,
            lost_count: snapshot.lost_count,
            run_id: snapshot.run_id,
            epoch_id: snapshot.epoch_id,
            epoch_first_abs: snapshot.epoch_first_abs,
            definition_hash: snapshot.definition_hash,
            prev_definition_hash: snapshot.prev_definition_hash,
        }
    }

    /// Rehydrates the full control snapshot constants used by captured ST fixtures.
    pub fn to_snapshot(&self, capacity: usize) -> ControlBlockSnapshot {
        ControlBlockSnapshot {
            version: 2,
            caps: 0,
            buffer_id: DEFAULT_BUFFER_ID,
            buffer_bytes: capacity as u32,
            head_abs: self.head_abs,
            oldest_abs: self.oldest_abs,
            lost_count: self.lost_count,
            run_id: self.run_id,
            epoch_id: self.epoch_id,
            epoch_first_abs: self.epoch_first_abs,
            definition_hash: self.definition_hash,
            prev_definition_hash: self.prev_definition_hash,
        }
    }
}

/// A captured or regenerated producer image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedImage {
    /// Physical ring bytes.
    pub ring_bytes: Vec<u8>,
    /// Dynamic control fields.
    pub control: CapturedControlFields,
}

/// One retained record observed during capture validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedRecordSummary {
    /// Absolute start offset.
    pub start_abs: u64,
    /// Absolute end offset.
    pub end_abs: u64,
    /// Record run id.
    pub run_id: u64,
    /// Record source id.
    pub source_id: u32,
    /// Record source-local sequence.
    pub seq: u64,
    /// Record event type id.
    pub event_type_id: u32,
    /// Epoch resolved from retained lifecycle records.
    pub resolved_epoch: Option<u64>,
}

/// Result of a successful capture validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureValidation {
    /// Raw read batch delivered from the captured bytes.
    pub batch: ReadBatch,
    /// Retained record summaries with resolved epochs.
    pub survivors: Vec<CapturedRecordSummary>,
    /// Reconciled loss events.
    pub loss_events: Vec<LossEvent>,
}

/// Capture-validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureValidationError {
    /// The Rust reference scenario could not be generated.
    Reference(EpochError),
    /// The captured ring has a different byte length from the reference.
    RingLengthMismatch {
        /// Expected byte length.
        expected: usize,
        /// Actual byte length.
        actual: usize,
    },
    /// The captured physical ring differs from the Rust reference at one byte.
    RingByteMismatch {
        /// Physical byte offset.
        index: usize,
        /// Expected byte from the Rust reference.
        expected: u8,
        /// Actual captured byte.
        actual: u8,
    },
    /// A dynamic numeric control field differs from the Rust reference.
    ControlFieldMismatch {
        /// Field name.
        field: &'static str,
        /// Expected value.
        expected: u64,
        /// Actual value.
        actual: u64,
    },
    /// A dynamic hash control field differs from the Rust reference.
    ControlHashMismatch {
        /// Field name.
        field: &'static str,
        /// Expected hash prefix.
        expected: [u8; 8],
        /// Actual hash prefix.
        actual: [u8; 8],
    },
    /// Captured bytes could not be consumed as a ring.
    Ring(RingError),
    /// A scenario-specific reconciliation or epoch invariant failed.
    ScenarioInvariant {
        /// Scenario under validation.
        scenario: CaptureScenario,
        /// Human-readable invariant failure.
        message: String,
    },
}

impl From<EpochError> for CaptureValidationError {
    fn from(value: EpochError) -> Self {
        Self::Reference(value)
    }
}

impl From<RingError> for CaptureValidationError {
    fn from(value: RingError) -> Self {
        Self::Ring(value)
    }
}

/// Regenerates the Rust reference image for an S4a scenario.
pub fn reference_capture(scenario: CaptureScenario) -> Result<CapturedImage, EpochError> {
    let mut producer = EpochProducer::new(S4A_CAPTURE_CAPACITY, 1, 1);
    match scenario {
        CaptureScenario::RichWrap => drive_rich_wrap(&mut producer)?,
        CaptureScenario::LifecycleSurvival => drive_lifecycle_survival(&mut producer)?,
    }

    Ok(CapturedImage {
        ring_bytes: producer.ring().physical_bytes().to_vec(),
        control: CapturedControlFields::from_snapshot(&producer.control_snapshot()),
    })
}

/// Validates a captured ST producer image against the Rust reference scenario.
pub fn validate_capture(
    scenario: CaptureScenario,
    captured_ring_bytes: &[u8],
    captured_control: &CapturedControlFields,
) -> Result<CaptureValidation, CaptureValidationError> {
    let reference = reference_capture(scenario)?;
    compare_ring_bytes(captured_ring_bytes, &reference.ring_bytes)?;
    compare_control(captured_control, &reference.control)?;

    let captured_ring = RingBuffer::from_captured(
        captured_ring_bytes.to_vec(),
        captured_control.head_abs,
        captured_control.oldest_abs,
        captured_control.lost_count,
    )?;
    let snapshot = captured_control.to_snapshot(S4A_CAPTURE_CAPACITY);
    let mut consumer = RawByteConsumer::new();
    let batch = consumer.poll_snapshot(&captured_ring, &snapshot)?;

    let mut resolver = EpochResolver::new(DEFAULT_BUFFER_ID, 1, 1);
    for read in &batch.records {
        resolver.observe(DEFAULT_BUFFER_ID, read);
    }

    let survivors = batch
        .records
        .iter()
        .map(|read| CapturedRecordSummary {
            start_abs: read.start_abs,
            end_abs: read.end_abs,
            run_id: read.record.run_id,
            source_id: read.record.source_id,
            seq: read.record.seq,
            event_type_id: read.record.event_type_id,
            resolved_epoch: resolver.resolve(DEFAULT_BUFFER_ID, read.record.run_id, read.start_abs),
        })
        .collect::<Vec<_>>();

    match scenario {
        CaptureScenario::RichWrap => {
            assert_rich_wrap_invariants(&batch, &consumer, &survivors)?;
        }
        CaptureScenario::LifecycleSurvival => {
            assert_lifecycle_survival_invariants(&batch, &consumer, &survivors)?;
        }
    }

    Ok(CaptureValidation {
        batch,
        survivors,
        loss_events: consumer.loss_events(),
    })
}

fn drive_rich_wrap(producer: &mut EpochProducer) -> Result<(), EpochError> {
    for _ in 0..3 {
        producer.emit_data(10, EVENT_MESSAGE)?;
    }
    for _ in 0..2 {
        producer.emit_data(20, EVENT_MESSAGE)?;
    }
    producer.checkpoint_high_water()?;
    producer.begin_epoch_transition(OLD_HASH, NEW_HASH, TRANSITION_EPOCH)?;
    producer.emit_definition_changed()?;
    producer.finish_definition_change()?;
    producer.emit_data(10, EVENT_MESSAGE)?;
    producer.emit_data(20, EVENT_MESSAGE)?;
    Ok(())
}

fn drive_lifecycle_survival(producer: &mut EpochProducer) -> Result<(), EpochError> {
    producer.emit_data(21, EVENT_MESSAGE)?;
    producer.begin_epoch_transition(OLD_HASH, NEW_HASH, TRANSITION_EPOCH)?;
    producer.emit_definition_changed()?;
    producer.finish_definition_change()?;
    Ok(())
}

fn compare_ring_bytes(actual: &[u8], expected: &[u8]) -> Result<(), CaptureValidationError> {
    if actual.len() != expected.len() {
        return Err(CaptureValidationError::RingLengthMismatch {
            expected: expected.len(),
            actual: actual.len(),
        });
    }

    for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
        if actual != expected {
            return Err(CaptureValidationError::RingByteMismatch {
                index,
                expected: *expected,
                actual: *actual,
            });
        }
    }
    Ok(())
}

fn compare_control(
    actual: &CapturedControlFields,
    expected: &CapturedControlFields,
) -> Result<(), CaptureValidationError> {
    compare_control_u64("headAbs", actual.head_abs, expected.head_abs)?;
    compare_control_u64("oldestAbs", actual.oldest_abs, expected.oldest_abs)?;
    compare_control_u64("lostCount", actual.lost_count, expected.lost_count)?;
    compare_control_u64("runId", actual.run_id, expected.run_id)?;
    compare_control_u64("epochId", actual.epoch_id, expected.epoch_id)?;
    compare_control_u64(
        "epochFirstAbs",
        actual.epoch_first_abs,
        expected.epoch_first_abs,
    )?;
    compare_control_hash(
        "definitionHash",
        actual.definition_hash,
        expected.definition_hash,
    )?;
    compare_control_hash(
        "prevDefinitionHash",
        actual.prev_definition_hash,
        expected.prev_definition_hash,
    )?;
    Ok(())
}

fn compare_control_u64(
    field: &'static str,
    actual: u64,
    expected: u64,
) -> Result<(), CaptureValidationError> {
    if actual == expected {
        return Ok(());
    }
    Err(CaptureValidationError::ControlFieldMismatch {
        field,
        expected,
        actual,
    })
}

fn compare_control_hash(
    field: &'static str,
    actual: [u8; 8],
    expected: [u8; 8],
) -> Result<(), CaptureValidationError> {
    if actual == expected {
        return Ok(());
    }
    Err(CaptureValidationError::ControlHashMismatch {
        field,
        expected,
        actual,
    })
}

fn assert_rich_wrap_invariants(
    batch: &ReadBatch,
    consumer: &RawByteConsumer,
    survivors: &[CapturedRecordSummary],
) -> Result<(), CaptureValidationError> {
    ensure(CaptureScenario::RichWrap, batch.lapped, "consumer must lap")?;
    let expected_survivors = [
        summary(
            568,
            648,
            1,
            SYSTEM_SOURCE_ID,
            1,
            EVENT_DEFINITION_CHANGED,
            Some(1),
        ),
        summary(
            648,
            700,
            1,
            SYSTEM_SOURCE_ID,
            2,
            EVENT_LOGGER_STARTED,
            Some(2),
        ),
        summary(700, 744, 1, 10, 5, EVENT_MESSAGE, Some(2)),
        summary(768, 812, 1, 20, 4, EVENT_MESSAGE, Some(2)),
    ];
    ensure_eq(
        CaptureScenario::RichWrap,
        survivors,
        expected_survivors.as_slice(),
        "rich-wrap survivors",
    )?;
    ensure_eq_u64(
        CaptureScenario::RichWrap,
        consumer.delivered_in_run(1, 10),
        1,
        "source 10 delivered",
    )?;
    ensure_eq_u64(
        CaptureScenario::RichWrap,
        consumer.lost_in_run(1, 10),
        5,
        "source 10 lost",
    )?;
    ensure_eq_u64(
        CaptureScenario::RichWrap,
        consumer.delivered_in_run(1, 20),
        1,
        "source 20 delivered",
    )?;
    ensure_eq_u64(
        CaptureScenario::RichWrap,
        consumer.lost_in_run(1, 20),
        4,
        "source 20 lost",
    )?;
    ensure_eq_u64(
        CaptureScenario::RichWrap,
        consumer.delivered_in_run(1, SYSTEM_SOURCE_ID),
        2,
        "system delivered",
    )?;
    ensure_eq_u64(
        CaptureScenario::RichWrap,
        consumer.lost_in_run(1, SYSTEM_SOURCE_ID),
        1,
        "system lost",
    )?;
    Ok(())
}

fn assert_lifecycle_survival_invariants(
    batch: &ReadBatch,
    consumer: &RawByteConsumer,
    survivors: &[CapturedRecordSummary],
) -> Result<(), CaptureValidationError> {
    ensure(
        CaptureScenario::LifecycleSurvival,
        batch.lapped,
        "consumer must lap",
    )?;
    let expected_survivors = [
        summary(88, 144, 1, 21, 1, EVENT_SOURCE_HIGH_WATER, Some(1)),
        summary(
            144,
            224,
            1,
            SYSTEM_SOURCE_ID,
            1,
            EVENT_DEFINITION_CHANGED,
            Some(1),
        ),
        summary(
            256,
            308,
            1,
            SYSTEM_SOURCE_ID,
            2,
            EVENT_LOGGER_STARTED,
            Some(2),
        ),
    ];
    ensure_eq(
        CaptureScenario::LifecycleSurvival,
        survivors,
        expected_survivors.as_slice(),
        "lifecycle-survival survivors",
    )?;
    let retained_data_for_source = batch
        .records
        .iter()
        .filter(|read| read.record.source_id == 21 && read.record.event_type_id == EVENT_MESSAGE)
        .count() as u64;
    ensure_eq_u64(
        CaptureScenario::LifecycleSurvival,
        retained_data_for_source,
        0,
        "source 21 retained data records",
    )?;
    ensure_eq_u64(
        CaptureScenario::LifecycleSurvival,
        consumer.lost_in_run(1, 21),
        1,
        "source 21 lost data records",
    )?;
    ensure_eq_u64(
        CaptureScenario::LifecycleSurvival,
        retained_data_for_source + consumer.lost_in_run(1, 21),
        1,
        "source 21 data delivered plus lost",
    )?;
    ensure_eq_u64(
        CaptureScenario::LifecycleSurvival,
        consumer.delivered_in_run(1, 21),
        1,
        "source 21 delivered high-water record",
    )?;
    ensure_eq_u64(
        CaptureScenario::LifecycleSurvival,
        consumer.delivered_in_run(1, SYSTEM_SOURCE_ID),
        2,
        "system delivered",
    )?;
    ensure_eq_u64(
        CaptureScenario::LifecycleSurvival,
        consumer.lost_in_run(1, SYSTEM_SOURCE_ID),
        1,
        "system lost",
    )?;
    Ok(())
}

fn summary(
    start_abs: u64,
    end_abs: u64,
    run_id: u64,
    source_id: u32,
    seq: u64,
    event_type_id: u32,
    resolved_epoch: Option<u64>,
) -> CapturedRecordSummary {
    CapturedRecordSummary {
        start_abs,
        end_abs,
        run_id,
        source_id,
        seq,
        event_type_id,
        resolved_epoch,
    }
}

fn ensure(
    scenario: CaptureScenario,
    condition: bool,
    message: &'static str,
) -> Result<(), CaptureValidationError> {
    if condition {
        return Ok(());
    }
    Err(CaptureValidationError::ScenarioInvariant {
        scenario,
        message: message.to_string(),
    })
}

fn ensure_eq<T>(
    scenario: CaptureScenario,
    actual: T,
    expected: T,
    name: &'static str,
) -> Result<(), CaptureValidationError>
where
    T: std::fmt::Debug + PartialEq,
{
    if actual == expected {
        return Ok(());
    }
    Err(CaptureValidationError::ScenarioInvariant {
        scenario,
        message: format!("{name}: expected {expected:?}, got {actual:?}"),
    })
}

fn ensure_eq_u64(
    scenario: CaptureScenario,
    actual: u64,
    expected: u64,
    name: &'static str,
) -> Result<(), CaptureValidationError> {
    ensure_eq(scenario, actual, expected, name)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn s4a_rich_wrap_reference_validates_itself() {
        let reference = reference_capture(CaptureScenario::RichWrap).unwrap();
        assert_eq!(reference.control.head_abs, 812);
        assert_eq!(reference.control.oldest_abs, 568);
        assert_eq!(reference.control.lost_count, 10);

        let validation = validate_capture(
            CaptureScenario::RichWrap,
            &reference.ring_bytes,
            &reference.control,
        )
        .unwrap();
        assert_eq!(validation.batch.next_abs, 812);
        assert_eq!(validation.survivors.len(), 4);
    }

    #[test]
    fn s4a_lifecycle_survival_reference_validates_itself() {
        let reference = reference_capture(CaptureScenario::LifecycleSurvival).unwrap();
        assert_eq!(reference.control.head_abs, 308);
        assert_eq!(reference.control.oldest_abs, 88);
        assert_eq!(reference.control.lost_count, 2);

        let validation = validate_capture(
            CaptureScenario::LifecycleSurvival,
            &reference.ring_bytes,
            &reference.control,
        )
        .unwrap();
        assert_eq!(validation.batch.next_abs, 308);
        assert_eq!(validation.survivors.len(), 3);
    }

    #[test]
    fn s4a_rich_wrap_fixture_validates_when_capture_exists() {
        validate_fixture_if_present(CaptureScenario::RichWrap);
    }

    #[test]
    fn s4a_lifecycle_survival_fixture_validates_when_capture_exists() {
        validate_fixture_if_present(CaptureScenario::LifecycleSurvival);
    }

    fn validate_fixture_if_present(scenario: CaptureScenario) {
        let dir = fixture_dir(scenario);
        let ring_path = dir.join("ring.hex");
        let control_path = dir.join("control.json");
        if !ring_path.exists() || !control_path.exists() {
            eprintln!(
                "skipping {} ST capture validation; ring.hex/control.json not present",
                scenario.fixture_name()
            );
            return;
        }

        let ring_hex = std::fs::read_to_string(&ring_path).unwrap();
        let control_json = std::fs::read_to_string(&control_path).unwrap();
        let ring = parse_hex(&ring_hex).unwrap();
        let control = parse_control_json(&control_json).unwrap();
        validate_capture(scenario, &ring, &control).unwrap();
    }

    fn fixture_dir(scenario: CaptureScenario) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/st-captures")
            .join(scenario.fixture_name())
    }

    fn parse_hex(input: &str) -> Result<Vec<u8>, String> {
        let hex = input
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>();
        if hex.len() % 2 != 0 {
            return Err("hex input has odd length".to_string());
        }

        let mut bytes = Vec::new();
        let mut index = 0;
        while index < hex.len() {
            let byte = u8::from_str_radix(&hex[index..index + 2], 16)
                .map_err(|err| format!("bad hex byte at {index}: {err}"))?;
            bytes.push(byte);
            index += 2;
        }
        Ok(bytes)
    }

    fn parse_control_json(input: &str) -> Result<CapturedControlFields, String> {
        Ok(CapturedControlFields {
            head_abs: json_u64(input, "headAbs")?,
            oldest_abs: json_u64(input, "oldestAbs")?,
            lost_count: json_u64(input, "lostCount")?,
            run_id: json_u64(input, "runId")?,
            epoch_id: json_u64(input, "epochId")?,
            epoch_first_abs: json_u64(input, "epochFirstAbs")?,
            definition_hash: json_hash(input, "definitionHash")?,
            prev_definition_hash: json_hash(input, "prevDefinitionHash")?,
        })
    }

    fn json_u64(input: &str, key: &'static str) -> Result<u64, String> {
        let after_key = after_json_key(input, key)?;
        let value_start = after_key
            .find(':')
            .map(|idx| idx + 1)
            .ok_or_else(|| format!("missing ':' after {key}"))?;
        let value = after_key[value_start..].trim_start();
        let end = value
            .find(|ch: char| !ch.is_ascii_digit())
            .unwrap_or(value.len());
        if end == 0 {
            return Err(format!("missing numeric value for {key}"));
        }
        value[..end]
            .parse()
            .map_err(|err| format!("bad numeric value for {key}: {err}"))
    }

    fn json_hash(input: &str, key: &'static str) -> Result<[u8; 8], String> {
        let after_key = after_json_key(input, key)?;
        let colon = after_key
            .find(':')
            .ok_or_else(|| format!("missing ':' after {key}"))?;
        let value = after_key[colon + 1..].trim_start();
        let value = value
            .strip_prefix('"')
            .ok_or_else(|| format!("missing opening quote for {key}"))?;
        let end = value
            .find('"')
            .ok_or_else(|| format!("missing closing quote for {key}"))?;
        let hex = &value[..end];
        let bytes = parse_hex(hex)?;
        if bytes.len() != 8 {
            return Err(format!("{key} must be 8 bytes, got {}", bytes.len()));
        }
        let mut hash = [0; 8];
        hash.copy_from_slice(&bytes);
        Ok(hash)
    }

    fn after_json_key<'a>(input: &'a str, key: &'static str) -> Result<&'a str, String> {
        let needle = format!("\"{key}\"");
        let offset = input
            .find(&needle)
            .ok_or_else(|| format!("missing key {key}"))?;
        Ok(&input[offset + needle.len()..])
    }
}
