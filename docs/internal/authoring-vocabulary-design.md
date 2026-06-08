# Design — truST authoring front-door for the spec-defined vocabulary + sampling

> **Status:** design spec (lead-authored) for Codex review, then implementation. Covers the
> genuinely-open items from the execution plan: V3 sampling **behavior**, and the **authoring
> mechanism** for the condition lifecycle, batch/recipe, and operator/regulated/e-signature events.

## Why this doc exists (and what it is *not*)

The OpenOT draft defines the **meaning and carriage** of these events — lifecycle, batch/recipe,
operator/regulated event families, their field keys/types, and their enum sets. The production
surface is intentionally vendor/toolchain-specific. So the wire encoders + definition tables + byte
vectors for these events are **spec-defined, mechanical work** (the ids/keys are already reserved in
`EVENT_SPECS` by Phase 0). This doc designs the thing the spec deliberately leaves to the vendor:
**how an ST program expresses these events via `{attribute 'oot'}`** — truST's production mechanism —
plus the **producer sampling behavior** (also production, outside the wire contract).

**Discipline this honors:** the event slot schemas are **frozen** (Phase 0, `EVENT_SPECS`). This
design should add mostly **authoring** — new `OotKind` variants, HIR keys, lowering, and
`docs/authoring-attributes.md` text — and emit records into the already-reserved slot schemas.
Definition content hashes may still change for programs that adopt new declarations/events; the
point is to avoid re-cutting the canonical event-schema shape after Phase 0. Every new authoring
slice is `[LOCKSTEP]` (it touches `docs/authoring-attributes.md`, the open-ot-ref vocabulary source
of truth).

---

## 0. The unifying pattern: triggered event attributes with field bindings

Today's four kinds (`value`/`state`/`alarm`/`message`) tag a variable whose *own value* is the datum.
The new events differ: they fire on a **trigger** and carry **several fields sourced from other
variables**. So this design adds one pattern, reused by every new kind:

- **Trigger.** The tagged variable is the trigger. A `BOOL` fires the event on its **rising edge**; an
  **enum** fires on **value change** (used by `BatchEvent`).
- **Field bindings.** Attribute keys bind the event's spec fields to **either a program variable
  (resolved by name, read at emit — exactly as `'cause' := 'Level'` already works) or a literal**.
  Notation in this doc: `'key' := VarName` (bareword = variable) vs `'key' := 'literal'` (quoted =
  constant). *(Codex: confirm the pragma parser can carry a bareword variable reference; if only
  quoted strings are accepted, use `'key' := 'VarName'` and resolve names that match an in-scope
  declaration — same approach as `cause`.)*
- **Parent reference.** Lifecycle events name their parent condition with `'of' := ConditionVarName`;
  the lowering resolves it to that condition's `conditionId` (and, for activation-scoped events, its
  live `correlationId`).
- **Type checking.** Each bound field is validated against its fixed TLV type (§6.2.1) at compile
  time: e.g. `ackBy`/`actor`/`reason`/`comment` → `String`; `shelveSecs`/`recipeId`/`batchId`/
  `materialId`/`actionId`/`intervalMs` → `UDInt`; `quantity` → `LReal`; `signedEventSeq` → `ULInt`;
  enum fields (`signatureMeaning`, `authResult`, `batchState`) → `UInt` with the §6.4 value set.

This pattern keeps the surface small and consistent, and maps 1:1 onto the spec's event field lists.

---

## 1. Sampling behavior (V3) — extends the `value` kind

Spec direction: deadband/sampling policy is declared per `valueId`; `intervalMs` also exists as a
registered field key (`0x002A`, UDInt) for event slots. The **declaration** belongs in the definition
file; the **behavior** is production → designed here. Adds keys to `value`:

| Key | Values | Meaning |
|---|---|---|
| `sampling` | `on-change` \| `deadband` \| `periodic` \| `hysteresis` | default = `deadband` if a `deadband` key is present, else `on-change`. |
| `deadband` | number | existing; the band for `deadband` and `hysteresis`. |
| `interval` | ms (integer) | for `periodic`; written to the def file as periodic sampling metadata. |

