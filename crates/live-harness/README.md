# OpenOT live harness

This crate contains a process-level shared-memory test harness for the experimental
OpenOT carriage protocol. It is intentionally separate from `open-ot-carriage` so the
carriage crate remains dependency-light and portable.

The harness maps a `/dev/shm` file into a producer process and a consumer process,
then runs the same fenced publish/read protocol used by the in-process concurrent
ring tests.
