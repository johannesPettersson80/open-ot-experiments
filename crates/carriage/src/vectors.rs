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
    EVENT_BATCH_EVENT, EVENT_CONDITION_ACKNOWLEDGED, EVENT_CONDITION_ACTIVE,
    EVENT_CONDITION_CLEARED, EVENT_CONDITION_COMMENTED, EVENT_CONDITION_CONFIRMED,
    EVENT_CONDITION_IN_SERVICE, EVENT_CONDITION_OUT_OF_SERVICE, EVENT_CONDITION_PRIORITY_CHANGED,
    EVENT_CONDITION_RESET, EVENT_CONDITION_SHELVED, EVENT_CONDITION_SUPPRESSED,
    EVENT_CONDITION_UNSHELVED, EVENT_CONDITION_UNSUPPRESSED, EVENT_DEFINITION_CHANGED,
    EVENT_ESIGNATURE, EVENT_LOGGER_STARTED, EVENT_LOGGER_STOPPED, EVENT_MATERIAL_ADDITION,
    EVENT_MESSAGE, EVENT_OPERATOR_ACTION, EVENT_OPERATOR_LOGIN, EVENT_OPERATOR_LOGOUT,
    EVENT_PARAMETER_CHANGE, EVENT_RECIPE_APPROVED, EVENT_RECIPE_LOADED,
    EVENT_SECURITY_ACCESS_FAILURE, EVENT_SOURCE_HIGH_WATER, EVENT_STATE_TRANSITION,
    EVENT_VALUE_CHANGED, KEY_ACK_BY, KEY_ACTION_ID, KEY_ACTOR, KEY_ARG, KEY_AUTH_RESULT,
    KEY_BATCH_ID, KEY_CATEGORY, KEY_CAUSE_OPERAND, KEY_COLD_START, KEY_COMMENT,
    KEY_CONDITION_CLASS, KEY_CONDITION_ID, KEY_CONTEXT_REF, KEY_CORRELATION_ID, KEY_DEF_HASH_NEW,
    KEY_DEF_HASH_OLD, KEY_DROPPED_COUNT, KEY_EPOCH_ID, KEY_FIRST_LOST_SEQ, KEY_LAST_LOST_SEQ,
    KEY_MATERIAL_ID, KEY_MESSAGE_TEMPLATE_ID, KEY_NEW_PRIORITY, KEY_NEW_STATE, KEY_NEW_VALUE,
    KEY_PREVIOUS_PRIORITY, KEY_PREVIOUS_STATE, KEY_PREVIOUS_VALUE, KEY_QUALITY, KEY_QUANTITY,
    KEY_REASON, KEY_RECIPE_ID, KEY_RECIPE_VERSION, KEY_ROLE, KEY_SEVERITY, KEY_SHELVE_SECS,
    KEY_SIGNATURE_MEANING, KEY_SIGNED_EVENT_SEQ, KEY_SOURCE_HIGH_WATER, KEY_STATE_MACHINE_ID,
    KEY_UNIT, KEY_VALUE_ID, KEY_WINDOW_END, KEY_WINDOW_START, KEY_WORKSTATION, SYSTEM_SOURCE_ID,
    TY_BOOL, TY_BYTES, TY_DATE_TIME, TY_DINT, TY_INT, TY_LINT, TY_LREAL, TY_REAL, TY_SINT,
    TY_STRING, TY_UDINT, TY_UINT, TY_ULINT, TY_USINT,
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
        "conformant_value_changed_real",
        "definition-layer positive ValueChanged record carrying a REAL newValue",
        &conformant_value_changed_real_record(),
        r#"{
  "eventName": "ValueChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000020,
    "runId": 1,
    "seq": 3,
    "sourceId": 66,
    "eventTypeId": "0x0002"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2001 },
    { "key": "0x0010", "type": "Real", "name": "newValue", "value": 12.5 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_value_changed_dint",
        "definition-layer positive ValueChanged record carrying DINT previousValue/newValue plus quality",
        &conformant_value_changed_dint_record(),
        r#"{
  "eventName": "ValueChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000030,
    "runId": 1,
    "seq": 4,
    "sourceId": 66,
    "eventTypeId": "0x0002"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2002 },
    { "key": "0x0010", "type": "DInt", "name": "newValue", "value": 42 },
    { "key": "0x000F", "type": "DInt", "name": "previousValue", "value": 40 },
    { "key": "0x0011", "type": "UInt", "name": "quality", "value": 0 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_value_changed_bool",
        "definition-layer positive ValueChanged record carrying a BOOL newValue",
        &conformant_value_changed_bool_record(),
        r#"{
  "eventName": "ValueChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000040,
    "runId": 1,
    "seq": 5,
    "sourceId": 66,
    "eventTypeId": "0x0002"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2003 },
    { "key": "0x0010", "type": "Bool", "name": "newValue", "value": true }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_value_changed_sint",
        "definition-layer positive ValueChanged record carrying an SINT newValue",
        &conformant_value_changed_sint_record(),
        r#"{
  "eventName": "ValueChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000050,
    "runId": 1,
    "seq": 6,
    "sourceId": 66,
    "eventTypeId": "0x0002"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2004 },
    { "key": "0x0010", "type": "SInt", "name": "newValue", "value": -5 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_value_changed_usint",
        "definition-layer positive ValueChanged record carrying a USINT newValue",
        &conformant_value_changed_usint_record(),
        r#"{
  "eventName": "ValueChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000060,
    "runId": 1,
    "seq": 7,
    "sourceId": 66,
    "eventTypeId": "0x0002"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2005 },
    { "key": "0x0010", "type": "USInt", "name": "newValue", "value": 250 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_value_changed_int",
        "definition-layer positive ValueChanged record carrying an INT newValue",
        &conformant_value_changed_int_record(),
        r#"{
  "eventName": "ValueChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000070,
    "runId": 1,
    "seq": 8,
    "sourceId": 66,
    "eventTypeId": "0x0002"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2006 },
    { "key": "0x0010", "type": "Int", "name": "newValue", "value": -1234 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_value_changed_uint",
        "definition-layer positive ValueChanged record carrying a UINT newValue",
        &conformant_value_changed_uint_record(),
        r#"{
  "eventName": "ValueChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000080,
    "runId": 1,
    "seq": 9,
    "sourceId": 66,
    "eventTypeId": "0x0002"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2007 },
    { "key": "0x0010", "type": "UInt", "name": "newValue", "value": 1234 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_value_changed_udint",
        "definition-layer positive ValueChanged record carrying a UDINT newValue",
        &conformant_value_changed_udint_record(),
        r#"{
  "eventName": "ValueChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000090,
    "runId": 1,
    "seq": 10,
    "sourceId": 66,
    "eventTypeId": "0x0002"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2008 },
    { "key": "0x0010", "type": "UDInt", "name": "newValue", "value": 123456 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_value_changed_ulint",
        "definition-layer positive ValueChanged record carrying a ULINT newValue",
        &conformant_value_changed_ulint_record(),
        r#"{
  "eventName": "ValueChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000100,
    "runId": 1,
    "seq": 11,
    "sourceId": 66,
    "eventTypeId": "0x0002"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2009 },
    { "key": "0x0010", "type": "ULInt", "name": "newValue", "value": 1234567890123 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_value_changed_lint",
        "definition-layer positive ValueChanged record carrying an LINT newValue",
        &conformant_value_changed_lint_record(),
        r#"{
  "eventName": "ValueChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000110,
    "runId": 1,
    "seq": 12,
    "sourceId": 66,
    "eventTypeId": "0x0002"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2010 },
    { "key": "0x0010", "type": "LInt", "name": "newValue", "value": -1234567890123 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_value_changed_lreal",
        "definition-layer positive ValueChanged record carrying an LREAL newValue",
        &conformant_value_changed_lreal_record(),
        r#"{
  "eventName": "ValueChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000120,
    "runId": 1,
    "seq": 13,
    "sourceId": 66,
    "eventTypeId": "0x0002"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2011 },
    { "key": "0x0010", "type": "LReal", "name": "newValue", "value": 12.25 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_value_changed_string",
        "definition-layer positive ValueChanged record carrying a STRING newValue",
        &conformant_value_changed_string_record(),
        r#"{
  "eventName": "ValueChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000130,
    "runId": 1,
    "seq": 14,
    "sourceId": 66,
    "eventTypeId": "0x0002"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2012 },
    { "key": "0x0010", "type": "String", "name": "newValue", "value": "ready" }
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
        "conformant_condition_active",
        "definition-layer positive ConditionActive record carrying correlation and cause operand",
        &conformant_condition_active_record(),
        r#"{
  "eventName": "ConditionActive",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000140,
    "runId": 1,
    "seq": 15,
    "sourceId": 66,
    "eventTypeId": "0x0200"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 },
    { "key": "0x0006", "type": "UInt", "name": "conditionClass", "value": 0 },
    { "key": "0x0007", "type": "UDInt", "name": "correlationId", "value": 77 },
    { "key": "0x0008", "type": "UInt", "name": "severity", "value": 900 },
    { "key": "0x0009", "type": "UDInt", "name": "causeOperand", "value": 1 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_condition_cleared",
        "definition-layer positive ConditionCleared record echoing activation correlation",
        &conformant_condition_cleared_record(),
        r#"{
  "eventName": "ConditionCleared",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000150,
    "runId": 1,
    "seq": 16,
    "sourceId": 66,
    "eventTypeId": "0x0201"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 },
    { "key": "0x0006", "type": "UInt", "name": "conditionClass", "value": 0 },
    { "key": "0x0007", "type": "UDInt", "name": "correlationId", "value": 77 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_condition_acknowledged",
        "definition-layer positive ConditionAcknowledged record with ackBy",
        &conformant_condition_acknowledged_record(),
        r#"{
  "eventName": "ConditionAcknowledged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000160,
    "runId": 1,
    "seq": 17,
    "sourceId": 66,
    "eventTypeId": "0x0202"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 },
    { "key": "0x0007", "type": "UDInt", "name": "correlationId", "value": 77 },
    { "key": "0x001D", "type": "String", "name": "ackBy", "value": "operator-a" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_condition_confirmed",
        "definition-layer positive ConditionConfirmed record with ackBy",
        &conformant_condition_confirmed_record(),
        r#"{
  "eventName": "ConditionConfirmed",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000200,
    "runId": 1,
    "seq": 21,
    "sourceId": 66,
    "eventTypeId": "0x0203"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 },
    { "key": "0x0007", "type": "UDInt", "name": "correlationId", "value": 77 },
    { "key": "0x001D", "type": "String", "name": "ackBy", "value": "operator-a" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_condition_shelved",
        "definition-layer positive ConditionShelved record with ackBy and shelveSecs",
        &conformant_condition_shelved_record(),
        r#"{
  "eventName": "ConditionShelved",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000170,
    "runId": 1,
    "seq": 18,
    "sourceId": 66,
    "eventTypeId": "0x0204"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 },
    { "key": "0x0007", "type": "UDInt", "name": "correlationId", "value": 77 },
    { "key": "0x001D", "type": "String", "name": "ackBy", "value": "operator-a" },
    { "key": "0x001E", "type": "UDInt", "name": "shelveSecs", "value": 300 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_condition_unshelved",
        "definition-layer positive ConditionUnshelved record with activation correlation",
        &conformant_condition_unshelved_record(),
        r#"{
  "eventName": "ConditionUnshelved",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000210,
    "runId": 1,
    "seq": 22,
    "sourceId": 66,
    "eventTypeId": "0x0205"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 },
    { "key": "0x0007", "type": "UDInt", "name": "correlationId", "value": 77 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_condition_suppressed",
        "definition-layer positive ConditionSuppressed record with reason and no correlation",
        &conformant_condition_suppressed_record(),
        r#"{
  "eventName": "ConditionSuppressed",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000180,
    "runId": 1,
    "seq": 19,
    "sourceId": 66,
    "eventTypeId": "0x0206"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 },
    { "key": "0x001F", "type": "String", "name": "reason", "value": "maintenance" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_condition_unsuppressed",
        "definition-layer positive ConditionUnsuppressed record with no correlation",
        &conformant_condition_unsuppressed_record(),
        r#"{
  "eventName": "ConditionUnsuppressed",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000220,
    "runId": 1,
    "seq": 23,
    "sourceId": 66,
    "eventTypeId": "0x0207"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_condition_out_of_service",
        "definition-layer positive ConditionOutOfService record with ackBy and no correlation",
        &conformant_condition_out_of_service_record(),
        r#"{
  "eventName": "ConditionOutOfService",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000190,
    "runId": 1,
    "seq": 20,
    "sourceId": 66,
    "eventTypeId": "0x0208"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 },
    { "key": "0x001D", "type": "String", "name": "ackBy", "value": "operator-a" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_condition_in_service",
        "definition-layer positive ConditionInService record with no correlation",
        &conformant_condition_in_service_record(),
        r#"{
  "eventName": "ConditionInService",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000230,
    "runId": 1,
    "seq": 24,
    "sourceId": 66,
    "eventTypeId": "0x0209"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_condition_reset",
        "definition-layer positive ConditionReset record with ackBy",
        &conformant_condition_reset_record(),
        r#"{
  "eventName": "ConditionReset",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000240,
    "runId": 1,
    "seq": 25,
    "sourceId": 66,
    "eventTypeId": "0x020B"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 },
    { "key": "0x0007", "type": "UDInt", "name": "correlationId", "value": 77 },
    { "key": "0x001D", "type": "String", "name": "ackBy", "value": "operator-a" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_condition_commented",
        "definition-layer positive ConditionCommented record with required comment before ackBy",
        &conformant_condition_commented_record(),
        r#"{
  "eventName": "ConditionCommented",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000250,
    "runId": 1,
    "seq": 26,
    "sourceId": 66,
    "eventTypeId": "0x020A"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 },
    { "key": "0x0007", "type": "UDInt", "name": "correlationId", "value": 77 },
    { "key": "0x0037", "type": "String", "name": "comment", "value": "operator comment" },
    { "key": "0x001D", "type": "String", "name": "ackBy", "value": "operator-a" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_condition_priority_changed",
        "definition-layer positive ConditionPriorityChanged record with required priority fields before ackBy",
        &conformant_condition_priority_changed_record(),
        r#"{
  "eventName": "ConditionPriorityChanged",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000260,
    "runId": 1,
    "seq": 27,
    "sourceId": 66,
    "eventTypeId": "0x020C"
  },
  "slots": [
    { "key": "0x0005", "type": "UDInt", "name": "conditionId", "value": 9001 },
    { "key": "0x002E", "type": "UInt", "name": "previousPriority", "value": 600 },
    { "key": "0x002C", "type": "UInt", "name": "newPriority", "value": 900 },
    { "key": "0x001D", "type": "String", "name": "ackBy", "value": "operator-a" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_recipe_loaded",
        "definition-layer positive RecipeLoaded record",
        &conformant_recipe_loaded_record(),
        r#"{
  "eventName": "RecipeLoaded",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000270,
    "runId": 1,
    "seq": 28,
    "sourceId": 66,
    "eventTypeId": "0x0301"
  },
  "slots": [
    { "key": "0x0023", "type": "UDInt", "name": "recipeId", "value": 3001 },
    { "key": "0x0024", "type": "String", "name": "recipeVersion", "value": "v1.2.3" },
    { "key": "0x0025", "type": "UDInt", "name": "batchId", "value": 4001 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_recipe_approved",
        "definition-layer positive RecipeApproved record",
        &conformant_recipe_approved_record(),
        r#"{
  "eventName": "RecipeApproved",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000280,
    "runId": 1,
    "seq": 29,
    "sourceId": 66,
    "eventTypeId": "0x0302"
  },
  "slots": [
    { "key": "0x0023", "type": "UDInt", "name": "recipeId", "value": 3001 },
    { "key": "0x0024", "type": "String", "name": "recipeVersion", "value": "v1.2.3" },
    { "key": "0x0020", "type": "UInt", "name": "authResult", "value": 1 },
    { "key": "0x001D", "type": "String", "name": "ackBy", "value": "approver-a" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_batch_event",
        "definition-layer positive BatchEvent record",
        &conformant_batch_event_record(),
        r#"{
  "eventName": "BatchEvent",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000290,
    "runId": 1,
    "seq": 30,
    "sourceId": 66,
    "eventTypeId": "0x0303"
  },
  "slots": [
    { "key": "0x0025", "type": "UDInt", "name": "batchId", "value": 4001 },
    { "key": "0x0004", "type": "UInt", "name": "newState", "value": 2 },
    { "key": "0x0023", "type": "UDInt", "name": "recipeId", "value": 3001 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_material_addition",
        "definition-layer positive MaterialAddition record",
        &conformant_material_addition_record(),
        r#"{
  "eventName": "MaterialAddition",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000300,
    "runId": 1,
    "seq": 31,
    "sourceId": 66,
    "eventTypeId": "0x0304"
  },
  "slots": [
    { "key": "0x0025", "type": "UDInt", "name": "batchId", "value": 4001 },
    { "key": "0x0026", "type": "UDInt", "name": "materialId", "value": 5001 },
    { "key": "0x0027", "type": "LReal", "name": "quantity", "value": 12.25 },
    { "key": "0x0013", "type": "UInt", "name": "unit", "value": 8 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_operator_action",
        "definition-layer positive OperatorAction record",
        &conformant_operator_action_record(),
        r#"{
  "eventName": "OperatorAction",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000310,
    "runId": 1,
    "seq": 32,
    "sourceId": 66,
    "eventTypeId": "0x0400"
  },
  "slots": [
    { "key": "0x000A", "type": "UDInt", "name": "actionId", "value": 6001 },
    { "key": "0x000B", "type": "String", "name": "actor", "value": "operator-a" },
    { "key": "0x000C", "type": "UDInt", "name": "contextRef", "value": 7001 },
    { "key": "0x000C", "type": "UDInt", "name": "contextRef", "value": 7002 },
    { "key": "0x0020", "type": "UInt", "name": "authResult", "value": 0 },
    { "key": "0x0021", "type": "String", "name": "workstation", "value": "station-1" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_operator_login",
        "definition-layer positive OperatorLogin record",
        &conformant_operator_login_record(),
        r#"{
  "eventName": "OperatorLogin",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000320,
    "runId": 1,
    "seq": 33,
    "sourceId": 66,
    "eventTypeId": "0x0401"
  },
  "slots": [
    { "key": "0x000B", "type": "String", "name": "actor", "value": "operator-a" },
    { "key": "0x0020", "type": "UInt", "name": "authResult", "value": 0 },
    { "key": "0x0021", "type": "String", "name": "workstation", "value": "station-1" },
    { "key": "0x0033", "type": "UInt", "name": "role", "value": 3 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_operator_logout",
        "definition-layer positive OperatorLogout record",
        &conformant_operator_logout_record(),
        r#"{
  "eventName": "OperatorLogout",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000330,
    "runId": 1,
    "seq": 34,
    "sourceId": 66,
    "eventTypeId": "0x0402"
  },
  "slots": [
    { "key": "0x000B", "type": "String", "name": "actor", "value": "operator-a" },
    { "key": "0x0021", "type": "String", "name": "workstation", "value": "station-1" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_security_access_failure",
        "definition-layer positive SecurityAccessFailure record",
        &conformant_security_access_failure_record(),
        r#"{
  "eventName": "SecurityAccessFailure",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000340,
    "runId": 1,
    "seq": 35,
    "sourceId": 66,
    "eventTypeId": "0x0405"
  },
  "slots": [
    { "key": "0x000B", "type": "String", "name": "actor", "value": "operator-x" },
    { "key": "0x0021", "type": "String", "name": "workstation", "value": "station-2" },
    { "key": "0x001F", "type": "String", "name": "reason", "value": "denied" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_parameter_change_real",
        "definition-layer positive ParameterChange record carrying REAL previous/new values",
        &conformant_parameter_change_real_record(),
        r#"{
  "eventName": "ParameterChange",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000350,
    "runId": 1,
    "seq": 36,
    "sourceId": 66,
    "eventTypeId": "0x0403"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2001 },
    { "key": "0x000F", "type": "Real", "name": "previousValue", "value": 12.5 },
    { "key": "0x0010", "type": "Real", "name": "newValue", "value": 13.75 },
    { "key": "0x000B", "type": "String", "name": "actor", "value": "operator-a" },
    { "key": "0x001F", "type": "String", "name": "reason", "value": "setpoint change" },
    { "key": "0x0020", "type": "UInt", "name": "authResult", "value": 0 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_parameter_change_dint",
        "definition-layer positive ParameterChange record carrying DINT previous/new values",
        &conformant_parameter_change_dint_record(),
        r#"{
  "eventName": "ParameterChange",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000360,
    "runId": 1,
    "seq": 37,
    "sourceId": 66,
    "eventTypeId": "0x0403"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2002 },
    { "key": "0x000F", "type": "DInt", "name": "previousValue", "value": 40 },
    { "key": "0x0010", "type": "DInt", "name": "newValue", "value": 42 },
    { "key": "0x000B", "type": "String", "name": "actor", "value": "operator-a" },
    { "key": "0x001F", "type": "String", "name": "reason", "value": "count corrected" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_parameter_change_bool",
        "definition-layer positive ParameterChange record carrying BOOL previous/new values through the bits path",
        &conformant_parameter_change_bool_record(),
        r#"{
  "eventName": "ParameterChange",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000370,
    "runId": 1,
    "seq": 38,
    "sourceId": 66,
    "eventTypeId": "0x0403"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2003 },
    { "key": "0x000F", "type": "Bool", "name": "previousValue", "value": false },
    { "key": "0x0010", "type": "Bool", "name": "newValue", "value": true },
    { "key": "0x000B", "type": "String", "name": "actor", "value": "operator-b" },
    { "key": "0x001F", "type": "String", "name": "reason", "value": "enable audit" }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_parameter_change_string",
        "definition-layer positive ParameterChange record carrying STRING previous/new values",
        &conformant_parameter_change_string_record(),
        r#"{
  "eventName": "ParameterChange",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000380,
    "runId": 1,
    "seq": 39,
    "sourceId": 66,
    "eventTypeId": "0x0403"
  },
  "slots": [
    { "key": "0x000D", "type": "UDInt", "name": "valueId", "value": 2012 },
    { "key": "0x000F", "type": "String", "name": "previousValue", "value": "manual" },
    { "key": "0x0010", "type": "String", "name": "newValue", "value": "auto" },
    { "key": "0x000B", "type": "String", "name": "actor", "value": "operator-c" },
    { "key": "0x001F", "type": "String", "name": "reason", "value": "mode note" },
    { "key": "0x0020", "type": "UInt", "name": "authResult", "value": 1 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "conformant_e_signature",
        "definition-layer positive ESignature record",
        &conformant_e_signature_record(),
        r#"{
  "eventName": "ESignature",
  "schemaExpected": "accept",
  "fields": {
    "sourceTime": 1000000390,
    "runId": 1,
    "seq": 40,
    "sourceId": 66,
    "eventTypeId": "0x0404"
  },
  "slots": [
    { "key": "0x000A", "type": "UDInt", "name": "actionId", "value": 6002 },
    { "key": "0x000B", "type": "String", "name": "actor", "value": "signer-a" },
    { "key": "0x0022", "type": "UInt", "name": "signatureMeaning", "value": 2 },
    { "key": "0x002F", "type": "ULInt", "name": "signedEventSeq", "value": 37 },
    { "key": "0x0020", "type": "UInt", "name": "authResult", "value": 0 }
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

    push_record_vector(
        &mut files,
        "minimal_message",
        "minimal valid Message record carrying a messageTemplateId",
        &minimal_message_record(),
        r#"{
  "eventName": "Message",
  "fields": {
    "sourceTime": 1780000000000000003,
    "runId": 1,
    "seq": 3,
    "sourceId": 7,
    "eventTypeId": "0x0003"
  },
  "slots": [
    { "key": "0x0014", "type": "UDInt", "name": "messageTemplateId", "value": 1001 }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "logger_stopped",
        "LoggerStopped lifecycle record",
        &logger_stopped_record(),
        r#"{
  "eventName": "LoggerStopped",
  "systemSourceSeq": 0,
  "fields": {
    "sourceTime": 0,
    "runId": 1,
    "seq": 0,
    "sourceId": 0,
    "eventTypeId": "0x0102"
  },
  "slots": []
}"#,
    );

    push_record_vector(
        &mut files,
        "definition_changed",
        "DefinitionChanged lifecycle record with epoch.rs emission slot order",
        &definition_changed_record(),
        r#"{
  "eventName": "DefinitionChanged",
  "systemSourceSeq": 1,
  "fields": {
    "sourceTime": 0,
    "runId": 1,
    "seq": 1,
    "sourceId": 0,
    "eventTypeId": "0x0106"
  },
  "slots": [
    { "key": "0x0039", "type": "Bytes", "name": "defHashOld", "value": "oldhash1", "order": 1 },
    { "key": "0x001C", "type": "Bytes", "name": "defHashNew", "value": "newhash2", "order": 2 },
    { "key": "0x003A", "type": "ULInt", "name": "epochId", "value": 2, "order": 3 }
  ],
  "slotOrderNote": "Matches epoch.rs emit_definition_changed: old, new, epoch; not numeric key order."
}"#,
    );

    push_record_vector(
        &mut files,
        "logger_started_warm",
        "LoggerStarted warm lifecycle record",
        &logger_started_warm_record(),
        r#"{
  "eventName": "LoggerStarted",
  "systemSourceSeq": 2,
  "fields": {
    "sourceTime": 0,
    "runId": 1,
    "seq": 2,
    "sourceId": 0,
    "eventTypeId": "0x0101"
  },
  "slots": [
    { "key": "0x003B", "type": "Bool", "name": "coldStart", "value": false }
  ]
}"#,
    );

    push_record_vector(
        &mut files,
        "logger_started_cold",
        "LoggerStarted cold lifecycle record after system sequence reset",
        &logger_started_cold_record(),
        r#"{
  "eventName": "LoggerStarted",
  "systemSourceSeq": 0,
  "fields": {
    "sourceTime": 0,
    "runId": 2,
    "seq": 0,
    "sourceId": 0,
    "eventTypeId": "0x0101"
  },
  "slots": [
    { "key": "0x003B", "type": "Bool", "name": "coldStart", "value": true }
  ],
  "systemSeqNote": "finish_cold_start resets system_seq before emitting LoggerStarted."
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

fn conformant_value_changed_real_record() -> Record {
    let mut record = Record::new(1_000_000_020, 1, 3, 66, EVENT_VALUE_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_NEW_VALUE, TY_REAL, 12.5f32.to_le_bytes()));
    record
}

fn conformant_value_changed_dint_record() -> Record {
    let mut record = Record::new(1_000_000_030, 1, 4, 66, EVENT_VALUE_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2002u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_NEW_VALUE, TY_DINT, 42i32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_PREVIOUS_VALUE, TY_DINT, 40i32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_QUALITY, TY_UINT, 0u16.to_le_bytes()));
    record
}

fn conformant_value_changed_bool_record() -> Record {
    let mut record = Record::new(1_000_000_040, 1, 5, 66, EVENT_VALUE_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2003u32.to_le_bytes()));
    record.slots.push(Slot::new(KEY_NEW_VALUE, TY_BOOL, [1]));
    record
}

fn conformant_value_changed_sint_record() -> Record {
    let mut record = Record::new(1_000_000_050, 1, 6, 66, EVENT_VALUE_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2004u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_NEW_VALUE, TY_SINT, (-5i8).to_le_bytes()));
    record
}

fn conformant_value_changed_usint_record() -> Record {
    let mut record = Record::new(1_000_000_060, 1, 7, 66, EVENT_VALUE_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2005u32.to_le_bytes()));
    record.slots.push(Slot::new(KEY_NEW_VALUE, TY_USINT, [250]));
    record
}

fn conformant_value_changed_int_record() -> Record {
    let mut record = Record::new(1_000_000_070, 1, 8, 66, EVENT_VALUE_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2006u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_NEW_VALUE, TY_INT, (-1234i16).to_le_bytes()));
    record
}

fn conformant_value_changed_uint_record() -> Record {
    let mut record = Record::new(1_000_000_080, 1, 9, 66, EVENT_VALUE_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2007u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_NEW_VALUE, TY_UINT, 1234u16.to_le_bytes()));
    record
}

fn conformant_value_changed_udint_record() -> Record {
    let mut record = Record::new(1_000_000_090, 1, 10, 66, EVENT_VALUE_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2008u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_NEW_VALUE, TY_UDINT, 123_456u32.to_le_bytes()));
    record
}

fn conformant_value_changed_ulint_record() -> Record {
    let mut record = Record::new(1_000_000_100, 1, 11, 66, EVENT_VALUE_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2009u32.to_le_bytes()));
    record.slots.push(Slot::new(
        KEY_NEW_VALUE,
        TY_ULINT,
        1_234_567_890_123u64.to_le_bytes(),
    ));
    record
}

fn conformant_value_changed_lint_record() -> Record {
    let mut record = Record::new(1_000_000_110, 1, 12, 66, EVENT_VALUE_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2010u32.to_le_bytes()));
    record.slots.push(Slot::new(
        KEY_NEW_VALUE,
        TY_LINT,
        (-1_234_567_890_123i64).to_le_bytes(),
    ));
    record
}