**Producer behavior** (all per-`valueId` state in the producer FB):
- `on-change` — emit on any change (today's int/bool/string path).
- `deadband` — emit when `|new − last| > deadband` (today's REAL path; now also valid for the integer
  widths and `LREAL`, using scaled-integer compare for ints per the existing `Deadband.scaled`).
- `periodic` — emit when `SourceTime − lastEmitSourceTime ≥ interval` **or** the value changed. truST
  has no wall-clock but the runtime injects `SourceTime` each scan (D13), so periodic uses that, no new
  builtin. New per-value state: `lastEmitSourceTime`.
- `hysteresis` — a two-sided deadband with band memory: after emitting at value `v`, suppress until
  the value rises above `v + deadband` or falls below `v − deadband`; emit and re-center on the cross.
  New per-value state: the current band center. (This is the honest "deadband with anti-chatter"; if a
  later need arises for asymmetric high/low thresholds, add `'high'`/`'low'` keys — out of this slice.)

**No wire record-shape change.** Definition metadata still needs one explicit implementation choice:
today `ValueDefinition` has `samplingPolicy` and `deadband`, but no separate interval field. Either add
an optional value-definition interval metadata field before implementing `periodic`, or encode the
interval in a constrained `samplingPolicy` form. Do **not** add `intervalMs` to `ValueChanged` records;
sampling policy controls when to emit, not what the datum is. Vectors: add a periodic
emit-vs-suppress pair and a hysteresis cross pair.

---

## 2. Condition lifecycle authoring — new kind `condition`

Parent is an existing `alarm`. Operator/logic commands (ack, shelve, suppress, …) arrive as program
`BOOL`s (typically HMI-written); each is tagged to reference its parent condition. On the rising edge,
the lowering emits the matching §7.3 event.

```iecst
HighPhAlarm  : BOOL {attribute 'oot' := 'alarm', 'class' := 'alarm', 'severity' := '900'};

AckHighPh    : BOOL {attribute 'oot' := 'condition', 'of' := HighPhAlarm, 'event' := 'acknowledge', 'by' := OperatorName};
ShelveHighPh : BOOL {attribute 'oot' := 'condition', 'of' := HighPhAlarm, 'event' := 'shelve', 'seconds' := ShelveSecs, 'by' := OperatorName};
SuppressHighPh : BOOL {attribute 'oot' := 'condition', 'of' := HighPhAlarm, 'event' := 'suppress', 'reason' := SuppressReason};
OosHighPh    : BOOL {attribute 'oot' := 'condition', 'of' := HighPhAlarm, 'event' := 'out-of-service'};
```

**`event` → event id**, with the §7.3 scope rule baked in:

| `event` | id | scope → correlationId |
|---|---|---|
| `acknowledge` | 0x0202 | activation → carries parent's live `correlationId` |
| `confirm` | 0x0203 | activation |
| `shelve` | 0x0204 | activation |
| `unshelve` | 0x0205 | activation |
| `comment` | 0x020A | activation (requires `comment`) |
| `reset` | 0x020B | activation |
| `suppress` | 0x0206 | **condition** → no correlationId, keyed by `conditionId` |
| `unsuppress` | 0x0207 | condition |
| `out-of-service` | 0x0208 | condition |
| `in-service` | 0x0209 | condition |
| `priority-changed` | 0x020C | condition (requires `priority`, `previous-priority`) |

**correlationId handling**: the producer already stores the live `correlationId` per condition (set
on `ConditionActive`, P2-c). Activation-scoped lifecycle events read that stored id; condition-scoped
events omit it. An activation-scoped event fired while the condition is **not active** is
compile-time-undetectable, so the implementation must define the runtime policy before coding.
Recommended first policy: suppress the activation-scoped lifecycle record when no live correlation id
exists, and surface that in diagnostics/tests; do not fabricate a stale correlation id.

**Field bindings** → spec fields: `by`→`ackBy`(String), `seconds`→`shelveSecs`(UDInt),
`reason`→`reason`(String), `comment`→`comment`(String), `priority`/`previous-priority`→`newPriority`/
`previousPriority`(UInt). Validated per §6.2.1.

*(Out of this design: `RefreshStart`/`RefreshEnd` (0x020D/E) are a consumer-reconnect stream-control
concern, not program-authored — left to the consumer/runtime, noted in limitations.)*

---

## 3. Batch / recipe authoring — kinds `batch`, `recipe-loaded`, `recipe-approved`, `material-addition`

`BatchEvent` is enum-triggered (like `state`); the rest are edge-triggered with bindings.

```iecst
(* fires BatchEvent on every batchState change *)
BatchState : E_BatchState {attribute 'oot' := 'batch', 'batchId' := CurrentBatchId, 'recipe' := CurrentRecipeId};

RecipeLoadedTrig   : BOOL {attribute 'oot' := 'recipe-loaded', 'recipe' := RecipeId, 'version' := RecipeVer, 'batch' := BatchId};
RecipeApprovedTrig : BOOL {attribute 'oot' := 'recipe-approved', 'recipe' := RecipeId, 'version' := RecipeVer, 'by' := ApproverName};
MaterialAddTrig    : BOOL {attribute 'oot' := 'material-addition', 'batch' := BatchId, 'material' := MaterialId, 'quantity' := Qty, 'unit' := 'kg'};
```

- `batch` → `0x0303 BatchEvent {batchId, newState, [recipeId?]}`. The tagged enum's variants **must map
  to the §6.4 `batchState` set** (0 Started · 1 Completed · 2 Held · 3 Resumed · 4 Aborted · 5 Paused)
  — validated the same way `category:=procedural` validates against a model's canonical set (C1
  machinery). `newState` = the new variant; `batchId`/`recipe` bound.
