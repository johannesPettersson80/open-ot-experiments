//! Record resolution (§9.3).
//!
//! Selects the definition by epoch, then turns a decoded record into typed, named fields, or a
//! placeholder that preserves the raw slots. Epoch selection is absolute-position based: a record
//! at or after `EpochFirstAbs` resolves against the current definition hash, otherwise against the
//! previous one. A definition-hash mismatch yields a drift placeholder rather than a guess.

use crate::hash::{DefinitionError, compute_content_hash};
use crate::model::{DefinitionFile, EnumMember, EventTypeDefinition, SourceDefinition};
use crate::schema::{
    PlaceholderRecord, SchemaValidation, SchemaViolation, encoded_record_len, validate_record,
};
use open_ot_carriage::control::ControlBlockSnapshot;
use open_ot_carriage::registry::{
    AUTH_RESULT_VALUES, CATEGORY_VALUES, FieldKind, KEY_AUTH_RESULT, KEY_CATEGORY,
    KEY_CAUSE_OPERAND, KEY_CONDITION_ID, KEY_MESSAGE_TEMPLATE_ID, KEY_NEW_STATE, KEY_NEW_VALUE,
    KEY_PREVIOUS_STATE, KEY_PREVIOUS_VALUE, KEY_SIGNATURE_MEANING, KEY_STATE_MACHINE_ID,
    KEY_VALUE_ID, SIGNATURE_MEANING_VALUES, SeverityBand, field_spec, severity_band, tlv_type_spec,
};
use open_ot_carriage::wire::{Record, Slot};
use std::fmt;

/// Current and immediately-prior definitions available to the consumer.
#[derive(Debug, Clone, Copy)]
pub struct DefinitionSet<'a> {
    /// The current definition, if the consumer holds one.
    pub current: Option<&'a DefinitionFile>,
    /// The immediately-prior definition, retained for prior-epoch records.
    pub prior: Option<&'a DefinitionFile>,
}

impl<'a> DefinitionSet<'a> {
    /// Builds a set with only a current definition (no retained prior).
    pub fn current(current: &'a DefinitionFile) -> Self {
        Self {
            current: Some(current),
            prior: None,
        }
    }

    /// Builds a set with both the current and the immediately-prior definition.
    pub fn current_and_prior(current: &'a DefinitionFile, prior: &'a DefinitionFile) -> Self {
        Self {
            current: Some(current),
            prior: Some(prior),
        }
    }
}

/// Resolution outcome for one record.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolution {
    /// The record resolved against a definition into typed, named fields.
    Resolved(ResolvedRecord),
    /// The record could not be resolved; raw slots are preserved for a later pass.
    Placeholder(ResolvedPlaceholder),
}

/// Resolved record with typed, named fields.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRecord {
    /// Human-facing event name from the definition.
    pub event_name: String,
    /// Resolved source metadata, if the source id is known.
    pub source: Option<ResolvedSource>,
    /// Source stream id from the record envelope.
    pub source_id: u32,
    /// Registry event-type id from the record envelope.
    pub event_type_id: u32,
    /// Run id from the record envelope.
    pub run_id: u64,
    /// Epoch id the record resolved under.
    pub epoch_id: u64,
    /// Producer timestamp (nanoseconds) from the record envelope.
    pub source_time: u64,
    /// Source-local sequence number from the record envelope.
    pub seq: u64,
    /// Whether the record resolved against the current or prior epoch.
    pub epoch: ResolvedEpoch,
    /// Resolved core fields, in slot order.
    pub fields: Vec<ResolvedField>,
    /// Preserved private-extension fields, in slot order.
    pub extension_fields: Vec<ResolvedExtensionField>,
}

/// Resolved source metadata from the definition's source table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSource {
    /// Human-facing source name.
    pub name: String,
    /// Logical path segments to the source.
    pub path: Vec<String>,
    /// Equipment hierarchy (for example ISA-95 levels).
    pub hierarchy: Vec<String>,
    /// Whether the source is created dynamically at runtime.
    pub dynamic: bool,
}

/// Which epoch a record resolved under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedEpoch {
    /// The current epoch (record at or after `EpochFirstAbs`).
    Current,
    /// The immediately-prior epoch (record before `EpochFirstAbs`).
    Prior,
}

/// Resolved field value.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedField {
    /// Value-key id of the field.
    pub key: u16,
    /// Human-facing field name from the registry or definition.
    pub name: String,
    /// Name of the field's TLV type.
    pub type_name: String,
    /// Decoded typed value.
    pub value: ResolvedValue,
    /// Engineering-unit symbol, if the field has one.
    pub unit: Option<String>,
    /// Enum label for the value, if the field is an enum and the value is known.
    pub enum_label: Option<String>,
}