fn conformant_value_changed_lreal_record() -> Record {
    let mut record = Record::new(1_000_000_120, 1, 13, 66, EVENT_VALUE_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2011u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_NEW_VALUE, TY_LREAL, 12.25f64.to_le_bytes()));
    record
}

fn conformant_value_changed_string_record() -> Record {
    let mut record = Record::new(1_000_000_130, 1, 14, 66, EVENT_VALUE_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2012u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_NEW_VALUE, TY_STRING, b"ready"));
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

fn conformant_condition_active_record() -> Record {
    let mut record = Record::new(1_000_000_140, 1, 15, 66, EVENT_CONDITION_ACTIVE);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_CONDITION_CLASS, TY_UINT, 0u16.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_CORRELATION_ID, TY_UDINT, 77u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_SEVERITY, TY_UINT, 900u16.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_CAUSE_OPERAND, TY_UDINT, 1u32.to_le_bytes()));
    record
}

fn conformant_condition_cleared_record() -> Record {
    let mut record = Record::new(1_000_000_150, 1, 16, 66, EVENT_CONDITION_CLEARED);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_CONDITION_CLASS, TY_UINT, 0u16.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_CORRELATION_ID, TY_UDINT, 77u32.to_le_bytes()));
    record
}

fn conformant_condition_acknowledged_record() -> Record {
    let mut record = Record::new(1_000_000_160, 1, 17, 66, EVENT_CONDITION_ACKNOWLEDGED);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_CORRELATION_ID, TY_UDINT, 77u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_ACK_BY, TY_STRING, b"operator-a"));
    record
}