- `recipe-loaded` → `0x0301 {recipeId, recipeVersion, [batchId?], [effectiveTime?]}`.
- `recipe-approved` → `0x0302 {recipeId, recipeVersion, [authResult?], [ackBy?]}` (`by`→`ackBy`).
- `material-addition` → `0x0304 {batchId, materialId, quantity, [unit?], [correctionOf?]}`.

Bindings → spec fields: `recipe`→`recipeId`(UDInt), `version`→`recipeVersion`(String),
`batch`→`batchId`(UDInt), `material`→`materialId`(UDInt), `quantity`→`quantity`(LReal),
`unit`→`unit`(UInt unitId, via the C2 registry).

**Definition side:** `recipeDefinitions[]`/`batchDefinitions[]`/`materialDefinitions[]` (the empty
P0-b tables) get **populated** — recipeId→name, batchId→name, materialId→name — from a small set of
authoring keys or a side declaration. *(Codex review question: do we want named recipe/batch/material
tables authored now, or resolve by raw id first and populate names later? The tables are reserved
either way — populating is content, not schema. Recommend: resolve by raw id in this slice, populate
names in a follow-up, to keep the slice bounded.)*

---

## 4. Operator / regulated / e-signature authoring — kinds `operator-action`, `operator-login`, `operator-logout`, `parameter-change`, `e-signature`, `security-failure`

Edge-triggered with bindings; this is the Producer-Audit surface (§7.5, §11.4).

```iecst
OpAction   : BOOL {attribute 'oot' := 'operator-action', 'action' := ActionId, 'actor' := OperatorName, 'workstation' := Ws};
Login      : BOOL {attribute 'oot' := 'operator-login', 'actor' := OperatorName, 'auth' := AuthResult, 'role' := Role};
Logout     : BOOL {attribute 'oot' := 'operator-logout', 'actor' := OperatorName};
ESign      : BOOL {attribute 'oot' := 'e-signature', 'action' := ActionId, 'actor' := Signer, 'meaning' := 'approved', 'signed-event' := AttestedSeq};
SecFail    : BOOL {attribute 'oot' := 'security-failure', 'actor' := WhoTried, 'reason' := DenyReason};
```

