# Finish the OpenOT reference implementation — execution plan

> **Rev 7 status note (2026-06-07):** this plan is partially executed in the current WIP. The live
> backlog/status source is [`completeness.md`](completeness.md) §13 plus the open rows in §§1–11.
> Remaining implementation work is V3 sampling/deadband, A2 full condition lifecycle, E1/E2
> batch/recipe/operator/regulated authoring, S2 real multi-FB integration, K2 ARM weak-memory evidence,
> and K3 standalone model-conformance fixtures.
>
> **Rev 6** — five Codex passes, converged. Rev 6 (residue): removed the last self-contradicting tag
> sentence, made `EVENT_SPECS` the **canonical event-schema source of truth** (+ a cross-repo equality
> test) so "canonical" is enforced not assumed, re-tagged P1-d-1 `[LOCKSTEP]` (authoring keys touch the
> open-ot-ref vocabulary doc), made the cause-operand decision (both, reserved in P0-b), moved P1-v
> before the wide types, and made K2's "Complete" accept strengthened-evidence. (Rev 5 reframed the
> hash discipline + tags-as-source-ownership; rev 4 resolved the three decisions — *Decisions
> resolved* below.)

## Decisions resolved (lead)

Three items were flagged "decide before implementing." Resolved here; the user can override:
- **Vocabulary resolution (P0-b-0):** **add real semantic tables** (recipe/batch/operator/e-signature
  id→meaning), designed concretely in P0-b-0 — not generic-field-name-only. Stronger resolution, and
  it makes the Phase-0 structural freeze honest.
- **I-D1 id stability (P0-d):** **manual author/tool pinning is the chosen stable contract**, and
  `completeness.md` is updated to say so (it previously implied content-derived/persisted). Content-
  derived ids remain a future enhancement, not a blocker for "complete."
- **C1 surface (P0-e):** **both** — a definition validator **and** a truST compile diagnostic, so the
  engineer gets a compile error on a model/state mismatch → `[LOCKSTEP]`.

## Context

`docs/internal/completeness.md` lists ~24 open items between today's working reference and a
*complete* OpenOT implementation. The predecessor plan (`PLAN.md`, rev 3) built the layers —
carriage, definition, document, authoring, the ST producer, the live truST capstone. This plan is
its successor: it finishes the **semantic vocabulary**, the **value-type matrix**, **conformance**,
and **upstreaming the spec**.

**Decision (user, 2026-06-07): build everything as final now** — including the items that would
normally wait on a working-group ballot. Today the numeric ids are "strawman the WG-owned integers"
(`PLAN.md` §0b, §14.2.4). "Final" means promoting those strawman ids to final and implementing the
full vocabulary against them. This **supersedes** the standing "don't pre-build the unratified
surface" decision — recorded as [`decisions.md` D16](../decisions.md) and amended in
`completeness.md`, so the docs no longer contradict the plan. **Accepted risk:** if a later WG ballot
renumbers anything, it re-cuts the byte vectors and the definition content-hash. This plan is
sequenced so that churn is paid *once*, not per slice.

**Process:** the LEAD authors this plan; **Codex reviews the plan**; then **Codex implements it**
slice by slice; the **LEAD reviews each code implementation** independently, against the tests and
the byte/hash gates in Verification below. Each slice is small and lands green with its tests/vectors.

**Out of scope** (per `CONTRIBUTING.md`): network transport above the ring (OPC UA / MQTT / REST),
MES/historian integration, a productized runtime.

## What the codebase already gives us (verified, load-bearing)

1. **The id catalog is already reconciled, not un-invented.** `crates/carriage/src/registry.rs`
   already defines the full final set — condition lifecycle (`EVENT_CONDITION_* 0x0200–0x020E`),
   recipe/batch (`0x0301–0x0304`), operator/regulated (`0x0400–0x0406`), and every audit/batch field
   key — wired into `EVENT_SPECS`/`FIELD_SPECS`. The registry comments now treat them as assigned
   reference values pending WG ballot (`registry.rs:1-5`); D16 promotes them to final inside this
   workbench. **Wire-side vocabulary is promotion of constants,
   not greenfield numbering — but see fact 5: that is *not* the same as authoring support.**
