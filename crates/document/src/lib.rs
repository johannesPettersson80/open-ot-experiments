//! Document-format proposal for the OpenOT experiment.
//!
//! The crate converts definition-layer [`Resolution`] values and carriage [`LossEvent`] ranges into
//! deterministic JSON documents. It does not re-resolve records; caller-supplied context provides
//! provenance that is not present in the resolver output, such as buffer id, receive time, flags,
//! and the selected definition hash.

use open_ot_carriage::loss::LossEvent;
use open_ot_carriage::wire::{FLAG_PARTIAL_PAYLOAD, FLAG_SYNTHETIC, FLAG_TIME_UNSYNCED, WireError};
use open_ot_definition::schema::{DefinitionSchemaViolation, SchemaViolation};
use open_ot_definition::{
    DefinitionErrorString, Resolution, ResolvePlaceholderReason, ResolvedEpoch,
    ResolvedExtensionField, ResolvedField, ResolvedPlaceholder, ResolvedRecord, ResolvedSource,
    ResolvedValue,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Provenance supplied by the caller for one resolved or placeholder record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordDocumentContext {
    /// Buffer id the record was read from.
    pub buffer_id: u32,
    /// Consumer-assigned receive timestamp (nanoseconds).
    pub receive_time_ns: u64,
    /// Carriage hash of the definition selected for resolution.
    pub definition_hash: [u8; 8],
    /// Semantic version of the selected definition, if known.
    pub semantic_version: Option<String>,
    /// Raw record flags (time-unsynced, synthetic, partial-payload).
    pub flags: u16,
    /// Producer timestamp (nanoseconds), supplied for placeholder records.
    pub source_time_ns: Option<u64>,
    /// Resolved source metadata, if the caller resolved it.
    pub source: Option<DocumentSource>,
}

impl RecordDocumentContext {
    /// Creates a context with the required fields; optional fields default to absent.
    pub fn new(buffer_id: u32, receive_time_ns: u64, definition_hash: [u8; 8], flags: u16) -> Self {
        Self {
            buffer_id,
            receive_time_ns,
            definition_hash,
            semantic_version: None,
            flags,
            source_time_ns: None,
            source: None,
        }
    }

    /// Sets the selected definition's semantic version.
    pub fn with_semantic_version(mut self, semantic_version: impl Into<String>) -> Self {
        self.semantic_version = Some(semantic_version.into());
        self
    }

    /// Sets the producer source timestamp (nanoseconds).
    pub fn with_source_time(mut self, source_time_ns: u64) -> Self {
        self.source_time_ns = Some(source_time_ns);
        self
    }

    /// Sets the resolved source metadata.
    pub fn with_source(mut self, source: DocumentSource) -> Self {
        self.source = Some(source);
        self
    }
}

/// Provenance supplied by the caller for one loss range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossDocumentContext {
    /// Consumer-assigned receive timestamp (nanoseconds).
    pub receive_time_ns: u64,
    /// Epoch id the loss range is attributed to.
    pub epoch_id: u64,
    /// Whether the range falls in the current or prior epoch.
    pub epoch: EpochRelation,
    /// Carriage hash of the definition in force for the range.
    pub definition_hash: [u8; 8],
    /// Semantic version of that definition, if known.
    pub semantic_version: Option<String>,
    /// Resolved source metadata, if the caller resolved it.
    pub source: Option<DocumentSource>,
}

impl LossDocumentContext {
    /// Creates a loss context with the required fields; optional fields default to absent.
    pub fn new(
        receive_time_ns: u64,
        epoch_id: u64,
        epoch: EpochRelation,
        definition_hash: [u8; 8],
    ) -> Self {
        Self {
            receive_time_ns,
            epoch_id,
            epoch,
            definition_hash,
            semantic_version: None,
            source: None,
        }
    }

    /// Sets the definition's semantic version.
    pub fn with_semantic_version(mut self, semantic_version: impl Into<String>) -> Self {
        self.semantic_version = Some(semantic_version.into());
        self
    }

    /// Sets the resolved source metadata.
    pub fn with_source(mut self, source: DocumentSource) -> Self {
        self.source = Some(source);
        self
    }
}

/// Top-level consumer document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Document {
    /// A resolved event with typed, named fields.
    Event(EventDocument),
    /// A lost sequence range.
    Loss(LossDocument),
    /// A record preserved unresolved, with its raw slots.
    Placeholder(PlaceholderDocument),
}

