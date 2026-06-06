//! Canonical ids used by the carriage and definition prototypes.
//!
//! The concrete integer values are provisional experiment values, but this module keeps
//! the crate on one append-only table so collisions are caught in tests.

/// Source id reserved for logger lifecycle records.
pub const SYSTEM_SOURCE_ID: u32 = 0;

/// Event id range for Base events.
pub const EVENT_RANGE_BASE: core::ops::RangeInclusive<u32> = 0x0000_0001..=0x0000_00FF;
/// Event id range for System events.
pub const EVENT_RANGE_SYSTEM: core::ops::RangeInclusive<u32> = 0x0000_0100..=0x0000_01FF;
/// Event id range for Condition events.
pub const EVENT_RANGE_CONDITION: core::ops::RangeInclusive<u32> = 0x0000_0200..=0x0000_02FF;
/// Event id range for Procedural events.
pub const EVENT_RANGE_PROCEDURAL: core::ops::RangeInclusive<u32> = 0x0000_0300..=0x0000_03FF;
/// Event id range for Regulated events.
pub const EVENT_RANGE_REGULATED: core::ops::RangeInclusive<u32> = 0x0000_0400..=0x0000_04FF;
/// Event id range for private extensions.
pub const EVENT_RANGE_VENDOR: core::ops::RangeInclusive<u32> = 0x8000_0000..=0xFFFF_FFFF;

/// Base event: state transition.
pub const EVENT_STATE_TRANSITION: u32 = 0x0001;
/// Base event: value changed.
pub const EVENT_VALUE_CHANGED: u32 = 0x0002;
/// Base event: message.
pub const EVENT_MESSAGE: u32 = 0x0003;

/// System event: heartbeat.
pub const EVENT_HEARTBEAT: u32 = 0x0100;
/// System event: logger started.
pub const EVENT_LOGGER_STARTED: u32 = 0x0101;
/// System event: logger stopped.
pub const EVENT_LOGGER_STOPPED: u32 = 0x0102;
/// System event: buffer cleared.
pub const EVENT_BUFFER_CLEARED: u32 = 0x0103;
/// System event: records dropped.
pub const EVENT_RECORDS_DROPPED: u32 = 0x0104;
/// System event: source registered.
pub const EVENT_SOURCE_REGISTERED: u32 = 0x0105;
/// System event: definition changed.
pub const EVENT_DEFINITION_CHANGED: u32 = 0x0106;
/// System event: time synchronization changed.
pub const EVENT_TIME_SYNC_CHANGED: u32 = 0x0107;
/// Vendor extension event: per-source produced-count checkpoint.
pub const EVENT_SOURCE_HIGH_WATER: u32 = 0x8000_0108;

/// Condition event: active.
pub const EVENT_CONDITION_ACTIVE: u32 = 0x0200;
/// Condition event: cleared.
pub const EVENT_CONDITION_CLEARED: u32 = 0x0201;
/// Condition event: acknowledged.
pub const EVENT_CONDITION_ACKNOWLEDGED: u32 = 0x0202;
/// Condition event: confirmed.
pub const EVENT_CONDITION_CONFIRMED: u32 = 0x0203;
/// Condition event: shelved.
pub const EVENT_CONDITION_SHELVED: u32 = 0x0204;
/// Condition event: unshelved.
pub const EVENT_CONDITION_UNSHELVED: u32 = 0x0205;
/// Condition event: suppressed.
pub const EVENT_CONDITION_SUPPRESSED: u32 = 0x0206;
/// Condition event: unsuppressed.
pub const EVENT_CONDITION_UNSUPPRESSED: u32 = 0x0207;
/// Condition event: out of service.
pub const EVENT_CONDITION_OUT_OF_SERVICE: u32 = 0x0208;
/// Condition event: in service.
pub const EVENT_CONDITION_IN_SERVICE: u32 = 0x0209;
/// Condition event: commented.
pub const EVENT_CONDITION_COMMENTED: u32 = 0x020A;
/// Condition event: reset.
pub const EVENT_CONDITION_RESET: u32 = 0x020B;
/// Condition event: priority changed.
pub const EVENT_CONDITION_PRIORITY_CHANGED: u32 = 0x020C;
/// Condition event: refresh start.
pub const EVENT_REFRESH_START: u32 = 0x020D;
/// Condition event: refresh end.
pub const EVENT_REFRESH_END: u32 = 0x020E;

/// Procedural event: recipe loaded.
pub const EVENT_RECIPE_LOADED: u32 = 0x0301;
/// Procedural event: recipe approved.
pub const EVENT_RECIPE_APPROVED: u32 = 0x0302;
/// Procedural event: batch event.
pub const EVENT_BATCH_EVENT: u32 = 0x0303;
/// Procedural event: material addition.
pub const EVENT_MATERIAL_ADDITION: u32 = 0x0304;

