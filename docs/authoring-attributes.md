# OpenOT Logging Attributes — truST authoring reference

How an engineer turns an ordinary ST variable into OpenOT telemetry: tag its
declaration with an `{attribute 'oot' := …}` pragma and the truST compiler emits
the matching OpenOT record whenever the variable changes, and generates the
[definition file](#identifiers) (id→meaning, hashed) that resolves it. **No log
calls in the program; the engineer never writes an id.**

> Scope note. The OpenOT *standard* defines three contracts — carriage (wire),
> definition-file, document — and leaves the *production mechanism* out of scope.
> These `{attribute 'oot'}` annotations are **truST's authoring layer**; they
> lower to standard records + a generated definition file.

## Syntax

```iecst
Name : TYPE {attribute 'oot' := '<kind>', '<key>' := '<value>', ...} := <init>;
```

The pragma attaches to the `VAR` declaration so it survives rename/refactor.

## The four kinds — what each means

| `'oot' :=` | Use it for… | Emits | Fires when |
|---|---|---|---|
| `'value'` | a measured/process value — temperature, level, pressure, a count, a setpoint | `ValueChanged` (0x0002) | the value moves more than `deadband` |
| `'state'` | the operating state of equipment/a procedure — a phase, a mode | `StateTransition` (0x0001) | the enum changes value |
| `'alarm'` | an abnormal condition the operator must see — a high temp, a fault, a safety interlock | `ConditionActive` (0x0200) on trip / `ConditionCleared` (0x0201) on reset | the BOOL goes TRUE / FALSE |
| `'message'` | a human-readable event or diagnostic line | `Message` (0x0003) | the BOOL goes TRUE |

---

## `'value'` — a value changed

Logs the new value of a tag. The ST producer has byte-exact encoders for `BOOL`,
the signed/unsigned integer widths, `REAL`, `LREAL`, and bounded `STRING`. The
consumer reads the emitted value back typed.

| Key | Meaning |
|---|---|
| `unit` | The **engineering unit** of the value — `'L'`, `'degC'`, `'bar'`, `'rpm'`. It does not change the logging logic; it tells the consumer how to label and convert the number. Lives **only in the definition file**, resolved by `valueId` — the wire carries **no unit**, just the `valueId` and the typed value bytes. |
| `deadband` | **Noise filter.** The smallest change worth logging. `'0.5'` on a temperature means "only emit a `ValueChanged` once it has moved more than 0.5 °C since the last logged value." Without it, every scan's tiny fluctuation would flood the log. `REAL` only. This is the value's *sampling policy*. |
| `quality` | Optional OPC-UA-style quality code (`good`, `uncertain`, `bad`, `unknown`, or `0`..`3`). When present it emits the `quality` slot on each `ValueChanged`. |
| `semanticRole` | Optional value role (`actual`, `setpoint`, `command`, `count`, `position`, `status`, or `0`..`5`). This is definition-file metadata, not a wire slot. |
| `previous` | `true`/`false`. Defaults to `true`: once the producer has a previous sample, it emits `previousValue`. Set `false` when the consumer only needs the new value. |

```iecst
Level : REAL {attribute 'oot' := 'value', 'unit' := 'L', 'deadband' := '0.5'};
```

## `'state'` — a state machine moved

Logs a transition of an enum-typed state variable (`From → To`). The enum's
variant names are written into the definition file so the consumer sees
`Idle → Filling`, not `0 → 1`.

### `category` — *what kind of state this is*

| Value | Meaning | Comparable across vendors? |
|---|---|---|
| `process` | A **process/equipment state** — e.g. `Running`, `Stopped`, `Faulted`. The states are *your* enum, meaningful only within this machine. | No — machine-local; a consumer compares states only within this one declared machine. |
| `mode` | An **operating mode** — `Auto`, `Manual`, `Maintenance`. Also machine-local. | No — same as above. |
| `procedural` | A **procedural state from a named industry model** (see `model`). | **Yes** — the states are canonical, so `Held` means the same thing across every vendor that uses the model. |

Rule of thumb: equipment-specific steps → `process`; a standard batch/packaging
lifecycle → `procedural` + a `model`.

### `model` — *which canonical procedural state model* (only with `category := 'procedural'`)

| Value | What it is | Canonical states |
|---|---|---|
| `ISA-88` | The **batch process** procedural state model (ISA-88.00.01 / S88). For batch and sequential processes — reactors, chemical, pharma, food. Tells a consumer this machine follows the S88 batch lifecycle. | `Idle, Running, Complete, Pausing, Paused, Holding, Held, Restarting, Stopping, Stopped, Aborting, Aborted` |
| `PackML` | The **packaging-machine** state model (ISA-TR88.00.02 / OMAC PackML). The OEM standard for discrete/packaging machines — fillers, cartoners, palletizers — so a line integrator drives every machine the same way. | `Idle, Starting, Execute, Completing, Complete, Holding, Held, Unholding, Suspending, Suspended, Unsuspending, Stopping, Stopped, Aborting, Aborted, Clearing, Resetting` |

Picking a `model` is a promise that your enum's states map onto that model's
canonical states, so a consumer interprets them with no per-vendor knowledge.
If your states are *not* one of these standard sets (e.g. equipment-specific
steps like `Fill`/`Mix`), use `category := 'process'` instead — that's the honest
choice for machine-local states.

```iecst
Step : E_ReactorStep {attribute 'oot' := 'state', 'category' := 'process'} := Idle;
```

## `'alarm'` — a condition began or ended

Logs the **ISA-18.2-style condition lifecycle**: rising edge → `ConditionActive`
(the condition started), falling edge → `ConditionCleared` (it ended). The
producer mints a `correlationId` on the rising edge and echoes it on the clear, so
the two records can be paired across interleaving conditions.

### `class` — *alarm vs. interlock*

| Value | Meaning |
|---|---|
| `alarm` | An **alarm**: an abnormal condition that needs **operator awareness/action** — high level, over-temp, a fault. Goes into alarm management (acknowledge, shelve, priority). |
| `interlock` | An **interlock**: a **protective/permissive** condition that prevents or forces an action to keep equipment/people safe — "guard door open," "low-low pressure trip." Same begin/end contract, but it's a protection, not an operator notification. A consumer treats the two differently (alarm summary vs. interlock/permissive logic). |

### `severity` — *how urgent* (the OPC-UA 1–1000 scale)

A number from **1 (least urgent) to 1000 (most urgent)** on the OPC-UA severity
scale, with three bands a consumer can rely on without knowing your plant:

| Band | Range | Typical use |
|---|---|---|
| Low | 1–332 | informational / low-priority |
| Medium | 333–666 | needs attention |
| High | 667–1000 | urgent / critical — act now |

Default `800` (high). A plant's own priority scale is mapped *into* 1–1000 so
consumers need no per-vendor severity knowledge.

### `cause` — *which local operand explains the trip*

`cause` names a variable that contributed to the condition. The definition file
registers the operand name and the record carries the operand id as `causeOperand`.
This is the first bounded cause model: one named operand per condition.

```iecst
HighPhAlarm : BOOL {attribute 'oot' := 'alarm', 'class' := 'alarm', 'severity' := '900', 'cause' := 'Level'};
```

## `'message'` — a human-readable event

Logs a `Message` keyed by a template id. The template **text stays in the
definition file** and the wire carries the template id plus optional typed
arguments and severity.

| Key | Meaning |
|---|---|
| `template` | The message text, registered once in the definition file and resolved by the consumer. |
| `severity` | Optional OPC-UA 1–1000 severity slot on the message record. |
| `arg1`…`arg4` | Optional variable references whose typed values are emitted as positional `arg` slots. The definition file records each argument's TLV type in `argTypes[]`. |

```iecst
BatchStarted : BOOL {attribute 'oot' := 'message', 'template' := 'batch started'};
BatchLevel : BOOL {attribute 'oot' := 'message', 'template' := 'level count', 'arg1' := 'Level', 'arg2' := 'BatchCount'};
```

---

## Defaults when a key is omitted

The lowering fills these in when you leave a key off. A **semantic** key with a
default still changes the generated definition (and therefore the content hash), so
prefer to state them explicitly rather than rely on the default.

| Kind | Omitted key | Default | Note |
|---|---|---|---|
| `value` | `unit` | none | no unit on the wire or in the def file |
| `value` | `deadband` | none | every supported non-REAL type is on-change |
| `value` | `quality` | none | no quality slot emitted |
| `value` | `semanticRole` | `actual` | definition metadata |
| `value` | `previous` | `true` | emit `previousValue` after the first sample |
| `state` | `category` | `process` | the machine-local default, matching the VS Code action |
| `state` | `model` | none | **required** when `category := 'procedural'` (compile error otherwise) |
| `alarm` | `class` | `alarm` | the alternative is `interlock` |
| `alarm` | `severity` | `800` (high) | OPC-UA 1–1000 scale |
| `alarm` | `cause` | none | no cause operand slot |
| `message` | `template` | the **variable name** | an untemplated message still resolves to *something* |
| `message` | `severity` | none | no message severity slot |
| `message` | `arg1`…`arg4` | none | no typed argument slots |

> **`procedural` and `model` are paired.** `category := 'procedural'` **requires** a
> `model`, and `model` is only valid with `procedural` — both are compile errors
> otherwise. The default `process` (machine-local) needs neither, and matches what the
> VS Code "Add OpenOT logging" action inserts.

## Identifiers

Numeric ids are **auto-assigned by declaration order**. The counter increments before
assignment, so the **first** generated id is `value` 2001, `state` 7001, `alarm` 9001,
`message` 10001.

> ⚠️ **Stability caveat.** Because ids follow declaration order, inserting or reordering
> tagged variables shifts ids and changes the definition hash — which spec §6.3 warns
> against (archived records must resolve under a stable id). **Pin an id explicitly** with
> `'id'` (or the typed forms `'valueid'`, `'statemachineid'`, `'conditionid'`, `'sourceid'`,
> `'machineid'`) for deployments where retained records must stay resolvable. In this
> workbench those pinning keys are the stable contract; a future WG authoring profile may
> choose different spelling.

## What a consumer does with all this

The wire records are **id-only numbers**. A consumer joins them against the
generated definition file to recover meaning: `valueId 2001 → "Level" (REAL, "L")`,
`state 1 → "Filling"`, `severity 900 → High`, `model "ISA-88" →` the canonical
batch states. That's how `ValueChanged valueId=2001 new=15.25` becomes
**`Level = 15.25 L`** in a readable log.

## Current limitations (honest status)

- **Strict validation.** Unknown `oot` kinds, keys, enumerated values
  (`category`, `class`, `model`), invalid severity ranges, invalid
  `model`/`category` combinations, unsupported value types, non-REAL
  deadbands, and procedural enum states outside the named model are compile errors.
- **Kinds covered:** `value`, `state`, `alarm`, `message` only. Batch/recipe,
  operator/regulated, and the full condition lifecycle (ack/shelve/suppress) are
  not yet exposed as attributes.
- **Deadband/sampling:** only `REAL` deadband and on-change sampling are
  implemented. Integer deadband, periodic, and hysteresis policies are not yet
  implemented.
- **Message templates:** typed `arg1`…`arg4` values are emitted, but literal `{n}`
  braces inside an IEC pragma string are not portable through the current parser;
  use template prose and positional arg definitions until pragma escaping is
  settled.
- **Alarm lifecycle:** active/cleared records are correlated; acknowledge, shelve,
  suppress, out-of-service, latch/return-to-normal are not yet implemented.
- **Ids are order-assigned** — see the stability caveat.

## See also

- [`carriage-contract.md`](carriage-contract.md) — the wire format these lower to.
- `examples/reactor/openot-definition.json` — a generated definition file.
- `examples/reactor/Reactor.st` — a worked program using all four kinds.
