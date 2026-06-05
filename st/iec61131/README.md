# OpenOT IEC 61131-3 Structured Text Encoder

This directory contains vendor-neutral IEC 61131-3 Structured Text for the OpenOT
carriage format. The current slices cover CRC-32C, the 40-byte `OOT2` record header,
TLV slots, 4-byte slot padding, the CRC trailer, the 88-byte control-block image,
a fixed 128-byte ring write/wrap image, producer-side loss-range formation, and
the producer `RecordsDropped` record.

The conformance contract is byte-exact comparison against:

- `crates/carriage/vectors/conformant_state_transition.hex`
- `crates/carriage/vectors/conformant_message.hex`
- `crates/carriage/vectors/conformant_records_dropped.hex`
- `crates/carriage/vectors/conformant_source_high_water.hex`
- `crates/carriage/vectors/control_block.hex`
- `crates/carriage/vectors/wrap_marker_boundary.hex`
- `crates/carriage/vectors/records_dropped.hex`

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
