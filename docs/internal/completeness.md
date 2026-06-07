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

### Standing decision — do not pre-build the unratified surface

We decided (and re-confirmed) **not** to implement the full unratified vocabulary or
freeze provisional ids just to look "complete." The reference exists to give the WG
*evidence*, and a reference that races ahead of the ballot manufactures churn. So:
fix real bugs, keep the docs honest, and keep `WG`-gated items as a backlog — not as
half-built code. The buildable-now work is the `engineering`/`truST` rows.

---

## 1. Value layer

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| V1 | Value types | Byte-exact ST encoders for **`REAL`** and **`DINT`** only; every other type is a compile error (`InvalidOpenOtAttribute`). | Encoders + Rust decoders + `valuePayload` type tags + LSP offering for `LREAL`, the integer widths (`SINT/INT/LINT` + unsigned), `BOOL`-as-value, and `STRING`. | `engineering` (+`truST` for LSP) |
| V2 | `previousValue` / `quality` | `newValue` always emitted; `previousValue` slot optional and not author-driven; `quality` slot fixed. | Author-controllable `quality` (OPC-UA quality), optional `previousValue` capture, and `semanticRole` from the attribute. | `truST` |
| V3 | Deadband / sampling | Deadband honored for `REAL` only, strict `>` comparator; `DINT` is on-change. | Integer/scaled deadband, declared sampling policies (periodic, on-change, deadband) per type. | `truST` |

> The 256-byte ST staging ring (see in-flight I1) bounds single-record payload size;
> wider value types and multi-slot records interact with it — revisit the ring size
> when V1 lands.

## 2. Message layer

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| M1 | Typed arguments | Template-id only on the wire; `argTypes[]` always empty; template text is static. | `{n}` placeholder parsing, `argTypes[]` population from the call site, typed args encoded on the wire, consumer-side formatting. | `truST` + `engineering` (wire slots) |
| M2 | Severity / category on messages | Plain `Message` (0x0003). | Optional severity/category facets if the WG message model grows them. | `WG` |

## 3. Alarm / condition layer

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| A1 | Correlation id | `ConditionActive`/`ConditionCleared` emitted as an **uncorrelated** begin/end pair. | A `correlationId` minted on the rising edge and echoed on the clear, so a consumer pairs them across interleaving and re-trips. | `engineering` + `truST` |
| A2 | Full ISA-18.2 lifecycle | Active / Cleared only. | Acknowledge, shelve, suppress, out-of-service, latch/return-to-normal — as attributes + records. | `WG` (vocabulary) |
| A3 | Cause operands | `causeOperands[]` always empty. | Capture the operands/expression that tripped the condition. | `truST` |

## 4. Event vocabulary

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| E1 | Batch / recipe events | Not exposed. | Batch start/end, phase, recipe-step events as attributes. | `WG` |
| E2 | Operator / regulated events | Not exposed. | Operator action, e-signature / 21 CFR Part 11 audit-trail events. | `WG` |
| E3 | Vocabulary ids | The five core event ids (`StateTransition` 1, `ValueChanged` 2, `Message` 3, `ConditionActive` 512, `ConditionCleared` 513) are **provisional**. | Ratified id/enum assignments. | `WG` |

## 5. Model & semantic conformance

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| C1 | Model/state conformance | `model := 'ISA-88'`/`'PackML'` stored as a **label**; enum variants recorded as-is, **not** checked against the canonical state set. | Verify declared enum variants map onto the model's canonical states; reject or warn on mismatch. | `engineering` (needs ratified canonical sets → partly `WG`) |
| C2 | Unit registry | `unit` strings pass through unchecked. | Validate against a unit registry (UCUM or a WG-blessed table); resolve `unitId`. | `WG` (registry) |
| C3 | Category/model validation | Structural only (known enum values, valid combinations). | Semantic checks once C1/C2 land. | depends on C1/C2 |