/// Regulated event: operator action.
pub const EVENT_OPERATOR_ACTION: u32 = 0x0400;
/// Regulated event: operator login.
pub const EVENT_OPERATOR_LOGIN: u32 = 0x0401;
/// Regulated event: operator logout.
pub const EVENT_OPERATOR_LOGOUT: u32 = 0x0402;
/// Regulated event: parameter change.
pub const EVENT_PARAMETER_CHANGE: u32 = 0x0403;
/// Regulated event: electronic signature.
pub const EVENT_ESIGNATURE: u32 = 0x0404;
/// Regulated event: security access failure.
pub const EVENT_SECURITY_ACCESS_FAILURE: u32 = 0x0405;
/// Regulated event: program download.
pub const EVENT_PROGRAM_DOWNLOAD: u32 = 0x0406;

/// Field key: state machine id.
pub const KEY_STATE_MACHINE_ID: u16 = 0x0001;
/// Field key: category.
pub const KEY_CATEGORY: u16 = 0x0002;
/// Field key: previous state.
pub const KEY_PREVIOUS_STATE: u16 = 0x0003;
/// Field key: new state.
pub const KEY_NEW_STATE: u16 = 0x0004;
/// Field key: condition id.
pub const KEY_CONDITION_ID: u16 = 0x0005;
/// Field key: condition class.
pub const KEY_CONDITION_CLASS: u16 = 0x0006;
/// Field key: correlation id.
pub const KEY_CORRELATION_ID: u16 = 0x0007;
/// Field key: severity.
pub const KEY_SEVERITY: u16 = 0x0008;
/// Field key: cause operand.
pub const KEY_CAUSE_OPERAND: u16 = 0x0009;
/// Field key: action id.
pub const KEY_ACTION_ID: u16 = 0x000A;
/// Field key: actor.
pub const KEY_ACTOR: u16 = 0x000B;
/// Field key: context reference.
pub const KEY_CONTEXT_REF: u16 = 0x000C;
/// Field key: value id.
pub const KEY_VALUE_ID: u16 = 0x000D;
/// Reserved field key: data type.
pub const KEY_DATA_TYPE_RESERVED: u16 = 0x000E;
/// Field key: previous value.
pub const KEY_PREVIOUS_VALUE: u16 = 0x000F;
/// Field key: new value.
pub const KEY_NEW_VALUE: u16 = 0x0010;
/// Field key: quality.
pub const KEY_QUALITY: u16 = 0x0011;
/// Field key: semantic role.
pub const KEY_SEMANTIC_ROLE: u16 = 0x0012;
/// Field key: unit.
pub const KEY_UNIT: u16 = 0x0013;
/// Field key: message template id.
pub const KEY_MESSAGE_TEMPLATE_ID: u16 = 0x0014;
/// Field key: argument.
pub const KEY_ARG: u16 = 0x0015;
/// Field key: count.
pub const KEY_COUNT: u16 = 0x0016;
/// Field key: dropped record count.
pub const KEY_DROPPED_COUNT: u16 = KEY_COUNT;
/// Field key: first lost sequence.
pub const KEY_FIRST_LOST_SEQ: u16 = 0x0017;
/// Field key: last lost sequence.
pub const KEY_LAST_LOST_SEQ: u16 = 0x0018;
/// Field key: window start.
pub const KEY_WINDOW_START: u16 = 0x0019;
/// Field key: window end.
pub const KEY_WINDOW_END: u16 = 0x001A;
/// Field key: source path.
pub const KEY_SOURCE_PATH: u16 = 0x001B;
/// Field key: new definition hash prefix.
pub const KEY_DEF_HASH_NEW: u16 = 0x001C;
/// Field key: acknowledged by.
pub const KEY_ACK_BY: u16 = 0x001D;
/// Field key: shelve seconds.
pub const KEY_SHELVE_SECS: u16 = 0x001E;
/// Field key: reason.
pub const KEY_REASON: u16 = 0x001F;
/// Field key: authorization result.
pub const KEY_AUTH_RESULT: u16 = 0x0020;
/// Field key: workstation.
pub const KEY_WORKSTATION: u16 = 0x0021;
/// Field key: signature meaning.
pub const KEY_SIGNATURE_MEANING: u16 = 0x0022;
/// Field key: recipe id.
pub const KEY_RECIPE_ID: u16 = 0x0023;
/// Field key: recipe version.
pub const KEY_RECIPE_VERSION: u16 = 0x0024;
/// Field key: batch id.
pub const KEY_BATCH_ID: u16 = 0x0025;
/// Field key: material id.
pub const KEY_MATERIAL_ID: u16 = 0x0026;
/// Field key: quantity.
pub const KEY_QUANTITY: u16 = 0x0027;
/// Field key: program id.
pub const KEY_PROGRAM_ID: u16 = 0x0028;
/// Field key: registered source id.
pub const KEY_REGISTERED_SOURCE_ID: u16 = 0x0029;
/// Field key: interval in milliseconds.
pub const KEY_INTERVAL_MS: u16 = 0x002A;
/// Field key: sequence base.
pub const KEY_SEQ_BASE: u16 = 0x002B;
/// Field key: new priority.
pub const KEY_NEW_PRIORITY: u16 = 0x002C;
/// Reserved field key.
pub const KEY_RESERVED_002D: u16 = 0x002D;
/// Field key: previous priority.
pub const KEY_PREVIOUS_PRIORITY: u16 = 0x002E;
/// Field key: signed event sequence.
pub const KEY_SIGNED_EVENT_SEQ: u16 = 0x002F;
/// Field key: effective time.
pub const KEY_EFFECTIVE_TIME: u16 = 0x0030;
/// Field key: correction of.
pub const KEY_CORRECTION_OF: u16 = 0x0031;
/// Field key: clock quality.
pub const KEY_CLOCK_QUALITY: u16 = 0x0032;
/// Field key: role.
pub const KEY_ROLE: u16 = 0x0033;
/// Field key: refresh id.
pub const KEY_REFRESH_ID: u16 = 0x0034;
/// Field key: group id.
pub const KEY_GROUP_ID: u16 = 0x0035;
/// Field key: first in group.
pub const KEY_FIRST_IN_GROUP: u16 = 0x0036;
/// Field key: comment.
pub const KEY_COMMENT: u16 = 0x0037;
/// Field key: source produced-count checkpoint.
pub const KEY_PRODUCED_COUNT: u16 = 0x0038;
/// Field key: source produced-count checkpoint.
pub const KEY_SOURCE_HIGH_WATER: u16 = KEY_PRODUCED_COUNT;
/// Field key: old definition hash prefix.
pub const KEY_DEF_HASH_OLD: u16 = 0x0039;
/// Field key: epoch id.
pub const KEY_EPOCH_ID: u16 = 0x003A;
/// Field key: cold-start flag.
pub const KEY_COLD_START: u16 = 0x003B;

