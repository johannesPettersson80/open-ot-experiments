# OpenOT Definition File Draft

The definition file is the id-to-meaning contract. Wire records carry ids and typed
values; the definition resolves those ids to names, units, enum labels, templates,
conditions, and event schemas.

## Top Level

The JSON object contains:

- `header`
- `eventTypes`
- `sources`
- `stateMachines`
- `conditions`
- `messageTemplates`
- `values`
- `units`
- `enumSets`
- `recipeDefinitions`
- `batchDefinitions`
- `materialDefinitions`
- `operatorDefinitions`
- `eSignatureMeanings`
- `severityScale`

Unknown top-level fields are rejected by the reference model.

## Header

`header` carries:

- `wireVersion`
- `semanticVersion`
- `profiles`
- `conformanceLevel`
- `caps`
- `constraints`
- `epochStrategy`
- `contentHash`

The current producer cap is `constraints.maxRecordSize = 256`, matching the ST
producer's fixed record staging buffer. `constraints.maxSlots` is 16.

## Event Schemas

`eventTypes[]` entries are `{ id, name, profile, slots[] }`. Each slot carries
`key`, `minOccurs`, `maxOccurs`, and `orderClass`, plus either a fixed `type` or
`valuePayload: true`.

The canonical event schemas are derived from the carriage registry, not hand-written
per generator. Optional fields such as message `arg`, message `severity`, condition
`correlationId`, and condition `causeOperand` are reserved in the canonical schemas
with `minOccurs: 0`.

## Semantic Tables

`values[]` resolves `valueId` to name, data type, optional unit id, deadband,
sampling policy, and semantic role.

`stateMachines[]` resolves state-machine ids to a name, category, optional
procedural model, and enum set. `enumSets[]` resolves state integer values to labels.

`conditions[]` resolves condition ids to a name, class, default severity, and
optional cause operands.

`messageTemplates[]` resolves message-template ids to names, format text, and
argument TLV types. Template text and argument meaning live here, not in the wire
record.

`units[]` resolves unit ids to symbols. truST validates unit symbols against the
canonical unit registry before generating the definition.

The recipe, batch, material, operator, and e-signature tables are part of the frozen
definition shape so richer event families can add content without adding new
top-level structure.

## Hash

`contentHash` is SHA-256 over the RFC-8785/JCS canonical JSON bytes with
`header.contentHash` set to the empty string during computation. The stored hash is
lowercase hex. The shared-memory control block carries the first eight digest bytes
in digest order as the carriage hash.

A consumer compares the carriage hash before resolving records and can verify the
full hash once it has the definition file. A mismatch produces placeholder records
instead of guessed semantics.

## Authoring Generation

The truST reference generates the file from `{attribute 'oot' := ...}` declaration
attributes. Default ids are declaration-order based:

- values start at 2001;
- state machines start at 7001;
- conditions start at 9001;
- message templates start at 10001.

Authors or tools can pin ids with the supported id keys when stability across source
reordering matters.
