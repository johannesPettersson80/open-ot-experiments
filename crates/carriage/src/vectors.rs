//! Deterministic conformance-vector generation.
//!
//! [`write_vectors`] / [`generate_files`] emit byte-exact `.hex` fixtures and `.json`
//! interpretations under `vectors/`. The test suite regenerates and compares them, so
//! checked-in fixtures cannot drift from the encoder.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

use crate::consumer::RawByteConsumer;
use crate::control::{CONTROL_BLOCK_LEN, ControlBlockSnapshot};
use crate::loss::{EVENT_RECORDS_DROPPED, records_dropped_record};
use crate::registry::{
    EVENT_MESSAGE, EVENT_SOURCE_HIGH_WATER, EVENT_STATE_TRANSITION, KEY_ARG, KEY_CATEGORY,
    KEY_DROPPED_COUNT, KEY_FIRST_LOST_SEQ, KEY_LAST_LOST_SEQ, KEY_MESSAGE_TEMPLATE_ID,
    KEY_NEW_STATE, KEY_PREVIOUS_STATE, KEY_SEVERITY, KEY_SOURCE_HIGH_WATER, KEY_STATE_MACHINE_ID,
    KEY_WINDOW_END, KEY_WINDOW_START, TY_DATE_TIME, TY_STRING, TY_UDINT, TY_UINT, TY_ULINT,
};
use crate::ring::{LossRange, RingBuffer};
use crate::wire::{FLAG_HAS_CRC, HEADER_LEN, Record, SYNC, Slot, WireError, decode};

/// One generated conformance-vector file: a relative path and its byte contents.
pub struct VectorFile {
    /// Path of the file relative to the vectors root.
    pub path: &'static str,
    /// Full file contents (hex vector or JSON manifest).
    pub contents: String,
}

/// Writes all generated vector files under `root`, creating directories as needed.
///
/// Returns the number of files written. The checked-in fixtures under `vectors/` are
/// produced by this function and compared against it by the test suite.
pub fn write_vectors(root: &Path) -> io::Result<usize> {
    let files = generate_files();
    fs::create_dir_all(root)?;
    for file in &files {
        let path = root.join(file.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &file.contents)?;
    }
    Ok(files.len())
}