/// Private-extension field preserved from a resolved record.
///
/// Extension slots are allowed by the schema validator but are not named by the core registry or
/// the definition file. The raw payload is always retained; typed decoding is best-effort because
/// a private extension may use a type tag or payload width unknown to this prototype.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedExtensionField {
    /// Value-key id of the extension slot.
    pub key: u16,
    /// TLV type tag carried on the wire.
    pub type_tag: u8,
    /// Name of the TLV type, if recognized.
    pub type_name: Option<String>,
    /// Best-effort decoded value; `None` if the type or width is unknown to this prototype.
    pub value: Option<ResolvedValue>,
    /// Raw payload bytes, always retained.
    pub payload: Vec<u8>,
}

/// A decoded IEC 61131-3 typed value.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedValue {
    /// `BOOL`.
    Bool(bool),
    /// `SINT` (signed 8-bit).
    SInt(i8),
    /// `USINT` (unsigned 8-bit).
    USInt(u8),
    /// `UINT` (unsigned 16-bit).
    UInt(u16),
    /// `INT` (signed 16-bit).
    Int(i16),
    /// `UDINT` (unsigned 32-bit).
    UDInt(u32),
    /// `DINT` (signed 32-bit).
    DInt(i32),
    /// `ULINT` (unsigned 64-bit).
    ULInt(u64),
    /// `LINT` (signed 64-bit).
    LInt(i64),
    /// `REAL` (32-bit float).
    Real(f32),
    /// `LREAL` (64-bit float).
    LReal(f64),
    /// `DATE_AND_TIME` as nanoseconds.
    DateTime(u64),
    /// UTF-8 string.
    String(String),
    /// Raw byte string.
    Bytes(Vec<u8>),
}

/// Placeholder result with envelope and raw slots preserved.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPlaceholder {
    /// Source stream id from the record envelope.
    pub source_id: u32,
    /// Registry event-type id from the record envelope.
    pub event_type_id: u32,
    /// Run id from the record envelope.
    pub run_id: u64,
    /// Epoch id the record was read under.
    pub epoch_id: u64,
    /// Source-local sequence number from the record envelope.
    pub seq: u64,
    /// Whether the record fell in the current or prior epoch.
    pub epoch: ResolvedEpoch,
    /// Raw slots preserved for a later correct-definition pass.
    pub slots: Vec<Slot>,
    /// Why the record could not be resolved.
    pub reason: ResolvePlaceholderReason,
}

/// Why a record resolved to a placeholder instead of typed fields.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvePlaceholderReason {
    /// No current definition was available to resolve the record.
    MissingCurrentDefinition,
    /// The record fell in the prior epoch and no prior definition is retained.
    StalePriorEpoch,
    /// The selected definition's carriage hash did not match the record's epoch hash.
    Drift {
        /// Epoch whose hash was compared.
        epoch: ResolvedEpoch,
        /// Carriage hash expected by the consumer's definition.
        expected: [u8; 8],
        /// Carriage hash the record's epoch advertised.
        actual: [u8; 8],
    },
    /// The full content hash mismatched (a stronger drift signal than the 8-byte carriage hash).
    FullHashDrift {
        /// Full content hash expected by the consumer's definition.
        expected: String,
        /// Full content hash the record's epoch advertised.
        actual: String,
    },
    /// The event-type id is not present in the selected definition.
    UnknownEventId(u32),
    /// The record violated the event's slot schema.
    Schema(SchemaViolation),
    /// A string field's payload was not valid UTF-8.
    InvalidUtf8 {
        /// Value key of the offending field.
        key: u16,
    },
    /// A slot carried a TLV type tag unknown to this prototype.
    UnknownTlvType {
        /// Value key of the offending slot.
        key: u16,
        /// The unknown TLV type tag.
        ty: u8,
    },
    /// Computing the definition hash failed.
    Hash(DefinitionErrorString),
}

/// A definition error rendered as a string, carried as a placeholder reason payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionErrorString(
    /// The formatted error message.
    pub String,
);

