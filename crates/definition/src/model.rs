//! Definition-file content model (§9.1).
//!
//! These typed structs map record ids, value keys, enum sets, and units to human meaning.
//! [`sample_definition`] builds the positive spine that the hash, schema, and resolver tests
//! resolve carriage vectors against.

use open_ot_carriage::registry::*;
use serde::{Deserialize, Serialize};

/// Hash-bound definition file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionFile {
    /// Top-level metadata and producer declaration.
    pub header: DefinitionHeader,
    /// Event id to slot-schema mappings.
    #[serde(default)]
    pub event_types: Vec<EventTypeDefinition>,
    /// Source id to human meaning mappings.
    #[serde(default)]
    pub sources: Vec<SourceDefinition>,
    /// State-machine declarations.
    #[serde(default)]
    pub state_machines: Vec<StateMachineDefinition>,
    /// Condition (alarm/cause) declarations.
    #[serde(default)]
    pub conditions: Vec<ConditionDefinition>,
    /// Message-template declarations.
    #[serde(default)]
    pub message_templates: Vec<MessageTemplateDefinition>,
    /// Analog/value declarations.
    #[serde(default)]
    pub values: Vec<ValueDefinition>,
    /// Engineering-unit declarations.
    #[serde(default)]
    pub units: Vec<UnitDefinition>,
    /// Enumeration-set declarations.
    #[serde(default)]
    pub enum_sets: Vec<EnumSetDefinition>,
    /// Recipe id to human meaning mappings.
    #[serde(default)]
    pub recipe_definitions: Vec<RecipeDefinition>,
    /// Batch id to human meaning mappings.
    #[serde(default)]
    pub batch_definitions: Vec<BatchDefinition>,
    /// Material id to human meaning mappings.
    #[serde(default)]
    pub material_definitions: Vec<MaterialDefinition>,
    /// Operator id to human meaning mappings.
    #[serde(default)]
    pub operator_definitions: Vec<OperatorDefinition>,
    /// Electronic-signature meaning mappings.
    #[serde(default)]
    pub e_signature_meanings: Vec<ESignatureMeaningDefinition>,
    /// Severity scale and band thresholds.
    pub severity_scale: SeverityScale,
}

/// Top-level definition metadata and producer declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionHeader {
    /// Wire format version this definition targets.
    pub wire_version: u16,
    /// Human-facing semantic version of the definition content.
    pub semantic_version: String,
    /// Conformance profiles the producer claims.
    pub profiles: Vec<String>,
    /// Declared conformance level.
    pub conformance_level: String,
    /// Producer capability declaration.
    pub caps: DefinitionCaps,
    /// Machine-checkable carriage constraints.
    pub constraints: DefinitionConstraints,
    /// Whether prior-epoch records are retained or cleared.
    pub epoch_strategy: EpochStrategy,
    /// Lowercase 64-hex SHA-256 over the canonical form; empty while hashing (self-exclusion).
    pub content_hash: String,
}

/// Producer capability declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionCaps {
    /// Producer emits the optional CRC-32C record trailer.
    pub crc: bool,
    /// Producer emits per-source high-water checkpoints.
    pub source_high_water: bool,
}

/// Machine-checkable carriage constraints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinitionConstraints {
    /// Maximum encoded record size in bytes.
    pub max_record_size: u16,
    /// Maximum number of TLV slots per record.
    pub max_slots: u16,
    /// What the producer does when the ring is full.
    pub overflow_policy: OverflowPolicy,
}

/// What a producer does when the ring buffer is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverflowPolicy {
    /// Reclaim the oldest retained record to make room (the carriage default).
    OverwriteOldest,
    /// Drop the incoming record and keep existing data.
    DropNewest,
    /// Block the producer until space is available.
    HoldProducer,
}

/// Whether retained prior-epoch records survive a definition change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EpochStrategy {
    /// Keep prior-epoch records, resolvable against the previous definition hash.
    Retain,
    /// Discard prior-epoch records at the transition.
    Clear,
}

/// Event id to slot-schema mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventTypeDefinition {
    /// Registry event-type id.
    pub id: u32,
    /// Human-facing event name.
    pub name: String,
    /// Conformance profile this event belongs to.
    pub profile: String,
    /// Ordered slot schema for the event's payload.
    pub slots: Vec<SlotDefinition>,
}

