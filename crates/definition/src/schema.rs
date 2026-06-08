//! Record and definition schema validation (§5.2, §6.2.1).
//!
//! Checks a decoded record against the event's declared slots — fixed type per core key,
//! occurrence bounds, order class, repeated-slot contiguity, and the trailing/ascending rule for
//! private-extension keys. On any violation the record becomes a placeholder with raw slots kept,
//! never a guessed resolution. A defective definition makes every matching record placeholder.

use crate::model::{DefinitionFile, EventTypeDefinition, MaxOccurs, SlotDefinition};
use open_ot_carriage::registry::{
    FieldKind, KEY_NEW_VALUE, KEY_PREVIOUS_VALUE, KEY_VALUE_ID, PROCEDURAL_MODELS, TY_BYTES,
    TY_STRING, field_spec, is_core_key, is_vendor_key, tlv_type_spec,
};
use open_ot_carriage::wire::{FLAG_HAS_CRC, Record, Slot, WireError};
use std::collections::{BTreeMap, BTreeSet};

const VALUE_PAYLOAD_SCHEMA_TYPE: u8 = 0xFF;

/// Result of validating a record against a definition-file event schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaValidation {
    /// The record satisfies the declared schema. Unknown private-extension keys were skipped.
    Valid {
        /// Value keys of the trailing private-extension slots that were accepted.
        extension_keys: Vec<u16>,
    },
    /// The stream must continue, but this record must resolve as a placeholder.
    Placeholder(PlaceholderRecord),
}

/// Envelope and raw slots retained when schema validation fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceholderRecord {
    /// Source stream id from the record envelope.
    pub source_id: u32,
    /// Registry event-type id from the record envelope.
    pub event_type_id: u32,
    /// Run id from the record envelope.
    pub run_id: u64,
    /// Source-local sequence number from the record envelope.
    pub seq: u64,
    /// Raw slots preserved for a later correct-definition pass.
    pub slots: Vec<Slot>,
    /// The schema violation that forced the placeholder.
    pub reason: SchemaViolation,
}

/// Concrete reason a record cannot be semantically resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaViolation {
    /// The definition itself is defective (applies to every matching record).
    Definition(DefinitionSchemaViolation),
    /// The event-type id is not present in the definition.
    UnknownEventId(u32),
    /// The encoded record exceeds the definition's maximum record size.
    RecordTooLarge {
        /// Actual encoded record size in bytes.
        actual: usize,
        /// Maximum allowed record size in bytes.
        max: usize,
    },
    /// The record carries more slots than the definition allows.
    TooManySlots {
        /// Actual slot count.
        actual: usize,
        /// Maximum allowed slot count.
        max: usize,
    },
    /// A core value key appeared that the event schema does not declare.
    UnexpectedCoreKey {
        /// The unexpected core value key.
        key: u16,
    },
    /// A private-extension slot appeared before a core slot instead of trailing.
    VendorExtensionNotTrailing {
        /// Value key of the misplaced extension slot.
        key: u16,
    },
    /// Private-extension slots were not in ascending key order.
    VendorExtensionOutOfOrder {
        /// Value key of the preceding extension slot.
        previous: u16,
        /// Value key that broke ascending order.
        key: u16,
    },
    /// A slot's TLV type did not match the type the schema requires.
    TypeMismatch {
        /// Value key of the slot.
        key: u16,
        /// TLV type actually present.
        actual: u8,
        /// TLV type the schema requires.
        expected: u8,
    },
    /// A slot carried a TLV type tag unknown to this prototype.
    UnknownTlvType {
        /// Value key of the slot.
        key: u16,
        /// The unknown TLV type tag.
        ty: u8,
    },
    /// A reserved core key was used.
    ReservedCoreKey {
        /// The reserved core value key.
        key: u16,
    },
    /// A fixed-width slot's payload length did not match the type's width.
    FixedWidthMismatch {
        /// Value key of the slot.
        key: u16,
        /// TLV type tag of the slot.
        ty: u8,
        /// Actual payload length in bytes.
        actual: usize,
        /// Required payload length in bytes.
        expected: usize,
    },
    /// A required slot occurred fewer times than its minimum.
    MissingRequired {
        /// Value key of the slot.
        key: u16,
        /// Minimum required occurrences.
        min: u16,
        /// Actual number of occurrences.
        actual: usize,
    },
    /// A slot occurred more times than its maximum.
    TooManyOccurrences {
        /// Value key of the slot.
        key: u16,
        /// Maximum allowed occurrences.
        max: u16,
        /// Actual number of occurrences.
        actual: usize,
    },
    /// Repeated occurrences of a slot were not contiguous.
    RepeatedSlotNotContiguous {
        /// Value key of the non-contiguous slot.
        key: u16,
    },
    /// Slots were not in ascending order class.
    OrderClassViolation {
        /// Order class of the preceding slot.
        previous: u16,
        /// Order class that broke ascending order.
        current: u16,
        /// Value key of the offending slot.
        key: u16,
    },
    /// Re-encoding the record for validation failed.
    Encode(WireError),
}