/// Resolved event document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDocument {
    /// Document kind discriminator (`event`).
    pub kind: DocumentKind,
    /// Record provenance (source, run, epoch, timestamps, flags).
    pub provenance: Provenance,
    /// Human-facing event name.
    pub event_name: String,
    /// Registry event-type id.
    pub event_type_id: u32,
    /// Source-local sequence number.
    pub seq: u64,
    /// Resolved core fields, in slot order.
    pub fields: Vec<DocumentField>,
    /// Preserved private-extension fields; omitted from JSON when empty.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub extension_fields: Vec<ExtensionField>,
}

/// Loss-range document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LossDocument {
    /// Document kind discriminator (`loss`).
    pub kind: DocumentKind,
    /// Range provenance (source, run, epoch, receive time).
    pub provenance: Provenance,
    /// First lost source-local sequence number.
    pub first_seq: u64,
    /// Last lost source-local sequence number.
    pub last_seq: u64,
    /// Number of lost records in the range.
    pub count: u64,
    /// Whether the range is authoritative or inferred from a gap.
    pub basis: LossBasis,
}

/// Placeholder document for a record that must not be semantically resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceholderDocument {
    /// Document kind discriminator (`placeholder`).
    pub kind: DocumentKind,
    /// Record provenance (source, run, epoch, timestamps, flags).
    pub provenance: Provenance,
    /// Registry event-type id from the record envelope.
    pub event_type_id: u32,
    /// Source-local sequence number.
    pub seq: u64,
    /// Why the record was not resolved.
    pub reason: PlaceholderReasonDocument,
    /// Raw slots preserved for a later correct-definition pass.
    pub raw_slots: Vec<RawSlot>,
}

/// Discriminator naming which kind of document this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentKind {
    /// A resolved event.
    Event,
    /// A lost sequence range.
    Loss,
    /// An unresolved record preserved with raw slots.
    Placeholder,
}

/// Common provenance carried by every record-backed document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    /// Buffer id the record was read from.
    pub buffer_id: u32,
    /// Source metadata (id plus resolved name/path/hierarchy when known).
    pub source: DocumentSource,
    /// Run id from the record envelope.
    pub run_id: u64,
    /// Epoch context (id, relation, definition hash).
    pub epoch: DocumentEpoch,
    /// Producer timestamp (nanoseconds); omitted from JSON for loss ranges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_time_ns: Option<u64>,
    /// Consumer-assigned receive timestamp (nanoseconds).
    pub receive_time_ns: u64,
    /// Decoded record flags.
    pub flags: DocumentFlags,
}

/// Source metadata: always an id, plus resolved descriptors when the source is known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSource {
    /// Source stream id.
    pub id: u32,
    /// Human-facing source name, when resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Logical path segments, when resolved; omitted from JSON when empty.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub path: Vec<String>,
    /// Equipment hierarchy, when resolved; omitted from JSON when empty.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub hierarchy: Vec<String>,
    /// Whether the source is dynamic, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic: Option<bool>,
}

impl DocumentSource {
    /// Builds an id-only source for when no definition entry resolves it.
    pub fn unresolved(id: u32) -> Self {
        Self {
            id,
            name: None,
            path: Vec::new(),
            hierarchy: Vec::new(),
            dynamic: None,
        }
    }
}

/// Epoch context attached to a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentEpoch {
    /// Epoch id.
    pub id: u64,
    /// Whether the document resolved against the current or prior epoch.
    pub relation: EpochRelation,
    /// Full definition content hash (lowercase hex) in force for the epoch.
    pub definition_hash: String,
    /// Semantic version of that definition, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_version: Option<String>,
}

/// Whether a document resolved against the current or the prior epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EpochRelation {
    /// The current epoch.
    Current,
    /// The immediately-prior epoch.
    Prior,
}

/// Decoded record flags surfaced in a document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentFlags {
    /// The producer clock was not synchronized when the record was written.
    pub time_unsynced: bool,
    /// The record is synthetic (inserted by the consumer, not produced).
    pub synthetic_record: bool,
    /// The record's payload was truncated.
    pub partial_payload: bool,
}