/// One ordered slot schema entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SlotDefinition {
    /// Value-key id of the slot.
    pub key: u16,
    /// Required TLV type tag for fixed-type slot payloads.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tlv_type: Option<u8>,
    /// Whether the slot carries the referenced datum's own TLV type.
    #[serde(default, skip_serializing_if = "is_false")]
    pub value_payload: bool,
    /// Minimum number of occurrences (0 means optional).
    pub min_occurs: u16,
    /// Maximum number of occurrences.
    pub max_occurs: MaxOccurs,
    /// Ordering class; slots must appear in ascending order class.
    pub order_class: u16,
}

/// Occurrence upper bound. `unbounded` is serialized as a string to avoid sentinel integers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MaxOccurs {
    /// A fixed upper bound.
    Count(u16),
    /// No upper bound; serialized as the string `"unbounded"` to avoid sentinel integers.
    Unbounded(String),
}

/// Source id to human meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceDefinition {
    /// Source stream id.
    pub source_id: u32,
    /// Human-facing source name.
    pub name: String,
    /// Logical path segments to the source.
    pub path: Vec<String>,
    /// Equipment hierarchy (for example ISA-95 levels).
    pub hierarchy: Vec<String>,
    /// Whether the source is created dynamically at runtime.
    pub dynamic: bool,
}

/// State machine declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StateMachineDefinition {
    /// State-machine id.
    pub state_machine_id: u32,
    /// Human-facing state-machine name.
    pub name: String,
    /// State-machine category code.
    pub category: u16,
    /// Optional procedural model (for example an ISA-88 model name).
    pub procedural_model: Option<String>,
    /// Name of the enum set that names this machine's states.
    pub enum_set: String,
}

/// Condition declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConditionDefinition {
    /// Condition id.
    pub condition_id: u32,
    /// Human-facing condition name.
    pub name: String,
    /// Condition class code.
    pub condition_class: u16,
    /// Default severity (1..1000) when an instance does not override it.
    pub default_severity: u16,
    /// Named operands that parameterize the condition's cause.
    #[serde(default)]
    pub cause_operands: Vec<CauseOperandDefinition>,
}

/// One named operand contributing to a condition's cause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CauseOperandDefinition {
    /// Operand id.
    pub operand_id: u32,
    /// Human-facing operand name.
    pub name: String,
}

/// Message template declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessageTemplateDefinition {
    /// Message-template id.
    pub message_template_id: u32,
    /// Human-facing template name.
    pub name: String,
    /// Format string with positional argument placeholders.
    pub format: String,
    /// TLV type tag expected for each positional argument.
    pub arg_types: Vec<u8>,
}

/// Value declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValueDefinition {
    /// Value id.
    pub value_id: u32,
    /// Human-facing value name.
    pub name: String,
    /// TLV type tag of the value's payload.
    pub data_type: u8,
    /// Semantic role code (what the value means).
    pub semantic_role: u16,
    /// Optional engineering-unit id.
    pub unit: Option<u16>,
    /// Optional deadband for change reporting.
    pub deadband: Option<Deadband>,
    /// Optional sampling-policy name.
    pub sampling_policy: Option<String>,
}

/// Deadband representation without JSON floating-point numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Deadband {
    /// Deadband as a decimal string (no JSON float); mutually exclusive with `scaled`.
    pub decimal: Option<String>,
    /// Deadband as a scaled integer; mutually exclusive with `decimal`.
    pub scaled: Option<ScaledInteger>,
}

/// A decimal encoded as `value * 10^-scale`, avoiding JSON floating-point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScaledInteger {
    /// Integer mantissa.
    pub value: i64,
    /// Number of decimal places (the power-of-ten divisor).
    pub scale: u32,
}

/// An engineering unit: an id and its display symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnitDefinition {
    /// Unit id referenced by values.
    pub unit_id: u16,
    /// Display symbol (for example `degC`).
    pub symbol: String,
}

/// Recipe declaration used by procedural events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeDefinition {
    /// Recipe id referenced by records.
    pub recipe_id: u32,
    /// Human-facing recipe name.
    pub name: String,
    /// Optional recipe version label.
    pub version: Option<String>,
}

/// Batch declaration used by procedural events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchDefinition {
    /// Batch id referenced by records.
    pub batch_id: u32,
    /// Human-facing batch name.
    pub name: String,
    /// Optional recipe id this batch instantiates.
    pub recipe: Option<u32>,
}