## 6. Sources & identity

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| S1 | Source naming | Auto-named `source1`, `source2`, …; `path[]` = `[name]`; `hierarchy[]` empty. | Derive a real equipment-model path/hierarchy (`Reactor/R201`) from the program or a binding. | `truST` |
| S2 | Multi-source / multi-FB | Single producer FB exercised in the example. | Many sources/FBs into one ring with correct per-source seq/high-water at scale. | `engineering` |

## 7. ID stability & def-file hashing

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| I-D1 | Order-assigned ids | Ids follow declaration order (first `value` 2001, `state` 7001, `alarm` 9001, `message` 10001); reordering shifts ids → def-hash drift. Pin keys (`id`/`valueid`/…) exist but are **provisional**. | A stable assignment strategy (content-derived or a persisted id map) so retained records stay resolvable across edits, per spec §6.3. | `WG` (id policy) + `truST` |

## 8. Time

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| T1 | Wall-clock source | truST has **no `CURRENT_DT`**; `SourceTime` is host-injected Unix-ns each scan (this *is* the standard's platform-glue model). | Optional pure-ST `CURRENT_DT()` builtin so a program can stamp without host glue. | `truST` |

## 9. Lowering mechanism (truST)

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| L1 | Authoring lowering | A source-instrumentation / HIR pass (`trust-runtime/src/openot_authoring.rs`) rewrites tagged declarations into producer calls + a generated def file. | Native lowering in the compiler backend (or a settled decision that the HIR pass *is* the production path), with the id/key/enum/hash ownership formalized. | `truST` |

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
| R4 | `SourceHighWater` id | vendor range **`0x80000108`** | propose as core, or keep vendor |
| R5 | Publish ordering normativity | Release/Acquire fences are load-bearing and ARM-proven | make conformance-testable in the text |
| R6 | Upstreaming | reference impl is here | WG `spec/core.md`, `definition-file.md`, `doc-format.md` are still empty |

## 11. Conformance evidence

| # | Item | Now | Complete | Gate |
|---|---|---|---|---|
| K1 | Vector breadth | Byte-exact ST↔Rust vectors for the implemented encoders. | Vectors for every value type (V1), message args (M1), correlation (A1), more epoch/burst/loss negatives. | `engineering` |
| K2 | Unfenced weak-memory hole | Documented **non-reproduction** on ARM (the hole isn't reliably forced; correctness rests on the fences). | A litmus that reliably exposes the unfenced bug, strengthening the fence argument. | `engineering` |
| K3 | Model-conformance vectors | none | Vectors that assert C1 once it exists. | depends on C1 |

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
| I1 | `maxRecordSize` emitted **512** while the producer caps at the **256**-byte staging ring | Generator now caps the constant to **256**; `examples/reactor` artifacts regenerated (`maxRecordSize: 256`). |
| I2 | Lossy value types silently coerced to `DINT` | Non-`REAL`/`DINT` value attributes are now rejected (`hir_openot::is_supported_value_type_id` = `REAL`\|`DINT`); no silent coerce. |
| I3 | LSP offered value-logging on types the encoder can't emit | LSP classifies via `is_supported_value_type_id`; only `REAL`/`DINT` are offered. |
| L2 | `state` `category` default mismatch | Runtime now defaults an omitted `category` to **`process`** (`STATE_CATEGORY_PROCESS`), matching the LSP; and `category := 'procedural'` now **requires** a `model` (compile error otherwise), closing the model-less-procedural hole. |

## 14. Suggested sequencing

1. **Done (§13):** the correctness fixes (I1–I3) and the `category` default (L2) have
   landed and are verified — the active *incorrectness* is gone.
2. **Engineering, unblocked — do next:** V1 value types → M1 message args → A1
   correlation id, each with K1 vectors. These are the highest-leverage *correct*
   extensions and don't wait on the WG.
3. **truST:** S1 source hierarchy, T1 wall-clock builtin, V2/V3 value facets — as the
   runtime roadmap allows.
4. **WG-gated:** the vocabulary (E1–E3, A2), conformance registries (C1–C2), id policy
   (I-D1), and the impl↔proposal reconciliation (R1–R6) move when the ballot does. Feed
   the impl evidence into the WG; don't pre-build.
