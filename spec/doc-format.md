# OpenOT Document Format Draft

The document format is the consumer-facing JSON contract above carriage and
definition resolution. It serializes the result of a resolver pass; it does not
perform resolution itself.

## Kinds

Every document has one of these shapes:

- `event` for a resolved record;
- `loss` for a reconciled lost sequence range;
- `placeholder` for a record that must not be semantically resolved.

## Event Documents

An event document contains:

- `kind: "event"`
- `provenance`
- `eventName`
- `eventTypeId`
- `seq`
- `fields[]`
- optional `extensionFields[]`

`fields[]` preserves record slot order after schema validation. Each field carries
`key`, resolved `name`, resolved `type`, JSON `value`, optional `unit`, and
`enumLabel`. Id reference fields resolve to definition names, for example
`valueId -> value`, `stateMachineId -> stateMachine`, `conditionId -> condition`,
and `messageTemplateId -> messageTemplate`.

Unknown enum values are still data: the numeric value is serialized and `enumLabel`
is `null`.

Private extension slots are serialized as `extensionFields[]` only when they satisfy
the extension ordering rules. Raw payload hex is always preserved; typed `value` is
included only when decoding is safe.

## Provenance

Record-backed documents carry:

- `bufferId`
- `source`
- `runId`
- `epoch`
- `sourceTimeNs`
- `receiveTimeNs`
- `flags`

The source object always has an id and includes resolved name/path/hierarchy/dynamic
fields when the definition provides them.

## Loss Documents

A loss document contains `firstSeq`, `lastSeq`, `count`, and `basis`.

`basis: "authoritative"` means the range came from a producer-authoritative signal
such as `SourceHighWater` or `RecordsDropped`. `basis: "inferred"` means the range
was inferred from a sequence gap.

Loss documents do not invent `sourceTimeNs`; they carry receive-time provenance.

## Placeholders

A placeholder document carries `eventTypeId`, `seq`, `reason`, and `rawSlots[]`.
Raw slots preserve `key`, `type`, and `payloadHex`.

Placeholder reason kinds are:

- `missingCurrentDefinition`
- `stalePriorEpoch`
- `drift`
- `fullHashDrift`
- `unknownEventId`
- `schemaViolation`
- `invalidPayload`
- `hashError`

The resolver emits placeholders for missing or mismatched definitions, unknown event
ids, schema violations, invalid payloads, and hash failures. It does not silently
resolve data against the wrong definition.
