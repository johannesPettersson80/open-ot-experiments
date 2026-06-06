//! Definition-file content model (§9.1).
//!
//! These typed structs map record ids, value keys, enum sets, and units to human meaning.
//! [`sample_definition`] builds the positive spine that the hash, schema, and resolver tests
//! resolve carriage vectors against.

use open_ot_carriage::registry::{
    EVENT_MESSAGE, EVENT_RECORDS_DROPPED, EVENT_SOURCE_HIGH_WATER, EVENT_STATE_TRANSITION,
    EVENT_VALUE_CHANGED, KEY_ARG, KEY_CATEGORY, KEY_DROPPED_COUNT, KEY_FIRST_LOST_SEQ,
    KEY_LAST_LOST_SEQ, KEY_MESSAGE_TEMPLATE_ID, KEY_NEW_STATE, KEY_NEW_VALUE, KEY_PREVIOUS_STATE,
    KEY_PREVIOUS_VALUE, KEY_QUALITY, KEY_SEVERITY, KEY_SOURCE_HIGH_WATER, KEY_STATE_MACHINE_ID,
    KEY_VALUE_ID, KEY_WINDOW_END, KEY_WINDOW_START, TY_DATE_TIME, TY_DINT, TY_REAL, TY_STRING,
    TY_UDINT, TY_UINT, TY_ULINT,
};
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
    /// Required TLV type tag for the slot payload.
    #[serde(rename = "type")]
    pub tlv_type: u8,
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
                max_record_size: 512,
                max_slots: 16,
                overflow_policy: OverflowPolicy::OverwriteOldest,
            },
            epoch_strategy: EpochStrategy::Retain,
            content_hash: String::new(),
        },
        event_types: vec![
            EventTypeDefinition {
                id: EVENT_STATE_TRANSITION,
                name: "StateTransition".to_string(),
                profile: "Core".to_string(),
                slots: vec![
                    slot(KEY_STATE_MACHINE_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
                    slot(KEY_CATEGORY, TY_UINT, 1, MaxOccurs::Count(1), 2),
                    slot(KEY_PREVIOUS_STATE, TY_UINT, 1, MaxOccurs::Count(1), 3),
                    slot(KEY_NEW_STATE, TY_UINT, 1, MaxOccurs::Count(1), 4),
                ],
            },
            EventTypeDefinition {
                id: EVENT_VALUE_CHANGED,
                name: "ValueChanged".to_string(),
                profile: "Core".to_string(),
                slots: vec![
                    slot(KEY_VALUE_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
                    slot(KEY_PREVIOUS_VALUE, TY_REAL, 0, MaxOccurs::Count(1), 2),
                    slot(KEY_NEW_VALUE, TY_REAL, 1, MaxOccurs::Count(1), 3),
                    slot(KEY_QUALITY, TY_UINT, 0, MaxOccurs::Count(1), 4),
                ],
            },
            EventTypeDefinition {
                id: EVENT_MESSAGE,
                name: "Message".to_string(),
                profile: "Core".to_string(),
                slots: vec![
                    slot(KEY_MESSAGE_TEMPLATE_ID, TY_UDINT, 1, MaxOccurs::Count(1), 1),
                    slot(
                        KEY_ARG,
                        TY_STRING,
                        0,
                        MaxOccurs::Unbounded("unbounded".to_string()),
                        2,
                    ),
                    slot(KEY_SEVERITY, TY_UINT, 0, MaxOccurs::Count(1), 3),
                ],
            },
            EventTypeDefinition {
                id: EVENT_RECORDS_DROPPED,
                name: "RecordsDropped".to_string(),
                profile: "Full".to_string(),
                slots: vec![
                    slot(KEY_DROPPED_COUNT, TY_UDINT, 1, MaxOccurs::Count(1), 1),
                    slot(KEY_FIRST_LOST_SEQ, TY_ULINT, 1, MaxOccurs::Count(1), 2),
                    slot(KEY_LAST_LOST_SEQ, TY_ULINT, 1, MaxOccurs::Count(1), 3),
                    slot(KEY_WINDOW_START, TY_DATE_TIME, 0, MaxOccurs::Count(1), 4),
                    slot(KEY_WINDOW_END, TY_DATE_TIME, 0, MaxOccurs::Count(1), 5),
                ],
            },
            EventTypeDefinition {
                id: EVENT_SOURCE_HIGH_WATER,
                name: "SourceHighWater".to_string(),
                profile: "Full".to_string(),
                slots: vec![slot(
                    KEY_SOURCE_HIGH_WATER,
                    TY_ULINT,
                    1,
                    MaxOccurs::Count(1),
                    1,
                )],
            },
        ],
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
            procedural_model: Some("CoreProcedure".to_string()),
            enum_set: "CoreProcedureStates".to_string(),
        }],
        conditions: Vec::new(),
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
        ],
        units: Vec::new(),
        enum_sets: vec![EnumSetDefinition {
            name: "CoreProcedureStates".to_string(),
            members: vec![
                EnumMember {
                    value: 3,
                    label: "Previous".to_string(),
                },
                EnumMember {
                    value: 4,
                    label: "Current".to_string(),
                },
            ],
        }],
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
        tlv_type,
        min_occurs,
        max_occurs,
        order_class,
    }
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

        assert_eq!(
            events,
            vec![
                EVENT_STATE_TRANSITION,
                EVENT_VALUE_CHANGED,
                EVENT_MESSAGE,
                EVENT_RECORDS_DROPPED,
                EVENT_SOURCE_HIGH_WATER,
            ]
        );
        assert_eq!(definition.event_types[0].slots[0].key, KEY_STATE_MACHINE_ID);
        assert_eq!(definition.event_types[0].slots[0].tlv_type, TY_UDINT);
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