/// Builds the in-memory set of conformance-vector files from the wire encoder.
///
/// This is the single source of truth for the checked-in fixtures; tests assert the
/// files on disk match what this returns so the vectors cannot drift from the codec.
pub fn generate_files() -> Vec<VectorFile> {
    let mut files = Vec::new();
    files.push(VectorFile {
        path: "README.md",
        contents: readme(),
    });

    push_record_vector(
        &mut files,
        "state_transition",
        "wire-codec StateTransition record; schema-negative because stateMachineId is UInt and required state fields are absent",
        &state_transition_record(),
        r#"{
  "eventName": "StateTransition",
  "schemaExpected": "reject",
  "schemaViolation": "stateMachineId uses UInt instead of UDInt and previousState/newState are absent",
  "fields": {
    "sourceTime": "0x0102030405060708",
    "runId": 1,
    "seq": 2,
    "sourceId": 42,
    "eventTypeId": "0x0001"
  },
  "slots": [
    { "key": "0x0001", "type": "UInt", "value": "0x1234" },
    { "key": "0x0002", "type": "UInt", "value": 2 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_state_transition",
        "definition-layer positive StateTransition record",
        &conformant_state_transition_record(),
        r#"{
  "eventName": "StateTransition",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000000,
    "runId": 1,
    "seq": 1,
    "sourceId": 66,
    "eventTypeId": "0x0001"
  },
  "slots": [
    { "key": "0x0001", "type": "UDInt", "name": "stateMachineId", "value": 7 },
    { "key": "0x0002", "type": "UInt", "name": "category", "value": 2 },
    { "key": "0x0003", "type": "UInt", "name": "previousState", "value": 3 },
    { "key": "0x0004", "type": "UInt", "name": "newState", "value": 4 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_message",
        "definition-layer positive Message record",
        &conformant_message_record(),
        r#"{
  "eventName": "Message",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000010,
    "runId": 1,
    "seq": 2,
    "sourceId": 66,
    "eventTypeId": "0x0003"
  },
  "slots": [
    { "key": "0x0014", "type": "UDInt", "name": "messageTemplateId", "value": 1001 },
    { "key": "0x0015", "type": "String", "name": "arg", "value": "phase ready" },
    { "key": "0x0008", "type": "UInt", "name": "severity", "value": 500 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_records_dropped",
        "definition-layer positive RecordsDropped record",
        &conformant_records_dropped_record(),
        r#"{
  "eventName": "RecordsDropped",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 0,
    "runId": 9,
    "seq": 100,
    "sourceId": 66,
    "eventTypeId": "0x0104"
  },
  "slots": [
    { "key": "0x0016", "type": "UDInt", "name": "count", "value": 18 },
    { "key": "0x0017", "type": "ULInt", "name": "firstLostSeq", "value": 40 },
    { "key": "0x0018", "type": "ULInt", "name": "lastLostSeq", "value": 57 },
    { "key": "0x0019", "type": "DateTime", "name": "windowStart", "value": 1000000000 },
    { "key": "0x001A", "type": "DateTime", "name": "windowEnd", "value": 1000010000 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_source_high_water",
        "definition-layer positive SourceHighWater record",
        &conformant_source_high_water_record(),
        r#"{
  "eventName": "SourceHighWater",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 0,
    "runId": 1,
    "seq": 5,
    "sourceId": 88,
    "eventTypeId": "0x0108"
  },
  "slots": [
    { "key": "0x0038", "type": "ULInt", "name": "producedCount", "value": 5 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "records_dropped",
        "canonical RecordsDropped record",
        &records_dropped_vector_record(),
        r#"{
  "eventName": "RecordsDropped",
  "fields": {
    "sourceTime": 0,
    "runId": 9,
    "seq": 44,
    "sourceId": 42,
    "eventTypeId": "0x0104"
  },
  "slots": [
    { "key": "0x0016", "type": "UDInt", "name": "droppedCount", "value": 5 },
    { "key": "0x0017", "type": "ULInt", "name": "firstLostSeq", "value": 100 },
    { "key": "0x0018", "type": "ULInt", "name": "lastLostSeq", "value": 104 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "source_high_water",
        "per-source SourceHighWater checkpoint",
        &source_high_water_record(),
        r#"{
  "eventName": "SourceHighWater",
  "fields": {
    "sourceTime": 0,
    "runId": 1,
    "seq": 5,
    "sourceId": 88,
    "eventTypeId": "0x0108"
  },
  "slots": [
    { "key": "0x0038", "type": "ULInt", "name": "producedCount", "value": 5 }
  ]
}"#,
    );

    push_control_block_vectors(&mut files);
    push_wrap_marker_boundary(&mut files);
    push_torn_record(&mut files);
    push_overflow_records_dropped(&mut files);

    files
}

fn push_record_vector(
    files: &mut Vec<VectorFile>,
    stem: &'static str,
    description: &str,
    record: &Record,
    detail_json: &str,
) {
    let bytes = record.encode(true).expect("record vector encodes");
    assert_eq!(
        decode(&bytes).expect("record vector decodes").record,
        record_with_crc(record.clone())
    );
    let crc = trailer_crc(&bytes);

    files.push(VectorFile {
        path: hex_path(stem),
        contents: hex_dump(&bytes),
    });
    files.push(VectorFile {
        path: json_path(stem),
        contents: format!(
            "{{
  \"name\": \"{stem}\",
  \"description\": \"{description}\",
  \"kind\": \"record\",
  \"encoding\": \"wire-v2\",
  \"expected\": \"accept\",
  \"hexFile\": \"{stem}.hex\",
  \"byteCount\": {},
  \"headerLength\": {},
  \"totalRecordLength\": {},
  \"flags\": \"0x{:04X}\",
  \"crc32c\": \"0x{crc:08X}\",
  \"detail\": {}
}}
",
            bytes.len(),
            HEADER_LEN,
            read_u16(&bytes, 4),
            read_u16(&bytes, 6),
            indent_json(detail_json, 2),
        ),
    });
}

fn push_control_block_vectors(files: &mut Vec<VectorFile>) {
    let snapshot = control_snapshot();
    let bytes = snapshot.encode(2);
    files.push(VectorFile {
        path: "control_block.hex",
        contents: hex_dump(&bytes),
    });
    files.push(VectorFile {
        path: "control_block.json",
        contents: format!(
            "{{
  \"name\": \"control_block\",
  \"kind\": \"control-block\",
  \"encoding\": \"open-ot-control-v2\",
  \"expected\": \"accept\",
  \"hexFile\": \"control_block.hex\",
  \"byteCount\": {},
  \"seqLock\": 2,
  \"fields\": {{
    \"bufferId\": {},
    \"bufferBytes\": {},
    \"headAbs\": {},
    \"oldestAbs\": {},
    \"lostCount\": {},
    \"runId\": {},
    \"epochId\": {},
    \"epochFirstAbs\": {},
    \"definitionHash\": \"{}\",
    \"prevDefinitionHash\": \"{}\"
  }}
}}
",
            CONTROL_BLOCK_LEN,
            snapshot.buffer_id,
            snapshot.buffer_bytes,
            snapshot.head_abs,
            snapshot.oldest_abs,
            snapshot.lost_count,
            snapshot.run_id,
            snapshot.epoch_id,
            snapshot.epoch_first_abs,
            hex_compact(&snapshot.definition_hash),
            hex_compact(&snapshot.prev_definition_hash),
        ),
    });

    let mut torn = bytes;
    torn[16..20].copy_from_slice(&3u32.to_le_bytes());
    files.push(VectorFile {
        path: "control_block_torn_snapshot.hex",
        contents: hex_dump(&torn),
    });
    files.push(VectorFile {
        path: "control_block_torn_snapshot.json",
        contents: r#"{
  "name": "control_block_torn_snapshot",
  "kind": "control-block",
  "encoding": "open-ot-control-v2",
  "expected": "reject",
  "expectedError": "Updating",
  "hexFile": "control_block_torn_snapshot.hex",
  "observedSeqLock": 3
}
"#
        .to_string(),
    });

    files.push(VectorFile {
        path: "control_block_stale_snapshot.hex",
        contents: hex_dump(&bytes),
    });
    files.push(VectorFile {
        path: "control_block_stale_snapshot.json",
        contents: r#"{
  "name": "control_block_stale_snapshot",
  "kind": "control-block",
  "encoding": "open-ot-control-v2",
  "expected": "reject",
  "expectedError": "StaleSnapshot",
  "hexFile": "control_block_stale_snapshot.hex",
  "observedSeqLockFirst": 2,
  "observedSeqLockSecond": 4
}
"#
        .to_string(),
    });

    let mut overwrite = snapshot;
    overwrite.oldest_abs = 256;
    let overwrite_bytes = overwrite.encode(4);
    files.push(VectorFile {
        path: "control_block_overwrite_snapshot.hex",
        contents: hex_dump(&overwrite_bytes),
    });
    files.push(VectorFile {
        path: "control_block_overwrite_snapshot.json",
        contents: r#"{
  "name": "control_block_overwrite_snapshot",
  "kind": "control-block",
  "encoding": "open-ot-control-v2",
  "expected": "reject-record",
  "reason": "OldestAbs > recordAbs",
  "hexFile": "control_block_overwrite_snapshot.hex",
  "recordAbs": 128,
  "oldestAbs": 256
}
"#
        .to_string(),
    });
}

