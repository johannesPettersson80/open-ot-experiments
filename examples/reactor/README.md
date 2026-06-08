# OpenOT Reactor Example

This example is the attribute-authored OpenOT path:

- `Reactor.st` contains normal Structured Text and `{attribute 'oot' := ...}` declarations.
- The process state is declared as `E_ReactorStep`; the generated definition file carries that enum set.
- The truST compiler lowers those declarations to a hidden `OotProducer : OPENOT_Producer`.
- The existing ST-FB telemetry path publishes `Main.OotProducer` records to shared memory.
- The runtime supplies the source clock: hosted build -> host Unix clock; real hardware -> RTC/NTP.
- `batch-log.json`, `batch-log.txt`, and `openot-definition.json` are generated from a truST -> shm -> consumer run.

Run the proof from the sibling `trust-platform` checkout:

```sh
cargo test -p trust-runtime --test openot_telemetry openot_telemetry_authoring_showcase_renders_typed_audit_log -- --nocapture
```

The authoring surface in `Reactor.st` is declaration-only. It must not contain `Op :=`, `Execute :=`, or `OOT_Log`.

Rendered batch log:

```text
OpenOT Reactor Batch Log

2026-06-08T05:20:57.464Z  Message source=1 seq=0 templateId=10001 severity=100 args=[DINT(1)]
2026-06-08T05:20:57.464Z  StateTransition source=1 seq=1 machine=7001 category=0 previous=0 new=1
2026-06-08T05:20:57.464Z  ValueChanged source=1 seq=2 valueId=2001 new=REAL(0)
2026-06-08T05:20:57.464Z  ValueChanged source=1 seq=3 valueId=2002 new=DINT(1)
2026-06-08T05:20:59.613Z  ValueChanged source=1 seq=4 valueId=2001 previous=REAL(0) new=REAL(6)
2026-06-08T05:21:00.603Z  StateTransition source=1 seq=5 machine=7001 category=0 previous=1 new=2
2026-06-08T05:21:00.603Z  ValueChanged source=1 seq=6 valueId=2001 previous=REAL(6) new=REAL(12)
2026-06-08T05:21:02.092Z  ValueChanged source=1 seq=7 valueId=2001 previous=REAL(12) new=REAL(13.5)
2026-06-08T05:21:03.088Z  StateTransition source=1 seq=8 machine=7001 category=0 previous=2 new=3
2026-06-08T05:21:03.088Z  ValueChanged source=1 seq=9 valueId=2001 previous=REAL(13.5) new=REAL(15)
2026-06-08T05:21:03.088Z  ConditionActive source=1 seq=10 conditionId=9001 class=0 severity=900 correlation=1 causes=[1]
2026-06-08T05:21:05.137Z  ValueChanged source=1 seq=11 valueId=2001 previous=REAL(15) new=REAL(7.5)
2026-06-08T05:21:06.103Z  StateTransition source=1 seq=12 machine=7001 category=0 previous=3 new=4
2026-06-08T05:21:06.103Z  ValueChanged source=1 seq=13 valueId=2001 previous=REAL(7.5) new=REAL(0)
2026-06-08T05:21:06.103Z  ConditionCleared source=1 seq=14 conditionId=9001 class=0 correlation=1

Forced overflow
retained_records: 2
lapped_batches: 1
shm_lost_count: 13
source_1_delivered: 2
source_1_lost: 13
source_1_reconciled: 15
```