impl DocumentFlags {
    /// Decodes the raw `u16` record flag bits into named flags.
    pub fn from_record_flags(flags: u16) -> Self {
        Self {
            time_unsynced: flags & FLAG_TIME_UNSYNCED != 0,
            synthetic_record: flags & FLAG_SYNTHETIC != 0,
            partial_payload: flags & FLAG_PARTIAL_PAYLOAD != 0,
        }
    }
}

/// One resolved field in an event document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentField {
    /// Value-key id of the field.
    pub key: u16,
    /// Human-facing field name.
    pub name: String,
    /// Name of the field's TLV type.
    #[serde(rename = "type")]
    pub type_name: String,
    /// Decoded value as JSON.
    pub value: Value,
    /// Engineering-unit symbol, when the field has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Enum label for the value; `null` when the value is outside the known enum set.
    pub enum_label: Option<String>,
}

/// One preserved private-extension field in an event document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionField {
    /// Value-key id of the extension slot.
    pub key: u16,
    /// TLV type tag carried on the wire.
    #[serde(rename = "type")]
    pub type_tag: u8,
    /// Name of the TLV type, when recognized.
    pub type_name: Option<String>,
    /// Field name; `null` because extension keys are not named by the core registry.
    pub name: Option<String>,
    /// Best-effort decoded value; omitted from JSON when decoding is unsafe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// Raw payload as lowercase hex, always present.
    pub payload_hex: String,
}

/// How a loss range was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LossBasis {
    /// From a producer-authoritative `RecordsDropped` or source high-water signal.
    Authoritative,
    /// From a sequence gap only (no authoritative signal).
    Inferred,
}

/// A placeholder reason: a stable kind plus optional structured detail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceholderReasonDocument {
    /// The reason kind.
    pub kind: PlaceholderReasonKind,
    /// Structured detail keyed by the kind (for example expected/actual hashes); omitted when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<Value>,
}

/// Stable, language-neutral kind for a placeholder reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaceholderReasonKind {
    /// No current definition was available.
    MissingCurrentDefinition,
    /// The record fell in the prior epoch with no prior definition retained.
    StalePriorEpoch,
    /// The 8-byte carriage hash did not match.
    Drift,
    /// The full content hash did not match.
    FullHashDrift,
    /// The event-type id is not in the definition.
    UnknownEventId,
    /// The record violated the event's slot schema.
    SchemaViolation,
    /// A payload could not be decoded (invalid UTF-8 or unknown TLV type).
    InvalidPayload,
    /// Computing the definition hash failed.
    HashError,
}

/// A raw, unresolved slot preserved on a placeholder document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSlot {
    /// Value-key id of the slot.
    pub key: u16,
    /// TLV type tag carried on the wire.
    #[serde(rename = "type")]
    pub type_tag: u8,
    /// Raw payload as lowercase hex.
    pub payload_hex: String,
}

/// Converts one definition-layer resolution into a document.
pub fn document_from_resolution(
    resolution: &Resolution,
    context: &RecordDocumentContext,
) -> Document {
    match resolution {
        Resolution::Resolved(record) => {
            Document::Event(event_document_from_record(record, context))
        }
        Resolution::Placeholder(placeholder) => {
            Document::Placeholder(placeholder_document_from_record(placeholder, context))
        }
    }
}

/// Converts one reconciled loss interval into a document.
pub fn document_from_loss(event: &LossEvent, context: &LossDocumentContext) -> Document {
    Document::Loss(LossDocument {
        kind: DocumentKind::Loss,
        provenance: Provenance {
            buffer_id: event.buffer_id,
            source: context
                .source
                .clone()
                .unwrap_or_else(|| DocumentSource::unresolved(event.source_id)),
            run_id: event.run_id,
            epoch: DocumentEpoch {
                id: context.epoch_id,
                relation: context.epoch,
                definition_hash: hex_lower(&context.definition_hash),
                semantic_version: context.semantic_version.clone(),
            },
            source_time_ns: None,
            receive_time_ns: context.receive_time_ns,
            flags: DocumentFlags::default(),
        },
        first_seq: event.first_seq,
        last_seq: event.last_seq,
        count: event.count,
        basis: if event.synthetic {
            LossBasis::Inferred
        } else {
            LossBasis::Authoritative
        },
    })
}

/// Serializes a document with stable struct field ordering.
pub fn to_json(document: &Document) -> serde_json::Result<String> {
    serde_json::to_string(document)
}