fn push_wrap_marker_boundary(files: &mut Vec<VectorFile>) {
    let mut ring = RingBuffer::new(128);
    let source = 81;
    let (_, first_end) = ring
        .write_record(&minimal_record(source, 0))
        .expect("first boundary record writes");
    let (_, second_end) = ring
        .write_record(&minimal_record(source, 1))
        .expect("second boundary record writes");
    assert_eq!((first_end, second_end), (44, 88));

    let mut wrapped = minimal_record(source, 2);
    wrapped
        .slots
        .push(Slot::new(0x1000, TY_UINT, 0x2222u16.to_le_bytes()));
    assert_eq!(
        wrapped.encode(true).expect("wrapped record encodes").len(),
        52
    );
    let (third_start, third_end) = ring.write_record(&wrapped).expect("wrapped record writes");
    assert_eq!((third_start, third_end), (128, 180));
    assert_eq!(ring.physical_bytes()[88], 0);
    assert_eq!(&ring.physical_bytes()[0..4], &SYNC);

    let batch = ring.read_raw_from(88).expect("boundary fixture reads");
    assert!(batch.lapped);
    assert_eq!(batch.records.len(), 1);
    assert_eq!(batch.records[0].record.seq, 2);

    files.push(VectorFile {
        path: "wrap_marker_boundary.hex",
        contents: hex_dump(ring.physical_bytes()),
    });
    files.push(VectorFile {
        path: "wrap_marker_boundary.json",
        contents: format!(
            "{{
  \"name\": \"wrap_marker_boundary\",
  \"kind\": \"physical-ring-image\",
  \"encoding\": \"wire-v2-ring\",
  \"expected\": \"accept-after-lap\",
  \"hexFile\": \"wrap_marker_boundary.hex\",
  \"capacity\": {},
  \"headAbs\": {},
  \"oldestAbs\": {},
  \"wrapMarker\": {{ \"abs\": 88, \"phys\": 88, \"value\": \"0x00\" }},
  \"cursorAbs\": 88,
  \"expectedLapped\": true,
  \"expectedDelivered\": [
    {{ \"sourceId\": {}, \"seq\": 2, \"startAbs\": {}, \"endAbs\": {} }}
  ]
}}
",
            ring.capacity(),
            ring.head_abs(),
            ring.oldest_abs(),
            source,
            batch.records[0].start_abs,
            batch.records[0].end_abs,
        ),
    });
}