fn conformant_condition_confirmed_record() -> Record {
    let mut record = Record::new(1_000_000_200, 1, 21, 66, EVENT_CONDITION_CONFIRMED);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_CORRELATION_ID, TY_UDINT, 77u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_ACK_BY, TY_STRING, b"operator-a"));
    record
}

fn conformant_condition_shelved_record() -> Record {
    let mut record = Record::new(1_000_000_170, 1, 18, 66, EVENT_CONDITION_SHELVED);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_CORRELATION_ID, TY_UDINT, 77u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_ACK_BY, TY_STRING, b"operator-a"));
    record
        .slots
        .push(Slot::new(KEY_SHELVE_SECS, TY_UDINT, 300u32.to_le_bytes()));
    record
}

fn conformant_condition_unshelved_record() -> Record {
    let mut record = Record::new(1_000_000_210, 1, 22, 66, EVENT_CONDITION_UNSHELVED);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_CORRELATION_ID, TY_UDINT, 77u32.to_le_bytes()));
    record
}

fn conformant_condition_suppressed_record() -> Record {
    let mut record = Record::new(1_000_000_180, 1, 19, 66, EVENT_CONDITION_SUPPRESSED);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_REASON, TY_STRING, b"maintenance"));
    record
}

fn conformant_condition_unsuppressed_record() -> Record {
    let mut record = Record::new(1_000_000_220, 1, 23, 66, EVENT_CONDITION_UNSUPPRESSED);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record
}

