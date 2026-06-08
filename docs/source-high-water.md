# Source High-Water Checkpoints

Per-source sequence gaps prove loss once a later record from the same source arrives. They do not prove loss for a source whose retained data records were overwritten and then went silent.

Source high-water checkpoints close that boundary by publishing a source-local produced count as an ordinary record in that source stream.

## Event Shape

The current experiment uses one checkpoint record per affected source:

```text
Envelope SourceId: affected source
EventTypeId:       EVENT_SOURCE_HIGH_WATER = 0x0108
Envelope Seq:      producedCount
ValueKeyId:        KEY_SOURCE_HIGH_WATER = 0x0038
Payload:           producedCount: u64 little-endian
```

The payload is a scalar `ULInt`. There is no packed `{source_id, produced_count}` slot; the source comes from the record envelope.

> **Core system event.** `EVENT_SOURCE_HIGH_WATER = 0x0108` lives in the system range
> (`0x0100–0x01FF`). High-water is this reference's reconciliation aid for silent-source tails.
> A consumer that doesn't recognize it skips it like any unknown id; loss accounting still works
> from seq gaps + `RecordsDropped`, just without the silent-tail proof.

## Consumer Rule

When the consumer reads a high-water record with `producedCount = P`, it compares `P` to the next expected sequence for that `(runId, sourceId)` stream.

```text
if P > expected:
    authoritative loss = [expected .. P - 1]
then account the checkpoint record itself as delivered control
```

This is inline accounting. There is no deferred finalize step after draining to head.

## Scope

High-water checkpoints reconcile total stream continuity. They are control records, so document/reporting layers may exclude them from data-only counts while still using them for loss proof.

If the checkpoint itself is overwritten before a consumer sees it, it cannot reconcile that silent tail. Producers that need this guarantee should emit checkpoints periodically and at orderly shutdown or epoch boundaries.