/// TLV type tag: Bool.
pub const TY_BOOL: u8 = 0x00;
/// TLV type tag: SInt.
pub const TY_SINT: u8 = 0x01;
/// TLV type tag: USInt.
pub const TY_USINT: u8 = 0x02;
/// TLV type tag: UInt.
pub const TY_UINT: u8 = 0x03;
/// TLV type tag: Int.
pub const TY_INT: u8 = 0x04;
/// TLV type tag: UDInt.
pub const TY_UDINT: u8 = 0x05;
/// TLV type tag: DInt.
pub const TY_DINT: u8 = 0x06;
/// TLV type tag: ULInt.
pub const TY_ULINT: u8 = 0x07;
/// TLV type tag: LInt.
pub const TY_LINT: u8 = 0x08;
/// TLV type tag: Real.
pub const TY_REAL: u8 = 0x09;
/// TLV type tag: LReal.
pub const TY_LREAL: u8 = 0x0A;
/// TLV type tag: DateTime.
pub const TY_DATE_TIME: u8 = 0x0B;
/// TLV type tag: String.
pub const TY_STRING: u8 = 0x0C;
/// TLV type tag: Bytes.
pub const TY_BYTES: u8 = 0x0D;

/// Event registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventSpec {
    /// Event type id.
    pub id: u32,
    /// Canonical event name.
    pub name: &'static str,
    /// Event profile group.
    pub group: EventGroup,
}

/// Event profile group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventGroup {
    /// Base events.
    Base,
    /// System/diagnostic events.
    System,
    /// Condition lifecycle events.
    Condition,
    /// Procedural events.
    Procedural,
    /// Regulated/operator events.
    Regulated,
}

/// Field-key registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSpec {
    /// Value key id.
    pub key: u16,
    /// Canonical field name.
    pub name: &'static str,
    /// Canonical type rule for this field.
    pub kind: FieldKind,
    /// True when the field may appear multiple times.
    pub repeatable: bool,
}

/// Canonical type rule for a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// Fixed TLV type tag.
    Fixed(u8),
    /// The slot type is the logged value's data type.
    ValuePayload,
    /// Reserved key; a core producer must not emit it.
    Reserved,
}

/// TLV type registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlvTypeSpec {
    /// TLV type tag.
    pub code: u8,
    /// Canonical type name.
    pub name: &'static str,
    /// Fixed payload width in bytes, or `None` for variable-length types.
    pub fixed_width: Option<usize>,
}

/// Enum registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumValue {
    /// Canonical integer value.
    pub value: u16,
    /// Canonical label.
    pub label: &'static str,
}

/// Severity band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeverityBand {
    /// 1..=332.
    Low,
    /// 333..=666.
    Medium,
    /// 667..=1000.
    High,
}

/// Procedural model registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProceduralModelSpec {
    /// Canonical model name.
    pub name: &'static str,
    /// Model-specific state values.
    pub states: &'static [EnumValue],
}