fn event_document_from_record(
    record: &ResolvedRecord,
    context: &RecordDocumentContext,
) -> EventDocument {
    EventDocument {
        kind: DocumentKind::Event,
        provenance: provenance_for_record(
            context,
            record.source_id,
            record.run_id,
            record.epoch_id,
            record.epoch,
            Some(record.source_time),
            record.source.as_ref(),
        ),
        event_name: record.event_name.clone(),
        event_type_id: record.event_type_id,
        seq: record.seq,
        fields: record.fields.iter().map(field_document).collect(),
        extension_fields: record
            .extension_fields
            .iter()
            .map(extension_field_document)
            .collect(),
    }
}

fn placeholder_document_from_record(
    placeholder: &ResolvedPlaceholder,
    context: &RecordDocumentContext,
) -> PlaceholderDocument {
    PlaceholderDocument {
        kind: DocumentKind::Placeholder,
        provenance: provenance_for_record(
            context,
            placeholder.source_id,
            placeholder.run_id,
            placeholder.epoch_id,
            placeholder.epoch,
            context.source_time_ns,
            None,
        ),
        event_type_id: placeholder.event_type_id,
        seq: placeholder.seq,
        reason: reason_document(&placeholder.reason),
        raw_slots: placeholder.slots.iter().map(raw_slot_document).collect(),
    }
}

fn provenance_for_record(
    context: &RecordDocumentContext,
    source_id: u32,
    run_id: u64,
    epoch_id: u64,
    epoch: ResolvedEpoch,
    source_time_ns: Option<u64>,
    resolved_source: Option<&ResolvedSource>,
) -> Provenance {
    Provenance {
        buffer_id: context.buffer_id,
        source: resolved_source
            .map(|source| source_document(source_id, source))
            .or_else(|| context.source.clone())
            .unwrap_or_else(|| DocumentSource::unresolved(source_id)),
        run_id,
        epoch: DocumentEpoch {
            id: epoch_id,
            relation: epoch_relation(epoch),
            definition_hash: hex_lower(&context.definition_hash),
            semantic_version: context.semantic_version.clone(),
        },
        source_time_ns,
        receive_time_ns: context.receive_time_ns,
        flags: DocumentFlags::from_record_flags(context.flags),
    }
}

fn source_document(id: u32, source: &ResolvedSource) -> DocumentSource {
    DocumentSource {
        id,
        name: Some(source.name.clone()),
        path: source.path.clone(),
        hierarchy: source.hierarchy.clone(),
        dynamic: Some(source.dynamic),
    }
}

fn epoch_relation(epoch: ResolvedEpoch) -> EpochRelation {
    match epoch {
        ResolvedEpoch::Current => EpochRelation::Current,
        ResolvedEpoch::Prior => EpochRelation::Prior,
    }
}

fn field_document(field: &ResolvedField) -> DocumentField {
    DocumentField {
        key: field.key,
        name: field.name.clone(),
        type_name: field.type_name.clone(),
        value: value_document(&field.value),
        unit: field.unit.clone(),
        enum_label: field.enum_label.clone(),
    }
}

fn extension_field_document(field: &ResolvedExtensionField) -> ExtensionField {
    ExtensionField {
        key: field.key,
        type_tag: field.type_tag,
        type_name: field.type_name.clone(),
        name: None,
        value: field.value.as_ref().map(value_document),
        payload_hex: hex_lower(&field.payload),
    }
}

fn value_document(value: &ResolvedValue) -> Value {
    match value {
        ResolvedValue::Bool(value) => json!(value),
        ResolvedValue::SInt(value) => json!(value),
        ResolvedValue::USInt(value) => json!(value),
        ResolvedValue::UInt(value) => json!(value),
        ResolvedValue::Int(value) => json!(value),
        ResolvedValue::UDInt(value) => json!(value),
        ResolvedValue::DInt(value) => json!(value),
        ResolvedValue::ULInt(value) => json!(value),
        ResolvedValue::LInt(value) => json!(value),
        ResolvedValue::Real(value) => json!(value),
        ResolvedValue::LReal(value) => json!(value),
        ResolvedValue::DateTime(value) => json!(value),
        ResolvedValue::String(value) => json!(value),
        ResolvedValue::Bytes(value) => json!({ "payloadHex": hex_lower(value) }),
    }
}

