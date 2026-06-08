# OpenOT Reference — Completeness Backlog (internal)

**Status: internal working note, not public documentation.** The public docs
(`overview.md`, `authoring-attributes.md`, `definition-file.md`, …) describe the
implemented subset honestly and exhaustively. *This* file is the complement: the
delta between that subset and a **complete** OpenOT implementation, written for the
people building it (the lead + Codex), not for end users.

Read it together with the existing forward-looking notes — this consolidates and
extends them, it does not replace them:

- [`roadmap.md`](../roadmap.md) — the phased plan and "Next" follow-ups.
- [`decisions.md`](../decisions.md) — *why* each built thing is the way it is, plus
  the "Open items / known divergences" list.
- [`spec-feedback.md`](../spec-feedback.md) — implementation→proposal wire deltas.
- [`authoring-attributes.md`](../authoring-attributes.md) §"Current limitations".

## How to read each item

Every item carries:

- **Now** — what the implementation actually does today (verified against the live
  tree / generated `examples/reactor` artifacts, not against intent).
- **Complete** — what "done" looks like.
- **Gate** — what unblocks it:
  - `engineering` — buildable now; no external dependency. Safe to do.
  - `truST` — needs a change in the sibling **trust-platform** runtime/compiler.
  - `WG` — **blocked on working-group ratification.** Numeric ids, enum values, the
    full event vocabulary, and the wire/BCB reconciliation all depend on a ballot.
    Building these now hardens choices the ballot may overturn — so they are
    *deliberately* deferred. See the standing decision below.

### Decision — superseded 2026-06-07: build the full surface as final

**Original stance (kept for context):** we initially decided **not** to implement the
full unratified vocabulary or freeze provisional ids just to look "complete" — a
reference that races ahead of the ballot manufactures churn.

**Superseded (2026-06-07):** the decision was reversed — **build everything as final
now**, including the WG-gated vocabulary/ids/registry, per
[`execution-plan.md`](execution-plan.md) and [`decisions.md` D16](../decisions.md). The
strawman ids are promoted to final. **Accepted risk:** if a later WG ballot renumbers
anything, it re-cuts the byte vectors and the definition content-hash; the plan pays
that churn once via a Phase-0 schema freeze (definition fields **and** every event's
slot schema), not per slice. The `WG` gate labels below now read as "implemented against
assigned-final ids, pending ballot confirmation," not "deferred."

---

## 1. Value layer

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| V1 | Value types | Byte-exact ST encoders and truST lowering cover `BOOL`, signed/unsigned integer widths, `REAL`, `LREAL`, and bounded `STRING`; consumer/schema validation accepts the datum's TLV type for value-bearing slots. | Keep coverage aligned with any future WG-added value encodings. | implemented against assigned-final TLV set |
| V2 | `previousValue` / `quality` | `quality`, `semanticRole`, and `previous` are author-controlled. `previous := 'false'` suppresses previous-value emission; quality emits the `quality` slot. | Keep the keys aligned with any future WG vocabulary changes. | implemented against assigned-final keys |
| V3 | Deadband / sampling | Deadband honored for `REAL` only, strict `>` comparator; every other supported value type is on-change. Generated definitions can describe the current `on-change` / deadband floor, but not periodic or hysteresis behavior. | Integer/scaled deadband, declared sampling policies (periodic, on-change, deadband/hysteresis) per type. | `truST` |

> P0-0 resolved the record-size floor: generated definitions declare
> `maxRecordSize = 256`, matching the IEC producer's staging buffer. Bounded
> `STRING[96]` payloads keep the implemented record shapes below that cap.

## 2. Message layer

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| M1 | Typed arguments | `arg1`…`arg4` variable references populate `messageTemplates[].argTypes` and emit typed `arg` slots on the wire. Literal `{n}` placeholder parsing/escaping remains deferred. | Portable IEC pragma escaping for literal placeholder braces, if the WG requires it. | partially implemented; placeholder parsing deferred |
| M2 | Severity on messages | Optional message `severity` emits a `severity` slot and resolves through the baseline scale. **Message *category* is deferred** — `KEY_CATEGORY` is state-specific and `MessageTemplateDefinition` has no category field; it needs a new key/enum, not in this plan's scope. | Keep severity aligned with the registry; decide message category only if a use case appears. | implemented for severity |

## 3. Alarm / condition layer

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| A1 | Correlation id | `ConditionActive`/`ConditionCleared` carry a `correlationId` minted on the rising edge and echoed on clear. | Keep correlation semantics through future lifecycle records. | implemented |
| A2 | Full ISA-18.2 lifecycle | Active / Cleared only. | Acknowledge, shelve, suppress, out-of-service, latch/return-to-normal — as attributes + records. | `WG` (vocabulary) |
| A3 | Cause operands | One named `cause` operand is registered in `conditions[].causeOperands[]` and emitted as a `causeOperand` slot. Full expression capture is deferred. | Multi-operand/expression capture if required by the WG. | partially implemented; expression capture deferred |

