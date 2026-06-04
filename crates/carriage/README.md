# open-ot-carriage

The carriage layer of [open-ot-experiments](../../): the OpenOT wire format, in-controller ring
buffer, loss accounting, epochs, and the concurrent publish/read protocol. An experimental
prototype — implemented and tested, not a settled standard.

See the [workspace README](../../README.md) for the bigger picture, and
[`../../docs`](../../docs) for the carriage contract, architecture, conformance matrix, and design
notes.

## Modules

| Module | Purpose |
| --- | --- |
| `wire` | Record header, TLV slots, CRC trailer, and decoder validation. |
| `registry` | Provisional canonical event ids, value keys, TLV type tags, and enum tables. |
| `ring` | Single-threaded ring buffer, wrap handling, and overwrite detection. |
| `control` | 88-byte shared-memory control block and seqlock snapshot validation. |
| `loss` | Loss model and accounting math (seq gaps, `RecordsDropped`, high-water checkpoints). |
| `consumer` | Reading consumers (index-based and raw byte-walking) that drive loss accounting. |
| `epoch` | Run, epoch, definition-change, and source high-water producer logic. |
| `concurrent` | Concurrent publish/read protocol and memory-ordering checks. |
| `vectors` | Deterministic conformance-vector generator (fixtures in `vectors/`). |

## Build

```sh
cargo test -p open-ot-carriage
RUSTFLAGS="--cfg loom" cargo test -p open-ot-carriage --release
cargo run -p open-ot-carriage --example end_to_end
cargo run -p open-ot-carriage --bin dump_vectors
```