impl fmt::Display for DefinitionErrorString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Resolves one decoded record against the epoch-selected definition.
pub fn resolve_record(
    record: &Record,
    record_abs: u64,
    snapshot: &ControlBlockSnapshot,
    definitions: &DefinitionSet<'_>,
) -> Resolution {
    let epoch = if record_abs >= snapshot.epoch_first_abs {
        ResolvedEpoch::Current
    } else {
        ResolvedEpoch::Prior
    };
    let selected_hash = match epoch {
        ResolvedEpoch::Current => snapshot.definition_hash,
        ResolvedEpoch::Prior => snapshot.prev_definition_hash,
    };

    let Some(definition) = selected_definition(epoch, definitions) else {
        return placeholder(
            record,
            snapshot,
            epoch,
            match epoch {
                ResolvedEpoch::Current => ResolvePlaceholderReason::MissingCurrentDefinition,
                ResolvedEpoch::Prior => ResolvePlaceholderReason::StalePriorEpoch,
            },
        );
    };

    if let Err(reason) = verify_definition_hash(definition, epoch, selected_hash) {
        return placeholder(record, snapshot, epoch, reason);
    }

    let record_len = match encoded_record_len(record) {
        Ok(len) => len,
        Err(err) => {
            return placeholder(
                record,
                snapshot,
                epoch,
                ResolvePlaceholderReason::Schema(SchemaViolation::Encode(err)),
            );
        }
    };

    match validate_record(record, record_len, definition) {
        SchemaValidation::Valid { .. } => {}
        SchemaValidation::Placeholder(PlaceholderRecord { reason, .. }) => {
            return placeholder(
                record,
                snapshot,
                epoch,
                match reason {
                    SchemaViolation::UnknownEventId(id) => {
                        ResolvePlaceholderReason::UnknownEventId(id)
                    }
                    other => ResolvePlaceholderReason::Schema(other),
                },
            );
        }
    }

    let Some(event) = definition
        .event_types
        .iter()
        .find(|event| event.id == record.event_type_id)
    else {
        return placeholder(
            record,
            snapshot,
            epoch,
            ResolvePlaceholderReason::UnknownEventId(record.event_type_id),
        );
    };

    match resolve_slots(record, event, definition) {
        Ok(resolved) => Resolution::Resolved(ResolvedRecord {
            event_name: event.name.clone(),
            source: resolve_source(record.source_id, definition),
            source_id: record.source_id,
            event_type_id: record.event_type_id,
            run_id: record.run_id,
            epoch_id: snapshot.epoch_id,
            source_time: record.source_time,
            seq: record.seq,
            epoch,
            fields: resolved.fields,
            extension_fields: resolved.extension_fields,
        }),
        Err(reason) => placeholder(record, snapshot, epoch, reason),
    }
}

fn selected_definition<'a>(
    epoch: ResolvedEpoch,
    definitions: &'a DefinitionSet<'a>,
) -> Option<&'a DefinitionFile> {
    match epoch {
        ResolvedEpoch::Current => definitions.current,
        ResolvedEpoch::Prior => definitions.prior,
    }
}

fn verify_definition_hash(
    definition: &DefinitionFile,
    epoch: ResolvedEpoch,
    expected: [u8; 8],
) -> Result<(), ResolvePlaceholderReason> {
    let hash = compute_content_hash(definition).map_err(hash_error)?;
    if hash.carriage_hash != expected {
        return Err(ResolvePlaceholderReason::Drift {
            epoch,
            expected,
            actual: hash.carriage_hash,
        });
    }

    if !definition.header.content_hash.is_empty()
        && definition.header.content_hash != hash.content_hash
    {
        return Err(ResolvePlaceholderReason::FullHashDrift {
            expected: definition.header.content_hash.clone(),
            actual: hash.content_hash,
        });
    }

    Ok(())
}

struct ResolvedSlots {
    fields: Vec<ResolvedField>,
    extension_fields: Vec<ResolvedExtensionField>,
}

fn resolve_slots(
    record: &Record,
    event: &EventTypeDefinition,
    definition: &DefinitionFile,
) -> Result<ResolvedSlots, ResolvePlaceholderReason> {
    let mut context = FieldContext::from_record(record);
    let mut fields = Vec::new();
    let mut extension_fields = Vec::new();

    for slot in &record.slots {
        if slot.key >= 0x8000 {
            extension_fields.push(resolve_extension_field(slot));
            continue;
        }

        let Some(schema_slot) = event.slots.iter().find(|schema| schema.key == slot.key) else {
            continue;
        };
        let Some(field) = field_spec(slot.key) else {
            continue;
        };
        if matches!(field.kind, FieldKind::Reserved) {
            continue;
        }

        let Some(type_spec) = tlv_type_spec(slot.ty) else {
            return Err(ResolvePlaceholderReason::UnknownTlvType {
                key: slot.key,
                ty: slot.ty,
            });
        };
        let mut value = decode_value(slot).map_err(|reason| match reason {
            ValueDecodeError::InvalidUtf8 => {
                ResolvePlaceholderReason::InvalidUtf8 { key: slot.key }
            }
            ValueDecodeError::UnknownTlvType => ResolvePlaceholderReason::UnknownTlvType {
                key: slot.key,
                ty: slot.ty,
            },
        })?;
        let mut name = field.name.to_string();
        let mut type_name = type_spec.name.to_string();
        let mut enum_label = enum_label(slot.key, &value, &mut context, definition);
        let unit = unit_label(slot.key, &value, &context, definition);
        value = semantic_value(
            slot.key,
            value,
            &context,
            definition,
            &mut name,
            &mut type_name,
            &mut enum_label,
        );

        fields.push(ResolvedField {
            key: slot.key,
            name,
            type_name,
            enum_label,
            unit,
            value,
        });

        let _ = schema_slot;
    }

    Ok(ResolvedSlots {
        fields,
        extension_fields,
    })
}