2. **The consumer DECODES the full TLV matrix** (`resolver.rs::decode_value`,
   `document/src/lib.rs::value_document`), and P1-v is now implemented: schema validation checks
   value-bearing slots against the referenced `values[].dataType`. The trust audit renderer also covers
   the current wide value/message/condition surface. The remaining gaps are not generic decoding; they
   are the unimplemented event families and V3 sampling/deadband semantics.
3. **The sharpest cross-repo coupling is a test in the *other* repo.** `model.rs::DefinitionFile` is
   `deny_unknown_fields` (`model.rs:17-20`); trust-platform's live gate
   `crates/trust-runtime/tests/openot_telemetry.rs:917-923` parses truST's emitted definition JSON
   through it. **Any new definition JSON key added in truST must land in `model.rs` in the same
   change**, or trust-platform's suite fails to deserialize.
4. **ST encoders are hand-written with hardcoded byte geometry** (`openot_value_state.st:25-31`,
   literal `TotalLength`/`Key`/`TypeTag`). Byte-exactness is a manual triple-entry contract: ST
   literal bytes ≡ Rust `vectors.rs` `Slot::new(...)` ≡ truST lowering's slot order. Vectors *check*
   it, they don't *generate* it.
5. **`OotKind` has only `Value`/`State`/`Alarm`/`Message`** (`trust-hir …openot_authoring.rs:14-25`).
   So batch/recipe/operator/regulated are **not** expressible as attributes today. Registry constants
   alone are *wire* support, not *authoring* support — exposing them "as attributes" (what
   `completeness §E1/E2` requires) needs new kinds + the whole HIR/LSP/lowering chain.

## The two churn axes, and the discipline that pays them once

**Axis 1 — definition content-hash (JCS preimage).** The hash covers the *entire* per-program document
— `header` + `eventTypes` + `sources` + `stateMachines` + `conditions` + `messageTemplates` + `values`
+ `units` + `enumSets` + `severityScale` (`model.rs:17-49`; the golden serializes them all,
`hash.rs:288-290`). So a given program's hash changes whenever **its content** changes — including the
first time it emits a new event family (its `eventTypes[]` array gains an entry), value, or source.
That per-program content growth is **expected and localized**: it re-cuts *that program's own fixture*
(e.g. `examples/reactor`) in a `cargo test` update, and is not what the discipline avoids.

What the discipline pays once is **structural** change — editing a `model.rs` struct, a key/slot
definition, an event-type's canonical slot schema, or the top-level collection set. **P0-b freezes all
of it:** it establishes the **canonical slot schema for every event family in the registry** (not just
the 5 in `sample_definition`, `model.rs:348` — also the condition lifecycle, batch/recipe, operator/
regulated families), reserves every optional slot (`correlationId`, `causeOperand`, message
`arg`/`severity`) at `minOccurs:0`, and adds the new top-level semantic tables (P0-b-0). A later phase
then emits a record or `eventTypes[]` entry built from the *already-final* schema — content, not a
schema edit.

> **Rule after P0-b: no slice edits a `model.rs` struct, a key/slot definition, an event-type's
> canonical slot schema, or the collection set.** Feature slices add *content* (a new `eventTypes`
> entry from the canonical schema, a populated slot, a new value) → re-cuts that program's own fixture,
> expected. A genuinely new *structure* later is a reviewed P0-b amendment, never inline. Phase 2/3 are
> **not** "hash-neutral" for a program that adopts their events — they require **no schema edits**,
> which is the property that actually matters.

**Axis 2 — byte-exact record vectors.** These re-cut per encoder (correct — a new type *is* new
bytes). Pay once-per-feature via a fixed slice order:

> **(1)** Rust `carriage/src/vectors.rs` + `model.rs` types → **(2)** regenerate via `cargo run -p
> open-ot-carriage --bin dump_vectors` → **(3)** ST FB to match the new `.hex` → **(4)** truST
> `tlv_type`/lowering to emit it.

`vectors.rs::vectors_directory_matches_generator` fails on stale regen, so a missed regeneration
can't merge. "Re-ran `dump_vectors`?" goes in every encoder slice's prompt.

## Phased plan (each row = one Codex slice; tag = repos touched)