fn conformant_condition_out_of_service_record() -> Record {
    let mut record = Record::new(1_000_000_190, 1, 20, 66, EVENT_CONDITION_OUT_OF_SERVICE);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_ACK_BY, TY_STRING, b"operator-a"));
    record
}

fn conformant_condition_in_service_record() -> Record {
    let mut record = Record::new(1_000_000_230, 1, 24, 66, EVENT_CONDITION_IN_SERVICE);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record
}

fn conformant_condition_reset_record() -> Record {
    let mut record = Record::new(1_000_000_240, 1, 25, 66, EVENT_CONDITION_RESET);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_CORRELATION_ID, TY_UDINT, 77u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_ACK_BY, TY_STRING, b"operator-a"));
    record
}

fn conformant_condition_commented_record() -> Record {
    let mut record = Record::new(1_000_000_250, 1, 26, 66, EVENT_CONDITION_COMMENTED);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_CORRELATION_ID, TY_UDINT, 77u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_COMMENT, TY_STRING, b"operator comment"));
    record
        .slots
        .push(Slot::new(KEY_ACK_BY, TY_STRING, b"operator-a"));
    record
}

fn conformant_condition_priority_changed_record() -> Record {
    let mut record = Record::new(1_000_000_260, 1, 27, 66, EVENT_CONDITION_PRIORITY_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9001u32.to_le_bytes()));
    record.slots.push(Slot::new(
        KEY_PREVIOUS_PRIORITY,
        TY_UINT,
        600u16.to_le_bytes(),
    ));
    record
        .slots
        .push(Slot::new(KEY_NEW_PRIORITY, TY_UINT, 900u16.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_ACK_BY, TY_STRING, b"operator-a"));
    record
}