fn resolve_extension_field(slot: &Slot) -> ResolvedExtensionField {
    let type_name = tlv_type_spec(slot.ty).map(|spec| spec.name.to_string());
    let value = decode_extension_value(slot);
    ResolvedExtensionField {
        key: slot.key,
        type_tag: slot.ty,
        type_name,
        value,
        payload: slot.payload.clone(),
    }
}

fn decode_extension_value(slot: &Slot) -> Option<ResolvedValue> {
    let spec = tlv_type_spec(slot.ty)?;
    if let Some(expected) = spec.fixed_width
        && slot.payload.len() != expected
    {
        return None;
    }
    if slot.payload.is_empty()
        && !matches!(
            slot.ty,
            open_ot_carriage::registry::TY_STRING | open_ot_carriage::registry::TY_BYTES
        )
    {
        return None;
    }
    decode_value(slot).ok()
}

#[derive(Debug)]
enum ValueDecodeError {
    InvalidUtf8,
    UnknownTlvType,
}

fn decode_value(slot: &Slot) -> Result<ResolvedValue, ValueDecodeError> {
    match slot.ty {
        0x00 => Ok(ResolvedValue::Bool(slot.payload[0] != 0)),
        0x01 => Ok(ResolvedValue::SInt(slot.payload[0] as i8)),
        0x02 => Ok(ResolvedValue::USInt(slot.payload[0])),
        0x03 => Ok(ResolvedValue::UInt(read_u16(&slot.payload))),
        0x04 => Ok(ResolvedValue::Int(read_u16(&slot.payload) as i16)),
        0x05 => Ok(ResolvedValue::UDInt(read_u32(&slot.payload))),
        0x06 => Ok(ResolvedValue::DInt(read_u32(&slot.payload) as i32)),
        0x07 => Ok(ResolvedValue::ULInt(read_u64(&slot.payload))),
        0x08 => Ok(ResolvedValue::LInt(read_u64(&slot.payload) as i64)),
        0x09 => Ok(ResolvedValue::Real(f32::from_bits(read_u32(&slot.payload)))),
        0x0A => Ok(ResolvedValue::LReal(f64::from_bits(read_u64(
            &slot.payload,
        )))),
        0x0B => Ok(ResolvedValue::DateTime(read_u64(&slot.payload))),
        0x0C => String::from_utf8(slot.payload.clone())
            .map(ResolvedValue::String)
            .map_err(|_| ValueDecodeError::InvalidUtf8),
        0x0D => Ok(ResolvedValue::Bytes(slot.payload.clone())),
        _ => Err(ValueDecodeError::UnknownTlvType),
    }
}

#[derive(Default)]
struct FieldContext {
    state_machine_id: Option<u32>,
    category: Option<u16>,
    value_id: Option<u32>,
    condition_id: Option<u32>,
    message_template_id: Option<u32>,
}

impl FieldContext {
    fn from_record(record: &Record) -> Self {
        let mut context = Self::default();
        for slot in &record.slots {
            match slot.key {
                KEY_STATE_MACHINE_ID if slot.payload.len() == 4 => {
                    context.state_machine_id = Some(read_u32(&slot.payload));
                }
                KEY_CATEGORY if slot.payload.len() == 2 => {
                    context.category = Some(read_u16(&slot.payload));
                }
                KEY_VALUE_ID if slot.payload.len() == 4 => {
                    context.value_id = Some(read_u32(&slot.payload));
                }
                KEY_CONDITION_ID if slot.payload.len() == 4 => {
                    context.condition_id = Some(read_u32(&slot.payload));
                }
                KEY_MESSAGE_TEMPLATE_ID if slot.payload.len() == 4 => {
                    context.message_template_id = Some(read_u32(&slot.payload));
                }
                _ => {}
            }
        }
        context
    }
}