`[ref]` open-ot-ref source only · `[trust]` trust-platform source only · `[LOCKSTEP]` source in **both
repos**, one logical PR, both suites green. **Tags track source ownership, not blast radius** — a
`[ref]` slice can still change record bytes/definition content but edits no truST source, in which case
it runs the **trust gate** (Shared-surface rule). See the Lockstep map for the full rule.

### Phase 0 — Foundations: lock the contracts (size M–L)
| Slice | Tag | Work |
|---|---|---|
| **P0-0** | [LOCKSTEP] | **Landed in current WIP.** `maxRecordSize` is reconciled to the IEC producer cap (**256**) across the model, reactor artifacts, and generated definitions; bounded `STRING[96]` keeps implemented records under the cap. |
| P0-a | **[LOCKSTEP]** | **Landed in current WIP.** The registry comments now treat ids as assigned reference values; `SourceHighWater` is core system event `0x0108`; carriage/spec docs and ST/truST/vectors use that id. |
| **P0-b-0** | [LOCKSTEP] (design) | **Design the new definition tables (the one blocking item).** `DefinitionFile` is `deny_unknown_fields` (`model.rs:17`) with no recipe/batch/operator/e-signature table, so "add empty tables" needs real schema: name + type every new top-level struct, its field set, JCS ordering, sample JSON, the `model.rs` parser, and the truST emitter output. Output: the concrete schema the freeze locks. |
| P0-b | [LOCKSTEP] | **Landed in current WIP.** Definition structures include the semantic tables; `canonical_event_types()` derives schemas from `EVENT_SPECS`; trust emits those canonical schemas; optional slots are reserved in the canonical event schemas. |
| P0-c | [ref] | **Landed in current WIP.** `UNIT_SPECS` exists in `registry.rs`; the definition's own `units[]` remains the resolution source. |
| P0-d | **[LOCKSTEP]** | **Landed in current WIP.** Order-assigned ids remain the default; the pin keys are the chosen stability contract and have reorder-vs-pinned tests. |
| P0-e | **[LOCKSTEP]** | **Landed in current WIP.** C1 model/state conformance is enforced by both the definition validator and the truST attribute diagnostic. |

### Phase 1 — Value completeness (size L; the keystone)
| Slice | Tag | Work |
|---|---|---|
| **P1-design** | [ref] (design) | **Landed in current WIP.** The producer uses typed REAL/DINT/string paths plus a generic fixed-width value op; `STRING[96]` is the current payload policy under the 256-byte cap. |
| P1-v | [ref] | **Landed in current WIP.** Value-type validation now checks `KEY_NEW_VALUE`/`KEY_PREVIOUS_VALUE` TLV type against the referenced `values[].dataType` via `KEY_VALUE_ID` context. |
| P1-a | [LOCKSTEP] | **Landed in current WIP.** Integer widths (`SINT`/`USINT`/`INT`/`UINT`/`UDINT`/`ULINT`/`LINT`) emit through the generic fixed-width value path, with HIR/LSP/runtime support. |
| P1-b | [LOCKSTEP] | **Landed in current WIP.** `LREAL` and BOOL-as-value are supported. |
| P1-c | [LOCKSTEP] | **Landed in current WIP.** Bounded `STRING` values are supported. |
| P1-d-1 | **[LOCKSTEP]** | **Landed in current WIP.** `quality` and `semanticRole` are author-controlled and documented. |
| P1-d-2 | [LOCKSTEP] | **Landed in current WIP.** `previous := 'false'` suppresses `previousValue`; default behavior emits previous after the first sample. |
| P1-e-0 | [ref] (design) | **Still open.** The producer does only `REAL` deadband and on-change for the other supported types. Periodic needs a scan/time basis; hysteresis needs a two-threshold state. Design the authoring keys + scheduling semantics + producer state *before* coding. |
| P1-e | [LOCKSTEP] | **V3 implement** — integer/scaled deadband, then the on-change/periodic/hysteresis policies per the design; `Deadband.scaled`/`samplingPolicy` already modeled. Add suppressed-vs-emitted + periodic vectors. |

