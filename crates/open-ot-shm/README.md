# open-ot-shm

`open-ot-shm` provides a safe shared-memory store for the experimental OpenOT
carriage protocol. It owns the mmap mapping and keeps all raw pointer and atomic
view construction inside this crate.

The crate depends on `open-ot-carriage` for the protocol and `memmap2` for the
shared mapping. The carriage crate remains dependency-light and has no normal
dependencies.

`FenceMode::Unfenced` is present only for diagnostic A/B stress runs. Product
publishers use `FenceMode::Fenced`.
