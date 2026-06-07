# Document Format

This is an experimental proposal for OpenOT's consumer-facing document contract. It is not a
ratified standard. The format sits above the carriage and definition layers:

- carriage decodes bytes and reconciles loss;
- definition resolves an event into named, typed fields or a placeholder;
- document serializes that result without re-resolving it.

The serializer produces deterministic JSON with stable field order. The checked-in fixtures under
`crates/document/fixtures/` are exact outputs from the implementation.

> **This doc describes the formal `document`-crate contract** (resolved fields with names/units,
> epoch context, loss ranges). The reactor showcase's `examples/reactor/batch-log.json` is generated
> **through this resolver**: it carries `provenance` (buffer / epoch+hash / source / source+receive
> time / flags) and a **`fields[]` array** of resolved `{ key, name, type, value, unit?, enumLabel? }`
> entries — e.g. a `ValueChanged` carries `{ name: "newValue", value: 0.0, unit: "L" }`; a
> `StateTransition` carries `{ name: "newState", value: "Filling", enumLabel: "Filling" }`. The
> companion `batch-log.txt` is a readable, id-light rendering for humans.

## Document Kinds

Every output is one of three kinds.

| Kind | Input | Purpose |
| --- | --- | --- |
| `event` | `Resolution::Resolved` | A semantically resolved record with typed fields. |
| `loss` | `LossEvent` | A lost sequence range, either authoritative or inferred. |
| `placeholder` | `Resolution::Placeholder` | A record that must not be resolved, with raw slots preserved. |

## Provenance

Record documents carry:

- `bufferId`;
- `source` with at least `id`, and resolved `name`, `path`, `hierarchy`, `dynamic` when known;
- `runId`;
- `epoch` with `id`, `relation`, `definitionHash`, and optional `semanticVersion`;
- `sourceTimeNs` for record-backed documents;
- `receiveTimeNs`, assigned by the consumer;
- `flags` with `timeUnsynced`, `syntheticRecord`, and `partialPayload`.

The resolver output does not contain buffer id, receive time, record flags, or selected definition
hash. Callers provide that information through `RecordDocumentContext`.

Loss documents are range documents. They do not invent a `sourceTimeNs`; they carry the
source/run/buffer, epoch context, receive time, and `basis`.

## Events

An `event` document contains `eventName`, `eventTypeId`, `seq`, `fields`, and optional
`extensionFields`.

Resolved fields preserve the canonical key, name, type, value, optional unit, and enum label. Unknown
enum values are still data: the numeric `value` is emitted and `enumLabel` is `null`.

Private extension slots are not placeholders when they satisfy the schema rule that they are trailing
and ordered. They are emitted as `extensionFields` with raw `payloadHex` always present and typed
`value` only when decoding is safe.

## Loss

A `loss` document contains `firstSeq`, `lastSeq`, `count`, and `basis`.

`basis:"authoritative"` means the range came from a producer-authoritative `RecordsDropped` or
source high-water signal. `basis:"inferred"` means the range came only from a sequence gap. This is
separate from the wire record's synthetic flag.

## Placeholders

A `placeholder` document contains `eventTypeId`, `seq`, `reason`, and `rawSlots`.

Placeholder reasons are typed:

- `missingCurrentDefinition`;
- `stalePriorEpoch`;
- `drift`;
- `fullHashDrift`;
- `unknownEventId`;
- `schemaViolation`;
- `invalidPayload`;
- `hashError`.

Raw slots preserve `{key, type, payloadHex}` so a later pass with the correct definition can
re-resolve the record. Unknown core keys are schema violations and become placeholders.
`schemaViolation.detail` is structured JSON named by the violation variant, not language-specific
debug text.

## Golden Fixtures

The document crate asserts exact JSON for:

- a resolved StateTransition;
- a resolved StateTransition with an unknown enum value kept as data;
- a resolved Message with a private extension field;
- schema-violation, drift, stale-prior-epoch, and unknown-event placeholders;
- authoritative and inferred loss ranges.