### Phase 2 — Messages & conditions (size M–L)
| Slice | Tag | Work |
|---|---|---|
| P2-a | [LOCKSTEP] | **Landed in current WIP.** `arg1`…`arg4` variable references populate `argTypes[]` and emit typed `arg` slots. Literal placeholder-brace parsing remains deferred. |
| P2-b | **[LOCKSTEP]** | **Landed in current WIP for severity.** `Message` can carry an optional severity slot. Message *category* remains out of scope because `KEY_CATEGORY` is state-specific and `MessageTemplateDefinition` has no category field. |
| P2-c | [LOCKSTEP] | **Landed in current WIP.** `ConditionActive` mints a correlation id and `ConditionCleared` echoes it. |
| P2-d | [LOCKSTEP] | **Landed in current WIP.** One named cause operand is registered in the definition and emitted as a `causeOperand` slot. Full expression capture remains deferred. |
| P2-e-0 | [trust] (design) | **A2 lifecycle design (new)** — today alarm authoring is only `class`/`severity` and lowering emits active/cleared from a BOOL edge (`hir …:58-59`, `openot_authoring.rs:803-819`). ack/shelve/suppress/oos/latch are *commands/state*, not BOOL edges — design how an ST program expresses them (authoring verbs/inputs + the condition state model) before any encoder. |
| P2-e | [LOCKSTEP] | **A2 implement per the design** — ids `0x0202–0x020E` + keys exist. **Split per family** (ack/reset · shelve/unshelve · suppress/oos), each with encoders + authoring + vectors. |

### Phase 3 — Event vocabulary (size **L**; full authoring chain, not just encoders)
Per review fact 5: "complete as attributes" (`completeness §E1/E2`) requires the whole chain, so each
family splits into a **[ref] wire layer** and a **[LOCKSTEP] authoring layer**. **Resolution
decision:** the model now has recipe/batch/operator/e-signature semantic tables, reserved by P0-b. The
wire/authoring slices still need to populate and exercise them.
| Slice | Tag | Work |
|---|---|---|
| P3-a1 | [ref] | **E1 wire layer:** batch/recipe encoders + `vectors.rs` + definition entries for `0x0301–0x0304` / keys `0x0023–0x0027`. |
| P3-a2 | [LOCKSTEP] | **E1 authoring layer:** new `OotKind` variant(s) + HIR vocabulary/validation + LSP completions/hints + lowering + a worked example. |
| P3-b1 | [ref] | **E2 wire layer:** operator/regulated/e-signature encoders + vectors + definition entries (`0x0400–0x0406`; 21-CFR-Part-11 keys). Split operator-action vs e-signature (heavier: `SIGNED_EVENT_SEQ`/`EFFECTIVE_TIME`/`CORRECTION_OF`). |
| P3-b2 | [LOCKSTEP] | **E2 authoring layer:** kinds + HIR + LSP + lowering + example. |

### Phase 4 — Conformance & sources (size M)
| Slice | Tag | Work |
|---|---|---|
| P4-a | [LOCKSTEP] | **Landed in current WIP.** truST emits canonical unit ids from `UNIT_SPECS`, rejects unknown symbols, and enforces category/model semantic checks. |
| P4-b | **[LOCKSTEP]** | **Landed in current WIP for the source metadata floor.** The generated source name/path/hierarchy derive from file stem + `PROGRAM` name (`Reactor.Main`, `["Reactor","Main"]`, `["file","program"]`). ISA-95 plant/equipment binding remains a future extension. |
| P4-c1 | [ref] | **S2 synthetic multi-source carriage** — a `conformance`-crate test: many sources into one ring, per-source seq/high-water correct. |
| P4-c2 | [LOCKSTEP] | **S2 truST multi-FB integration** — many ST FBs into one shared ring (today the authoring path creates **one** hidden producer instance, `openot_authoring.rs:676-680`); prove the real multi-FB path. |
| P4-d | [ref] | **K3 model-conformance vectors** — a definition whose enum violates its model → placeholder (depends on P0-e). |