fn semantic_value(
    key: u16,
    value: ResolvedValue,
    context: &FieldContext,
    definition: &DefinitionFile,
    name: &mut String,
    type_name: &mut String,
    enum_label: &mut Option<String>,
) -> ResolvedValue {
    match key {
        KEY_STATE_MACHINE_ID => {
            if let Some(id) = value.as_u32()
                && let Some(machine_name) = state_machine_name(id, definition)
            {
                *name = "stateMachine".to_string();
                *type_name = "StateMachineRef".to_string();
                return ResolvedValue::String(machine_name);
            }
        }
        KEY_VALUE_ID => {
            if let Some(id) = value.as_u32()
                && let Some(value_name) = value_name(id, definition)
            {
                *name = "value".to_string();
                *type_name = "ValueRef".to_string();
                return ResolvedValue::String(value_name);
            }
        }
        KEY_CONDITION_ID => {
            if let Some(id) = value.as_u32()
                && let Some(condition_name) = condition_name(id, definition)
            {
                *name = "condition".to_string();
                *type_name = "ConditionRef".to_string();
                return ResolvedValue::String(condition_name);
            }
        }
        KEY_MESSAGE_TEMPLATE_ID => {
            if let Some(id) = value.as_u32()
                && let Some(template_format) = message_template_format(id, definition)
            {
                *name = "messageTemplate".to_string();
                *type_name = "MessageTemplateRef".to_string();
                return ResolvedValue::String(template_format);
            }
        }
        KEY_CAUSE_OPERAND => {
            if let Some(condition_id) = context.condition_id
                && let Some(id) = value.as_u32()
                && let Some(operand_name) = cause_operand_name(condition_id, id, definition)
            {
                *name = "causeOperand".to_string();
                *type_name = "CauseOperandRef".to_string();
                return ResolvedValue::String(operand_name);
            }
        }
        KEY_PREVIOUS_STATE | KEY_NEW_STATE => {
            if let Some(label) = enum_label.clone() {
                *type_name = "StateRef".to_string();
                return ResolvedValue::String(label);
            }
        }
        _ => {}
    }
    value
}

fn enum_label(
    key: u16,
    value: &ResolvedValue,
    context: &mut FieldContext,
    definition: &DefinitionFile,
) -> Option<String> {
    let int_value = value.as_u16()?;
    match key {
        KEY_CATEGORY => enum_value_label(CATEGORY_VALUES, int_value),
        KEY_AUTH_RESULT => enum_value_label(AUTH_RESULT_VALUES, int_value),
        KEY_SIGNATURE_MEANING => enum_value_label(SIGNATURE_MEANING_VALUES, int_value),
        KEY_PREVIOUS_STATE | KEY_NEW_STATE => {
            state_label(context.state_machine_id?, int_value, definition)
        }
        open_ot_carriage::registry::KEY_SEVERITY => severity_label(int_value),
        _ => None,
    }
}

fn state_label(state_machine_id: u32, value: u16, definition: &DefinitionFile) -> Option<String> {
    let machine = definition
        .state_machines
        .iter()
        .find(|machine| machine.state_machine_id == state_machine_id)?;
    let enum_set = definition
        .enum_sets
        .iter()
        .find(|enum_set| enum_set.name == machine.enum_set)?;
    enum_set_member_label(&enum_set.members, value)
}

fn unit_label(
    key: u16,
    value: &ResolvedValue,
    context: &FieldContext,
    definition: &DefinitionFile,
) -> Option<String> {
    if key == open_ot_carriage::registry::KEY_UNIT {
        let unit_id = value.as_u16()?;
        return definition
            .units
            .iter()
            .find(|unit| unit.unit_id == unit_id)
            .map(|unit| unit.symbol.clone());
    }

    if !matches!(key, KEY_PREVIOUS_VALUE | KEY_NEW_VALUE) {
        return None;
    }
    let value_id = context.value_id?;
    let unit_id = definition
        .values
        .iter()
        .find(|entry| entry.value_id == value_id)?
        .unit?;
    definition
        .units
        .iter()
        .find(|unit| unit.unit_id == unit_id)
        .map(|unit| unit.symbol.clone())
}

fn state_machine_name(id: u32, definition: &DefinitionFile) -> Option<String> {
    definition
        .state_machines
        .iter()
        .find(|machine| machine.state_machine_id == id)
        .map(|machine| machine.name.clone())
}

fn value_name(id: u32, definition: &DefinitionFile) -> Option<String> {
    definition
        .values
        .iter()
        .find(|value| value.value_id == id)
        .map(|value| value.name.clone())
}

fn condition_name(id: u32, definition: &DefinitionFile) -> Option<String> {
    definition
        .conditions
        .iter()
        .find(|condition| condition.condition_id == id)
        .map(|condition| condition.name.clone())
}

fn message_template_format(id: u32, definition: &DefinitionFile) -> Option<String> {
    definition
        .message_templates
        .iter()
        .find(|template| template.message_template_id == id)
        .map(|template| template.format.clone())
}

fn cause_operand_name(
    condition_id: u32,
    operand_id: u32,
    definition: &DefinitionFile,
) -> Option<String> {
    definition
        .conditions
        .iter()
        .find(|condition| condition.condition_id == condition_id)?
        .cause_operands
        .iter()
        .find(|operand| operand.operand_id == operand_id)
        .map(|operand| operand.name.clone())
}

fn resolve_source(source_id: u32, definition: &DefinitionFile) -> Option<ResolvedSource> {
    definition
        .sources
        .iter()
        .find(|source| source.source_id == source_id)
        .map(source_to_resolved)
}