/// Complete core event catalog used by the prototype.
pub const EVENT_SPECS: &[EventSpec] = &[
    EventSpec {
        id: EVENT_STATE_TRANSITION,
        name: "StateTransition",
        group: EventGroup::Base,
    },
    EventSpec {
        id: EVENT_VALUE_CHANGED,
        name: "ValueChanged",
        group: EventGroup::Base,
    },
    EventSpec {
        id: EVENT_MESSAGE,
        name: "Message",
        group: EventGroup::Base,
    },
    EventSpec {
        id: EVENT_HEARTBEAT,
        name: "Heartbeat",
        group: EventGroup::System,
    },
    EventSpec {
        id: EVENT_LOGGER_STARTED,
        name: "LoggerStarted",
        group: EventGroup::System,
    },
    EventSpec {
        id: EVENT_LOGGER_STOPPED,
        name: "LoggerStopped",
        group: EventGroup::System,
    },
    EventSpec {
        id: EVENT_BUFFER_CLEARED,
        name: "BufferCleared",
        group: EventGroup::System,
    },
    EventSpec {
        id: EVENT_RECORDS_DROPPED,
        name: "RecordsDropped",
        group: EventGroup::System,
    },
    EventSpec {
        id: EVENT_SOURCE_REGISTERED,
        name: "SourceRegistered",
        group: EventGroup::System,
    },
    EventSpec {
        id: EVENT_DEFINITION_CHANGED,
        name: "DefinitionChanged",
        group: EventGroup::System,
    },
    EventSpec {
        id: EVENT_TIME_SYNC_CHANGED,
        name: "TimeSyncChanged",
        group: EventGroup::System,
    },
    EventSpec {
        id: EVENT_SOURCE_HIGH_WATER,
        name: "SourceHighWater",
        group: EventGroup::System,
    },
    EventSpec {
        id: EVENT_CONDITION_ACTIVE,
        name: "ConditionActive",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_CONDITION_CLEARED,
        name: "ConditionCleared",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_CONDITION_ACKNOWLEDGED,
        name: "ConditionAcknowledged",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_CONDITION_CONFIRMED,
        name: "ConditionConfirmed",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_CONDITION_SHELVED,
        name: "ConditionShelved",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_CONDITION_UNSHELVED,
        name: "ConditionUnshelved",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_CONDITION_SUPPRESSED,
        name: "ConditionSuppressed",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_CONDITION_UNSUPPRESSED,
        name: "ConditionUnsuppressed",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_CONDITION_OUT_OF_SERVICE,
        name: "ConditionOutOfService",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_CONDITION_IN_SERVICE,
        name: "ConditionInService",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_CONDITION_COMMENTED,
        name: "ConditionCommented",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_CONDITION_RESET,
        name: "ConditionReset",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_CONDITION_PRIORITY_CHANGED,
        name: "ConditionPriorityChanged",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_REFRESH_START,
        name: "RefreshStart",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_REFRESH_END,
        name: "RefreshEnd",
        group: EventGroup::Condition,
    },
    EventSpec {
        id: EVENT_RECIPE_LOADED,
        name: "RecipeLoaded",
        group: EventGroup::Procedural,
    },
    EventSpec {
        id: EVENT_RECIPE_APPROVED,
        name: "RecipeApproved",
        group: EventGroup::Procedural,
    },
    EventSpec {
        id: EVENT_BATCH_EVENT,
        name: "BatchEvent",
        group: EventGroup::Procedural,
    },
    EventSpec {
        id: EVENT_MATERIAL_ADDITION,
        name: "MaterialAddition",
        group: EventGroup::Procedural,
    },
    EventSpec {
        id: EVENT_OPERATOR_ACTION,
        name: "OperatorAction",
        group: EventGroup::Regulated,
    },
    EventSpec {
        id: EVENT_OPERATOR_LOGIN,
        name: "OperatorLogin",
        group: EventGroup::Regulated,
    },
    EventSpec {
        id: EVENT_OPERATOR_LOGOUT,
        name: "OperatorLogout",
        group: EventGroup::Regulated,
    },
    EventSpec {
        id: EVENT_PARAMETER_CHANGE,
        name: "ParameterChange",
        group: EventGroup::Regulated,
    },
    EventSpec {
        id: EVENT_ESIGNATURE,
        name: "ESignature",
        group: EventGroup::Regulated,
    },
    EventSpec {
        id: EVENT_SECURITY_ACCESS_FAILURE,
        name: "SecurityAccessFailure",
        group: EventGroup::Regulated,
    },
    EventSpec {
        id: EVENT_PROGRAM_DOWNLOAD,
        name: "ProgramDownload",
        group: EventGroup::Regulated,
    },
];

