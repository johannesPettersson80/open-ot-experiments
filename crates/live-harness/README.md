# OpenOT live harness

This crate contains a process-level shared-memory test harness for the experimental
OpenOT carriage protocol. It is intentionally separate from `open-ot-carriage` so the
carriage crate remains dependency-light and portable.

The harness maps a `/dev/shm` file into a producer process and a consumer process,
then runs the same fenced publish/read protocol used by the in-process concurrent
ring tests.

The normal A/B command is:

```sh
cargo run -p open-ot-live-harness -- run --mode litmus --fenced
cargo run -p open-ot-live-harness -- run --mode litmus --unfenced
```

For the K2 weak-memory diagnostic, repeat the deliberately unfenced run and
aggregate any stale-oracle, rejected-record, or poll-error evidence:

```sh
cargo run -p open-ot-live-harness -- run --mode litmus --unfenced --iterations 100
```

A clean repeated run is a bounded non-reproduction on that machine, not proof of
safety; the fence proof remains the release/acquire model plus the fence-hook
tests.
