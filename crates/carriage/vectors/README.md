# OpenOT wire-v2 conformance vectors

These files are generated, not hand-written. Regenerate them with:

```sh
cargo run -p open-ot-carriage --bin dump_vectors
```

`cargo test` compares the checked-in files with the generator output, so byte
fixture drift is a test failure. `.hex` files are byte-exact hexadecimal dumps.
The adjacent `.json` files describe the expected interpretation or rejection.

Files named `conformant_*` are definition-layer positive record fixtures. The
older codec fixtures remain valid wire records; where their `.json` marks
`schemaExpected: reject`, they are intended as future schema-violation negatives.