/// Definition-schema defect. A defective definition means every matching record placeholders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionSchemaViolation {
    /// Two event types share an id.
    DuplicateEventId(u32),
    /// An event declares the same slot key twice.
    DuplicateSlotKey {
        /// Event-type id with the duplicate.
        event_id: u32,
        /// The duplicated value key.
        key: u16,
    },
    /// An event's slots are not declared in ascending order class.
    OrderClassNotAscending {
        /// Event-type id with the ordering defect.
        event_id: u32,
        /// Order class of the preceding slot.
        previous: u16,
        /// Order class that broke ascending order.
        current: u16,
    },
    /// A core slot's declared type does not match the registry's fixed type.
    CoreTypeMismatch {
        /// Event-type id with the mismatch.
        event_id: u32,
        /// Value key of the slot.
        key: u16,
        /// Type the definition declared.
        actual: u8,
        /// Type the registry fixes for this core key.
        expected: u8,
    },
    /// A slot declares a TLV type tag unknown to this prototype.
    UnknownTlvType {
        /// Event-type id with the unknown type.
        event_id: u32,
        /// Value key of the slot.
        key: u16,
        /// The unknown TLV type tag.
        ty: u8,
    },
    /// A reserved core key was declared.
    ReservedCoreKey {
        /// Event-type id with the reserved key.
        event_id: u32,
        /// The reserved core value key.
        key: u16,
    },
    /// A slot's `maxOccurs` is invalid (for example below `minOccurs`).
    InvalidMaxOccurs {
        /// Event-type id with the invalid bound.
        event_id: u32,
        /// Value key of the slot.
        key: u16,
    },
    /// A procedural state machine names a model not in the registry.
    UnknownProceduralModel {
        /// State-machine id with the unknown model.
        state_machine_id: u32,
        /// Unknown procedural model label.
        model: String,
    },
    /// A procedural state machine references an enum set missing from the definition.
    MissingEnumSet {
        /// State-machine id with the missing enum-set reference.
        state_machine_id: u32,
        /// Missing enum-set name.
        enum_set: String,
    },
    /// A procedural state does not match the named model's canonical value/label pair.
    ProceduralStateMismatch {
        /// State-machine id with the mismatch.
        state_machine_id: u32,
        /// Procedural model label.
        model: String,
        /// Numeric state value in the enum set.
        value: u16,
        /// Label in the enum set.
        label: String,
    },
}

/// Validates definition event schemas independent of a record.
pub fn validate_definition(definition: &DefinitionFile) -> Result<(), DefinitionSchemaViolation> {
    let mut event_ids = BTreeSet::new();
    for event in &definition.event_types {
        if !event_ids.insert(event.id) {
            return Err(DefinitionSchemaViolation::DuplicateEventId(event.id));
        }
        validate_event_definition(event)?;
    }
    validate_state_machine_models(definition)?;
    Ok(())
}