/// Material declaration used by material-addition events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MaterialDefinition {
    /// Material id referenced by records.
    pub material_id: u32,
    /// Human-facing material name.
    pub name: String,
    /// Optional unit id for material quantities.
    pub unit: Option<u16>,
}

/// Operator declaration used by regulated/audit events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperatorDefinition {
    /// Stable operator id or account name.
    pub actor: String,
    /// Human-facing display name.
    pub name: String,
    /// Role labels attached to the operator.
    #[serde(default)]
    pub roles: Vec<String>,
}

/// Electronic-signature meaning declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ESignatureMeaningDefinition {
    /// Signature-meaning enum value.
    pub meaning: u16,
    /// Human-facing meaning label.
    pub label: String,
}

/// A named set of enumeration members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnumSetDefinition {
    /// Enum-set name, referenced by state machines and fields.
    pub name: String,
    /// Members of the set.
    pub members: Vec<EnumMember>,
}

/// One enumeration member: a numeric value and its label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnumMember {
    /// Numeric enum value as carried on the wire.
    pub value: u16,
    /// Human-facing label for the value.
    pub label: String,
}

/// The severity scale and its low/medium/high band thresholds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SeverityScale {
    /// Scale name.
    pub name: String,
    /// Low-severity band.
    pub low: SeverityBandDefinition,
    /// Medium-severity band.
    pub medium: SeverityBandDefinition,
    /// High-severity band.
    pub high: SeverityBandDefinition,
}

/// Inclusive severity range for one band.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SeverityBandDefinition {
    /// Inclusive lower bound (1..1000).
    pub min: u16,
    /// Inclusive upper bound (1..1000).
    pub max: u16,
}

/// Maximum record size supported by the IEC producer's staging buffer.
pub const PRODUCER_MAX_RECORD_SIZE: u16 = 256;

/// Canonical event-type schemas for the complete OpenOT reference vocabulary.
pub fn canonical_event_types() -> Vec<EventTypeDefinition> {
    EVENT_SPECS
        .iter()
        .map(|spec| canonical_event_type(spec.id).expect("registry event has canonical schema"))
        .collect()
}