fn conformant_recipe_loaded_record() -> Record {
    let mut record = Record::new(1_000_000_270, 1, 28, 66, EVENT_RECIPE_LOADED);
    record
        .slots
        .push(Slot::new(KEY_RECIPE_ID, TY_UDINT, 3001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_RECIPE_VERSION, TY_STRING, b"v1.2.3"));
    record
        .slots
        .push(Slot::new(KEY_BATCH_ID, TY_UDINT, 4001u32.to_le_bytes()));
    record
}

fn conformant_recipe_approved_record() -> Record {
    let mut record = Record::new(1_000_000_280, 1, 29, 66, EVENT_RECIPE_APPROVED);
    record
        .slots
        .push(Slot::new(KEY_RECIPE_ID, TY_UDINT, 3001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_RECIPE_VERSION, TY_STRING, b"v1.2.3"));
    record
        .slots
        .push(Slot::new(KEY_AUTH_RESULT, TY_UINT, 1u16.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_ACK_BY, TY_STRING, b"approver-a"));
    record
}

fn conformant_batch_event_record() -> Record {
    let mut record = Record::new(1_000_000_290, 1, 30, 66, EVENT_BATCH_EVENT);
    record
        .slots
        .push(Slot::new(KEY_BATCH_ID, TY_UDINT, 4001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_NEW_STATE, TY_UINT, 2u16.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_RECIPE_ID, TY_UDINT, 3001u32.to_le_bytes()));
    record
}

fn conformant_material_addition_record() -> Record {
    let mut record = Record::new(1_000_000_300, 1, 31, 66, EVENT_MATERIAL_ADDITION);
    record
        .slots
        .push(Slot::new(KEY_BATCH_ID, TY_UDINT, 4001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_MATERIAL_ID, TY_UDINT, 5001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_QUANTITY, TY_LREAL, 12.25f64.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_UNIT, TY_UINT, 8u16.to_le_bytes()));
    record
}

fn conformant_operator_action_record() -> Record {
    let mut record = Record::new(1_000_000_310, 1, 32, 66, EVENT_OPERATOR_ACTION);
    record
        .slots
        .push(Slot::new(KEY_ACTION_ID, TY_UDINT, 6001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_ACTOR, TY_STRING, b"operator-a"));
    record
        .slots
        .push(Slot::new(KEY_CONTEXT_REF, TY_UDINT, 7001u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_CONTEXT_REF, TY_UDINT, 7002u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_AUTH_RESULT, TY_UINT, 0u16.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_WORKSTATION, TY_STRING, b"station-1"));
    record
}

fn conformant_operator_login_record() -> Record {
    let mut record = Record::new(1_000_000_320, 1, 33, 66, EVENT_OPERATOR_LOGIN);
    record
        .slots
        .push(Slot::new(KEY_ACTOR, TY_STRING, b"operator-a"));
    record
        .slots
        .push(Slot::new(KEY_AUTH_RESULT, TY_UINT, 0u16.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_WORKSTATION, TY_STRING, b"station-1"));
    record
        .slots
        .push(Slot::new(KEY_ROLE, TY_UINT, 3u16.to_le_bytes()));
    record
}

fn conformant_operator_logout_record() -> Record {
    let mut record = Record::new(1_000_000_330, 1, 34, 66, EVENT_OPERATOR_LOGOUT);
    record
        .slots
        .push(Slot::new(KEY_ACTOR, TY_STRING, b"operator-a"));
    record
        .slots
        .push(Slot::new(KEY_WORKSTATION, TY_STRING, b"station-1"));
    record
}

fn conformant_security_access_failure_record() -> Record {
    let mut record = Record::new(1_000_000_340, 1, 35, 66, EVENT_SECURITY_ACCESS_FAILURE);
    record
        .slots
        .push(Slot::new(KEY_ACTOR, TY_STRING, b"operator-x"));
    record
        .slots
        .push(Slot::new(KEY_WORKSTATION, TY_STRING, b"station-2"));
    record
        .slots
        .push(Slot::new(KEY_REASON, TY_STRING, b"denied"));
    record
}

fn conformant_parameter_change_real_record() -> Record {
    let mut record = Record::new(1_000_000_350, 1, 36, 66, EVENT_PARAMETER_CHANGE);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2001u32.to_le_bytes()));
    record.slots.push(Slot::new(
        KEY_PREVIOUS_VALUE,
        TY_REAL,
        12.5f32.to_le_bytes(),
    ));
    record
        .slots
        .push(Slot::new(KEY_NEW_VALUE, TY_REAL, 13.75f32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_ACTOR, TY_STRING, b"operator-a"));
    record
        .slots
        .push(Slot::new(KEY_REASON, TY_STRING, b"setpoint change"));
    record
        .slots
        .push(Slot::new(KEY_AUTH_RESULT, TY_UINT, 0u16.to_le_bytes()));
    record
}

fn conformant_parameter_change_dint_record() -> Record {
    let mut record = Record::new(1_000_000_360, 1, 37, 66, EVENT_PARAMETER_CHANGE);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2002u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_PREVIOUS_VALUE, TY_DINT, 40i32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_NEW_VALUE, TY_DINT, 42i32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_ACTOR, TY_STRING, b"operator-a"));
    record
        .slots
        .push(Slot::new(KEY_REASON, TY_STRING, b"count corrected"));
    record
}