fn source_to_resolved(source: &SourceDefinition) -> ResolvedSource {
    ResolvedSource {
        name: source.name.clone(),
        path: source.path.clone(),
        hierarchy: source.hierarchy.clone(),
        dynamic: source.dynamic,
    }
}

fn enum_value_label(
    values: &[open_ot_carriage::registry::EnumValue],
    value: u16,
) -> Option<String> {
    values
        .iter()
        .find(|entry| entry.value == value)
        .map(|entry| entry.label.to_string())
}

fn enum_set_member_label(values: &[EnumMember], value: u16) -> Option<String> {
    values
        .iter()
        .find(|entry| entry.value == value)
        .map(|entry| entry.label.clone())
}

fn severity_label(value: u16) -> Option<String> {
    match severity_band(value)? {
        SeverityBand::Low => Some("Low".to_string()),
        SeverityBand::Medium => Some("Medium".to_string()),
        SeverityBand::High => Some("High".to_string()),
    }
}

impl ResolvedValue {
    fn as_u32(&self) -> Option<u32> {
        match self {
            ResolvedValue::UDInt(value) => Some(*value),
            _ => None,
        }
    }

    fn as_u16(&self) -> Option<u16> {
        match self {
            ResolvedValue::UInt(value) => Some(*value),
            _ => None,
        }
    }
}

fn placeholder(
    record: &Record,
    snapshot: &ControlBlockSnapshot,
    epoch: ResolvedEpoch,
    reason: ResolvePlaceholderReason,
) -> Resolution {
    Resolution::Placeholder(ResolvedPlaceholder {
        source_id: record.source_id,
        event_type_id: record.event_type_id,
        run_id: record.run_id,
        epoch_id: snapshot.epoch_id,
        seq: record.seq,
        epoch,
        slots: record.slots.clone(),
        reason,
    })
}

fn hash_error(err: DefinitionError) -> ResolvePlaceholderReason {
    ResolvePlaceholderReason::Hash(DefinitionErrorString(err.to_string()))
}

fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes.try_into().expect("u16 payload"))
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("u32 payload"))
}

