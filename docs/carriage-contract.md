# Carriage Contract

This document records the amended carriage contract implemented by the prototype. It is a draft input for a wire-format update, not a complete semantic or document-format standard.

## Record Header

Every record uses a 40-byte little-endian header followed by TLV slots, padding, and a CRC-32C trailer.

| Offset | Size | Field | Notes |
| --- | ---: | --- | --- |
| 0 | 4 | Sync | `OOT2` |
| 4 | 2 | TotalRecordLength | Header + slots + padding + CRC trailer. |
| 6 | 2 | Flags | bit0 `TimeUnsynced`, bit1 `Synthetic`, bit2 `PartialPayload`, bit3 `HasCrc`; remaining bits reserved 0. A memory-buffer record sets `HasCrc`; `append_encoded` rejects a record without it. |
| 8 | 8 | SourceTime | Producer timestamp. |
| 16 | 8 | RunId | Cold-start identity. |
| 24 | 8 | Seq | Source-local sequence number. |
| 32 | 4 | SourceId | Producer source stream. |
| 36 | 4 | EventTypeId | Registry event id. |
| 40 | ... | TLV slots | Ordered payload slots. |
| ... | 0..3 | Padding | Zero bytes to a 4-byte boundary. |
| ... | 4 | CRC-32C | Covers all preceding record bytes. |

`RunId` increments only on a cold start. `Seq` is local to `(RunId, SourceId)` and advances for data and control records.

## Registry Deltas

The prototype uses a registry module so record tests and fixture generation share one source of truth.

| Item | Id | Meaning |
| --- | ---: | --- |
| `EVENT_SOURCE_HIGH_WATER` | `0x0108` | Per-source high-water checkpoint in the system event range. |
| `KEY_DROPPED_COUNT` | `0x0016` | Dropped record count. |
| `KEY_FIRST_LOST_SEQ` | `0x0017` | First lost source-local sequence. |
| `KEY_LAST_LOST_SEQ` | `0x0018` | Last lost source-local sequence. |
| `KEY_DEF_HASH_NEW` | `0x001C` | New definition hash. |
| `KEY_SOURCE_HIGH_WATER` | `0x0038` | Scalar `producedCount`. |
| `KEY_DEF_HASH_OLD` | `0x0039` | Previous definition hash. |
| `KEY_EPOCH_ID` | `0x003A` | Epoch id. |
| `KEY_COLD_START` | `0x003B` | LoggerStarted cold-start flag. |

`EVENT_SOURCE_HIGH_WATER` intentionally does not use `0x0105`; that id is reserved for `SourceRegistered`. The old packed `{source_id, produced_count}` high-water slot is retired.

## Control Block

Shared-memory consumers read producer state through an 88-byte little-endian control block at a known address.

| Offset | Size | Field | Notes |
| --- | ---: | --- | --- |
| 0 | 4 | Sync | `OOT2` |
| 4 | 1 | Version | Current prototype version. |
| 5 | 1 | Caps | Capability flags. |
| 6 | 2 | Reserved | Must be zero. |
| 8 | 4 | BufferId | Buffer identity. |
| 12 | 4 | BufferBytes | Ring byte capacity. |
| 16 | 4 | SeqLock | Aligned 32-bit atomic. Even means stable; odd means updating. |
| 20 | 4 | Reserved2 | Must be zero. |
| 24 | 8 | HeadAbs | Absolute published head. |
| 32 | 8 | OldestAbs | Absolute oldest retained byte. |
| 40 | 8 | LostCount | Persisted non-wrapping lost count. |
| 48 | 8 | RunId | Current run. |
| 56 | 8 | EpochId | Current epoch. |
| 64 | 8 | EpochFirstAbs | Absolute offset of the LoggerStarted record that opened the current epoch. |
| 72 | 8 | DefinitionHash | Current definition hash. |
| 80 | 8 | PrevDefinitionHash | Previous retained definition hash. |
| 88 |  | Total |  |

There is no `HeadOffset`. Consumers keep an absolute cursor and parse `[cursor_abs, HeadAbs)`.

## Producer Commit Order

For a record write, the producer follows this order:

1. Compute `record_start_abs` and `final_head`.
2. Set `SeqLock` odd.
3. Advance `OldestAbs` and `LostCount` for reclaimed records.
4. Execute a Release fence before touching reclaimed bytes.
5. Write any wrap marker and then the record bytes.
6. Set `HeadAbs = final_head`.
7. Set `SeqLock` even with Release ordering.

The important ordering is evict-before-clobber. `OldestAbs` is published before reclaimed bytes are overwritten, and new record bytes are visible before the stable `HeadAbs` snapshot is published.

## Consumer Read Rule

The consumer reads a coherent snapshot:

1. Read `SeqLock`; retry if it is odd.
2. Read fields at offsets 24..87.
3. Re-read `SeqLock`; retry if it changed.
4. If the saved cursor is below `OldestAbs`, lap to `OldestAbs` and account loss.
5. Walk raw bytes until `HeadAbs`, following wrap markers and `TotalRecordLength`.
6. For each candidate record, read bytes, execute an Acquire fence, read a fresh `OldestAbs`, reject if `OldestAbs > record_abs`, then decode and verify CRC.

The raw byte walk is the same parse/wrap/overwrite logic used by the ring implementation. The control block changes where `HeadAbs` and `OldestAbs` come from; it does not create a second parser.

## Epochs

Epoch resolution is absolute-position based.

- Cold start: `RunId` increments and per-source `Seq` resets.
- Warm definition change: `RunId` stays stable, `epochId` increments, and per-source `Seq` continues.
- `EpochFirstAbs` is the absolute start offset of the `LoggerStarted` record that opens the epoch.
- Retained records before `EpochFirstAbs` resolve to the previous retained epoch; records at or after it resolve to the current epoch.

The implementation keeps current plus at most one prior epoch.

## Source High-Water

`SourceHighWater` is an ordinary record in the affected source stream:

```text
SourceId       = affected source
EventTypeId    = EVENT_SOURCE_HIGH_WATER
Seq            = producedCount
Payload        = KEY_SOURCE_HIGH_WATER: ULInt producedCount
```

When a consumer reads the checkpoint, it inserts authoritative loss for `[expected_seq, producedCount - 1]` if `producedCount > expected_seq`, then accounts the checkpoint itself as delivered control. There is no deferred finalization step.

## Draft Patch Targets

The prototype implies these standard-text changes:

- §5 wire record: use the 40-byte header with per-record `RunId`.
- §5 shared memory: define the 88-byte absolute-position control block and remove `HeadOffset`.
- §6 registry: reserve the ids listed above and retire the packed high-water and epoch-first-seq struct slots.
- §7.2 and §9.3: resolve epochs by `EpochFirstAbs`; make definition changes warm unless they are paired with a true cold start.
- §11.5: make the Release/Acquire publish ordering conformance-testable; the oldest re-check is only meaningful with that ordering.