## 4. Event vocabulary

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| E1 | Batch / recipe events | Not exposed. | Batch start/end, phase, recipe-step events as attributes. | `WG` |
| E2 | Operator / regulated events | Not exposed. | Operator action, e-signature / 21 CFR Part 11 audit-trail events. | `WG` |
| E3 | Vocabulary ids | `registry.rs` carries the assigned OpenOT reference catalog for base, system, condition lifecycle, procedural/batch, regulated/operator, and core key ranges; code treats these ids as final inside this workbench. | WG ballot confirmation or renumbering feedback. | implemented against assigned-final ids, pending ballot confirmation |

## 5. Model & semantic conformance

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| C1 | Model/state conformance | Procedural models are checked against canonical state sets in the definition validator and as a truST compile diagnostic. | Keep the check aligned with any future WG ratified state-set changes. | implemented against assigned-final sets, pending ballot confirmation |
| C2 | Unit registry | `unit` strings are validated against the reference unit registry and emitted as canonical `unitId`s. | Keep the registry aligned with the WG table. | implemented against assigned-final table |
| C3 | Category/model validation | Known enum values, valid combinations, procedural model state sets, and unit registry checks are enforced. | Keep checks aligned with future WG registry changes. | implemented against assigned-final tables |

## 6. Sources & identity

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| S1 | Source naming | Source definitions are derived from source-file stem + `PROGRAM` name (for example `Reactor.Main`, path `["Reactor", "Main"]`, hierarchy `["file", "program"]`) instead of `source1`. | Optional plant/equipment binding (`Reactor/R201`, ISA-95 levels) if the WG wants more than compiler-derived identity. | partially implemented; equipment binding deferred |
| S2 | Multi-source / multi-FB | The carriage and ST producer have per-source sequence/high-water machinery, and the producer can register up to 16 source ids. The truST authoring example/runtime gate still exercises one generated producer instance. | Many OpenOT producer FB instances into one shared ring with correct per-source seq/high-water at scale. | `engineering` |

## 7. ID stability & def-file hashing

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| I-D1 | Order-assigned ids | Ids follow declaration order (first `value` 2001, `state` 7001, `alarm` 9001, `message` 10001); reordering shifts ids → def-hash drift. | **Chosen strategy (2026-06-07): manual author/tool pinning** via the pin keys (`id`/`valueid`/…) is the normative stable contract — pin ids for records that must stay resolvable across edits (per spec §6.3). Content-derived / persisted-map auto-stability is a future enhancement. | `truST` (P0-d) |

## 8. Time

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| T1 | Wall-clock source | `SourceTime` is host-injected Unix-ns each scan, and truST also exposes `CURRENT_DT()` as a wall-clock `DATE_AND_TIME` builtin. | Keep hosted and hardware time sources documented as platform glue. | implemented |

## 9. Lowering mechanism (truST)

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| L1 | Authoring lowering | A source-instrumentation / HIR pass (`trust-runtime/src/openot_authoring.rs`) rewrites tagged declarations into producer calls + a generated def file; `decisions.md` D17 records this as the production path for the supported subset. | Native backend lowering remains out of scope unless it preserves the same bytes and hashes. | implemented for supported subset |

## 10. Impl ↔ proposal reconciliation (all `WG`)

These are the deltas in [`spec-feedback.md`](../spec-feedback.md) and
[`decisions.md` D5](../decisions.md). The implementation is the **ARM-validated**
one, so the intent is to update the *proposal draft* to match the impl — but nothing
is reconciled until the WG accepts it.

| # | Item | Impl | Proposal draft |
|---|---|---|---|
| R1 | Control block size | **88** bytes | 80 bytes |
| R2 | Record header | **40**-byte (per-record `RunId`) | 32-byte |
| R3 | Overwrite check space | **absolute byte offset** (`OldestAbs > record_start_abs`) | seq-space (`OldestSeq`) |
| R4 | `SourceHighWater` id | core system event **`0x0108`** | feed the core allocation back to the WG |
| R5 | Publish ordering normativity | Release/Acquire fences are load-bearing and ARM-proven | make conformance-testable in the text |
| R6 | Upstreaming | repo-local `spec/core.md`, `spec/definition-file.md`, and `spec/doc-format.md` now draft the implemented contracts | feed these drafts back to the WG |