fn push_torn_record(files: &mut Vec<VectorFile>) {
    let mut record = minimal_record(73, 0);
    record
        .slots
        .push(Slot::new(0x1000, TY_UDINT, 123u32.to_le_bytes()));
    let complete = record.encode(true).expect("torn fixture source encodes");

    let copied = HEADER_LEN + 4;
    let mut torn = vec![0; complete.len()];
    torn[..copied].copy_from_slice(&complete[..copied]);
    torn[copied..].fill(0xA5);
    assert_eq!(&torn[0..4], &SYNC);
    assert_eq!(read_u16(&torn, 4) as usize, torn.len());
    assert!(matches!(decode(&torn), Err(WireError::CrcMismatch { .. })));

    files.push(VectorFile {
        path: "torn_record_must_reject.hex",
        contents: hex_dump(&torn),
    });
    files.push(VectorFile {
        path: "torn_record_must_reject.json",
        contents: format!(
            "{{
  \"name\": \"torn_record_must_reject\",
  \"kind\": \"record-fragment\",
  \"encoding\": \"wire-v2\",
  \"expected\": \"reject\",
  \"expectedError\": \"CrcMismatch\",
  \"hexFile\": \"torn_record_must_reject.hex\",
  \"byteCount\": {},
  \"advertisedTotalRecordLength\": {},
  \"copiedPrefixBytes\": {},
  \"fault\": \"valid sync and length, copied header plus slot header, unwritten tail filled with 0xA5\"
}}
",
            torn.len(),
            read_u16(&torn, 4),
            copied,
        ),
    });
}

fn push_overflow_records_dropped(files: &mut Vec<VectorFile>) {
    let silent_source = 88;
    let noisy_source = 99;
    let mut ring = RingBuffer::new(256);

    for seq in 0..5 {
        ring.write_record(&minimal_record(silent_source, seq))
            .expect("silent source record writes");
    }
    for seq in 0..80 {
        ring.write_record(&minimal_record(noisy_source, seq))
            .expect("noisy source record writes");
    }
    let loss = ring
        .take_producer_loss_ranges()
        .into_iter()
        .find(|range| range.source_id == silent_source)
        .expect("silent source loss should be retained by producer");
    assert_eq!(
        loss,
        LossRange {
            run_id: 1,
            source_id: silent_source,
            first_seq: 0,
            last_seq: 4,
        }
    );

    ring.write_record(&records_dropped_record(5, &loss))
        .expect("RecordsDropped record writes");
    let mut raw = RawByteConsumer::new();
    let batch = raw.poll(&ring).expect("overflow fixture reads");
    assert!(batch.lapped);
    assert_eq!(raw.lost_in_run(1, silent_source), 5);
    assert!(
        batch
            .records
            .iter()
            .any(|read| read.record.event_type_id == EVENT_RECORDS_DROPPED
                && read.record.source_id == silent_source)
    );

    let mut stream = Vec::new();
    for read in &batch.records {
        stream.extend(
            read.record
                .encode(true)
                .expect("overflow stream record encodes"),
        );
    }
    files.push(VectorFile {
        path: "overflow_records_dropped_sequence.hex",
        contents: hex_dump(&stream),
    });
    files.push(VectorFile {
        path: "overflow_records_dropped_sequence.json",
        contents: format!(
            "{{
  \"name\": \"overflow_records_dropped_sequence\",
  \"kind\": \"logical-record-stream\",
  \"encoding\": \"wire-v2\",
  \"expected\": \"accept\",
  \"hexFile\": \"overflow_records_dropped_sequence.hex\",
  \"ringCapacity\": {},
  \"batchLapped\": true,
  \"recordsInStream\": {},
  \"producerLoss\": {{
    \"runId\": {},
    \"sourceId\": {},
    \"firstSeq\": {},
    \"lastSeq\": {},
    \"count\": {}
  }},
  \"expectedAccounting\": {{
    \"sourceId\": {},
    \"lost\": 5,
    \"deliveredRecordsDroppedRecord\": true
  }}
}}
",
            ring.capacity(),
            batch.records.len(),
            loss.run_id,
            loss.source_id,
            loss.first_seq,
            loss.last_seq,
            loss.count(),
            silent_source,
        ),
    });
}