/// Complete field catalog from the full draft plus experiment deltas.
pub const FIELD_SPECS: &[FieldSpec] = &[
    field(
        KEY_STATE_MACHINE_ID,
        "stateMachineId",
        FieldKind::Fixed(TY_UDINT),
        false,
    ),
    field(KEY_CATEGORY, "category", FieldKind::Fixed(TY_UINT), false),
    field(
        KEY_PREVIOUS_STATE,
        "previousState",
        FieldKind::Fixed(TY_UINT),
        false,
    ),
    field(KEY_NEW_STATE, "newState", FieldKind::Fixed(TY_UINT), false),
    field(
        KEY_CONDITION_ID,
        "conditionId",
        FieldKind::Fixed(TY_UDINT),
        false,
    ),
    field(
        KEY_CONDITION_CLASS,
        "conditionClass",
        FieldKind::Fixed(TY_UINT),
        false,
    ),
    field(
        KEY_CORRELATION_ID,
        "correlationId",
        FieldKind::Fixed(TY_UDINT),
        false,
    ),
    field(KEY_SEVERITY, "severity", FieldKind::Fixed(TY_UINT), false),
    field(
        KEY_CAUSE_OPERAND,
        "causeOperand",
        FieldKind::Fixed(TY_UDINT),
        true,
    ),
    field(KEY_ACTION_ID, "actionId", FieldKind::Fixed(TY_UDINT), false),
    field(KEY_ACTOR, "actor", FieldKind::Fixed(TY_STRING), false),
    field(
        KEY_CONTEXT_REF,
        "contextRef",
        FieldKind::Fixed(TY_UDINT),
        true,
    ),
    field(KEY_VALUE_ID, "valueId", FieldKind::Fixed(TY_UDINT), false),
    field(
        KEY_DATA_TYPE_RESERVED,
        "dataType",
        FieldKind::Reserved,
        false,
    ),
    field(
        KEY_PREVIOUS_VALUE,
        "previousValue",
        FieldKind::ValuePayload,
        false,
    ),
    field(KEY_NEW_VALUE, "newValue", FieldKind::ValuePayload, false),
    field(KEY_QUALITY, "quality", FieldKind::Fixed(TY_UINT), false),
    field(
        KEY_SEMANTIC_ROLE,
        "semanticRole",
        FieldKind::Fixed(TY_UINT),
        false,
    ),
    field(KEY_UNIT, "unit", FieldKind::Fixed(TY_UINT), false),
    field(
        KEY_MESSAGE_TEMPLATE_ID,
        "messageTemplateId",
        FieldKind::Fixed(TY_UDINT),
        false,
    ),
    field(KEY_ARG, "arg", FieldKind::ValuePayload, true),
    field(KEY_COUNT, "count", FieldKind::Fixed(TY_UDINT), false),
    field(
        KEY_FIRST_LOST_SEQ,
        "firstLostSeq",
        FieldKind::Fixed(TY_ULINT),
        false,
    ),
    field(
        KEY_LAST_LOST_SEQ,
        "lastLostSeq",
        FieldKind::Fixed(TY_ULINT),
        false,
    ),
    field(
        KEY_WINDOW_START,
        "windowStart",
        FieldKind::Fixed(TY_DATE_TIME),
        false,
    ),
    field(
        KEY_WINDOW_END,
        "windowEnd",
        FieldKind::Fixed(TY_DATE_TIME),
        false,
    ),
    field(
        KEY_SOURCE_PATH,
        "sourcePath",
        FieldKind::Fixed(TY_STRING),
        false,
    ),
    field(
        KEY_DEF_HASH_NEW,
        "defHashNew",
        FieldKind::Fixed(TY_BYTES),
        false,
    ),
    field(KEY_ACK_BY, "ackBy", FieldKind::Fixed(TY_STRING), false),
    field(
        KEY_SHELVE_SECS,
        "shelveSecs",
        FieldKind::Fixed(TY_UDINT),
        false,
    ),
    field(KEY_REASON, "reason", FieldKind::Fixed(TY_STRING), false),
    field(
        KEY_AUTH_RESULT,
        "authResult",
        FieldKind::Fixed(TY_UINT),
        false,
    ),
    field(
        KEY_WORKSTATION,
        "workstation",
        FieldKind::Fixed(TY_STRING),
        false,
    ),
    field(
        KEY_SIGNATURE_MEANING,
        "signatureMeaning",
        FieldKind::Fixed(TY_UINT),
        false,
    ),
    field(KEY_RECIPE_ID, "recipeId", FieldKind::Fixed(TY_UDINT), false),
    field(
        KEY_RECIPE_VERSION,
        "recipeVersion",
        FieldKind::Fixed(TY_STRING),
        false,
    ),
    field(KEY_BATCH_ID, "batchId", FieldKind::Fixed(TY_UDINT), false),
    field(
        KEY_MATERIAL_ID,
        "materialId",
        FieldKind::Fixed(TY_UDINT),
        false,
    ),
    field(KEY_QUANTITY, "quantity", FieldKind::Fixed(TY_LREAL), false),
    field(
        KEY_PROGRAM_ID,
        "programId",
        FieldKind::Fixed(TY_UDINT),
        false,
    ),
    field(
        KEY_REGISTERED_SOURCE_ID,
        "registeredSourceId",
        FieldKind::Fixed(TY_UDINT),
        false,
    ),
    field(
        KEY_INTERVAL_MS,
        "intervalMs",
        FieldKind::Fixed(TY_UDINT),
        false,
    ),
    field(KEY_SEQ_BASE, "seqBase", FieldKind::Fixed(TY_ULINT), false),
    field(
        KEY_NEW_PRIORITY,
        "newPriority",
        FieldKind::Fixed(TY_UINT),
        false,
    ),
    field(
        KEY_RESERVED_002D,
        "reserved002D",
        FieldKind::Reserved,
        false,
    ),
    field(
        KEY_PREVIOUS_PRIORITY,
        "previousPriority",
        FieldKind::Fixed(TY_UINT),
        false,
    ),
    field(
        KEY_SIGNED_EVENT_SEQ,
        "signedEventSeq",
        FieldKind::Fixed(TY_ULINT),
        false,
    ),
    field(
        KEY_EFFECTIVE_TIME,
        "effectiveTime",
        FieldKind::Fixed(TY_DATE_TIME),
        false,
    ),
    field(
        KEY_CORRECTION_OF,
        "correctionOf",
        FieldKind::Fixed(TY_ULINT),
        false,
    ),
    field(
        KEY_CLOCK_QUALITY,
        "clockQuality",
        FieldKind::Fixed(TY_UINT),
        false,
    ),
    field(KEY_ROLE, "role", FieldKind::Fixed(TY_UINT), false),
    field(
        KEY_REFRESH_ID,
        "refreshId",
        FieldKind::Fixed(TY_UDINT),
        false,
    ),
    field(KEY_GROUP_ID, "groupId", FieldKind::Fixed(TY_UDINT), false),
    field(
        KEY_FIRST_IN_GROUP,
        "firstInGroup",
        FieldKind::Fixed(TY_BOOL),
        false,
    ),
    field(KEY_COMMENT, "comment", FieldKind::Fixed(TY_STRING), false),
    field(
        KEY_PRODUCED_COUNT,
        "producedCount",
        FieldKind::Fixed(TY_ULINT),
        false,
    ),
    field(
        KEY_DEF_HASH_OLD,
        "defHashOld",
        FieldKind::Fixed(TY_BYTES),
        false,
    ),
    field(KEY_EPOCH_ID, "epochId", FieldKind::Fixed(TY_ULINT), false),
    field(
        KEY_COLD_START,
        "coldStart",
        FieldKind::Fixed(TY_BOOL),
        false,
    ),
];