fn reason_document(reason: &ResolvePlaceholderReason) -> PlaceholderReasonDocument {
    match reason {
        ResolvePlaceholderReason::MissingCurrentDefinition => PlaceholderReasonDocument {
            kind: PlaceholderReasonKind::MissingCurrentDefinition,
            detail: None,
        },
        ResolvePlaceholderReason::StalePriorEpoch => PlaceholderReasonDocument {
            kind: PlaceholderReasonKind::StalePriorEpoch,
            detail: None,
        },
        ResolvePlaceholderReason::Drift {
            epoch,
            expected,
            actual,
        } => PlaceholderReasonDocument {
            kind: PlaceholderReasonKind::Drift,
            detail: Some(json!({
                "epoch": epoch_relation(*epoch),
                "expected": hex_lower(expected),
                "actual": hex_lower(actual)
            })),
        },
        ResolvePlaceholderReason::FullHashDrift { expected, actual } => PlaceholderReasonDocument {
            kind: PlaceholderReasonKind::FullHashDrift,
            detail: Some(json!({
                "expected": expected,
                "actual": actual
            })),
        },
        ResolvePlaceholderReason::UnknownEventId(event_type_id) => PlaceholderReasonDocument {
            kind: PlaceholderReasonKind::UnknownEventId,
            detail: Some(json!({ "eventTypeId": event_type_id })),
        },
        ResolvePlaceholderReason::Schema(reason) => PlaceholderReasonDocument {
            kind: PlaceholderReasonKind::SchemaViolation,
            detail: Some(schema_violation_document(reason)),
        },
        ResolvePlaceholderReason::InvalidUtf8 { key } => PlaceholderReasonDocument {
            kind: PlaceholderReasonKind::InvalidPayload,
            detail: Some(json!({ "key": key, "reason": "invalidUtf8" })),
        },
        ResolvePlaceholderReason::UnknownTlvType { key, ty } => PlaceholderReasonDocument {
            kind: PlaceholderReasonKind::InvalidPayload,
            detail: Some(json!({ "key": key, "type": ty, "reason": "unknownTlvType" })),
        },
        ResolvePlaceholderReason::Hash(DefinitionErrorString(error)) => PlaceholderReasonDocument {
            kind: PlaceholderReasonKind::HashError,
            detail: Some(json!({ "message": error })),
        },
    }
}

fn schema_violation_document(reason: &SchemaViolation) -> Value {
    match reason {
        SchemaViolation::Definition(reason) => {
            json!({ "definition": definition_schema_violation_document(reason) })
        }
        SchemaViolation::UnknownEventId(event_type_id) => {
            json!({ "unknownEventId": { "eventTypeId": event_type_id } })
        }
        SchemaViolation::RecordTooLarge { actual, max } => {
            json!({ "recordTooLarge": { "actual": actual, "max": max } })
        }
        SchemaViolation::TooManySlots { actual, max } => {
            json!({ "tooManySlots": { "actual": actual, "max": max } })
        }
        SchemaViolation::UnexpectedCoreKey { key } => {
            json!({ "unexpectedCoreKey": { "key": key } })
        }
        SchemaViolation::VendorExtensionNotTrailing { key } => {
            json!({ "vendorExtensionNotTrailing": { "key": key } })
        }
        SchemaViolation::VendorExtensionOutOfOrder { previous, key } => {
            json!({ "vendorExtensionOutOfOrder": { "previous": previous, "key": key } })
        }
        SchemaViolation::TypeMismatch {
            key,
            actual,
            expected,
        } => json!({ "typeMismatch": { "key": key, "actual": actual, "expected": expected } }),
        SchemaViolation::UnknownTlvType { key, ty } => {
            json!({ "unknownTlvType": { "key": key, "type": ty } })
        }
        SchemaViolation::ReservedCoreKey { key } => {
            json!({ "reservedCoreKey": { "key": key } })
        }
        SchemaViolation::FixedWidthMismatch {
            key,
            ty,
            actual,
            expected,
        } => json!({
            "fixedWidthMismatch": {
                "key": key,
                "type": ty,
                "actual": actual,
                "expected": expected
            }
        }),
        SchemaViolation::MissingRequired { key, min, actual } => {
            json!({ "missingRequired": { "key": key, "min": min, "actual": actual } })
        }
        SchemaViolation::TooManyOccurrences { key, max, actual } => {
            json!({ "tooManyOccurrences": { "key": key, "max": max, "actual": actual } })
        }
        SchemaViolation::RepeatedSlotNotContiguous { key } => {
            json!({ "repeatedSlotNotContiguous": { "key": key } })
        }
        SchemaViolation::OrderClassViolation {
            previous,
            current,
            key,
        } => json!({
            "orderClassViolation": {
                "previous": previous,
                "current": current,
                "key": key
            }
        }),
        SchemaViolation::Encode(reason) => json!({ "encode": wire_error_document(reason) }),
    }
}