fn readme() -> String {
    r#"# OpenOT wire-v2 conformance vectors

These files are generated, not hand-written. Regenerate them with:

```sh
cargo run -p open-ot-carriage --bin dump_vectors
```

`cargo test` compares the checked-in files with the generator output, so byte
fixture drift is a test failure. `.hex` files are byte-exact hexadecimal dumps.
The adjacent `.json` files describe the expected interpretation or rejection.

Files named `conformant_*` are definition-layer positive record fixtures. The
older codec fixtures remain valid wire records; where their `.json` marks
`schemaExpected: reject`, they are intended as future schema-violation negatives.
"#
    .to_string()
}

fn control_snapshot() -> ControlBlockSnapshot {
    ControlBlockSnapshot {
        version: 2,
        caps: 0b0000_0011,
        buffer_id: 7,
        buffer_bytes: 4096,
        head_abs: 8192,
        oldest_abs: 6144,
        lost_count: 12,
        run_id: 3,
        epoch_id: 5,
        epoch_first_abs: 7000,
        definition_hash: *b"defhash1",
        prev_definition_hash: *b"oldhash1",
    }
}

fn state_transition_record() -> Record {
    let mut record = Record::new(0x0102_0304_0506_0708, 1, 2, 42, EVENT_STATE_TRANSITION);
    record
        .slots
        .push(Slot::new(0x0001, TY_UINT, 0x1234u16.to_le_bytes()));
    record
        .slots
        .push(Slot::new(0x0002, TY_UINT, 2u16.to_le_bytes()));
    record
}

fn conformant_state_transition_record() -> Record {
    let mut record = Record::new(1_000_000_000, 1, 1, 66, EVENT_STATE_TRANSITION);
    record.slots.push(Slot::new(
        KEY_STATE_MACHINE_ID,
        TY_UDINT,
        7u32.to_le_bytes(),
    ));
    record
        .slots
        .push(Slot::new(KEY_CATEGORY, TY_UINT, 2u16.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_PREVIOUS_STATE, TY_UINT, 3u16.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_NEW_STATE, TY_UINT, 4u16.to_le_bytes()));
    record
}

fn conformant_message_record() -> Record {
    let mut record = Record::new(1_000_000_010, 1, 2, 66, EVENT_MESSAGE);
    record.slots.push(Slot::new(
        KEY_MESSAGE_TEMPLATE_ID,
        TY_UDINT,
        1001u32.to_le_bytes(),
    ));
    record
        .slots
        .push(Slot::new(KEY_ARG, TY_STRING, b"phase ready"));
    record
        .slots
        .push(Slot::new(KEY_SEVERITY, TY_UINT, 500u16.to_le_bytes()));
    record
}

fn conformant_records_dropped_record() -> Record {
    let mut record = Record::new(0, 9, 100, 66, EVENT_RECORDS_DROPPED);
    record
        .slots
        .push(Slot::new(KEY_DROPPED_COUNT, TY_UDINT, 18u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_FIRST_LOST_SEQ, TY_ULINT, 40u64.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_LAST_LOST_SEQ, TY_ULINT, 57u64.to_le_bytes()));
    record.slots.push(Slot::new(
        KEY_WINDOW_START,
        TY_DATE_TIME,
        1_000_000_000u64.to_le_bytes(),
    ));
    record.slots.push(Slot::new(
        KEY_WINDOW_END,
        TY_DATE_TIME,
        1_000_010_000u64.to_le_bytes(),
    ));
    record
}

fn conformant_source_high_water_record() -> Record {
    source_high_water_record()
}

fn records_dropped_vector_record() -> Record {
    let mut record = Record::new(0, 9, 44, 42, EVENT_RECORDS_DROPPED);
    record
        .slots
        .push(Slot::new(KEY_DROPPED_COUNT, TY_UDINT, 5u32.to_le_bytes()));
    record.slots.push(Slot::new(
        KEY_FIRST_LOST_SEQ,
        TY_ULINT,
        100u64.to_le_bytes(),
    ));
    record
        .slots
        .push(Slot::new(KEY_LAST_LOST_SEQ, TY_ULINT, 104u64.to_le_bytes()));
    record
}