fn read_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().expect("u64 payload"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::compute_content_hash;
    use crate::model::{CauseOperandDefinition, ConditionDefinition, sample_definition};
    use open_ot_carriage::registry::{
        EVENT_CONDITION_ACTIVE, KEY_ARG, KEY_AUTH_RESULT, KEY_CATEGORY, KEY_CAUSE_OPERAND,
        KEY_CONDITION_CLASS, KEY_CONDITION_ID, KEY_CORRELATION_ID, KEY_MESSAGE_TEMPLATE_ID,
        KEY_NEW_STATE, KEY_PREVIOUS_STATE, KEY_SEVERITY, KEY_STATE_MACHINE_ID, TY_STRING, TY_UDINT,
        TY_UINT,
    };
    use open_ot_carriage::wire::{Record, Slot, decode};

    #[test]
    fn prior_epoch_uses_prev_definition_hash_not_current_hash() {
        let prior = sample_definition();
        let mut current = sample_definition();
        current.header.semantic_version = "1.0.1".to_string();
        current.header.profiles.push("Audit".to_string());
        let prior_hash = compute_content_hash(&prior).unwrap().carriage_hash;
        let current_hash = compute_content_hash(&current).unwrap().carriage_hash;
        assert_ne!(prior_hash, current_hash);

        let snapshot = snapshot(current_hash, prior_hash, 100);
        let record = conformant_state_transition_record();

        let resolved = resolve_record(
            &record,
            99,
            &snapshot,
            &DefinitionSet::current_and_prior(&current, &prior),
        );

        let Resolution::Resolved(record) = resolved else {
            panic!("prior record should resolve against prior hash");
        };
        assert_eq!(record.epoch, ResolvedEpoch::Prior);
        assert_eq!(record.event_name, "StateTransition");
    }

    #[test]
    fn current_epoch_uses_current_definition_hash() {
        let current = sample_definition();
        let hash = compute_content_hash(&current).unwrap().carriage_hash;
        let snapshot = snapshot(hash, [0xAA; 8], 100);

        let resolved = resolve_record(
            &conformant_state_transition_record(),
            100,
            &snapshot,
            &DefinitionSet::current(&current),
        );

        let Resolution::Resolved(record) = resolved else {
            panic!("current record should resolve");
        };
        assert_eq!(record.epoch, ResolvedEpoch::Current);
    }

    #[test]
    fn conformant_state_transition_resolves_named_typed_enum_labeled_fields() {
        let definition = sample_definition();
        let hash = compute_content_hash(&definition).unwrap().carriage_hash;
        let snapshot = snapshot(hash, [0; 8], 0);
        let resolved = resolve_record(
            &conformant_state_transition_record(),
            0,
            &snapshot,
            &DefinitionSet::current(&definition),
        );

        let Resolution::Resolved(record) = resolved else {
            panic!("expected resolved record");
        };
        assert_eq!(record.event_name, "StateTransition");
        assert_eq!(record.source.as_ref().unwrap().name, "UnitA.Phase1");
        assert_eq!(
            field(&record, KEY_STATE_MACHINE_ID),
            &ResolvedField {
                key: KEY_STATE_MACHINE_ID,
                name: "stateMachine".to_string(),
                type_name: "StateMachineRef".to_string(),
                value: ResolvedValue::String("CoreProcedure".to_string()),
                unit: None,
                enum_label: None,
            }
        );
        assert_eq!(
            field(&record, KEY_CATEGORY).enum_label.as_deref(),
            Some("Procedural")
        );
        assert_eq!(
            field(&record, KEY_PREVIOUS_STATE).value,
            ResolvedValue::String("Pausing".to_string())
        );
        assert_eq!(
            field(&record, KEY_PREVIOUS_STATE).enum_label.as_deref(),
            Some("Pausing")
        );
        assert_eq!(
            field(&record, KEY_NEW_STATE).value,
            ResolvedValue::String("Paused".to_string())
        );
        assert_eq!(
            field(&record, KEY_NEW_STATE).enum_label.as_deref(),
            Some("Paused")
        );
    }

    #[test]
    fn conformant_message_resolves_template_and_arg_fields() {
        let definition = sample_definition();
        let hash = compute_content_hash(&definition).unwrap().carriage_hash;
        let snapshot = snapshot(hash, [0; 8], 0);
        let resolved = resolve_record(
            &conformant_message_record(),
            0,
            &snapshot,
            &DefinitionSet::current(&definition),
        );

        let Resolution::Resolved(record) = resolved else {
            panic!("expected resolved message");
        };
        assert_eq!(record.event_name, "Message");
        assert_eq!(
            field(&record, KEY_MESSAGE_TEMPLATE_ID).value,
            ResolvedValue::String("Status: {1}".to_string())
        );
        assert_eq!(
            field(&record, KEY_ARG).value,
            ResolvedValue::String("phase ready".to_string())
        );
    }

    #[test]
    fn conformant_operator_login_resolves_auth_result_label() {
        let definition = sample_definition();
        let hash = compute_content_hash(&definition).unwrap().carriage_hash;
        let snapshot = snapshot(hash, [0; 8], 0);
        let resolved = resolve_record(
            &conformant_operator_login_record(),
            0,
            &snapshot,
            &DefinitionSet::current(&definition),
        );

        let Resolution::Resolved(record) = resolved else {
            panic!("expected resolved operator login");
        };
        assert_eq!(record.event_name, "OperatorLogin");
        assert_eq!(
            field(&record, KEY_AUTH_RESULT).enum_label.as_deref(),
            Some("Granted")
        );
    }

    #[test]
    fn condition_cause_operand_resolves_against_condition_definition() {
        let mut definition = sample_definition();
        definition.conditions.push(ConditionDefinition {
            condition_id: 9001,
            name: "HighLevel".to_string(),
            condition_class: 0,
            default_severity: 900,
            cause_operands: vec![CauseOperandDefinition {
                operand_id: 1,
                name: "Level".to_string(),
            }],
        });
        let hash = compute_content_hash(&definition).unwrap().carriage_hash;
        let snapshot = snapshot(hash, [0; 8], 0);
        let mut record = Record::new(1_000_000_000, 1, 0, 1, EVENT_CONDITION_ACTIVE);
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

        let resolved = resolve_record(&record, 0, &snapshot, &DefinitionSet::current(&definition));

        let Resolution::Resolved(record) = resolved else {
            panic!("expected resolved condition");
        };
        assert_eq!(
            field(&record, KEY_CAUSE_OPERAND).value,
            ResolvedValue::String("Level".to_string())
        );
        assert_eq!(
            field(&record, KEY_CAUSE_OPERAND).type_name,
            "CauseOperandRef"
        );
    }

    #[test]
    fn prior_epoch_without_prior_definition_placeholders_stale_prior_epoch() {
        let current = sample_definition();
        let hash = compute_content_hash(&current).unwrap().carriage_hash;
        let snapshot = snapshot(hash, [0xBB; 8], 100);
        let resolved = resolve_record(
            &conformant_state_transition_record(),
            99,
            &snapshot,
            &DefinitionSet::current(&current),
        );

        assert_placeholder_reason(resolved, ResolvePlaceholderReason::StalePriorEpoch);
    }

    #[test]
    fn hash_mismatch_placeholders_drift_on_selected_epoch() {
        let current = sample_definition();
        let snapshot = snapshot([0x11; 8], [0x22; 8], 100);
        let resolved = resolve_record(
            &conformant_state_transition_record(),
            100,
            &snapshot,
            &DefinitionSet::current(&current),
        );

        let actual = compute_content_hash(&current).unwrap().carriage_hash;
        assert_placeholder_reason(
            resolved,
            ResolvePlaceholderReason::Drift {
                epoch: ResolvedEpoch::Current,
                expected: [0x11; 8],
                actual,
            },
        );
    }

    #[test]
    fn full_hash_mismatch_placeholders_drift() {
        let mut current = sample_definition();
        current.header.content_hash = "00".repeat(32);
        let hash = compute_content_hash(&current).unwrap().carriage_hash;
        let snapshot = snapshot(hash, [0; 8], 0);
        let actual = compute_content_hash(&current).unwrap().content_hash;
        let resolved = resolve_record(
            &conformant_state_transition_record(),
            0,
            &snapshot,
            &DefinitionSet::current(&current),
        );

        assert_placeholder_reason(
            resolved,
            ResolvePlaceholderReason::FullHashDrift {
                expected: "00".repeat(32),
                actual,
            },
        );
    }

    #[test]
    fn unknown_event_id_placeholders_after_hash_verification() {
        let current = sample_definition();
        let hash = compute_content_hash(&current).unwrap().carriage_hash;
        let snapshot = snapshot(hash, [0; 8], 0);
        let mut record = conformant_state_transition_record();
        record.event_type_id = 0x7777;

        let resolved = resolve_record(&record, 0, &snapshot, &DefinitionSet::current(&current));

        assert_placeholder_reason(resolved, ResolvePlaceholderReason::UnknownEventId(0x7777));
    }

    #[test]
    fn schema_violation_is_preserved_as_placeholder_reason() {
        let current = sample_definition();
        let hash = compute_content_hash(&current).unwrap().carriage_hash;
        let snapshot = snapshot(hash, [0; 8], 0);
        let resolved = resolve_record(
            &codec_state_transition_negative_record(),
            0,
            &snapshot,
            &DefinitionSet::current(&current),
        );

        assert!(matches!(
            placeholder_reason(resolved),
            ResolvePlaceholderReason::Schema(SchemaViolation::TypeMismatch {
                key: KEY_STATE_MACHINE_ID,
                ..
            })
        ));
    }

    #[test]
    fn invalid_string_payload_placeholders() {
        let current = sample_definition();
        let hash = compute_content_hash(&current).unwrap().carriage_hash;
        let snapshot = snapshot(hash, [0; 8], 0);
        let mut record = conformant_message_record();
        let arg = record
            .slots
            .iter_mut()
            .find(|slot| slot.key == KEY_ARG)
            .unwrap();
        *arg = Slot::new(KEY_ARG, TY_STRING, [0xFF]);

        let resolved = resolve_record(&record, 0, &snapshot, &DefinitionSet::current(&current));

        assert_placeholder_reason(
            resolved,
            ResolvePlaceholderReason::InvalidUtf8 { key: KEY_ARG },
        );
    }

    fn snapshot(
        definition_hash: [u8; 8],
        prev_definition_hash: [u8; 8],
        epoch_first_abs: u64,
    ) -> ControlBlockSnapshot {
        ControlBlockSnapshot {
            version: 2,
            caps: 0,
            buffer_id: 1,
            buffer_bytes: 4096,
            head_abs: 4096,
            oldest_abs: 0,
            lost_count: 0,
            run_id: 1,
            epoch_id: 7,
            epoch_first_abs,
            definition_hash,
            prev_definition_hash,
        }
    }

    fn field(record: &ResolvedRecord, key: u16) -> &ResolvedField {
        record.fields.iter().find(|field| field.key == key).unwrap()
    }

    fn assert_placeholder_reason(actual: Resolution, expected: ResolvePlaceholderReason) {
        assert_eq!(placeholder_reason(actual), expected);
    }

    fn placeholder_reason(actual: Resolution) -> ResolvePlaceholderReason {
        let Resolution::Placeholder(placeholder) = actual else {
            panic!("expected placeholder");
        };
        placeholder.reason
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

    fn conformant_operator_login_record() -> Record {
        let bytes = hex_bytes(include_str!(
            "../../carriage/vectors/conformant_operator_login.hex"
        ));
        decode(&bytes).unwrap().record
    }

    fn codec_state_transition_negative_record() -> Record {
        let bytes = hex_bytes(include_str!("../../carriage/vectors/state_transition.hex"));
        decode(&bytes).unwrap().record
    }

    fn hex_bytes(input: &str) -> Vec<u8> {
        input
            .split_whitespace()
            .map(|byte| u8::from_str_radix(byte, 16).unwrap())
            .collect()
    }
}
