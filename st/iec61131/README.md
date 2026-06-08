# OpenOT IEC 61131-3 Structured Text Encoder

This directory contains vendor-neutral IEC 61131-3 Structured Text for the OpenOT
carriage format. The current slices cover CRC-32C, the 40-byte `OOT2` record header,
TLV slots, 4-byte slot padding, the CRC trailer, the 88-byte control-block image,
a fixed 128-byte ring write/wrap image, producer-side loss-range formation, and
the producer `RecordsDropped` record. It also includes a minimal message encoder
and a producer function block that assigns independent per-source sequence numbers
before writing each record into the S2 producer ring. The producer can also emit
per-source `SourceHighWater` checkpoints where the envelope sequence and
`producedCount` payload are the same value by construction. Lifecycle encoders cover
the system-source `LoggerStarted`, `LoggerStopped`, and `DefinitionChanged` records,
and the producer orchestrates cold and warm epoch transitions with a system sequence
counter, source high-water checkpoints, typed `ValueChanged`, `StateTransition`,
`Message`, and active/cleared condition records, exposed transition state, and a
fixed per-scan record-list output for multi-record transition bursts.
The capture POUs drive fixed multi-record scenarios and expose the final
256-byte ring plus dynamic control fields for cross-language validation by the
Rust carriage harness.

The conformance contract is byte-exact comparison against:

- `crates/carriage/vectors/conformant_state_transition.hex`
- `crates/carriage/vectors/conformant_value_changed_{bool,sint,usint,int,uint,dint,udint,ulint,lint,real,lreal,string}.hex`
- `crates/carriage/vectors/conformant_message.hex`
- `crates/carriage/vectors/conformant_condition_{active,cleared}.hex`
- `crates/carriage/vectors/conformant_records_dropped.hex`
- `crates/carriage/vectors/conformant_source_high_water.hex`
- `crates/carriage/vectors/control_block.hex`
- `crates/carriage/vectors/wrap_marker_boundary.hex`
- `crates/carriage/vectors/records_dropped.hex`
- `crates/carriage/vectors/minimal_message.hex`
- `crates/carriage/vectors/logger_started_cold.hex`
- `crates/carriage/vectors/logger_started_warm.hex`
- `crates/carriage/vectors/logger_stopped.hex`
- `crates/carriage/vectors/definition_changed.hex`

No specific toolchain, runtime, or test framework is required by this public artifact.
The test POUs expose pass/fail state and mismatch metadata for harnesses that can run
this ST subset.

## Conservative ST Subset

The ST source stays inside this subset:

- fixed `ARRAY[..] OF BYTE` buffers;
- explicit output lengths;
- explicit little-endian byte writers;
- `DWORD` CRC arithmetic using `SHR`, `AND`, and `XOR`;
- scalar payload packing by byte arithmetic or typed bit-string conversion.

The S1 source does not use:

- pointers or references;
- variable-length arrays;
- overlapping memory access or overlay declarations;
- generic `ANY` parameters;
- `STRING` internal layout;
- assertion functions or a runtime-specific test framework;
- endian conversion helpers.

For the message vector, text is encoded as explicit payload bytes and length, not by
reading a `STRING` representation.

## Files

- `src/openot_crc32c.st` defines `OPENOT_BYTE_BUFFER` and `OPENOT_Crc32c`.
- `src/openot_wire_encode.st` defines little-endian writer helpers and one encoder
  function block per conformant vector.
- `src/openot_control_block.st` defines the 88-byte control-block writer.
- `src/openot_ring.st` defines the fixed-capacity ring write path used by the
  wrap-marker boundary vector.
- `src/openot_records_dropped.st` defines the producer `RecordsDropped` encoder.
- `src/openot_ring256_producer.st` defines the fixed-capacity producer loss-range
  formation path.
- `src/openot_message.st` defines the `Message` encoder with a template id,
  optional typed argument slots, and optional severity.
- `src/openot_value_state.st` defines `ValueChanged` encoders for `REAL`,
  `DINT`, generic fixed-width value payloads (`BOOL`, integer widths, `LREAL`),
  bounded `STRING`, plus the parameterized `StateTransition` and
  `ConditionActive`/`ConditionCleared` encoders.
- `src/openot_source_high_water.st` defines the parameterized `SourceHighWater`
  encoder used by producer checkpoints.
- `src/openot_lifecycle.st` defines byte-exact encoders for system lifecycle
  records.
- `src/openot_producer.st` composes the 256-byte producer ring with a fixed
  per-source sequence table, typed authoring ops (`Op = 6` for `REAL`
  `ValueChanged`, `Op = 7` for `DINT` `ValueChanged`, `Op = 8` for
  `StateTransition`, `Op = 9` for active/cleared conditions, `Op = 10` for
  generic fixed-width values, and `Op = 11` for bounded `STRING` values),
  generalized pre-encoded staging (`Op = 5`), per-scan record-list outputs, and
  the cold/warm epoch transition state machine.
- `captures/openot_s4a_capture.st` defines the S4a scenario drivers
  `OPENOT_CaptureRichWrap` and `OPENOT_CaptureLifecycleSurvival`. Call the POU
  once, confirm `Complete = TRUE`, then dump its public `Ring` and control-field
  outputs.
- `tests/*.st` defines self-checking POUs. Each test exposes:
  - `Passed : BOOL`
  - `MismatchIndex : UINT`
  - `ActualLength : UINT`
  - `ExpectedLength : UINT`

`MismatchIndex = 65535` means no byte mismatch was found. If `Passed = FALSE` and
the lengths differ, inspect `ActualLength` and `ExpectedLength`.

## Harness Contract

A conforming harness loads the source files, instantiates each test POU, executes one
scan, and reads the exposed outputs. A vector test passes only when:

1. `ActualLength = ExpectedLength`;
2. every byte in `Buffer[0..ActualLength-1]` equals the embedded expected bytes from
   the matching `.hex` vector.

The CRC test passes only when CRC-32C over explicit ASCII bytes `16#31..16#39`
equals `16#E3069283`.

## Authoring Lowering Target

Engineers should author OpenOT logging through declaration attributes in the
truST compiler, for example:

```iecst
Level : REAL {attribute 'oot' := 'value', 'unit' := 'L', 'deadband' := '0.5'};
```

The `OPENOT_Producer` typed ops are retained as the compiler/internal lowering
target. `Op = 6` emits a `ValueChanged` record for a `REAL`, `Op = 7` emits one
for a `DINT`, `Op = 8` emits a `StateTransition`, `Op = 9` emits
`ConditionActive`/`ConditionCleared` on a BOOL alarm edge, `Op = 10` emits
fixed-width value payloads, and `Op = 11` emits bounded `STRING` values. These
ops track last value/state/condition inside the producer and emit only on
change/deadband/edge.
If `SourceId` is omitted or zero, the producer uses source `1`; the generated
call does not carry an `EventTypeId`.

When multiple records may be emitted in one PLC scan, generated code sets
`ResetScanRecords := TRUE` once at the top of the logging block, then immediately
clears it. IEC FB inputs retain their last assigned values, so omitting the clear
would keep resetting the handoff batch.