- `operator-action` → `0x0400 {actionId, actor, [contextRef?]*, [authResult?], [workstation?]}`.
- `operator-login` → `0x0401 {actor, authResult, [workstation?], [role?]}`; `operator-logout` → `0x0402`.
- `e-signature` → `0x0404 {actionId, actor, signatureMeaning, signedEventSeq, [authResult?]}`. `meaning`
  → `signatureMeaning` UInt enum (§6.4: 0 Authored..5 Witnessed). `signed-event` → `signedEventSeq`
  (ULInt) = the envelope `seq` of the event this signature attests to, supplied by the program (a
  variable that captured it). This is the heaviest binding — call it out for review.
- `security-failure` → `0x0405 {actor, [workstation?], [reason?]}`.

**`parameter-change` (0x0403)** is special — it is *audited `ValueChanged`*: `{valueId, previousValue,
newValue, actor, reason, [authResult?]}`. Design it as a **facet on the existing `value` kind**, not a
trigger: a value tagged `'audit' := 'true'` with `'actor'`/`'reason'` bindings emits `ParameterChange`
(carrying its own before/after) instead of `ValueChanged` when those are present. `reason` is REQUIRED
at the Audit profile (§11.4) — enforce it.

`ProgramDownload`/`DefinitionChanged` (0x0406/0x0106) are epoch-protocol records the runtime emits on
a definition change (§9.3), **not** program-authored — left to the runtime, noted in limitations.

---

## 5. Multi-FB (S2) — still an integration decision

The carriage is single-writer by construction, but the current truST authoring path still hardcodes a
single hidden producer instance (`GENERATED_PRODUCER_NAME`). S2 is therefore not closed by vocabulary
work alone. The safe options to decide before coding are:

- one hidden `OPENOT_Producer` + one shared-memory buffer/BCB per writer scope, with the consumer
  merging by `(source, seq)` across buffers; or
- one explicit fan-in owner that serializes records from many generated producers into one buffer.

The first option is cleaner for the single-writer carriage contract, but it changes runtime
configuration/discovery more than today's one-buffer capstone. Keep this as a separate `truST`
integration slice with a clear source-ownership and buffer-routing decision.

---

## 6. What stays out (honest limitations)

- `RefreshStart`/`RefreshEnd` (consumer-reconnect replay) — consumer/runtime concern, not authored.
- `ProgramDownload`/`DefinitionChanged`/`Heartbeat`/`LoggerStarted/Stopped` — runtime epoch/liveness
  records, not program-authored.
- Named recipe/batch/material **tables** — reserved (empty) in P0-b; populating names is a bounded
  follow-up (see §3 review question).
- `contextRef`/`correctionOf` repeatable/linkage fields — supported by binding but exercised minimally
  in the first slice.

## 7. Implementation shape (for the slices that follow this design)

Each area is `[LOCKSTEP]` and lands as: **new `OotKind` variant(s)** (`trust-hir openot_authoring.rs`)
+ **HIR vocabulary/validation** (keys, the `event`/`meaning`/enum value sets, the per-field §6.2.1 type
check, the `of`/`batch`-enum reference resolution) + **lowering** (read bindings, resolve parent
condition/correlation, emit the already-defined event into its frozen slot schema) + **the spec-defined
wire encoder** (ST FB per event family, if not already present) + **vectors** + **`authoring-attributes.md`**
+ an **`openot_telemetry`** round-trip assert. Canonical event-schema fixtures should not move after
Phase 0; generated program definitions move only when their actual content adopts new metadata or
event families.

Suggested order (smallest/most-isolated first): **V3 sampling** → **condition lifecycle** →
**batch/recipe** → **operator/e-signature**.