fn conformant_parameter_change_bool_record() -> Record {
    let mut record = Record::new(1_000_000_370, 1, 38, 66, EVENT_PARAMETER_CHANGE);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2003u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_PREVIOUS_VALUE, TY_BOOL, [0]));
    record.slots.push(Slot::new(KEY_NEW_VALUE, TY_BOOL, [1]));
    record
        .slots
        .push(Slot::new(KEY_ACTOR, TY_STRING, b"operator-b"));
    record
        .slots
        .push(Slot::new(KEY_REASON, TY_STRING, b"enable audit"));
    record
}

fn conformant_parameter_change_string_record() -> Record {
    let mut record = Record::new(1_000_000_380, 1, 39, 66, EVENT_PARAMETER_CHANGE);
    record
        .slots
        .push(Slot::new(KEY_VALUE_ID, TY_UDINT, 2012u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_PREVIOUS_VALUE, TY_STRING, b"manual"));
    record
        .slots
        .push(Slot::new(KEY_NEW_VALUE, TY_STRING, b"auto"));
    record
        .slots
        .push(Slot::new(KEY_ACTOR, TY_STRING, b"operator-c"));
    record
        .slots
        .push(Slot::new(KEY_REASON, TY_STRING, b"mode note"));
    record
        .slots
        .push(Slot::new(KEY_AUTH_RESULT, TY_UINT, 1u16.to_le_bytes()));
    record
}

fn conformant_e_signature_record() -> Record {
    let mut record = Record::new(1_000_000_390, 1, 40, 66, EVENT_ESIGNATURE);
    record
        .slots
        .push(Slot::new(KEY_ACTION_ID, TY_UDINT, 6002u32.to_le_bytes()));
    record
        .slots
        .push(Slot::new(KEY_ACTOR, TY_STRING, b"signer-a"));
    record.slots.push(Slot::new(
        KEY_SIGNATURE_MEANING,
        TY_UINT,
        2u16.to_le_bytes(),
    ));
    record.slots.push(Slot::new(
        KEY_SIGNED_EVENT_SEQ,
        TY_ULINT,
        37u64.to_le_bytes(),
    ));
    record
        .slots
        .push(Slot::new(KEY_AUTH_RESULT, TY_UINT, 0u16.to_le_bytes()));
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

fn minimal_message_record() -> Record {
    let mut record = minimal_record(7, 3);
    record.slots.push(Slot::new(
        KEY_MESSAGE_TEMPLATE_ID,
        TY_UDINT,
        1001u32.to_le_bytes(),
    ));
    record
}

fn logger_stopped_record() -> Record {
    Record::new(0, 1, 0, SYSTEM_SOURCE_ID, EVENT_LOGGER_STOPPED)
}

fn definition_changed_record() -> Record {
    let mut record = Record::new(0, 1, 1, SYSTEM_SOURCE_ID, EVENT_DEFINITION_CHANGED);
    record
        .slots
        .push(Slot::new(KEY_DEF_HASH_OLD, TY_BYTES, b"oldhash1"));
    record
        .slots
        .push(Slot::new(KEY_DEF_HASH_NEW, TY_BYTES, b"newhash2"));
    record
        .slots
        .push(Slot::new(KEY_EPOCH_ID, TY_ULINT, 2u64.to_le_bytes()));
    record
}

fn logger_started_warm_record() -> Record {
    logger_started_record(1, 2, false)
}

fn logger_started_cold_record() -> Record {
    logger_started_record(2, 0, true)
}

fn logger_started_record(run_id: u64, seq: u64, cold_start: bool) -> Record {
    let mut record = Record::new(0, run_id, seq, SYSTEM_SOURCE_ID, EVENT_LOGGER_STARTED);
    record
        .slots
        .push(Slot::new(KEY_COLD_START, TY_BOOL, [u8::from(cold_start)]));
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
        "conformant_value_changed_real" => "conformant_value_changed_real.hex",
        "conformant_value_changed_dint" => "conformant_value_changed_dint.hex",
        "conformant_value_changed_bool" => "conformant_value_changed_bool.hex",
        "conformant_value_changed_sint" => "conformant_value_changed_sint.hex",
        "conformant_value_changed_usint" => "conformant_value_changed_usint.hex",
        "conformant_value_changed_int" => "conformant_value_changed_int.hex",
        "conformant_value_changed_uint" => "conformant_value_changed_uint.hex",
        "conformant_value_changed_udint" => "conformant_value_changed_udint.hex",
        "conformant_value_changed_ulint" => "conformant_value_changed_ulint.hex",
        "conformant_value_changed_lint" => "conformant_value_changed_lint.hex",
        "conformant_value_changed_lreal" => "conformant_value_changed_lreal.hex",
        "conformant_value_changed_string" => "conformant_value_changed_string.hex",
        "conformant_message" => "conformant_message.hex",
        "conformant_condition_active" => "conformant_condition_active.hex",
        "conformant_condition_cleared" => "conformant_condition_cleared.hex",
        "conformant_condition_acknowledged" => "conformant_condition_acknowledged.hex",
        "conformant_condition_confirmed" => "conformant_condition_confirmed.hex",
        "conformant_condition_shelved" => "conformant_condition_shelved.hex",
        "conformant_condition_unshelved" => "conformant_condition_unshelved.hex",
        "conformant_condition_suppressed" => "conformant_condition_suppressed.hex",
        "conformant_condition_unsuppressed" => "conformant_condition_unsuppressed.hex",
        "conformant_condition_out_of_service" => "conformant_condition_out_of_service.hex",
        "conformant_condition_in_service" => "conformant_condition_in_service.hex",
        "conformant_condition_reset" => "conformant_condition_reset.hex",
        "conformant_condition_commented" => "conformant_condition_commented.hex",
        "conformant_condition_priority_changed" => "conformant_condition_priority_changed.hex",
        "conformant_recipe_loaded" => "conformant_recipe_loaded.hex",
        "conformant_recipe_approved" => "conformant_recipe_approved.hex",
        "conformant_batch_event" => "conformant_batch_event.hex",
        "conformant_material_addition" => "conformant_material_addition.hex",
        "conformant_operator_action" => "conformant_operator_action.hex",
        "conformant_operator_login" => "conformant_operator_login.hex",
        "conformant_operator_logout" => "conformant_operator_logout.hex",
        "conformant_security_access_failure" => "conformant_security_access_failure.hex",
        "conformant_parameter_change_real" => "conformant_parameter_change_real.hex",
        "conformant_parameter_change_dint" => "conformant_parameter_change_dint.hex",
        "conformant_parameter_change_bool" => "conformant_parameter_change_bool.hex",
        "conformant_parameter_change_string" => "conformant_parameter_change_string.hex",
        "conformant_e_signature" => "conformant_e_signature.hex",
        "conformant_records_dropped" => "conformant_records_dropped.hex",
        "conformant_source_high_water" => "conformant_source_high_water.hex",
        "records_dropped" => "records_dropped.hex",
        "source_high_water" => "source_high_water.hex",
        "minimal_message" => "minimal_message.hex",
        "logger_stopped" => "logger_stopped.hex",
        "definition_changed" => "definition_changed.hex",
        "logger_started_warm" => "logger_started_warm.hex",
        "logger_started_cold" => "logger_started_cold.hex",
        _ => unreachable!("unknown vector stem"),
    }
}