/// Validates one decoded record against the matching definition event schema.
pub fn validate_record(
    record: &Record,
    record_len: usize,
    definition: &DefinitionFile,
) -> SchemaValidation {
    if let Err(reason) = validate_definition(definition) {
        return placeholder(record, SchemaViolation::Definition(reason));
    }

    if record_len > definition.header.constraints.max_record_size as usize {
        return placeholder(
            record,
            SchemaViolation::RecordTooLarge {
                actual: record_len,
                max: definition.header.constraints.max_record_size as usize,
            },
        );
    }

    if record.slots.len() > definition.header.constraints.max_slots as usize {
        return placeholder(
            record,
            SchemaViolation::TooManySlots {
                actual: record.slots.len(),
                max: definition.header.constraints.max_slots as usize,
            },
        );
    }

    let Some(event) = definition
        .event_types
        .iter()
        .find(|event| event.id == record.event_type_id)
    else {
        return placeholder(
            record,
            SchemaViolation::UnknownEventId(record.event_type_id),
        );
    };

    match validate_record_slots(record, event, definition) {
        Ok(extension_keys) => SchemaValidation::Valid { extension_keys },
        Err(reason) => placeholder(record, reason),
    }
}

fn validate_state_machine_models(
    definition: &DefinitionFile,
) -> Result<(), DefinitionSchemaViolation> {
    for machine in &definition.state_machines {
        let Some(model_name) = machine.procedural_model.as_deref() else {
            continue;
        };
        let Some(model) = PROCEDURAL_MODELS
            .iter()
            .find(|model| model.name.eq_ignore_ascii_case(model_name))
        else {
            return Err(DefinitionSchemaViolation::UnknownProceduralModel {
                state_machine_id: machine.state_machine_id,
                model: model_name.to_string(),
            });
        };
        let Some(enum_set) = definition
            .enum_sets
            .iter()
            .find(|enum_set| enum_set.name == machine.enum_set)
        else {
            return Err(DefinitionSchemaViolation::MissingEnumSet {
                state_machine_id: machine.state_machine_id,
                enum_set: machine.enum_set.clone(),
            });
        };
        for member in &enum_set.members {
            if !model
                .states
                .iter()
                .any(|state| state.value == member.value && state.label == member.label)
            {
                return Err(DefinitionSchemaViolation::ProceduralStateMismatch {
                    state_machine_id: machine.state_machine_id,
                    model: model.name.to_string(),
                    value: member.value,
                    label: member.label.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Computes encoded length using the record's CRC flag.
pub fn encoded_record_len(record: &Record) -> Result<usize, WireError> {
    let with_crc = record.flags & FLAG_HAS_CRC != 0;
    Ok(record.encode(with_crc)?.len())
}

fn validate_event_definition(event: &EventTypeDefinition) -> Result<(), DefinitionSchemaViolation> {
    let mut previous_order_class = None;
    let mut keys = BTreeSet::new();

    for slot in &event.slots {
        if !keys.insert(slot.key) {
            return Err(DefinitionSchemaViolation::DuplicateSlotKey {
                event_id: event.id,
                key: slot.key,
            });
        }

        if let Some(previous) = previous_order_class
            && slot.order_class <= previous
        {
            return Err(DefinitionSchemaViolation::OrderClassNotAscending {
                event_id: event.id,
                previous,
                current: slot.order_class,
            });
        }
        previous_order_class = Some(slot.order_class);

        validate_max_occurs(event.id, slot)?;
        validate_slot_definition_type(event.id, slot)?;
    }

    Ok(())
}

fn validate_max_occurs(
    event_id: u32,
    slot: &SlotDefinition,
) -> Result<(), DefinitionSchemaViolation> {
    match &slot.max_occurs {
        MaxOccurs::Count(max) if *max < slot.min_occurs || *max == 0 => {
            Err(DefinitionSchemaViolation::InvalidMaxOccurs {
                event_id,
                key: slot.key,
            })
        }
        MaxOccurs::Unbounded(value) if value != "unbounded" => {
            Err(DefinitionSchemaViolation::InvalidMaxOccurs {
                event_id,
                key: slot.key,
            })
        }
        _ => Ok(()),
    }
}

fn validate_slot_definition_type(
    event_id: u32,
    slot: &SlotDefinition,
) -> Result<(), DefinitionSchemaViolation> {
    if let Some(field) = field_spec(slot.key) {
        match field.kind {
            FieldKind::Fixed(expected) => {
                if slot.tlv_type != Some(expected) || slot.value_payload {
                    return Err(DefinitionSchemaViolation::CoreTypeMismatch {
                        event_id,
                        key: slot.key,
                        actual: slot.tlv_type.unwrap_or(VALUE_PAYLOAD_SCHEMA_TYPE),
                        expected,
                    });
                }
            }
            FieldKind::ValuePayload => {
                if !slot.value_payload || slot.tlv_type.is_some() {
                    return Err(DefinitionSchemaViolation::CoreTypeMismatch {
                        event_id,
                        key: slot.key,
                        actual: slot.tlv_type.unwrap_or(VALUE_PAYLOAD_SCHEMA_TYPE),
                        expected: VALUE_PAYLOAD_SCHEMA_TYPE,
                    });
                }
                return Ok(());
            }
            FieldKind::Reserved => {
                return Err(DefinitionSchemaViolation::ReservedCoreKey {
                    event_id,
                    key: slot.key,
                });
            }
        }
    } else if is_core_key(slot.key) {
        return Err(DefinitionSchemaViolation::ReservedCoreKey {
            event_id,
            key: slot.key,
        });
    }

    if slot.value_payload {
        return Err(DefinitionSchemaViolation::CoreTypeMismatch {
            event_id,
            key: slot.key,
            actual: VALUE_PAYLOAD_SCHEMA_TYPE,
            expected: slot.tlv_type.unwrap_or(VALUE_PAYLOAD_SCHEMA_TYPE),
        });
    }

    let Some(tlv_type) = slot.tlv_type else {
        return Err(DefinitionSchemaViolation::UnknownTlvType {
            event_id,
            key: slot.key,
            ty: VALUE_PAYLOAD_SCHEMA_TYPE,
        });
    };

    if tlv_type_spec(tlv_type).is_none() {
        return Err(DefinitionSchemaViolation::UnknownTlvType {
            event_id,
            key: slot.key,
            ty: tlv_type,
        });
    }

    Ok(())
}

fn validate_record_slots(
    record: &Record,
    event: &EventTypeDefinition,
    definition: &DefinitionFile,
) -> Result<Vec<u16>, SchemaViolation> {
    let schema_by_key = event
        .slots
        .iter()
        .map(|slot| (slot.key, slot))
        .collect::<BTreeMap<_, _>>();

    let mut counts = BTreeMap::<u16, usize>::new();
    let mut positions = BTreeMap::<u16, Vec<usize>>::new();
    let mut known_order = Vec::<(u16, u16)>::new();
    let mut extension_keys = Vec::new();
    let mut previous_vendor_key = None;
    let mut saw_vendor_extension = false;

    for (index, slot) in record.slots.iter().enumerate() {
        if let Some(schema) = schema_by_key.get(&slot.key).copied() {
            if saw_vendor_extension {
                return Err(SchemaViolation::VendorExtensionNotTrailing { key: slot.key });
            }

            validate_record_slot_type(slot, schema)?;
            validate_value_payload_type(slot, record, definition)?;
            validate_payload_width(slot)?;

            known_order.push((schema.order_class, slot.key));
            *counts.entry(slot.key).or_default() += 1;
            positions.entry(slot.key).or_default().push(index);
        } else if is_vendor_key(slot.key) {
            saw_vendor_extension = true;
            if let Some(previous) = previous_vendor_key
                && slot.key <= previous
            {
                return Err(SchemaViolation::VendorExtensionOutOfOrder {
                    previous,
                    key: slot.key,
                });
            }
            previous_vendor_key = Some(slot.key);
            extension_keys.push(slot.key);
        } else {
            return Err(SchemaViolation::UnexpectedCoreKey { key: slot.key });
        }
    }

    for (key, occurrences) in &positions {
        if !is_contiguous(occurrences) {
            return Err(SchemaViolation::RepeatedSlotNotContiguous { key: *key });
        }
    }

    let mut previous_order_class = None;
    for (order_class, key) in known_order {
        if let Some(previous) = previous_order_class
            && order_class < previous
        {
            return Err(SchemaViolation::OrderClassViolation {
                previous,
                current: order_class,
                key,
            });
        }
        previous_order_class = Some(order_class);
    }

    for schema in &event.slots {
        let actual = counts.get(&schema.key).copied().unwrap_or_default();
        if actual < schema.min_occurs as usize {
            return Err(SchemaViolation::MissingRequired {
                key: schema.key,
                min: schema.min_occurs,
                actual,
            });
        }

        if let MaxOccurs::Count(max) = schema.max_occurs
            && actual > max as usize
        {
            return Err(SchemaViolation::TooManyOccurrences {
                key: schema.key,
                max,
                actual,
            });
        }
    }

    Ok(extension_keys)
}

fn validate_record_slot_type(slot: &Slot, schema: &SlotDefinition) -> Result<(), SchemaViolation> {
    if let Some(field) = field_spec(slot.key) {
        match field.kind {
            FieldKind::ValuePayload => {
                if tlv_type_spec(slot.ty).is_none() {
                    Err(SchemaViolation::UnknownTlvType {
                        key: slot.key,
                        ty: slot.ty,
                    })
                } else {
                    Ok(())
                }
            }
            FieldKind::Fixed(expected) if slot.ty != expected => {
                Err(SchemaViolation::TypeMismatch {
                    key: slot.key,
                    actual: slot.ty,
                    expected,
                })
            }
            FieldKind::Fixed(_) => Ok(()),
            FieldKind::Reserved => Err(SchemaViolation::ReservedCoreKey { key: slot.key }),
        }
    } else if is_core_key(slot.key) {
        Err(SchemaViolation::ReservedCoreKey { key: slot.key })
    } else if Some(slot.ty) != schema.tlv_type {
        Err(SchemaViolation::TypeMismatch {
            key: slot.key,
            actual: slot.ty,
            expected: schema.tlv_type.unwrap_or(VALUE_PAYLOAD_SCHEMA_TYPE),
        })
    } else {
        Ok(())
    }
}

fn validate_value_payload_type(
    slot: &Slot,
    record: &Record,
    definition: &DefinitionFile,
) -> Result<(), SchemaViolation> {
    if !matches!(slot.key, KEY_PREVIOUS_VALUE | KEY_NEW_VALUE) {
        return Ok(());
    }
    let Some(value_id) = record.slots.iter().find_map(|candidate| {
        (candidate.key == KEY_VALUE_ID && candidate.payload.len() == 4).then(|| {
            u32::from_le_bytes([
                candidate.payload[0],
                candidate.payload[1],
                candidate.payload[2],
                candidate.payload[3],
            ])
        })
    }) else {
        return Ok(());
    };
    let Some(value_def) = definition
        .values
        .iter()
        .find(|value_def| value_def.value_id == value_id)
    else {
        return Ok(());
    };
    if slot.ty != value_def.data_type {
        return Err(SchemaViolation::TypeMismatch {
            key: slot.key,
            actual: slot.ty,
            expected: value_def.data_type,
        });
    }
    Ok(())
}

fn validate_payload_width(slot: &Slot) -> Result<(), SchemaViolation> {
    let Some(spec) = tlv_type_spec(slot.ty) else {
        return Err(SchemaViolation::UnknownTlvType {
            key: slot.key,
            ty: slot.ty,
        });
    };

    if let Some(expected) = spec.fixed_width
        && slot.payload.len() != expected
    {
        return Err(SchemaViolation::FixedWidthMismatch {
            key: slot.key,
            ty: slot.ty,
            actual: slot.payload.len(),
            expected,
        });
    }

    if slot.payload.is_empty() && !matches!(slot.ty, TY_STRING | TY_BYTES) {
        return Err(SchemaViolation::FixedWidthMismatch {
            key: slot.key,
            ty: slot.ty,
            actual: 0,
            expected: spec.fixed_width.unwrap_or(1),
        });
    }

    Ok(())
}

fn placeholder(record: &Record, reason: SchemaViolation) -> SchemaValidation {
    SchemaValidation::Placeholder(PlaceholderRecord {
        source_id: record.source_id,
        event_type_id: record.event_type_id,
        run_id: record.run_id,
        seq: record.seq,
        slots: record.slots.clone(),
        reason,
    })
}

fn is_contiguous(positions: &[usize]) -> bool {
    positions
        .windows(2)
        .all(|window| window[1] == window[0] + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sample_definition;
    use open_ot_carriage::registry::{
        KEY_ARG, KEY_CATEGORY, KEY_CONDITION_ID, KEY_NEW_STATE, KEY_NEW_VALUE, KEY_SEVERITY,
        KEY_STATE_MACHINE_ID, TY_DINT, TY_REAL, TY_STRING, TY_UDINT, TY_UINT,
    };
    use open_ot_carriage::wire::{Record, Slot, decode};

    #[test]
    fn conformant_record_vectors_validate_against_sample_definition() {
        for bytes in [
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_state_transition.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_value_changed_real.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_value_changed_dint.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_value_changed_bool.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_value_changed_sint.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_value_changed_usint.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_value_changed_int.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_value_changed_uint.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_value_changed_udint.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_value_changed_ulint.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_value_changed_lint.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_value_changed_lreal.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_value_changed_string.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_message.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_active.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_cleared.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_acknowledged.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_confirmed.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_shelved.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_unshelved.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_suppressed.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_unsuppressed.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_out_of_service.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_in_service.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_reset.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_commented.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_condition_priority_changed.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_recipe_loaded.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_recipe_approved.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_batch_event.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_material_addition.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_operator_action.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_operator_login.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_operator_logout.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_security_access_failure.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_parameter_change_real.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_parameter_change_dint.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_parameter_change_bool.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_parameter_change_string.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_e_signature.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_records_dropped.hex"
            )),
            hex_bytes(include_str!(
                "../../carriage/vectors/conformant_source_high_water.hex"
            )),
        ] {
            let decoded = decode(&bytes).unwrap();
            assert_eq!(
                validate_record(&decoded.record, decoded.consumed, &sample_definition()),
                SchemaValidation::Valid {
                    extension_keys: Vec::new()
                }
            );
        }
    }

    #[test]
    fn codec_state_transition_vector_is_schema_violation_negative() {
        let bytes = hex_bytes(include_str!("../../carriage/vectors/state_transition.hex"));
        let decoded = decode(&bytes).unwrap();

        assert_placeholder_reason(
            validate_record(&decoded.record, decoded.consumed, &sample_definition()),
            SchemaViolation::TypeMismatch {
                key: KEY_STATE_MACHINE_ID,
                actual: TY_UINT,
                expected: TY_UDINT,
            },
        );
    }

    #[test]
    fn missing_required_field_placeholders() {
        let mut record = conformant_state_transition_record();
        record.slots.retain(|slot| slot.key != KEY_NEW_STATE);

        assert_placeholder_reason(
            validate_owned_record(&record),
            SchemaViolation::MissingRequired {
                key: KEY_NEW_STATE,
                min: 1,
                actual: 0,
            },
        );
    }

    #[test]
    fn occurrence_limit_placeholders() {
        let mut record = conformant_message_record();
        record
            .slots
            .push(Slot::new(KEY_SEVERITY, TY_UINT, 700u16.to_le_bytes()));

        assert_placeholder_reason(
            validate_owned_record(&record),
            SchemaViolation::TooManyOccurrences {
                key: KEY_SEVERITY,
                max: 1,
                actual: 2,
            },
        );
    }

    #[test]
    fn order_class_violation_placeholders() {
        let mut record = conformant_state_transition_record();
        record.slots.swap(0, 1);

        assert_placeholder_reason(
            validate_owned_record(&record),
            SchemaViolation::OrderClassViolation {
                previous: 2,
                current: 1,
                key: KEY_STATE_MACHINE_ID,
            },
        );
    }

    #[test]
    fn repeated_slots_must_be_contiguous() {
        let mut record = conformant_message_record();
        record.slots.push(Slot::new(KEY_ARG, TY_STRING, b"late"));

        assert_placeholder_reason(
            validate_owned_record(&record),
            SchemaViolation::RepeatedSlotNotContiguous { key: KEY_ARG },
        );
    }

    #[test]
    fn unknown_vendor_keys_are_skipped_only_when_trailing_and_ascending() {
        let mut record = conformant_state_transition_record();
        record
            .slots
            .push(Slot::new(0x8001, 0xFE, [0x01, 0x02, 0x03]));
        record.slots.push(Slot::new(0x8002, TY_UINT, [0x34, 0x12]));

        assert_eq!(
            validate_owned_record(&record),
            SchemaValidation::Valid {
                extension_keys: vec![0x8001, 0x8002]
            }
        );
    }

    #[test]
    fn vendor_extension_before_core_slot_placeholders() {
        let mut record = conformant_state_transition_record();
        record
            .slots
            .insert(1, Slot::new(0x8001, TY_UINT, [0x34, 0x12]));

        assert_placeholder_reason(
            validate_owned_record(&record),
            SchemaViolation::VendorExtensionNotTrailing { key: KEY_CATEGORY },
        );
    }

    #[test]
    fn vendor_extensions_must_be_ascending() {
        let mut record = conformant_state_transition_record();
        record.slots.push(Slot::new(0x8002, TY_UINT, [0x34, 0x12]));
        record.slots.push(Slot::new(0x8001, TY_UINT, [0x34, 0x12]));

        assert_placeholder_reason(
            validate_owned_record(&record),
            SchemaViolation::VendorExtensionOutOfOrder {
                previous: 0x8002,
                key: 0x8001,
            },
        );
    }

    #[test]
    fn unknown_core_key_placeholders() {
        let mut record = conformant_state_transition_record();
        record
            .slots
            .push(Slot::new(KEY_CONDITION_ID, TY_UDINT, 9u32.to_le_bytes()));

        assert_placeholder_reason(
            validate_owned_record(&record),
            SchemaViolation::UnexpectedCoreKey {
                key: KEY_CONDITION_ID,
            },
        );
    }

    #[test]
    fn scalar_zero_length_placeholders_but_string_zero_length_is_valid() {
        let mut scalar = conformant_message_record();
        scalar.slots.retain(|slot| slot.key != KEY_ARG);
        scalar.slots.retain(|slot| slot.key != KEY_SEVERITY);
        scalar.slots.push(Slot::new(KEY_SEVERITY, TY_UINT, []));
        assert_placeholder_reason(
            validate_owned_record(&scalar),
            SchemaViolation::FixedWidthMismatch {
                key: KEY_SEVERITY,
                ty: TY_UINT,
                actual: 0,
                expected: 2,
            },
        );

        let mut string = conformant_message_record();
        let arg_slot = string
            .slots
            .iter_mut()
            .find(|slot| slot.key == KEY_ARG)
            .unwrap();
        arg_slot.payload.clear();
        assert!(matches!(
            validate_owned_record(&string),
            SchemaValidation::Valid { .. }
        ));
    }

    #[test]
    fn constraints_placeholders() {
        let record = conformant_state_transition_record();
        let mut definition = sample_definition();
        definition.header.constraints.max_slots = 3;
        assert_placeholder_reason(
            validate_record(&record, encoded_record_len(&record).unwrap(), &definition),
            SchemaViolation::TooManySlots { actual: 4, max: 3 },
        );

        definition.header.constraints.max_slots = 16;
        definition.header.constraints.max_record_size = 64;
        assert_placeholder_reason(
            validate_record(&record, encoded_record_len(&record).unwrap(), &definition),
            SchemaViolation::RecordTooLarge {
                actual: 76,
                max: 64,
            },
        );
    }

    #[test]
    fn definition_core_type_mismatch_placeholders() {
        let record = conformant_state_transition_record();
        let mut definition = sample_definition();
        definition.event_types[0].slots[0].tlv_type = Some(TY_UINT);

        assert_placeholder_reason(
            validate_record(&record, encoded_record_len(&record).unwrap(), &definition),
            SchemaViolation::Definition(DefinitionSchemaViolation::CoreTypeMismatch {
                event_id: record.event_type_id,
                key: KEY_STATE_MACHINE_ID,
                actual: TY_UINT,
                expected: TY_UDINT,
            }),
        );
    }

    #[test]
    fn procedural_model_state_mismatch_rejects_definition() {
        let mut definition = sample_definition();
        let enum_set = definition
            .enum_sets
            .iter_mut()
            .find(|enum_set| enum_set.name == "CoreProcedureStates")
            .expect("sample ISA-88 enum set");
        enum_set.members[1].label = "Filling".to_string();

        assert_eq!(
            validate_definition(&definition),
            Err(DefinitionSchemaViolation::ProceduralStateMismatch {
                state_machine_id: 7,
                model: "ISA-88".to_string(),
                value: 4,
                label: "Filling".to_string(),
            })
        );
    }

    #[test]
    fn value_payload_slot_type_is_the_logged_data_type() {
        let mut record = conformant_value_changed_real_record();
        let slot = record
            .slots
            .iter_mut()
            .find(|slot| slot.key == KEY_NEW_VALUE)
            .unwrap();
        slot.ty = TY_DINT;
        slot.payload = 12i32.to_le_bytes().to_vec();

        assert_placeholder_reason(
            validate_owned_record(&record),
            SchemaViolation::TypeMismatch {
                key: KEY_NEW_VALUE,
                actual: TY_DINT,
                expected: TY_REAL,
            },
        );
    }

    #[test]
    fn value_payload_schema_must_not_pin_a_tlv_type() {
        let record = conformant_value_changed_real_record();
        let mut definition = sample_definition();
        let new_value_schema = definition.event_types[1]
            .slots
            .iter_mut()
            .find(|slot| slot.key == KEY_NEW_VALUE)
            .unwrap();
        new_value_schema.tlv_type = Some(TY_REAL);
        new_value_schema.value_payload = false;

        assert_placeholder_reason(
            validate_record(&record, encoded_record_len(&record).unwrap(), &definition),
            SchemaViolation::Definition(DefinitionSchemaViolation::CoreTypeMismatch {
                event_id: record.event_type_id,
                key: KEY_NEW_VALUE,
                actual: TY_REAL,
                expected: VALUE_PAYLOAD_SCHEMA_TYPE,
            }),
        );
    }

    fn validate_owned_record(record: &Record) -> SchemaValidation {
        validate_record(
            record,
            encoded_record_len(record).unwrap(),
            &sample_definition(),
        )
    }

    fn assert_placeholder_reason(actual: SchemaValidation, expected: SchemaViolation) {
        let SchemaValidation::Placeholder(placeholder) = actual else {
            panic!("expected placeholder");
        };
        assert_eq!(placeholder.reason, expected);
    }

    fn conformant_state_transition_record() -> Record {
        let bytes = hex_bytes(include_str!(
            "../../carriage/vectors/conformant_state_transition.hex"
        ));
        decode(&bytes).unwrap().record
    }

    fn conformant_message_record() -> Record {
        let bytes = hex_bytes(include_str!(
            "../../carriage/vectors/conformant_message.hex"
        ));
        decode(&bytes).unwrap().record
    }

    fn conformant_value_changed_real_record() -> Record {
        let bytes = hex_bytes(include_str!(
            "../../carriage/vectors/conformant_value_changed_real.hex"
        ));
        decode(&bytes).unwrap().record
    }

    fn hex_bytes(input: &str) -> Vec<u8> {
        input
            .split_whitespace()
            .map(|byte| u8::from_str_radix(byte, 16).unwrap())
            .collect()
    }
}