fn definition_schema_violation_document(reason: &DefinitionSchemaViolation) -> Value {
    match reason {
        DefinitionSchemaViolation::DuplicateEventId(event_type_id) => {
            json!({ "duplicateEventId": { "eventTypeId": event_type_id } })
        }
        DefinitionSchemaViolation::DuplicateSlotKey { event_id, key } => {
            json!({ "duplicateSlotKey": { "eventTypeId": event_id, "key": key } })
        }
        DefinitionSchemaViolation::OrderClassNotAscending {
            event_id,
            previous,
            current,
        } => json!({
            "orderClassNotAscending": {
                "eventTypeId": event_id,
                "previous": previous,
                "current": current
            }
        }),
        DefinitionSchemaViolation::CoreTypeMismatch {
            event_id,
            key,
            actual,
            expected,
        } => json!({
            "coreTypeMismatch": {
                "eventTypeId": event_id,
                "key": key,
                "actual": actual,
                "expected": expected
            }
        }),
        DefinitionSchemaViolation::UnknownTlvType { event_id, key, ty } => {
            json!({ "unknownTlvType": { "eventTypeId": event_id, "key": key, "type": ty } })
        }
        DefinitionSchemaViolation::ReservedCoreKey { event_id, key } => {
            json!({ "reservedCoreKey": { "eventTypeId": event_id, "key": key } })
        }
        DefinitionSchemaViolation::InvalidMaxOccurs { event_id, key } => {
            json!({ "invalidMaxOccurs": { "eventTypeId": event_id, "key": key } })
        }
        DefinitionSchemaViolation::UnknownProceduralModel {
            state_machine_id,
            model,
        } => json!({
            "unknownProceduralModel": {
                "stateMachineId": state_machine_id,
                "model": model
            }
        }),
        DefinitionSchemaViolation::MissingEnumSet {
            state_machine_id,
            enum_set,
        } => json!({
            "missingEnumSet": {
                "stateMachineId": state_machine_id,
                "enumSet": enum_set
            }
        }),
        DefinitionSchemaViolation::ProceduralStateMismatch {
            state_machine_id,
            model,
            value,
            label,
        } => json!({
            "proceduralStateMismatch": {
                "stateMachineId": state_machine_id,
                "model": model,
                "value": value,
                "label": label
            }
        }),
    }
}

fn wire_error_document(reason: &WireError) -> Value {
    match reason {
        WireError::CrcMismatch { expected, actual } => {
            json!({ "crcMismatch": { "expected": expected, "actual": actual } })
        }
        WireError::InvalidLength {
            total_len,
            available,
        } => json!({ "invalidLength": { "totalLength": total_len, "available": available } }),
        WireError::InvalidPadding { offset } => json!({ "invalidPadding": { "offset": offset } }),
        WireError::InvalidSlot { offset } => json!({ "invalidSlot": { "offset": offset } }),
        WireError::RecordTooLong { len } => json!({ "recordTooLong": { "len": len } }),
        WireError::SlotTooLong { key, len } => {
            json!({ "slotTooLong": { "key": key, "len": len } })
        }
        WireError::Truncated { needed, available } => {
            json!({ "truncated": { "needed": needed, "available": available } })
        }
        WireError::WrongSync => json!({ "wrongSync": {} }),
    }
}