fn json_path(stem: &'static str) -> &'static str {
    match stem {
        "state_transition" => "state_transition.json",
        "conformant_state_transition" => "conformant_state_transition.json",
        "conformant_value_changed_real" => "conformant_value_changed_real.json",
        "conformant_value_changed_dint" => "conformant_value_changed_dint.json",
        "conformant_value_changed_bool" => "conformant_value_changed_bool.json",
        "conformant_value_changed_sint" => "conformant_value_changed_sint.json",
        "conformant_value_changed_usint" => "conformant_value_changed_usint.json",
        "conformant_value_changed_int" => "conformant_value_changed_int.json",
        "conformant_value_changed_uint" => "conformant_value_changed_uint.json",
        "conformant_value_changed_udint" => "conformant_value_changed_udint.json",
        "conformant_value_changed_ulint" => "conformant_value_changed_ulint.json",
        "conformant_value_changed_lint" => "conformant_value_changed_lint.json",
        "conformant_value_changed_lreal" => "conformant_value_changed_lreal.json",
        "conformant_value_changed_string" => "conformant_value_changed_string.json",
        "conformant_message" => "conformant_message.json",
        "conformant_condition_active" => "conformant_condition_active.json",
        "conformant_condition_cleared" => "conformant_condition_cleared.json",
        "conformant_condition_acknowledged" => "conformant_condition_acknowledged.json",
        "conformant_condition_confirmed" => "conformant_condition_confirmed.json",
        "conformant_condition_shelved" => "conformant_condition_shelved.json",
        "conformant_condition_unshelved" => "conformant_condition_unshelved.json",
        "conformant_condition_suppressed" => "conformant_condition_suppressed.json",
        "conformant_condition_unsuppressed" => "conformant_condition_unsuppressed.json",
        "conformant_condition_out_of_service" => "conformant_condition_out_of_service.json",
        "conformant_condition_in_service" => "conformant_condition_in_service.json",
        "conformant_condition_reset" => "conformant_condition_reset.json",
        "conformant_condition_commented" => "conformant_condition_commented.json",
        "conformant_condition_priority_changed" => "conformant_condition_priority_changed.json",
        "conformant_recipe_loaded" => "conformant_recipe_loaded.json",
        "conformant_recipe_approved" => "conformant_recipe_approved.json",
        "conformant_batch_event" => "conformant_batch_event.json",
        "conformant_material_addition" => "conformant_material_addition.json",
        "conformant_operator_action" => "conformant_operator_action.json",
        "conformant_operator_login" => "conformant_operator_login.json",
        "conformant_operator_logout" => "conformant_operator_logout.json",
        "conformant_security_access_failure" => "conformant_security_access_failure.json",
        "conformant_parameter_change_real" => "conformant_parameter_change_real.json",
        "conformant_parameter_change_dint" => "conformant_parameter_change_dint.json",
        "conformant_parameter_change_bool" => "conformant_parameter_change_bool.json",
        "conformant_parameter_change_string" => "conformant_parameter_change_string.json",
        "conformant_e_signature" => "conformant_e_signature.json",
        "conformant_records_dropped" => "conformant_records_dropped.json",
        "conformant_source_high_water" => "conformant_source_high_water.json",
        "records_dropped" => "records_dropped.json",
        "source_high_water" => "source_high_water.json",
        "minimal_message" => "minimal_message.json",
        "logger_stopped" => "logger_stopped.json",
        "definition_changed" => "definition_changed.json",
        "logger_started_warm" => "logger_started_warm.json",
        "logger_started_cold" => "logger_started_cold.json",
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
            &conformant_value_changed_real_record(),
            &[(KEY_VALUE_ID, TY_UDINT, 4), (KEY_NEW_VALUE, TY_REAL, 4)],
        );
        assert_slots(
            &conformant_value_changed_dint_record(),
            &[
                (KEY_VALUE_ID, TY_UDINT, 4),
                (KEY_NEW_VALUE, TY_DINT, 4),
                (KEY_PREVIOUS_VALUE, TY_DINT, 4),
                (KEY_QUALITY, TY_UINT, 2),
            ],
        );
        assert_slots(
            &conformant_value_changed_bool_record(),
            &[(KEY_VALUE_ID, TY_UDINT, 4), (KEY_NEW_VALUE, TY_BOOL, 1)],
        );
        assert_slots(
            &conformant_value_changed_sint_record(),
            &[(KEY_VALUE_ID, TY_UDINT, 4), (KEY_NEW_VALUE, TY_SINT, 1)],
        );
        assert_slots(
            &conformant_value_changed_usint_record(),
            &[(KEY_VALUE_ID, TY_UDINT, 4), (KEY_NEW_VALUE, TY_USINT, 1)],
        );
        assert_slots(
            &conformant_value_changed_int_record(),
            &[(KEY_VALUE_ID, TY_UDINT, 4), (KEY_NEW_VALUE, TY_INT, 2)],
        );
        assert_slots(
            &conformant_value_changed_uint_record(),
            &[(KEY_VALUE_ID, TY_UDINT, 4), (KEY_NEW_VALUE, TY_UINT, 2)],
        );
        assert_slots(
            &conformant_value_changed_udint_record(),
            &[(KEY_VALUE_ID, TY_UDINT, 4), (KEY_NEW_VALUE, TY_UDINT, 4)],
        );
        assert_slots(
            &conformant_value_changed_ulint_record(),
            &[(KEY_VALUE_ID, TY_UDINT, 4), (KEY_NEW_VALUE, TY_ULINT, 8)],
        );
        assert_slots(
            &conformant_value_changed_lint_record(),
            &[(KEY_VALUE_ID, TY_UDINT, 4), (KEY_NEW_VALUE, TY_LINT, 8)],
        );
        assert_slots(
            &conformant_value_changed_lreal_record(),
            &[(KEY_VALUE_ID, TY_UDINT, 4), (KEY_NEW_VALUE, TY_LREAL, 8)],
        );
        assert_slots(
            &conformant_value_changed_string_record(),
            &[
                (KEY_VALUE_ID, TY_UDINT, 4),
                (KEY_NEW_VALUE, TY_STRING, "ready".len()),
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
            &conformant_condition_active_record(),
            &[
                (KEY_CONDITION_ID, TY_UDINT, 4),
                (KEY_CONDITION_CLASS, TY_UINT, 2),
                (KEY_CORRELATION_ID, TY_UDINT, 4),
                (KEY_SEVERITY, TY_UINT, 2),
                (KEY_CAUSE_OPERAND, TY_UDINT, 4),
            ],
        );
        assert_slots(
            &conformant_condition_cleared_record(),
            &[
                (KEY_CONDITION_ID, TY_UDINT, 4),
                (KEY_CONDITION_CLASS, TY_UINT, 2),
                (KEY_CORRELATION_ID, TY_UDINT, 4),
            ],
        );
        assert_slots(
            &conformant_condition_acknowledged_record(),
            &[
                (KEY_CONDITION_ID, TY_UDINT, 4),
                (KEY_CORRELATION_ID, TY_UDINT, 4),
                (KEY_ACK_BY, TY_STRING, "operator-a".len()),
            ],
        );
        assert_slots(
            &conformant_condition_confirmed_record(),
            &[
                (KEY_CONDITION_ID, TY_UDINT, 4),
                (KEY_CORRELATION_ID, TY_UDINT, 4),
                (KEY_ACK_BY, TY_STRING, "operator-a".len()),
            ],
        );
        assert_slots(
            &conformant_condition_shelved_record(),
            &[
                (KEY_CONDITION_ID, TY_UDINT, 4),
                (KEY_CORRELATION_ID, TY_UDINT, 4),
                (KEY_ACK_BY, TY_STRING, "operator-a".len()),
                (KEY_SHELVE_SECS, TY_UDINT, 4),
            ],
        );
        assert_slots(
            &conformant_condition_unshelved_record(),
            &[
                (KEY_CONDITION_ID, TY_UDINT, 4),
                (KEY_CORRELATION_ID, TY_UDINT, 4),
            ],
        );
        assert_slots(
            &conformant_condition_suppressed_record(),
            &[
                (KEY_CONDITION_ID, TY_UDINT, 4),
                (KEY_REASON, TY_STRING, "maintenance".len()),
            ],
        );
        assert_slots(
            &conformant_condition_unsuppressed_record(),
            &[(KEY_CONDITION_ID, TY_UDINT, 4)],
        );
        assert_slots(
            &conformant_condition_out_of_service_record(),
            &[
                (KEY_CONDITION_ID, TY_UDINT, 4),
                (KEY_ACK_BY, TY_STRING, "operator-a".len()),
            ],
        );
        assert_slots(
            &conformant_condition_in_service_record(),
            &[(KEY_CONDITION_ID, TY_UDINT, 4)],
        );
        assert_slots(
            &conformant_condition_reset_record(),
            &[
                (KEY_CONDITION_ID, TY_UDINT, 4),
                (KEY_CORRELATION_ID, TY_UDINT, 4),
                (KEY_ACK_BY, TY_STRING, "operator-a".len()),
            ],
        );
        assert_slots(
            &conformant_condition_commented_record(),
            &[
                (KEY_CONDITION_ID, TY_UDINT, 4),
                (KEY_CORRELATION_ID, TY_UDINT, 4),
                (KEY_COMMENT, TY_STRING, "operator comment".len()),
                (KEY_ACK_BY, TY_STRING, "operator-a".len()),
            ],
        );
        assert_slots(
            &conformant_condition_priority_changed_record(),
            &[
                (KEY_CONDITION_ID, TY_UDINT, 4),
                (KEY_PREVIOUS_PRIORITY, TY_UINT, 2),
                (KEY_NEW_PRIORITY, TY_UINT, 2),
                (KEY_ACK_BY, TY_STRING, "operator-a".len()),
            ],
        );
        assert_slots(
            &conformant_recipe_loaded_record(),
            &[
                (KEY_RECIPE_ID, TY_UDINT, 4),
                (KEY_RECIPE_VERSION, TY_STRING, "v1.2.3".len()),
                (KEY_BATCH_ID, TY_UDINT, 4),
            ],
        );
        assert_slots(
            &conformant_recipe_approved_record(),
            &[
                (KEY_RECIPE_ID, TY_UDINT, 4),
                (KEY_RECIPE_VERSION, TY_STRING, "v1.2.3".len()),
                (KEY_AUTH_RESULT, TY_UINT, 2),
                (KEY_ACK_BY, TY_STRING, "approver-a".len()),
            ],
        );
        assert_slots(
            &conformant_batch_event_record(),
            &[
                (KEY_BATCH_ID, TY_UDINT, 4),
                (KEY_NEW_STATE, TY_UINT, 2),
                (KEY_RECIPE_ID, TY_UDINT, 4),
            ],
        );
        assert_slots(
            &conformant_material_addition_record(),
            &[
                (KEY_BATCH_ID, TY_UDINT, 4),
                (KEY_MATERIAL_ID, TY_UDINT, 4),
                (KEY_QUANTITY, TY_LREAL, 8),
                (KEY_UNIT, TY_UINT, 2),
            ],
        );
        assert_slots(
            &conformant_operator_action_record(),
            &[
                (KEY_ACTION_ID, TY_UDINT, 4),
                (KEY_ACTOR, TY_STRING, "operator-a".len()),
                (KEY_CONTEXT_REF, TY_UDINT, 4),
                (KEY_CONTEXT_REF, TY_UDINT, 4),
                (KEY_AUTH_RESULT, TY_UINT, 2),
                (KEY_WORKSTATION, TY_STRING, "station-1".len()),
            ],
        );
        assert_slots(
            &conformant_operator_login_record(),
            &[
                (KEY_ACTOR, TY_STRING, "operator-a".len()),
                (KEY_AUTH_RESULT, TY_UINT, 2),
                (KEY_WORKSTATION, TY_STRING, "station-1".len()),
                (KEY_ROLE, TY_UINT, 2),
            ],
        );
        assert_slots(
            &conformant_operator_logout_record(),
            &[
                (KEY_ACTOR, TY_STRING, "operator-a".len()),
                (KEY_WORKSTATION, TY_STRING, "station-1".len()),
            ],
        );
        assert_slots(
            &conformant_security_access_failure_record(),
            &[
                (KEY_ACTOR, TY_STRING, "operator-x".len()),
                (KEY_WORKSTATION, TY_STRING, "station-2".len()),
                (KEY_REASON, TY_STRING, "denied".len()),
            ],
        );
        assert_slots(
            &conformant_parameter_change_real_record(),
            &[
                (KEY_VALUE_ID, TY_UDINT, 4),
                (KEY_PREVIOUS_VALUE, TY_REAL, 4),
                (KEY_NEW_VALUE, TY_REAL, 4),
                (KEY_ACTOR, TY_STRING, "operator-a".len()),
                (KEY_REASON, TY_STRING, "setpoint change".len()),
                (KEY_AUTH_RESULT, TY_UINT, 2),
            ],
        );
        assert_slots(
            &conformant_parameter_change_dint_record(),
            &[
                (KEY_VALUE_ID, TY_UDINT, 4),
                (KEY_PREVIOUS_VALUE, TY_DINT, 4),
                (KEY_NEW_VALUE, TY_DINT, 4),
                (KEY_ACTOR, TY_STRING, "operator-a".len()),
                (KEY_REASON, TY_STRING, "count corrected".len()),
            ],
        );
        assert_slots(
            &conformant_parameter_change_bool_record(),
            &[
                (KEY_VALUE_ID, TY_UDINT, 4),
                (KEY_PREVIOUS_VALUE, TY_BOOL, 1),
                (KEY_NEW_VALUE, TY_BOOL, 1),
                (KEY_ACTOR, TY_STRING, "operator-b".len()),
                (KEY_REASON, TY_STRING, "enable audit".len()),
            ],
        );
        assert_slots(
            &conformant_parameter_change_string_record(),
            &[
                (KEY_VALUE_ID, TY_UDINT, 4),
                (KEY_PREVIOUS_VALUE, TY_STRING, "manual".len()),
                (KEY_NEW_VALUE, TY_STRING, "auto".len()),
                (KEY_ACTOR, TY_STRING, "operator-c".len()),
                (KEY_REASON, TY_STRING, "mode note".len()),
                (KEY_AUTH_RESULT, TY_UINT, 2),
            ],
        );
        assert_slots(
            &conformant_e_signature_record(),
            &[
                (KEY_ACTION_ID, TY_UDINT, 4),
                (KEY_ACTOR, TY_STRING, "signer-a".len()),
                (KEY_SIGNATURE_MEANING, TY_UINT, 2),
                (KEY_SIGNED_EVENT_SEQ, TY_ULINT, 8),
                (KEY_AUTH_RESULT, TY_UINT, 2),
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
    fn lifecycle_vectors_match_epoch_producer_emission() {
        let stopped = logger_stopped_record();
        assert_eq!(stopped.source_id, SYSTEM_SOURCE_ID);
        assert_eq!(stopped.seq, 0);
        assert_eq!(stopped.event_type_id, EVENT_LOGGER_STOPPED);
        assert_slots(&stopped, &[]);

        let changed = definition_changed_record();
        assert_eq!(changed.source_id, SYSTEM_SOURCE_ID);
        assert_eq!(changed.seq, 1);
        assert_eq!(changed.event_type_id, EVENT_DEFINITION_CHANGED);
        assert_slots(
            &changed,
            &[
                (KEY_DEF_HASH_OLD, TY_BYTES, 8),
                (KEY_DEF_HASH_NEW, TY_BYTES, 8),
                (KEY_EPOCH_ID, TY_ULINT, 8),
            ],
        );

        let warm = logger_started_warm_record();
        assert_eq!(warm.source_id, SYSTEM_SOURCE_ID);
        assert_eq!(warm.seq, 2);
        assert_eq!(warm.event_type_id, EVENT_LOGGER_STARTED);
        assert_slots(&warm, &[(KEY_COLD_START, TY_BOOL, 1)]);
        assert_eq!(warm.slots[0].payload, [0]);

        let cold = logger_started_cold_record();
        assert_eq!(cold.source_id, SYSTEM_SOURCE_ID);
        assert_eq!(cold.seq, 0);
        assert_eq!(cold.run_id, 2);
        assert_eq!(cold.event_type_id, EVENT_LOGGER_STARTED);
        assert_slots(&cold, &[(KEY_COLD_START, TY_BOOL, 1)]);
        assert_eq!(cold.slots[0].payload, [1]);
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
