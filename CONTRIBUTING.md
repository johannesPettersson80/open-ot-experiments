# Contributing

This workspace contains executable prototypes for draft OpenOT event-logging contracts. The goal
is to keep the behaviour reviewable: small, dependency-light at the carriage layer, and backed by
tests that provide evidence for candidate specification rules.

## Build and test

```sh
cargo test                                # behaviour + byte-vector suite
RUSTFLAGS="--cfg loom" cargo test --release   # concurrency model (loom)
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

All four must pass. CI runs the same set.

The minimum supported Rust version is **1.88** (edition 2024 plus stabilized let-chains); a dedicated
CI `msrv` job builds the workspace on 1.88 so the floor stays honest.

## Conformance vectors

The files under `crates/carriage/vectors/` are generated, not hand-edited:

```sh
cargo run -p open-ot-carriage --bin dump_vectors
```

`cargo test` compares the checked-in fixtures against the generator, so any drift is a
test failure. If you change the wire encoding, regenerate the vectors in the same commit.

## Scope

This workspace implements the three standards-facing contracts plus the supporting layers:

- `open-ot-carriage`: wire records, ring buffer, loss accounting, epochs, and the concurrent
  publish/read protocol.
- `open-ot-definition`: hash-bound ID-to-meaning files, schema validation, and record resolution.
- `open-ot-document`: resolved/loss/placeholder JSON documents.
- `open-ot-shm`: isolated-unsafe shared-memory transport behind a safe API (ARM-proven).
- `open-ot-conformance` + `live-harness`: reconciliation/stale-oracle helpers and the concurrency A/B.
- `st/iec61131`: the vendor-neutral IEC 61131-3 ST reference producer.

The engineer-facing authoring layer (`{attribute 'oot'}` → records + definition file) and the live
producer→shm→consumer integration live in the sibling **truST** runtime. Still out of scope: a network
transport *above* the buffer (OPC UA / MQTT / REST), MES integration, and a productized runtime.

## Style

- One module owns one protocol concern; keep storage, loss accounting, and concurrency
  separate.
- New behaviour comes with a test that asserts it, named for the behaviour it proves.
- Public items carry doc comments that explain the invariant, not just the signature.

## License

By contributing you agree that your contributions are licensed under Apache-2.0, the
license of this project.