### Phase 5 — Time & proof (size S–M)
| Slice | Tag | Work |
|---|---|---|
| P5-a | [trust] | **Landed in current WIP.** `CURRENT_DT()` exists as a truST wall-clock builtin; the producer source-time path remains host-injected Unix-ns for OpenOT records. |
| P5-b | [ref] | **K2 weak-memory litmus** — strengthen the unfenced proof (today documented non-reproduction on Cortex-A76). **Explicit exit:** "force it on ≥1 documented platform *or* ship an N-iteration stress harness + strengthened negative-proof," not a guaranteed positive. |
| P5-c | [LOCKSTEP] | **Partially landed in current WIP.** Vectors cover the implemented current surface (wide values, message arg/severity, active/cleared condition correlation/cause, lifecycle/high-water/loss). Still add vectors when the future A2/E1/E2/V3 event families land, plus standalone model-conformance fixtures (K3). |
| P5-d | **[ref]** | **Landed in current WIP.** `decisions.md` records HIR/source instrumentation as the production lowering path for the supported subset and scopes native-backend lowering out. |

### Phase 6 — Upstream the spec (size M; prose)
| Slice | Tag | Work |
|---|---|---|
| P6-a | [ref] | **Landed in current WIP.** `spec/core.md`, `spec/definition-file.md`, and `spec/doc-format.md` exist as upstream-ready drafts of the implemented contracts. |

## Lockstep map (the rule, so slices don't silently break the other repo)
- **Tags = source ownership, not blast radius.** `[LOCKSTEP]` = a slice that edits **source in both
  repos** in one logical PR (e.g. an encoder change: ST bytes + truST lowering + vectors). `[ref]` /
  `[trust]` = source in **one** repo. A slice can be `[ref]` and still change record bytes or
  definition content (the Phase-3 wire layers do) — it edits no truST source, so it's `[ref]`, but it
  **must run the trust gate** (Shared-surface rule). So "changes bytes" does **not** imply
  `[LOCKSTEP]`; "edits truST authoring/lowering source" does.
- **[ref] = open-ot-ref *source* only — NOT "trust-platform unaffected."** trust-runtime builds
  against the live open-ot-ref tree, so a [ref] change to definition / registry / document / ST /
  reactor-artifacts can still break the trust gate and **must run `openot_telemetry`** (see
  Verification). Examples: P0-c, P1-design, P4-c1, P4-d, P5-b, P5-d, P6-a, and the Phase-3 *wire*
  layers P3-a1/b1 (encoder bytes + def entries, no truST authoring source).
- **[trust]-only:** authoring-surface/lowering changes that reuse existing vectors, add no definition
  key/slot, *and* don't alter the committed reactor artifacts (P0-d, P1-d-1, P5-a). **Any truST change
  that moves `examples/reactor/*` — source naming, a showcased new attribute — is [LOCKSTEP].**
- **Trap:** a "[trust]-only" slice that adds a definition key *or slot* silently becomes [LOCKSTEP].
  **Every truST authoring PR: grep emitted JSON keys + event slots against `model.rs` before review.**
  P0-b makes this a non-event for planned features. **Also:** any slice adding an **authoring key/kind**
  must update `docs/authoring-attributes.md` (the open-ot-ref vocabulary source of truth) → [LOCKSTEP]
  even when the bytes don't change.

## Riskier than they look
1. **P0-0 record-size reconciliation** — landed, but keep it guarded: `maxRecordSize` is hashed,
   and any future wider record shape must stay under the 256-byte producer cap or explicitly revise it.
2. **P0-b slot-schema freeze** — the linchpin. If any future slot (correlationId, cause operand,
   message arg/severity) is *not* reserved here, the phase that adds it re-cuts the hash — exactly the
   churn this plan exists to avoid.
3. **Producer-FB value generalization** — landed for the current value matrix; future changes must not
   regress the existing byte-exact value tests.
4. **P1-c STRING** — first variable-length payload; the 255-byte slot cap is a hard limit, not a
   guideline.
5. **Phase 3 authoring layers** — the real cost; new `OotKind` + HIR + LSP + lowering per family,
   each a multi-file truST change, not an encoder tweak.
6. **K2 (P5-b)** — "reliably force a weak-memory bug" may be infeasible on the hardware; the "or" exit
   keeps it bounded.
7. **P0-b-0 semantic-table schema** — the new top-level structs must be right first time
   (`deny_unknown_fields` + hashed + the trust gate parses them); a wrong field/order re-cuts the freeze.
8. **P1-e periodic/hysteresis sampling** — needs a scan/time basis + producer state, not a comparator
   tweak; interacts with the SourceTime/scan cadence.
