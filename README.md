# open-ot-experiments

Experimental, exploratory implementations to help shape the proposed
[OpenOT](https://github.com/SASE-Space/open-ot) standard for OT (operational-technology)
event logging. This is **not** a normative standard and **not** a finished product — it is a
workbench: executable implementations and byte-exact evidence the working group can test ideas
against. Once the standard stabilizes, a clean implementation follows from what these
experiments establish.

OpenOT has three standards-facing contracts, plus supporting registry and authoring work. This
workspace implements the carriage, definition-file, and document-format contracts as experimental
prototypes; authoring work remains planned (see [`docs/roadmap.md`](docs/roadmap.md)).

| Contract / support layer | Crate | Status |
| --- | --- | --- |
| Wire + carriage — records, ring buffer, loss accounting, epochs, concurrency | [`carriage`](crates/carriage) | Implemented + tested (experimental prototype) |
| Definition file — hash-bound map from ids to meaning | [`definition`](crates/definition) | Content model, canonical hash, schema validation, and resolver implemented |
| Document format — resolved consumer-facing output | [`document`](crates/document) | Proposed JSON shape + exact fixtures implemented |
| Canonical registry — event / key / enum / type ids | `carriage::registry` | Implemented as provisional tables |
| Engineer-facing authoring workflow | `authoring` | Planned |

Planned work is not stubbed until real implementation begins — see the roadmap for the plan.
Other-language experiments (for example an IEC 61131-3 Structured Text logger) would join as
sibling trees rather than Rust crates.

## What's interesting here

The carriage prototype surfaced two results worth the read on their own:

- **Completeness needs three signals, not one.** A per-source sequence counter only reveals loss
  once a *later* record from that source arrives, so loss accounting combines three complementary
  mechanisms — seq gaps (mid-stream loss), authoritative `RecordsDropped` (known producer
  evictions), and source high-water checkpoints (the silent-source tail the other two cannot
  see). See [`docs/source-high-water.md`](docs/source-high-water.md).
- **The concurrency ordering cannot be left to testing.** The unfenced publish/overwrite protocol
  can accept overwritten data on weakly-ordered hardware — yet the included loom model does not
  surface it, and x86 testing is too strongly ordered to expose it. The deliberately broken model
  is checked in as a control test. The conclusion: the release/acquire ordering must be written
  into the spec, not left for an implementer to discover by testing. See
  [`docs/spec-feedback.md`](docs/spec-feedback.md).

## Layout

```
crates/carriage/     wire + carriage prototype (wire, ring, loss, consumer, epoch, concurrent) + its vectors
crates/definition/   definition-file model + canonical serialization/hash prototype
crates/document/     proposed resolved/loss/placeholder document JSON + fixtures
docs/                architecture, conformance, design notes, roadmap
```

Start with [`docs/carriage-contract.md`](docs/carriage-contract.md) for the implemented byte-level
contract, [`docs/document-format.md`](docs/document-format.md) for the proposed document shape, and
[`docs/architecture.md`](docs/architecture.md) for the module/data-flow view.

## Quick Start

Requires Rust 1.88+ (edition 2024).

```sh
cargo test                                          # all crates
RUSTFLAGS="--cfg loom" cargo test --release         # concurrency model (loom)
cargo run -p open-ot-carriage --example end_to_end  # produce, overflow, read back, reconcile
cargo run -p open-ot-carriage --bin dump_vectors    # regenerate conformance vectors
```

The vector generator is checked by the test suite, so checked-in fixtures cannot drift.

## Status

Exploratory work for a draft design, meant to feed the working group. Integer ids, event
vocabulary, and final conformance language remain working-group decisions. The carriage crate
intentionally stays small and dependency-light so the behavior can be reviewed without a
framework obscuring the rules. A clean, productized implementation is a job for after the
standard is ratified.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache-2.0 — see [LICENSE](LICENSE).
