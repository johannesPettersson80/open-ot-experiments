# OpenOT Core Draft

This draft records the executable core contract implemented by this reference
workbench. It is intended as input to the OpenOT working-group repository, not as a
separate product manual.

## Record

Every record is little-endian and starts with a 40-byte header:

| Offset | Size | Field |
| --- | ---: | --- |
| 0 | 4 | Sync `OOT2` |
| 4 | 2 | `TotalRecordLength` |
| 6 | 2 | Flags |
| 8 | 8 | `SourceTime` |
| 16 | 8 | `RunId` |
| 24 | 8 | source-local `Seq` |
| 32 | 4 | `SourceId` |
| 36 | 4 | `EventTypeId` |
| 40 | ... | ordered TLV slots |
| ... | 0..3 | zero padding to 4-byte alignment |
| ... | 4 | CRC-32C over all preceding record bytes |

Flags are `TimeUnsynced`, `Synthetic`, `PartialPayload`, and `HasCrc`; remaining
bits are reserved zero. Shared-memory records set `HasCrc`, and consumers reject a
record without it.

`RunId` changes only on cold start. `Seq` is local to `(RunId, SourceId)` and
advances for both data and control records in that source stream.

## TLV

Each slot is:

| Field | Size | Notes |
| --- | ---: | --- |
| `Key` | 2 | value-key id |
| `Type` | 1 | TLV type tag |
| `Length` | 1 | payload length, max 255 |
| `Payload` | `Length` | type-specific bytes |

Known TLV tags are Bool, SInt, USInt, UInt, Int, UDInt, DInt, ULInt, LInt, Real,
LReal, DateTime, String, and Bytes. Value-bearing slots carry the datum's TLV type;
the event schema marks those slots as `valuePayload` rather than pinning a single
type.

## Shared Memory

The shared-memory buffer uses an 88-byte control block followed by the byte ring.
The control block carries absolute byte positions: `HeadAbs`, `OldestAbs`,
`LostCount`, `RunId`, `EpochId`, `EpochFirstAbs`, the current definition-hash
prefix, and the previous retained definition-hash prefix.

Consumers keep an absolute cursor. If the cursor falls below `OldestAbs`, the
consumer laps to `OldestAbs` and accounts the lost range. There is no `HeadOffset`;
wrap is parsed from the raw bytes.

## Publish Ordering

The producer's required ordering is:

1. compute the record window;
2. set the seqlock odd;
3. publish `OldestAbs`/`LostCount` for reclaimed bytes;
4. execute a Release fence before clobbering reclaimed bytes;
5. write wrap marker and record bytes;
6. publish `HeadAbs`;
7. set the seqlock even with Release ordering.

The consumer reads the seqlock-protected snapshot, walks bytes up to `HeadAbs`,
executes an Acquire fence before accepting a candidate record, then rechecks
`OldestAbs`. A record is rejected if it has been overtaken.

## Epochs

Cold start increments `RunId` and resets source-local sequences. A warm definition
change keeps `RunId`, increments `EpochId`, and opens a new epoch with a
`LoggerStarted` record. `EpochFirstAbs` is the absolute offset of that record.
Consumers resolve records before `EpochFirstAbs` with the previous retained
definition and records at or after it with the current definition.

## Registry

The assigned reference registry lives in `crates/carriage/src/registry.rs`. It
contains base events, system events, condition lifecycle events, procedural
batch/recipe events, regulated/operator events, TLV tags, field keys, enum tables,
procedural model state sets, severity bands, and the canonical unit registry.

`SourceHighWater` is assigned as system event `0x0108` with payload key `0x0038`
(`producedCount`). A consumer uses it as an authoritative per-source high-water
checkpoint for loss reconciliation.
