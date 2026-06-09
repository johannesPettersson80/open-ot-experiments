# Roadmap

Exploratory work to help the OpenOT working group shape the standard; once it stabilizes, a clean
implementation follows from what these experiments establish.

## Done

| Layer | Where | State |
| --- | --- | --- |
| Wire + carriage (records, ring, loss accounting, epochs, concurrency) | `carriage` | Implemented + tested; loom + ARM A/B |
| Definition file (hash-bound id→meaning) | `definition` | Content model, canonical hash, schema, resolver |
| Document format (resolved consumer output) | `document` | Proposed JSON + golden fixtures |
| Shared-memory transport (isolated-unsafe mmap store) | `open-ot-shm` | Implemented; ARM fenced/unfenced A/B |
| Conformance helpers (reconciliation + stale oracle) | `conformance` | Implemented; shared by both harnesses |
| Concurrency A/B harness | `live-harness` | Implemented (ARM litmus) |
| IEC 61131-3 ST reference producer (encoders, producer FB) | `st/iec61131` | Implemented; byte-exact cross-language conformant |
| **Live truST integration** (ST → shm → concurrent Rust consumer) | truST runtime | Implemented; ARM capstone (fenced green; unfenced non-repro documented) |
| **Attribute-driven authoring** (base, lifecycle, batch/recipe, operator/regulated, e-signature, audited values → records + generated def file) | truST runtime | Implemented — see `examples/reactor` and `examples/multi-program` |
| **Multi-PROGRAM source routing** (distinct generated producers drained into one ring) | truST runtime | Implemented for `PROGRAM` blocks; FB-local authoring remains out of scope |

The earlier roadmap listed authoring and controller-language work as "planned / not started" —
**both are now built and running live.**

## Next

Engineering follow-ups (none blocking the capstone), roughly in priority order:

1. **Reconcile impl ↔ proposal divergences** — BCB 88 vs 80, the record-header layout, and the
   overwrite check (absolute-offset impl vs seq-space proposal). The impl is the ARM-validated one,
   so the intent is to update the proposal to match it.
2. **Optional authoring expansion** — `FUNCTION_BLOCK`-local OpenOT authoring and richer
   plant/equipment source ownership if the WG wants those as first-class source boundaries.
3. **Upstream the proposal + reference** — the WG repo's `spec/core.md`, `definition-file.md`, and
   `doc-format.md` are still empty; land the proposal and point at this reference implementation.

*Done since the last revision:* doc-format **name resolution** (the reactor's `batch-log.json` now
renders through the `document` resolver — resolved names/units/enum labels + provenance); the **LSP
authoring DX** (type-aware code actions, completions, inlay hints); and **strict attribute
validation** (`InvalidOpenOtAttribute` rejects unknown kind/key/category/model/class/severity); the
**Message `messageTemplateId`** slot (messages now resolve to their template); and the **value-typed
ValueChanged schema** (`valuePayload` slots — non-REAL values validate). Manual id pinning
(`id`/`valueid`/…) is the chosen stability contract for this workbench.

## Principle

Each layer lands with its own tests and conformance vectors, and the whole workspace stays green at
every step. Crates stay small and dependency-light so the rules are reviewable without a framework
obscuring them.
