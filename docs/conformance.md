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

The implementation validates ring-buffer carriage behavior, the definition-file canonical hash preimage, definition schema validation, record resolution, and the proposed document-format mapping. It does not yet prove generated controller code or a transport above the ring buffer. Those should be separate conformance surfaces.