/// Canonical event-type schema for one event id.
pub fn canonical_event_type(id: u32) -> Option<EventTypeDefinition> {
    let slots = match id {
        EVENT_STATE_TRANSITION => vec![
            slot(KEY_STATE_MACHINE_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_CATEGORY, TY_UINT, 1, MaxOccurs::Count(1), 2),
            slot(KEY_PREVIOUS_STATE, TY_UINT, 1, MaxOccurs::Count(1), 3),
            slot(KEY_NEW_STATE, TY_UINT, 1, MaxOccurs::Count(1), 4),
        ],
        EVENT_VALUE_CHANGED => vec![
            slot(KEY_VALUE_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            value_slot(KEY_PREVIOUS_VALUE, 0, MaxOccurs::Count(1), 2),
            value_slot(KEY_NEW_VALUE, 1, MaxOccurs::Count(1), 3),
            slot(KEY_QUALITY, TY_UINT, 0, MaxOccurs::Count(1), 4),
        ],
        EVENT_MESSAGE => vec![
            slot(KEY_MESSAGE_TEMPLATE_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            value_slot(KEY_ARG, 0, MaxOccurs::Unbounded("unbounded".to_string()), 2),
            slot(KEY_SEVERITY, TY_UINT, 0, MaxOccurs::Count(1), 3),
        ],
        EVENT_HEARTBEAT => vec![
            slot(KEY_INTERVAL_MS, TY_UDINT, 0, MaxOccurs::Count(1), 1),
            slot(KEY_SEQ_BASE, TY_ULINT, 0, MaxOccurs::Count(1), 2),
        ],
        EVENT_LOGGER_STARTED => {
            vec![slot(KEY_COLD_START, TY_BOOL, 1, MaxOccurs::Count(1), 1)]
        }
        EVENT_LOGGER_STOPPED | EVENT_BUFFER_CLEARED => Vec::new(),
        EVENT_RECORDS_DROPPED => vec![
            slot(KEY_DROPPED_COUNT, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_FIRST_LOST_SEQ, TY_ULINT, 1, MaxOccurs::Count(1), 2),
            slot(KEY_LAST_LOST_SEQ, TY_ULINT, 1, MaxOccurs::Count(1), 3),
            slot(KEY_WINDOW_START, TY_DATE_TIME, 0, MaxOccurs::Count(1), 4),
            slot(KEY_WINDOW_END, TY_DATE_TIME, 0, MaxOccurs::Count(1), 5),
        ],
        EVENT_SOURCE_REGISTERED => vec![
            slot(
                KEY_REGISTERED_SOURCE_ID,
                TY_UDINT,
                1,
                MaxOccurs::Count(1),
                1,
            ),
            slot(KEY_SOURCE_PATH, TY_STRING, 0, MaxOccurs::Count(1), 2),
        ],
        EVENT_DEFINITION_CHANGED => vec![
            slot(KEY_DEF_HASH_OLD, TY_BYTES, 1, MaxOccurs::Count(1), 1),
            slot(KEY_DEF_HASH_NEW, TY_BYTES, 1, MaxOccurs::Count(1), 2),
            slot(KEY_EPOCH_ID, TY_ULINT, 1, MaxOccurs::Count(1), 3),
        ],
        EVENT_TIME_SYNC_CHANGED => vec![
            slot(KEY_CLOCK_QUALITY, TY_UINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_WINDOW_START, TY_DATE_TIME, 0, MaxOccurs::Count(1), 2),
            slot(KEY_WINDOW_END, TY_DATE_TIME, 0, MaxOccurs::Count(1), 3),
        ],
        EVENT_SOURCE_HIGH_WATER => {
            vec![slot(
                KEY_SOURCE_HIGH_WATER,
                TY_ULINT,
                1,
                MaxOccurs::Count(1),
                1,
            )]
        }
        EVENT_CONDITION_ACTIVE | EVENT_CONDITION_CLEARED => condition_slots(true),
        EVENT_CONDITION_ACKNOWLEDGED | EVENT_CONDITION_CONFIRMED => vec![
            slot(KEY_CONDITION_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_CORRELATION_ID, TY_UDINT, 0, MaxOccurs::Count(1), 2),
            slot(KEY_ACK_BY, TY_STRING, 0, MaxOccurs::Count(1), 3),
            slot(KEY_REASON, TY_STRING, 0, MaxOccurs::Count(1), 4),
        ],
        EVENT_CONDITION_SHELVED | EVENT_CONDITION_UNSHELVED => vec![
            slot(KEY_CONDITION_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_CORRELATION_ID, TY_UDINT, 0, MaxOccurs::Count(1), 2),
            slot(KEY_SHELVE_SECS, TY_UDINT, 0, MaxOccurs::Count(1), 3),
            slot(KEY_REASON, TY_STRING, 0, MaxOccurs::Count(1), 4),
        ],
        EVENT_CONDITION_SUPPRESSED
        | EVENT_CONDITION_UNSUPPRESSED
        | EVENT_CONDITION_OUT_OF_SERVICE
        | EVENT_CONDITION_IN_SERVICE
        | EVENT_CONDITION_RESET => vec![
            slot(KEY_CONDITION_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_CORRELATION_ID, TY_UDINT, 0, MaxOccurs::Count(1), 2),
            slot(KEY_REASON, TY_STRING, 0, MaxOccurs::Count(1), 3),
        ],
        EVENT_CONDITION_COMMENTED => vec![
            slot(KEY_CONDITION_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_CORRELATION_ID, TY_UDINT, 0, MaxOccurs::Count(1), 2),
            slot(KEY_COMMENT, TY_STRING, 1, MaxOccurs::Count(1), 3),
        ],
        EVENT_CONDITION_PRIORITY_CHANGED => vec![
            slot(KEY_CONDITION_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_CORRELATION_ID, TY_UDINT, 0, MaxOccurs::Count(1), 2),
            slot(KEY_PREVIOUS_PRIORITY, TY_UINT, 0, MaxOccurs::Count(1), 3),
            slot(KEY_NEW_PRIORITY, TY_UINT, 1, MaxOccurs::Count(1), 4),
        ],
        EVENT_REFRESH_START | EVENT_REFRESH_END => vec![
            slot(KEY_REFRESH_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_GROUP_ID, TY_UDINT, 0, MaxOccurs::Count(1), 2),
        ],
        EVENT_RECIPE_LOADED | EVENT_RECIPE_APPROVED => vec![
            slot(KEY_RECIPE_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_RECIPE_VERSION, TY_STRING, 0, MaxOccurs::Count(1), 2),
            slot(KEY_ACTOR, TY_STRING, 0, MaxOccurs::Count(1), 3),
        ],
        EVENT_BATCH_EVENT => vec![
            slot(KEY_BATCH_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_ACTION_ID, TY_UDINT, 0, MaxOccurs::Count(1), 2),
            slot(KEY_REASON, TY_STRING, 0, MaxOccurs::Count(1), 3),
        ],
        EVENT_MATERIAL_ADDITION => vec![
            slot(KEY_BATCH_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_MATERIAL_ID, TY_UDINT, 1, MaxOccurs::Count(1), 2),
            slot(KEY_QUANTITY, TY_LREAL, 1, MaxOccurs::Count(1), 3),
            slot(KEY_UNIT, TY_UINT, 0, MaxOccurs::Count(1), 4),
        ],
        EVENT_OPERATOR_ACTION => vec![
            slot(KEY_ACTION_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_ACTOR, TY_STRING, 1, MaxOccurs::Count(1), 2),
            slot(KEY_CONTEXT_REF, TY_UDINT, 0, MaxOccurs::Count(1), 3),
            slot(KEY_WORKSTATION, TY_STRING, 0, MaxOccurs::Count(1), 4),
        ],
        EVENT_OPERATOR_LOGIN | EVENT_OPERATOR_LOGOUT => vec![
            slot(KEY_ACTOR, TY_STRING, 1, MaxOccurs::Count(1), 1),
            slot(KEY_WORKSTATION, TY_STRING, 0, MaxOccurs::Count(1), 2),
        ],
        EVENT_PARAMETER_CHANGE => vec![
            slot(KEY_ACTOR, TY_STRING, 1, MaxOccurs::Count(1), 1),
            slot(KEY_CONTEXT_REF, TY_UDINT, 0, MaxOccurs::Count(1), 2),
            slot(KEY_VALUE_ID, TY_UDINT, 1, MaxOccurs::Count(1), 3),
            value_slot(KEY_PREVIOUS_VALUE, 0, MaxOccurs::Count(1), 4),
            value_slot(KEY_NEW_VALUE, 1, MaxOccurs::Count(1), 5),
            slot(KEY_REASON, TY_STRING, 0, MaxOccurs::Count(1), 6),
        ],
        EVENT_ESIGNATURE => vec![
            slot(KEY_ACTOR, TY_STRING, 1, MaxOccurs::Count(1), 1),
            slot(KEY_SIGNATURE_MEANING, TY_UINT, 1, MaxOccurs::Count(1), 2),
            slot(KEY_SIGNED_EVENT_SEQ, TY_ULINT, 1, MaxOccurs::Count(1), 3),
            slot(KEY_EFFECTIVE_TIME, TY_DATE_TIME, 0, MaxOccurs::Count(1), 4),
            slot(KEY_CORRECTION_OF, TY_ULINT, 0, MaxOccurs::Count(1), 5),
            slot(KEY_WORKSTATION, TY_STRING, 0, MaxOccurs::Count(1), 6),
        ],
        EVENT_SECURITY_ACCESS_FAILURE => vec![
            slot(KEY_ACTOR, TY_STRING, 0, MaxOccurs::Count(1), 1),
            slot(KEY_AUTH_RESULT, TY_UINT, 1, MaxOccurs::Count(1), 2),
            slot(KEY_WORKSTATION, TY_STRING, 0, MaxOccurs::Count(1), 3),
            slot(KEY_REASON, TY_STRING, 0, MaxOccurs::Count(1), 4),
        ],
        EVENT_PROGRAM_DOWNLOAD => vec![
            slot(KEY_PROGRAM_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
            slot(KEY_ACTOR, TY_STRING, 0, MaxOccurs::Count(1), 2),
            slot(KEY_DEF_HASH_NEW, TY_BYTES, 0, MaxOccurs::Count(1), 3),
        ],
        _ => return None,
    };

    let spec = event_spec(id)?;
    Some(EventTypeDefinition {
        id,
        name: spec.name.to_string(),
        profile: match spec.group {
            EventGroup::Base => "Core".to_string(),
            EventGroup::System
            | EventGroup::Condition
            | EventGroup::Procedural
            | EventGroup::Regulated => "Full".to_string(),
        },
        slots,
    })
}

fn condition_slots(include_class_and_severity: bool) -> Vec<SlotDefinition> {
    let mut slots = vec![slot(KEY_CONDITION_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1)];
    let mut order = 2;
    if include_class_and_severity {
        slots.push(slot(
            KEY_CONDITION_CLASS,
            TY_UINT,
            1,
            MaxOccurs::Count(1),
            order,
        ));
        order += 1;
        slots.push(slot(KEY_SEVERITY, TY_UINT, 1, MaxOccurs::Count(1), order));
        order += 1;
    }
    slots.push(slot(
        KEY_CORRELATION_ID,
        TY_UDINT,
        0,
        MaxOccurs::Count(1),
        order,
    ));
    slots.push(slot(
        KEY_CAUSE_OPERAND,
        TY_UDINT,
        0,
        MaxOccurs::Unbounded("unbounded".to_string()),
        order + 1,
    ));
    slots
}

/// A small definition covering the positive record-vector spine.
pub fn sample_definition() -> DefinitionFile {
    DefinitionFile {
        header: DefinitionHeader {
            wire_version: 2,
            semantic_version: "1.0.0".to_string(),
            profiles: vec!["Core".to_string(), "Full".to_string()],
            conformance_level: "Producer-Full".to_string(),
            caps: DefinitionCaps {
                crc: true,
                source_high_water: true,
            },
            constraints: DefinitionConstraints {
                max_record_size: PRODUCER_MAX_RECORD_SIZE,
                max_slots: 16,
                overflow_policy: OverflowPolicy::OverwriteOldest,
            },
            epoch_strategy: EpochStrategy::Retain,
            content_hash: String::new(),
        },
        event_types: canonical_event_types(),
        sources: vec![SourceDefinition {
            source_id: 66,
            name: "UnitA.Phase1".to_string(),
            path: vec![
                "Area1".to_string(),
                "UnitA".to_string(),
                "Phase1".to_string(),
            ],
            hierarchy: vec!["area".to_string(), "unit".to_string(), "phase".to_string()],
            dynamic: false,
        }],
        state_machines: vec![StateMachineDefinition {
            state_machine_id: 7,
            name: "CoreProcedure".to_string(),
            category: 2,
            procedural_model: Some("ISA-88".to_string()),
            enum_set: "CoreProcedureStates".to_string(),
        }],
        conditions: vec![ConditionDefinition {
            condition_id: 9001,
            name: "HighPhAlarm".to_string(),
            condition_class: 0,
            default_severity: 900,
            cause_operands: vec![CauseOperandDefinition {
                operand_id: 1,
                name: "Level".to_string(),
            }],
        }],
        message_templates: vec![MessageTemplateDefinition {
            message_template_id: 1001,
            name: "Status".to_string(),
            format: "Status: {1}".to_string(),
            arg_types: vec![TY_STRING],
        }],
        values: vec![
            ValueDefinition {
                value_id: 2001,
                name: "Temperature".to_string(),
                data_type: TY_REAL,
                semantic_role: 0,
                unit: None,
                deadband: Some(Deadband {
                    decimal: Some("0.5".to_string()),
                    scaled: None,
                }),
                sampling_policy: None,
            },
            ValueDefinition {
                value_id: 2002,
                name: "BatchCount".to_string(),
                data_type: TY_DINT,
                semantic_role: 3,
                unit: None,
                deadband: None,
                sampling_policy: Some("on-change".to_string()),
            },
            ValueDefinition {
                value_id: 2003,
                name: "Enabled".to_string(),
                data_type: TY_BOOL,
                semantic_role: 0,
                unit: None,
                deadband: None,
                sampling_policy: Some("on-change".to_string()),
            },
            ValueDefinition {
                value_id: 2004,
                name: "SmallSigned".to_string(),
                data_type: TY_SINT,
                semantic_role: 0,
                unit: None,
                deadband: None,
                sampling_policy: Some("on-change".to_string()),
            },
            ValueDefinition {
                value_id: 2005,
                name: "SmallUnsigned".to_string(),
                data_type: TY_USINT,
                semantic_role: 0,
                unit: None,
                deadband: None,
                sampling_policy: Some("on-change".to_string()),
            },
            ValueDefinition {
                value_id: 2006,
                name: "SignedWord".to_string(),
                data_type: TY_INT,
                semantic_role: 0,
                unit: None,
                deadband: None,
                sampling_policy: Some("on-change".to_string()),
            },
            ValueDefinition {
                value_id: 2007,
                name: "UnsignedWord".to_string(),
                data_type: TY_UINT,
                semantic_role: 0,
                unit: None,
                deadband: None,
                sampling_policy: Some("on-change".to_string()),
            },
            ValueDefinition {
                value_id: 2008,
                name: "UnsignedDoubleWord".to_string(),
                data_type: TY_UDINT,
                semantic_role: 0,
                unit: None,
                deadband: None,
                sampling_policy: Some("on-change".to_string()),
            },
            ValueDefinition {
                value_id: 2009,
                name: "UnsignedLong".to_string(),
                data_type: TY_ULINT,
                semantic_role: 0,
                unit: None,
                deadband: None,
                sampling_policy: Some("on-change".to_string()),
            },
            ValueDefinition {
                value_id: 2010,
                name: "SignedLong".to_string(),
                data_type: TY_LINT,
                semantic_role: 0,
                unit: None,
                deadband: None,
                sampling_policy: Some("on-change".to_string()),
            },
            ValueDefinition {
                value_id: 2011,
                name: "HighPrecision".to_string(),
                data_type: TY_LREAL,
                semantic_role: 0,
                unit: None,
                deadband: None,
                sampling_policy: Some("on-change".to_string()),
            },
            ValueDefinition {
                value_id: 2012,
                name: "StatusText".to_string(),
                data_type: TY_STRING,
                semantic_role: 5,
                unit: None,
                deadband: None,
                sampling_policy: Some("on-change".to_string()),
            },
        ],
        units: Vec::new(),
        enum_sets: vec![EnumSetDefinition {
            name: "CoreProcedureStates".to_string(),
            members: vec![
                EnumMember {
                    value: 3,
                    label: "Pausing".to_string(),
                },
                EnumMember {
                    value: 4,
                    label: "Paused".to_string(),
                },
            ],
        }],
        recipe_definitions: Vec::new(),
        batch_definitions: Vec::new(),
        material_definitions: Vec::new(),
        operator_definitions: Vec::new(),
        e_signature_meanings: SIGNATURE_MEANING_VALUES
            .iter()
            .map(|meaning| ESignatureMeaningDefinition {
                meaning: meaning.value,
                label: meaning.label.to_string(),
            })
            .collect(),
        severity_scale: SeverityScale {
            name: "baseline".to_string(),
            low: SeverityBandDefinition { min: 1, max: 332 },
            medium: SeverityBandDefinition { min: 333, max: 666 },
            high: SeverityBandDefinition {
                min: 667,
                max: 1000,
            },
        },
    }
}

fn slot(
    key: u16,
    tlv_type: u8,
    min_occurs: u16,
    max_occurs: MaxOccurs,
    order_class: u16,
) -> SlotDefinition {
    SlotDefinition {
        key,
        tlv_type: Some(tlv_type),
        value_payload: false,
        min_occurs,
        max_occurs,
        order_class,
    }
}

fn value_slot(
    key: u16,
    min_occurs: u16,
    max_occurs: MaxOccurs,
    order_class: u16,
) -> SlotDefinition {
    SlotDefinition {
        key,
        tlv_type: None,
        value_payload: true,
        min_occurs,
        max_occurs,
        order_class,
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_definition_covers_phase2_positive_vector_events() {
        let definition = sample_definition();
        let events = definition
            .event_types
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>();

        assert_eq!(events.len(), EVENT_SPECS.len());
        assert!(events.contains(&EVENT_STATE_TRANSITION));
        assert!(events.contains(&EVENT_VALUE_CHANGED));
        assert!(events.contains(&EVENT_MESSAGE));
        assert!(events.contains(&EVENT_RECORDS_DROPPED));
        assert!(events.contains(&EVENT_SOURCE_HIGH_WATER));
        assert_eq!(definition.event_types[0].slots[0].key, KEY_STATE_MACHINE_ID);
        assert_eq!(definition.event_types[0].slots[0].tlv_type, Some(TY_UDINT));
        assert!(definition.event_types[1].slots[2].value_payload);
        assert_eq!(
            definition.header.constraints.max_record_size,
            PRODUCER_MAX_RECORD_SIZE
        );
        assert!(!definition.e_signature_meanings.is_empty());
    }

    #[test]
    fn deadband_model_contains_no_floating_point_type() {
        let deadband = Deadband {
            decimal: Some("0.125".to_string()),
            scaled: Some(ScaledInteger {
                value: 125,
                scale: 3,
            }),
        };

        assert_eq!(deadband.decimal.as_deref(), Some("0.125"));
        assert_eq!(deadband.scaled.as_ref().unwrap().value, 125);
    }
}
