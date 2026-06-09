# OpenOT Multi-PROGRAM Example

This example shows the multi-source authoring path: one ST source file contains
two attributed `PROGRAM` blocks, and truST generates one hidden
`OotProducer : OPENOT_Producer` per program.

- `Filler` emits `Message`, `StateTransition`, and `ValueChanged` records.
- `Quality` emits `Message`, `ValueChanged`, an audited `ParameterChange`, and
  a `ConditionActive` alarm record.
- Neither program writes OpenOT calls or ids; the authoring surface is only
  `{attribute 'oot' := ...}` on declarations.
- Because both programs omit `sourceid`, truST assigns deterministic source ids
  in program order: `Filler -> 1`, `Quality -> 2`.

The sibling truST example project at
`trust-platform/examples/openot_multi_program/` uses this same source shape and
configures:

```toml
[runtime.openot]
enabled = true
source = "st-fb"
producer_instances = ["Filler.OotProducer", "Quality.OotProducer"]
```

The runtime drains those generated producers in the configured order into one
shared-memory ring. The carriage still has one writer: the Rust runtime
serializes the per-program producer output before publishing.