fn raw_slot_document(slot: &open_ot_carriage::wire::Slot) -> RawSlot {
    RawSlot {
        key: slot.key,
        type_tag: slot.ty,
        payload_hex: hex_lower(&slot.payload),
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0F) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use open_ot_carriage::control::ControlBlockSnapshot;
    use open_ot_carriage::loss::LossEvent;
    use open_ot_carriage::registry::{KEY_CATEGORY, KEY_NEW_STATE, TY_STRING, TY_UDINT};
    use open_ot_carriage::wire::{Record, Slot, decode};
    use open_ot_definition::hash::compute_content_hash;
    use open_ot_definition::model::sample_definition;
    use open_ot_definition::schema::SchemaViolation;
    use open_ot_definition::{DefinitionSet, resolve_record};

    const HASH: [u8; 8] = [0x70, 0xDF, 0x73, 0x94, 0xB8, 0x75, 0xE4, 0x92];

    #[test]
    fn resolved_event_serializes_to_exact_json() {
        let resolution = resolved(conformant_state_transition_record(), 0);
        let context = RecordDocumentContext::new(9, 50_000, HASH, 0).with_semantic_version("1.0.0");

        let json = to_json(&document_from_resolution(&resolution, &context)).unwrap();

        assert_eq!(
            json,
            include_str!("../fixtures/event_state_transition.json").trim()
        );
    }

    #[test]
    fn vendor_extension_serializes_without_resolving_name() {
        let mut record = conformant_message_record();
        record
            .slots
            .push(Slot::new(0x8001, TY_STRING, b"private-note".to_vec()));
        let resolution = resolved(record, 0);
        let context = RecordDocumentContext::new(9, 50_000, HASH, 0).with_semantic_version("1.0.0");

        let json = to_json(&document_from_resolution(&resolution, &context)).unwrap();

        assert_eq!(
            json,
            include_str!("../fixtures/event_extension_field.json").trim()
        );
    }

    #[test]
    fn unknown_enum_value_keeps_numeric_value_with_null_label() {
        let mut record = conformant_state_transition_record();
        let category = record
            .slots
            .iter_mut()
            .find(|slot| slot.key == KEY_CATEGORY)
            .unwrap();
        category.payload = 99_u16.to_le_bytes().to_vec();
        let resolution = resolved(record, 0);
        let context = RecordDocumentContext::new(9, 50_005, HASH, 0).with_semantic_version("1.0.0");

        let json = to_json(&document_from_resolution(&resolution, &context)).unwrap();

        assert_eq!(
            json,
            include_str!("../fixtures/event_unknown_enum_value.json").trim()
        );
    }

    #[test]
    fn schema_violation_placeholder_preserves_raw_slots() {
        let resolution = resolved(codec_state_transition_negative_record(), 0);
        let context = RecordDocumentContext::new(9, 50_001, HASH, FLAG_TIME_UNSYNCED)
            .with_semantic_version("1.0.0")
            .with_source_time(1_000);

        let json = to_json(&document_from_resolution(&resolution, &context)).unwrap();

        assert_eq!(
            json,
            include_str!("../fixtures/placeholder_schema_violation.json").trim()
        );
    }

    #[test]
    fn drift_placeholder_has_expected_and_actual_hashes() {
        let definition = sample_definition();
        let snapshot = snapshot([0x11; 8], [0; 8], 0);
        let resolution = resolve_record(
            &conformant_state_transition_record(),
            0,
            &snapshot,
            &DefinitionSet::current(&definition),
        );
        let context = RecordDocumentContext::new(9, 50_002, [0x11; 8], 0)
            .with_semantic_version("1.0.0")
            .with_source_time(1_000);

        let json = to_json(&document_from_resolution(&resolution, &context)).unwrap();

        assert_eq!(
            json,
            include_str!("../fixtures/placeholder_drift.json").trim()
        );
    }

    #[test]
    fn stale_prior_epoch_placeholder_serializes() {
        let definition = sample_definition();
        let hash = compute_content_hash(&definition).unwrap().carriage_hash;
        let snapshot = snapshot(hash, [0x22; 8], 100);
        let resolution = resolve_record(
            &conformant_state_transition_record(),
            99,
            &snapshot,
            &DefinitionSet::current(&definition),
        );
        let context = RecordDocumentContext::new(9, 50_003, [0x22; 8], 0)
            .with_semantic_version("1.0.0")
            .with_source_time(1_000);

        let json = to_json(&document_from_resolution(&resolution, &context)).unwrap();

        assert_eq!(
            json,
            include_str!("../fixtures/placeholder_stale_prior_epoch.json").trim()
        );
    }

    #[test]
    fn unknown_event_id_placeholder_serializes() {
        let mut record = conformant_state_transition_record();
        record.event_type_id = 0x7777;
        let resolution = resolved(record, 0);
        let context = RecordDocumentContext::new(9, 50_004, HASH, 0)
            .with_semantic_version("1.0.0")
            .with_source_time(1_000);

        let json = to_json(&document_from_resolution(&resolution, &context)).unwrap();

        assert_eq!(
            json,
            include_str!("../fixtures/placeholder_unknown_event_id.json").trim()
        );
    }

    #[test]
    fn authoritative_loss_serializes_to_exact_json() {
        let event = LossEvent {
            buffer_id: 9,
            run_id: 1,
            source_id: 88,
            first_seq: 0,
            last_seq: 4,
            count: 5,
            synthetic: false,
        };
        let context = LossDocumentContext::new(60_000, 7, EpochRelation::Current, HASH)
            .with_semantic_version("1.0.0");

        let json = to_json(&document_from_loss(&event, &context)).unwrap();

        assert_eq!(
            json,
            include_str!("../fixtures/loss_authoritative.json").trim()
        );
    }

    #[test]
    fn inferred_loss_serializes_to_exact_json() {
        let event = LossEvent {
            buffer_id: 9,
            run_id: 1,
            source_id: 42,
            first_seq: 101,
            last_seq: 136,
            count: 36,
            synthetic: true,
        };
        let context = LossDocumentContext::new(60_001, 7, EpochRelation::Current, HASH)
            .with_semantic_version("1.0.0");

        let json = to_json(&document_from_loss(&event, &context)).unwrap();

        assert_eq!(json, include_str!("../fixtures/loss_inferred.json").trim());
    }

    #[test]
    fn placeholder_reason_taxonomy_covers_all_resolver_reasons() {
        let schema_reason = ResolvePlaceholderReason::Schema(SchemaViolation::MissingRequired {
            key: KEY_NEW_STATE,
            min: 1,
            actual: 0,
        });
        assert_eq!(
            reason_document(&schema_reason).kind,
            PlaceholderReasonKind::SchemaViolation
        );
        assert_eq!(
            reason_document(&ResolvePlaceholderReason::MissingCurrentDefinition).kind,
            PlaceholderReasonKind::MissingCurrentDefinition
        );
        assert_eq!(
            reason_document(&ResolvePlaceholderReason::FullHashDrift {
                expected: "00".to_string(),
                actual: "11".to_string(),
            })
            .kind,
            PlaceholderReasonKind::FullHashDrift
        );
        assert_eq!(
            reason_document(&ResolvePlaceholderReason::UnknownTlvType { key: 1, ty: 255 }).kind,
            PlaceholderReasonKind::InvalidPayload
        );
        assert_eq!(
            reason_document(&ResolvePlaceholderReason::Hash(DefinitionErrorString(
                "bad hash".to_string()
            )))
            .kind,
            PlaceholderReasonKind::HashError
        );
    }

    fn resolved(record: Record, record_abs: u64) -> Resolution {
        let definition = sample_definition();
        let hash = compute_content_hash(&definition).unwrap().carriage_hash;
        resolve_record(
            &record,
            record_abs,
            &snapshot(hash, [0; 8], 0),
            &DefinitionSet::current(&definition),
        )
    }

    fn snapshot(
        definition_hash: [u8; 8],
        prev_definition_hash: [u8; 8],
        epoch_first_abs: u64,
    ) -> ControlBlockSnapshot {
        ControlBlockSnapshot {
            version: 2,
            caps: 0,
            buffer_id: 9,
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

    #[test]
    fn extension_field_in_resolver_output_keeps_raw_payload() {
        let mut record = conformant_message_record();
        record
            .slots
            .push(Slot::new(0x8001, TY_UDINT, 99_u32.to_le_bytes()));
        let Resolution::Resolved(resolved) = resolved(record, 0) else {
            panic!("expected resolved record with extension");
        };
        assert_eq!(resolved.extension_fields.len(), 1);
        assert_eq!(resolved.extension_fields[0].key, 0x8001);
        assert_eq!(
            resolved.extension_fields[0].type_name.as_deref(),
            Some("UDInt")
        );
        assert_eq!(
            resolved.extension_fields[0].value,
            Some(ResolvedValue::UDInt(99))
        );
        assert_eq!(resolved.extension_fields[0].payload, 99_u32.to_le_bytes());
    }
}
