# Definition File

The **id→meaning registry** — the second of OpenOT's three contracts (carriage → **definition** →
document). Wire records are id-only numbers; this file says what those numbers mean, and a content
hash binds a record stream to exactly one definition. In truST it is a **compiler artifact**:
generated from the `{attribute 'oot'}` annotations, not hand-maintained (see
[`decisions.md` D11–D12](decisions.md)). Upstream `spec/definition-file.md` is empty; this documents
the format this workspace generates and resolves.

Worked example: [`examples/reactor/openot-definition.json`](../examples/reactor/openot-definition.json).
Canonical JSON model lives in `crates/definition`.

## Top-level structure

```jsonc
{
  "header":          { … trust tuple, profiles, caps, constraints, contentHash … },
  "eventTypes":      [ … id → name + slot schema … ],
  "values":          [ … valueId → name, data type, unit, deadband … ],
  "sources":         [ … sourceId → path / name … ],
  "stateMachines":   [ … stateMachineId → category, model, enumSet … ],
  "conditions":      [ … conditionId → class, default severity … ],
  "enumSets":        [ … { name, members[] of { value, label } } … ],
  "units":           [ … unitId → symbol … ],
  "messageTemplates":[ … templateId → format text + arg types … ],
  "severityScale":   { … band thresholds … }
}
```

## Sections

**`header`** — the trust tuple a consumer checks before trusting a record:
`wireVersion` (2), `semanticVersion`, `profiles[]` + `conformanceLevel` (machine-readable
declaration, e.g. `Producer-Full`), `caps` (`crc`, `sourceHighWater`), `constraints`
(`maxRecordSize`, `maxSlots`, `overflowPolicy`), `epochStrategy` (`retain`|`clear`), and
**`contentHash`** (see below).

**`eventTypes[]`** — `id → { name, profile, slots[] }`. Each slot is `{ key, minOccurs, maxOccurs,
orderClass }` plus **either** a fixed `type` **or** `valuePayload: true` for a value-bearing slot
(whose runtime type is carried in the record itself, not fixed by the schema) — the schema a consumer
validates each record against (canonical order by `orderClass`). E.g. `2 → "ValueChanged"` with slots
`valueId` (fixed type) / `previousValue?` (`valuePayload`) / `newValue` (`valuePayload`) / `quality?`.

**`values[]`** — `valueId → { name, dataType, unit, deadband, samplingPolicy, semanticRole }`.
This is what turns `valueId 2001` into `"Level" (REAL, unit "L", deadband 0.5)`. `unit` is a *unit id*
into `units[]`.

**`sources[]`** — `sourceId → { name, path[], hierarchy[], dynamic }`. Resolves the emitting entity.
The current authoring layer auto-names sources `source1`, `source2`, … (`sourceId 1 → "source1"`); a
richer hierarchy/path is reserved in the schema but not yet derived from the program. System source 0
is reserved.

**`stateMachines[]`** — `stateMachineId → { name, category, proceduralModel, enumSet }`. Names the
machine and points at the `enumSets[]` entry that holds its states.

**`conditions[]`** — `conditionId → { name, conditionClass, defaultSeverity, causeOperands[] }`.
Resolves an alarm/interlock (`9001 → "HighPhAlarm", Alarm, sev 900`).

**`enumSets[]`** — `{ name, members[] }`, each member `{ value, label }` (e.g.
`E_ReactorStep → [{0, "Idle"}, {1, "Filling"}, {2, "Mixing"}, …]`), so a `StateTransition` resolves
`new=1` to `"Filling"`.

**`units[]`** — `unitId → symbol` (`1 → "L"`). **`messageTemplates[]`** — `templateId → { name,
format, argTypes[] }`; the template **text lives here**, never on the wire. **`severityScale`** —
the band thresholds (baseline OPC-UA: low 1–332, medium 333–666, high 667–1000).

## Content hash (drift binding)

- **Canonical bytes:** RFC 8785 (JCS) — UTF-8, sorted keys, shortest round-trip numbers, no
  insignificant whitespace.
- **Hash:** SHA-256 over the JCS bytes with `header.contentHash` set to `""` during computation
  (self-exclusion), stored as lowercase hex.
- **Binding:** the shared-memory control block carries the first 8 bytes of the digest (digest
  order). A consumer compares those 8 bytes on connect and verifies the full hex when it has the
  file; on mismatch it placeholder-resolves and surfaces the drift. Proves *drift*, not tamper-evidence.

## Generation & stability

The compiler derives entries from the attributes: value/state/condition **names from the variable
names**, `unit`/`deadband`/`category`/`model`/`severity` from the attribute keys, enum members from
the ST enum type. **Ids are auto-assigned by declaration order** — the counter increments before
assignment, so the **first** generated id is value `2001`, state `7001`, alarm `9001`, message
`10001` — see the stability caveat in [`authoring-attributes.md`](authoring-attributes.md):
reordering tagged variables shifts ids and changes the hash, so pin ids for deployments that retain
records.