/// Complete TLV type catalog.
pub const TLV_TYPE_SPECS: &[TlvTypeSpec] = &[
    ty(TY_BOOL, "Bool", Some(1)),
    ty(TY_SINT, "SInt", Some(1)),
    ty(TY_USINT, "USInt", Some(1)),
    ty(TY_UINT, "UInt", Some(2)),
    ty(TY_INT, "Int", Some(2)),
    ty(TY_UDINT, "UDInt", Some(4)),
    ty(TY_DINT, "DInt", Some(4)),
    ty(TY_ULINT, "ULInt", Some(8)),
    ty(TY_LINT, "LInt", Some(8)),
    ty(TY_REAL, "Real", Some(4)),
    ty(TY_LREAL, "LReal", Some(8)),
    ty(TY_DATE_TIME, "DateTime", Some(8)),
    ty(TY_STRING, "String", None),
    ty(TY_BYTES, "Bytes", None),
];

/// Category enum values.
pub const CATEGORY_VALUES: &[EnumValue] =
    &[ev(0, "ProcessState"), ev(1, "Mode"), ev(2, "Procedural")];

/// Condition class enum values.
pub const CONDITION_CLASS_VALUES: &[EnumValue] = &[ev(0, "Alarm"), ev(1, "Interlock")];

/// Quality enum values.
pub const QUALITY_VALUES: &[EnumValue] = &[
    ev(0, "Good"),
    ev(1, "Uncertain"),
    ev(2, "Bad"),
    ev(3, "Unknown"),
];

/// Authorization result enum values.
pub const AUTH_RESULT_VALUES: &[EnumValue] = &[
    ev(0, "Granted"),
    ev(1, "Denied"),
    ev(2, "NotRequired"),
    ev(3, "Pending"),
    ev(4, "Expired"),
];

/// Semantic role enum values.
pub const SEMANTIC_ROLE_VALUES: &[EnumValue] = &[
    ev(0, "Actual"),
    ev(1, "Setpoint"),
    ev(2, "Command"),
    ev(3, "Count"),
    ev(4, "Position"),
    ev(5, "Status"),
];

