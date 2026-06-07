# open-ot-experiments

Experimental, exploratory implementations to help shape the proposed
[OpenOT](https://github.com/SASE-Space/open-ot) standard for OT (operational-technology)
event logging. This is **not** a normative standard and **not** a finished product — it is a
workbench: executable implementations and byte-exact evidence the working group can test ideas
against, plus a working **truST reference** that runs OpenOT logging *live*.

OpenOT has three standards-facing contracts — carriage (wire), definition-file, document. This
workspace implements all three, a vendor-neutral **IEC 61131-3 Structured Text** reference
producer, the shared-memory transport, and an **attribute-driven authoring layer** that runs
inside the truST runtime.

| Layer | Where | Status |
| --- | --- | --- |
| Wire + carriage — records, ring buffer, loss accounting, epochs, concurrency | [`carriage`](crates/carriage) | Implemented + tested |
| Definition file — hash-bound id→meaning | [`definition`](crates/definition) | Content model, canonical hash, schema, resolver |
| Document format — resolved consumer output | [`document`](crates/document) | Proposed JSON shape + fixtures |
| Shared-memory transport — isolated-unsafe mmap store, safe API | [`open-ot-shm`](crates/open-ot-shm) | Implemented + ARM A/B |
| Conformance helpers — reconciliation + pluggable stale oracle | [`conformance`](crates/conformance) | Implemented |
| Concurrency A/B harness — fenced/unfenced, ARM litmus | [`live-harness`](crates/live-harness) | Implemented |
| IEC 61131-3 ST reference producer — encoders, producer FB, vectors | [`st/iec61131`](st/iec61131) | Implemented, cross-language conformant |
| Engineer-facing authoring — `{attribute 'oot'}` → records + def file | truST runtime (sibling repo) | Implemented — see [`examples/reactor`](examples/reactor) |
| Canonical registry — event / key / enum / type ids | `carriage::registry` | Provisional tables |

## What this proves

- **You log by tagging variables, not by writing log calls.** A control program annotates a
  variable — `{attribute 'oot' := 'value', 'unit' := 'L', 'deadband' := '0.5'} Level : REAL;` —
  and the compiler emits id-only OpenOT records plus the hash-bound definition file. The engineer
  never writes an id. See [`examples/reactor/`](examples/reactor) (`Reactor.st` → `batch-log.json`)
  and [`docs/authoring-attributes.md`](docs/authoring-attributes.md).
- **The whole path runs live.** truST executes the ST program → records into a shared-memory ring
  → a concurrent Rust consumer reads them back on ARM, with provable loss accounting (the capstone).
- **Completeness needs three signals, not one.** A per-source `Seq` only reveals loss once a
  *later* record arrives, so loss accounting combines seq gaps, authoritative `RecordsDropped`, and
  source high-water checkpoints. See [`docs/source-high-water.md`](docs/source-high-water.md).
- **Concurrency ordering cannot be left to testing.** The unfenced publish/overwrite protocol can
  accept overwritten data on weakly-ordered hardware — yet loom does not surface it and x86 is too
  strongly ordered to expose it. The release/acquire ordering must be in the spec. See
  [`docs/spec-feedback.md`](docs/spec-feedback.md).

## Layout

```
crates/carriage/      wire + carriage (records, ring, loss, consumer, epoch, concurrent) + vectors
crates/definition/    definition-file model + canonical serialization/hash + resolver
crates/document/      resolved consumer JSON + fixtures
crates/open-ot-shm/   isolated-unsafe shared-memory store (safe API; ARM-proven)
crates/conformance/   reconciliation + pluggable stale oracle (shared by the harnesses)
crates/live-harness/  fenced/unfenced concurrency A/B (ARM litmus)
st/iec61131/          vendor-neutral ST reference: encoders, producer FB, conformance tests
examples/reactor/     attribute-driven logging showcase: Reactor.st + its generated log + def file
docs/                 contracts, architecture, decisions, conformance, design notes
```

## Start here

- [`docs/overview.md`](docs/overview.md) — how the whole logging system fits together, end to end.
- [`docs/authoring-attributes.md`](docs/authoring-attributes.md) — how an engineer tags variables to log.
- [`docs/decisions.md`](docs/decisions.md) — why the system is built this way.
- [`docs/carriage-contract.md`](docs/carriage-contract.md) — the byte-level wire contract.
- [`examples/reactor/`](examples/reactor) — a worked program and the log it produces.

## Quick Start

Requires Rust 1.88+ (edition 2024).

```sh
cargo test                                          # all crates
RUSTFLAGS="--cfg loom" cargo test --release         # concurrency model (loom)
cargo run -p open-ot-carriage --example end_to_end  # produce, overflow, read back, reconcile
cargo run -p open-ot-carriage --bin dump_vectors    # regenerate conformance vectors
```

The live attribute-driven path (truST executes `examples/reactor/Reactor.st`, writes its log and
definition file) runs from the sibling truST runtime; see [`examples/reactor/README.md`](examples/reactor).

## Status

Exploratory work for a draft design, meant to feed the working group. Integer ids, event
vocabulary, and final conformance language remain working-group decisions. The crates stay small
and dependency-light so the behavior can be reviewed without a framework obscuring the rules. A
clean, productized implementation is a job for after the standard is ratified.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache-2.0 — see [LICENSE](LICENSE).
