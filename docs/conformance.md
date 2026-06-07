# Conformance Evidence

This repository is not a conformance test suite for a ratified standard. It is an executable evidence set for candidate rules that can later be turned into conformance requirements.

## Test Matrix

| Area | Evidence |
| --- | --- |
| CRC | Known-answer CRC-32C and empty-input tests. |
| Wire codec | Round trips, CRC rejection, length rejection, padding validation, and byte-exact StateTransition / RecordsDropped vectors. |
| Registry | Event ids, value-key ids, TLV type tags, enum values, severity bands, and experiment delta ids. |
| Definition hash | Typed definition model, duplicate-key rejection, no-float canonical form, exact canonical-byte fixtures, `contentHash=""` self-exclusion, SHA-256 lowercase hex, and 8-byte digest-order binding. |
| Definition schema | Conformant positive vector validation, codec-vector schema-negative validation, fixed type per core key, occurrence, order, repeated-contiguous slots, vendor-extension trailing/ascending rules, scalar width/zero-length rules, and max record/slot constraints. |
| Definition resolver | Current vs prior epoch hash selection, prior-definition staleness, drift placeholders, unknown-id placeholders, schema-placeholder preservation, typed field decoding, field names, source names, and enum labels. |
| Document format | Exact JSON fixtures for resolved events, private extension fields, schema/drift/stale-prior/unknown-id placeholders, and authoritative/inferred loss ranges. |
| Ring behavior | Keep-up reads, wrap markers, raw byte walking, lapped reconnects, and stale-cursor recovery. |
| Loss accounting | Per-source sequence gaps, producer-authoritative RecordsDropped events, overlap union, and inline silent-source high-water accounting. |
| Epoch handling | Warm definition change keeps `RunId` stable and source `Seq` continuous; cold start increments `RunId` and resets source `Seq`. |
| Concurrency | Real-thread stress plus loom runs for accepted-record safety and documented model-checker limits. |
| Fault injection | Forced wrap boundary, reconnect after overwrite, torn record rejection, and clock rollback with sequence-preserved ordering. |
| Typed event encoders | Byte-exact `ValueChanged` (REAL / DINT) and `StateTransition` vectors. |
| Cross-language conformance | ST-emitted record bytes equal the Rust reference vectors, byte for byte (per-record + the S4a multi-record ring composition). |
| ST reference producer | IEC 61131-3 producer FB + encoder POUs run under the truST runtime (`TestHarness`); per-source seq, checkpoints, cold/warm transitions, and the `ScanRecords` burst. |
| Attribute authoring | The reactor program (`{attribute 'oot'}` only) lowers to records and a generated definition file; the authoring POU passes. |
| Live truST integration | truST executes the ST producer → shared-memory ring → concurrent Rust consumer: data records, the transition burst, multi-record fail-closed, and the typed authoring-showcase render. Green on ARM and x86. |
| Live concurrency capstone | truST producer → mmap → concurrent consumer on ARM: **fenced** = full reconciliation + `rejected=0` + stale oracle silent; **unfenced** = documented non-reproduction (the weak-memory hole is not reliably forced; correctness rests on the fences, per `spec-feedback.md`). |
| Fixtures | Generated `.hex` and `.json` vectors under `crates/carriage/vectors/`, checked by the test suite. The `conformant_*` record vectors are the positive definition-layer spine; codec-only vectors marked `schemaExpected: reject` are reserved as schema-violation negatives. |

## Commands

```sh
cargo test
```

```sh
RUSTFLAGS="--cfg loom" cargo test --release
```

```sh
cargo run -p open-ot-carriage --bin dump_vectors
```

## Fixture Policy

Fixture bytes are generated from `crates/carriage/src/vectors.rs`. The checked-in files are still useful because they give reviewers stable bytes to inspect, but they are not maintained by hand. The generator and the checked-in files must match.

## Limits Of The Evidence

The implementation validates ring-buffer carriage behavior, the definition-file canonical hash preimage, definition schema validation, record resolution, the proposed document-format mapping, the IEC 61131-3 ST reference producer (byte-exact vs the Rust reference), and the **live truST path** — attribute-driven authoring → producer → shared-memory ring → concurrent consumer, proven on ARM. These run from the sibling truST repo as **separate** targets:

- **Live integration / ST-FB authoring path** — `cargo test -p trust-runtime --test openot_telemetry` (heartbeats, real ST-producer records, the transition burst, the typed authoring-showcase render).
- **Fenced ARM capstone** — `cargo test -p trust-runtime --test openot_capstone` (`openot_capstone_fenced_cross_process`: cross-process producer → mmap → concurrent consumer, full reconciliation).
- **Unfenced contrast** — a diagnostic, `#[ignore]`-gated experiment: `OPENOT_CAPSTONE_RUN_UNFENCED=1 cargo test -p trust-runtime --test openot_capstone openot_capstone_unfenced_contrast -- --ignored`. On the Cortex-A76 it is a documented non-reproduction (see the matrix row above).

Still out of scope as separate conformance surfaces: a network transport *above* the ring buffer (OPC UA / MQTT / REST), a productized runtime, and reconciling the few impl↔proposal divergences (BCB/header sizes — see [`decisions.md`](decisions.md)).