/// Signature meaning enum values.
pub const SIGNATURE_MEANING_VALUES: &[EnumValue] = &[
    ev(0, "Authored"),
    ev(1, "Reviewed"),
    ev(2, "Approved"),
    ev(3, "Verified"),
    ev(4, "Performed"),
    ev(5, "Witnessed"),
];

/// Clock quality enum values.
pub const CLOCK_QUALITY_VALUES: &[EnumValue] = &[
    ev(0, "Unknown"),
    ev(1, "FreeRunning"),
    ev(2, "Synced"),
    ev(3, "Holdover"),
];

/// Time synchronization state enum values.
pub const TIME_SYNC_STATE_VALUES: &[EnumValue] = &[
    ev(0, "Unsynced"),
    ev(1, "Synced"),
    ev(2, "SteppedForward"),
    ev(3, "SteppedBackward"),
];

/// Batch state enum values.
pub const BATCH_STATE_VALUES: &[EnumValue] = &[
    ev(0, "Started"),
    ev(1, "Completed"),
    ev(2, "Held"),
    ev(3, "Resumed"),
    ev(4, "Aborted"),
    ev(5, "Paused"),
];

/// Procedural model A state values.
pub const PROCEDURAL_MODEL_A_STATES: &[EnumValue] = &[
    ev(0, "Idle"),
    ev(1, "Running"),
    ev(2, "Complete"),
    ev(3, "Pausing"),
    ev(4, "Paused"),
    ev(5, "Holding"),
    ev(6, "Held"),
    ev(7, "Restarting"),
    ev(8, "Stopping"),
    ev(9, "Stopped"),
    ev(10, "Aborting"),
    ev(11, "Aborted"),
];

/// Procedural model B state values.
pub const PROCEDURAL_MODEL_B_STATES: &[EnumValue] = &[
    ev(0, "Idle"),
    ev(1, "Starting"),
    ev(2, "Execute"),
    ev(3, "Completing"),
    ev(4, "Complete"),
    ev(5, "Holding"),
    ev(6, "Held"),
    ev(7, "Unholding"),
    ev(8, "Suspending"),
    ev(9, "Suspended"),
    ev(10, "Unsuspending"),
    ev(11, "Stopping"),
    ev(12, "Stopped"),
    ev(13, "Aborting"),
    ev(14, "Aborted"),
    ev(15, "Clearing"),
    ev(16, "Resetting"),
];

/// Procedural model catalog.
pub const PROCEDURAL_MODELS: &[ProceduralModelSpec] = &[
    ProceduralModelSpec {
        name: "ISA-88",
        states: PROCEDURAL_MODEL_A_STATES,
    },
    ProceduralModelSpec {
        name: "PackML",
        states: PROCEDURAL_MODEL_B_STATES,
    },
];

/// Returns the event registry entry for `id`.
pub fn event_spec(id: u32) -> Option<&'static EventSpec> {
    EVENT_SPECS.iter().find(|spec| spec.id == id)
}

/// Returns the field registry entry for `key`.
pub fn field_spec(key: u16) -> Option<&'static FieldSpec> {
    FIELD_SPECS.iter().find(|spec| spec.key == key)
}

/// Returns the TLV type registry entry for `code`.
pub fn tlv_type_spec(code: u8) -> Option<&'static TlvTypeSpec> {
    TLV_TYPE_SPECS.iter().find(|spec| spec.code == code)
}

/// True when the event id is in a core range.
pub fn is_core_event_id(id: u32) -> bool {
    id & 0x8000_0000 == 0
}

/// True when the event id is in the private extension range.
pub fn is_vendor_event_id(id: u32) -> bool {
    EVENT_RANGE_VENDOR.contains(&id)
}

/// True when the value key is in the core range.
pub fn is_core_key(key: u16) -> bool {
    (0x0001..=0x7FFF).contains(&key)
}

/// True when the value key is in the private extension range.
pub fn is_vendor_key(key: u16) -> bool {
    key >= 0x8000
}

/// Returns the severity band for the baseline 1..=1000 scale.
pub fn severity_band(severity: u16) -> Option<SeverityBand> {
    match severity {
        1..=332 => Some(SeverityBand::Low),
        333..=666 => Some(SeverityBand::Medium),
        667..=1000 => Some(SeverityBand::High),
        _ => None,
    }
}

const fn field(key: u16, name: &'static str, kind: FieldKind, repeatable: bool) -> FieldSpec {
    FieldSpec {
        key,
        name,
        kind,
        repeatable,
    }
}

const fn ty(code: u8, name: &'static str, fixed_width: Option<usize>) -> TlvTypeSpec {
    TlvTypeSpec {
        code,
        name,
        fixed_width,
    }
}