fn source_high_water_record() -> Record {
    let mut record = Record::new(0, 1, 5, 88, EVENT_SOURCE_HIGH_WATER);
    record.slots.push(Slot::new(
        KEY_SOURCE_HIGH_WATER,
        TY_ULINT,
        5u64.to_le_bytes(),
    ));
    record
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

fn record_with_crc(mut record: Record) -> Record {
    record.flags |= FLAG_HAS_CRC;
    record
}

fn trailer_crc(bytes: &[u8]) -> u32 {
    let start = bytes.len() - 4;
    u32::from_le_bytes(bytes[start..].try_into().expect("crc trailer length"))
}

fn hex_path(stem: &'static str) -> &'static str {
    match stem {
        "state_transition" => "state_transition.hex",
        "conformant_state_transition" => "conformant_state_transition.hex",
        "conformant_message" => "conformant_message.hex",
        "conformant_records_dropped" => "conformant_records_dropped.hex",
        "conformant_source_high_water" => "conformant_source_high_water.hex",
        "records_dropped" => "records_dropped.hex",
        "source_high_water" => "source_high_water.hex",
        _ => unreachable!("unknown vector stem"),
    }
}

fn json_path(stem: &'static str) -> &'static str {
    match stem {
        "state_transition" => "state_transition.json",
        "conformant_state_transition" => "conformant_state_transition.json",
        "conformant_message" => "conformant_message.json",
        "conformant_records_dropped" => "conformant_records_dropped.json",
        "conformant_source_high_water" => "conformant_source_high_water.json",
        "records_dropped" => "records_dropped.json",
        "source_high_water" => "source_high_water.json",
        _ => unreachable!("unknown vector stem"),
    }
}

fn hex_compact(bytes: &[u8]) -> String {
    let mut out = String::new();
    for byte in bytes {
        write!(&mut out, "{byte:02X}").expect("write to string");
    }
    out
}

fn hex_dump(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            if index % 16 == 0 {
                out.push('\n');
            } else {
                out.push(' ');
            }
        }
        write!(&mut out, "{byte:02X}").expect("write to string");
    }
    out.push('\n');
    out
}

fn indent_json(input: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    input
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
        .trim_start()
        .to_string()
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conformant_record_vectors_match_phase2_contracts() {
        assert_slots(
            &conformant_state_transition_record(),
            &[
                (KEY_STATE_MACHINE_ID, TY_UDINT, 4),
                (KEY_CATEGORY, TY_UINT, 2),
                (KEY_PREVIOUS_STATE, TY_UINT, 2),
                (KEY_NEW_STATE, TY_UINT, 2),
            ],
        );
        assert_slots(
            &conformant_message_record(),
            &[
                (KEY_MESSAGE_TEMPLATE_ID, TY_UDINT, 4),
                (KEY_ARG, TY_STRING, "phase ready".len()),
                (KEY_SEVERITY, TY_UINT, 2),
            ],
        );
        assert_slots(
            &conformant_records_dropped_record(),
            &[
                (KEY_DROPPED_COUNT, TY_UDINT, 4),
                (KEY_FIRST_LOST_SEQ, TY_ULINT, 8),
                (KEY_LAST_LOST_SEQ, TY_ULINT, 8),
                (KEY_WINDOW_START, TY_DATE_TIME, 8),
                (KEY_WINDOW_END, TY_DATE_TIME, 8),
            ],
        );
        assert_slots(
            &conformant_source_high_water_record(),
            &[(KEY_SOURCE_HIGH_WATER, TY_ULINT, 8)],
        );
    }

    #[test]
    fn codec_state_transition_vector_is_schema_negative() {
        let record = state_transition_record();
        assert_eq!(record.event_type_id, EVENT_STATE_TRANSITION);
        assert_slots(
            &record,
            &[
                (KEY_STATE_MACHINE_ID, TY_UINT, 2),
                (KEY_CATEGORY, TY_UINT, 2),
            ],
        );
    }

    #[test]
    fn vectors_directory_matches_generator() -> io::Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("vectors");
        for generated in generate_files() {
            let actual = fs::read_to_string(root.join(generated.path))?;
            assert_eq!(
                actual, generated.contents,
                "stale vector {}",
                generated.path
            );
        }
        Ok(())
    }

    fn assert_slots(record: &Record, expected: &[(u16, u8, usize)]) {
        let actual = record
            .slots
            .iter()
            .map(|slot| (slot.key, slot.ty, slot.payload.len()))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}