## 11. Conformance evidence

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| K1 | Vector breadth | Byte-exact vectors cover the current implemented record surface: the full V1 value matrix, `Message` template/arg/severity, `ConditionActive`/`ConditionCleared` correlation and cause operand, lifecycle/high-water, records-dropped, epoch, capture, and schema-negative cases. | Add vectors as the remaining A2/E1/E2/V3 families land; keep epoch/burst/loss negatives aligned. | implemented for current surface; future event families pending |
| K2 | Unfenced weak-memory hole | Documented **non-reproduction** on ARM (the hole isn't reliably forced; correctness rests on the fences). | **Either** a litmus that reliably exposes the unfenced bug **or** — if no available platform reproduces it — a strengthened negative-proof harness (N-iteration stress + the loom/fence-hook proof) documented as the closure. Forcing the bug may be infeasible on the available ARM, so this can land as *strengthened evidence*, not necessarily a positive repro. | `engineering` (P5-b) |
| K3 | Model-conformance fixtures | Definition validation, truST diagnostics, and committed positive/negative definition fixtures reject procedural model/state mismatches at the fixture layer. | Closed for the current model-conformance contract; keep the fixture paired with any future procedural-model schema changes. | implemented |

## 12. Transport & productization (out of current scope)

Explicitly **not** in scope for the reference (see [`CONTRIBUTING.md`](../../CONTRIBUTING.md)),
listed so "complete product" ≠ "complete reference" stays explicit:

- Network transport *above* the buffer (OPC UA / MQTT / REST).
- MES / historian integration; definition-file hot-reload on warm-epoch change,
  end to end.
- A productized, supported runtime.

## 13. Recently resolved (landed in the current WIP — verified)

| # | Item | Resolution |
|---|---|---|
| I1 | `maxRecordSize` emitted **512** while the producer caps at the **256**-byte staging ring | Generator, reactor artifacts, and `model.rs::sample_definition()` now declare **256**, matching the ST producer cap. |
| C1 | Model/state conformance | Definition validation and truST diagnostics reject procedural enum states that are not in the named model's canonical set. |
| I2 | Lossy value types silently coerced to `DINT` | Value attributes are accepted only for encodable types (`BOOL`, integer widths, `REAL`, `LREAL`, bounded `STRING`); no silent coerce. |
| I3 | LSP offered value-logging on types the encoder couldn't emit | LSP classifies via `is_supported_value_type_id`; it now offers only the encodable value matrix. |
| L2 | `state` `category` default mismatch | Runtime now defaults an omitted `category` to **`process`** (`STATE_CATEGORY_PROCESS`), matching the LSP; and `category := 'procedural'` now **requires** a `model` (compile error otherwise), closing the model-less-procedural hole. |
| V1 | Wide value types | ST/truST now emit `BOOL`, integer widths, `REAL`, `LREAL`, and bounded `STRING` via value-bearing TLV slots; the schema validator checks `newValue`/`previousValue` against the referenced value's declared `dataType`. |
| V2 | Value metadata/control | `quality`, `semanticRole`, and `previous` are author-controlled and reflected in the definition/record bytes. |
| M1/M2 | Message metadata | `Message` records now carry `messageTemplateId`, optional typed `arg1`…`arg4` values, and optional severity; the definition stores `argTypes[]`. |
| A1/A3 | Condition correlation and cause | Active/cleared records carry a minted/echoed `correlationId`; a bounded single `cause` operand is defined and emitted. |
| C2/C3 | Unit and semantic validation | Unit symbols use canonical ids; category/model combinations and procedural enum state sets are validated. |
| S1 | Source metadata floor | Generated definitions derive source name/path/hierarchy from file stem + `PROGRAM` name instead of `source{id}`. Plant/equipment bindings remain a future extension. |
| T1 | Wall-clock builtin | truST exposes `CURRENT_DT()` in addition to host-injected Unix-ns `SourceTime`. |
| L1 | Lowering mechanism | `decisions.md` D17 settles the source-instrumentation/HIR pass as the production lowering path for its supported subset. |
| R6 | Spec drafts | `spec/core.md`, `spec/definition-file.md`, and `spec/doc-format.md` exist as WG-facing drafts of the implemented contracts. |

## 14. Suggested sequencing

**Superseded by [`execution-plan.md`](execution-plan.md).** Since D16 (build all as final),
the sequencing is the execution plan's: **Phase 0 first — the record-size audit + the definition
schema/slot/table freeze — before any feature work**, then values → messages/conditions → vocabulary
→ conformance/sources → time/proof → upstream. The earlier "V1 → M1 → A1, then defer the WG surface"
order is **no longer correct** (it predates the schema-freeze discipline and the build-all-final
decision).

*(Historical: the correctness fixes I1–I3 and the `category` default L2 are done — §13.)*