const fn ev(value: u16, label: &'static str) -> EnumValue {
    EnumValue { value, label }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_ids_are_unique() {
        let mut ids = EVENT_SPECS.iter().map(|spec| spec.id).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), EVENT_SPECS.len());
        assert_eq!(EVENT_SPECS.len(), 38);
        assert!(is_vendor_event_id(EVENT_SOURCE_HIGH_WATER));
        assert_eq!(
            event_spec(EVENT_SOURCE_HIGH_WATER).unwrap().name,
            "SourceHighWater"
        );
    }

    #[test]
    fn field_specs_cover_full_core_plus_phase0_delta() {
        let mut ids = FIELD_SPECS.iter().map(|spec| spec.key).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), FIELD_SPECS.len());
        assert_eq!(FIELD_SPECS.len(), KEY_COLD_START as usize);

        for key in 0x0001..=KEY_COLD_START {
            assert!(field_spec(key).is_some(), "missing key 0x{key:04X}");
        }
    }

    #[test]
    fn canonical_field_types_match_phase2_requirements() {
        assert_eq!(
            field_spec(KEY_STATE_MACHINE_ID).unwrap().kind,
            FieldKind::Fixed(TY_UDINT)
        );
        assert_eq!(
            field_spec(KEY_CATEGORY).unwrap().kind,
            FieldKind::Fixed(TY_UINT)
        );
        assert_eq!(
            field_spec(KEY_PREVIOUS_STATE).unwrap().kind,
            FieldKind::Fixed(TY_UINT)
        );
        assert_eq!(
            field_spec(KEY_NEW_STATE).unwrap().kind,
            FieldKind::Fixed(TY_UINT)
        );
        assert_eq!(
            field_spec(KEY_PREVIOUS_VALUE).unwrap().kind,
            FieldKind::ValuePayload
        );
        assert_eq!(
            field_spec(KEY_NEW_VALUE).unwrap().kind,
            FieldKind::ValuePayload
        );
        assert_eq!(field_spec(KEY_ARG).unwrap().kind, FieldKind::ValuePayload);
        assert!(field_spec(KEY_ARG).unwrap().repeatable);
        assert_eq!(
            field_spec(KEY_SOURCE_HIGH_WATER).unwrap().kind,
            FieldKind::Fixed(TY_ULINT)
        );
        assert_eq!(
            field_spec(KEY_DEF_HASH_OLD).unwrap().kind,
            FieldKind::Fixed(TY_BYTES)
        );
        assert_eq!(
            field_spec(KEY_EPOCH_ID).unwrap().kind,
            FieldKind::Fixed(TY_ULINT)
        );
        assert_eq!(
            field_spec(KEY_COLD_START).unwrap().kind,
            FieldKind::Fixed(TY_BOOL)
        );
    }

    #[test]
    fn tlv_type_tags_are_complete_and_widths_are_known() {
        let mut ids = TLV_TYPE_SPECS
            .iter()
            .map(|spec| spec.code)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), TLV_TYPE_SPECS.len());
        assert_eq!(TLV_TYPE_SPECS.len(), 14);
        assert_eq!(tlv_type_spec(TY_BOOL).unwrap().fixed_width, Some(1));
        assert_eq!(tlv_type_spec(TY_UDINT).unwrap().fixed_width, Some(4));
        assert_eq!(tlv_type_spec(TY_STRING).unwrap().fixed_width, None);
        assert_eq!(tlv_type_spec(TY_BYTES).unwrap().fixed_width, None);
    }

    #[test]
    fn enum_tables_include_required_values_and_severity_bands() {
        assert_eq!(CATEGORY_VALUES[2], ev(2, "Procedural"));
        assert_eq!(CONDITION_CLASS_VALUES[1], ev(1, "Interlock"));
        assert_eq!(QUALITY_VALUES[3], ev(3, "Unknown"));
        assert_eq!(AUTH_RESULT_VALUES[4], ev(4, "Expired"));
        assert_eq!(SEMANTIC_ROLE_VALUES[5], ev(5, "Status"));
        assert_eq!(SIGNATURE_MEANING_VALUES[5], ev(5, "Witnessed"));
        assert_eq!(CLOCK_QUALITY_VALUES[3], ev(3, "Holdover"));
        assert_eq!(TIME_SYNC_STATE_VALUES[3], ev(3, "SteppedBackward"));
        assert_eq!(BATCH_STATE_VALUES[5], ev(5, "Paused"));
        assert_eq!(severity_band(1), Some(SeverityBand::Low));
        assert_eq!(severity_band(333), Some(SeverityBand::Medium));
        assert_eq!(severity_band(1000), Some(SeverityBand::High));
        assert_eq!(severity_band(0), None);
    }

    #[test]
    fn procedural_models_are_present() {
        assert_eq!(PROCEDURAL_MODELS.len(), 2);
        assert_eq!(PROCEDURAL_MODELS[0].states[11], ev(11, "Aborted"));
        assert_eq!(PROCEDURAL_MODELS[1].states[16], ev(16, "Resetting"));
    }
}