9. **P2-e lifecycle semantics** — ack/shelve/suppress are commands/state, not BOOL edges; the authoring
   model (how a program expresses them) is the hard part, not the encoders.
10. **P4-c2 multi-FB** — proving many ST FBs into one ring needs the authoring path to instantiate >1
    producer (today a single hidden instance, `openot_authoring.rs:676-680`).
11. **Future variable-length payloads** — message args and STRING values are now bounded by the current
    authoring/producer policy (`arg1`…`arg4`, `STRING[96]`). Any new variable-length event family must
    stay under the 256-byte producer cap instead of relying on an unbounded schema maximum.

**Ordering constraints:** **P0-0 (size audit) → P0-b (freeze) before any Phase-1 vector work**, and
**P1-design before P1-a/b/c**.

## Verification (per slice + integration)
- **open-ot-ref:** `cargo test` (incl. `vectors_directory_matches_generator`, the `hash.rs` golden,
  schema/resolver/document fixtures); regen with `cargo run -p open-ot-carriage --bin dump_vectors`;
  `RUSTFLAGS="--cfg loom" cargo test --release`; `cargo fmt --all -- --check`; `cargo clippy
  --all-targets -- -D warnings`; MSRV `cargo build` on 1.88.
- **trust-platform (the integration tripwire):** `cargo test -p trust-runtime --test openot_telemetry`
  (ST→shm→consumer authoring gate that round-trips truST JSON through `model.rs` and asserts the audit
  log) and `--test openot_capstone` (fenced ARM). LSP tests for any `action_requests.rs` change.
- **Every [LOCKSTEP] slice:** both suites green in one PR before review.
- **Shared-surface rule:** any slice touching `crates/definition`, `crates/document`,
  `crates/carriage/src/registry.rs`, the ST sources, or `examples/reactor/*` **must run `cargo test -p
  trust-runtime --test openot_telemetry`** even if it edits no truST source — trust-runtime builds
  against the live open-ot-ref tree (path dep; `openot_telemetry.rs:74`).
- **Definition shape:** the `hash.rs` golden is the canary — it must change *only* in P0-0/P0-b-0/P0-b
  (and explicit P0 amendments), never incidentally.

## Coverage — every open item in `completeness.md` maps to a slice
| Item | Slice(s) | | Item | Slice(s) |
|---|---|---|---|---|
| V1 value types | P1-design, P1-a/b/c, P1-v | | C3 category/model checks | P4-a |
| V2 previousValue/quality | P1-d-1, P1-d-2 | | S1 source hierarchy | P4-b |
| V3 deadband/sampling | P1-e-0, P1-e | | S2 multi-source/FB | P4-c1, P4-c2 |
| M1 message args | P2-a | | I-D1 id stability | P0-d |
| M2 message severity | P2-b (category deferred) | | T1 wall-clock builtin | P5-a |
| A1 correlation id | P2-c | | L1 lowering mechanism | P5-d |
| A2 ISA-18.2 lifecycle | P2-e-0, P2-e | | R1–R3 reconcile · R5 fences | P0-a · R5 +P5-b+P6-a |
| A3 cause operands | P2-d | | R4 high-water core id | P0-a |
| E1 batch/recipe | P3-a1, P3-a2 | | R6 upstream spec | P6-a |
| E2 operator/regulated | P3-b1, P3-b2 | | K1 vector breadth | P5-c (+ per-feature) |
| E3 final vocabulary ids | P0-a | | K2 weak-memory litmus | P5-b |
| C1 model/state conformance | P0-e | | K3 model-conformance vectors | P4-d |
| C2 unit registry | P0-c + P4-a | | §12 transport/productization | out of scope |

Every backlog item is assigned, and the three earlier open decisions are resolved (see *Decisions
resolved*). Nothing is left open except the explicitly out-of-scope productization surface.

## Sequence summary
Phase 0 (**size audit → freeze**, then promote/units/ids/C1) → Phase 1 (**design → values**) → Phase
2 (messages/conditions) → Phase 3 (vocabulary: wire + authoring layers) → Phase 4 (conformance/
sources) → Phase 5 (time/proof) → Phase 6 (upstream). T1 floats after P0.
