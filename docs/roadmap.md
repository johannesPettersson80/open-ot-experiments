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
| **Attribute-driven authoring** (`value`/`state`/`alarm`/`message` → records + generated def file) | truST runtime | Implemented — see `examples/reactor` |

The earlier roadmap listed authoring and controller-language work as "planned / not started" —
**both are now built and running live.**

## Next

Engineering follow-ups (none blocking the capstone), roughly in priority order:

1. **Authoring DX in the LSP** — a type-aware "Add OpenOT logging" code action + completions + inlay
   hints (proposal Part II §18); driven by the attribute reference.
2. **Harden attribute validation** — reject unknown `category`/`class`/`model`/`unit` at compile time
   instead of silently defaulting ([`authoring-attributes.md`](authoring-attributes.md) limitations).
3. **Reconcile impl ↔ proposal divergences** — BCB 88 vs 80 bytes and the record-header layout. The
   impl is the ARM-validated one, so the intent is to update the proposal to match it (see
   [`decisions.md`](decisions.md) open items).
4. **Stable id pinning** — replace order-assigned ids (which drift the def hash on reorder) with
   pinned ids for deployments that retain records.
5. **Remaining event vocabulary** — batch/recipe, operator/regulated, and the full condition
   lifecycle (ack / shelve / suppress) as attributes.
6. **Upstream the proposal + reference** — the WG repo's `spec/core.md`, `definition-file.md`, and
   `doc-format.md` are still empty; land the proposal and point at this reference implementation.

*Done since the last revision:* doc-format **name resolution** — the reactor's `batch-log.json` now
renders through the `document` resolver (resolved names/units/enum labels + provenance).

## Principle

Each layer lands with its own tests and conformance vectors, and the whole workspace stays green at
every step. Crates stay small and dependency-light so the rules are reviewable without a framework
obscuring them.
