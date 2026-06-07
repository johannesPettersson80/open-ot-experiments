# OpenOT — Design Decisions

Why the logging system is built the way it is. The rest of the repo shows *what*
was built; this records *why*, and what alternatives were rejected. Distilled from
the phase log and four rounds of adversarial review. Numeric ids/enum values are
provisional (working-group ballot); the *structure* is the decision.

Each entry: **Decision** · *Why* · *Rejected* · *Status*.

---

## Wire & carriage

### D1 — Wire format v2 (`OOT2`), not the working group's v1
The records carry a per-source **`Seq`**, a **`RunId`**, and a **CRC** trailer that
v1 (`OOT!`) lacks; the byte layout is in [`carriage-contract.md`](carriage-contract.md).
*Why:* v1 made buffer **position** the sequence, which is not durable across a ring
wrap or a controller restart, so loss can't be proven. A per-source `Seq` + `RunId`
makes a gap explicit and countable. *Rejected:* extending v1. *Note:* the `OOT2`
sync (vs `OOT!`) deliberately makes a v1 consumer refuse to misparse v2.

### D2 — Per-source `Seq` is the ordering authority, not buffer position
Ordering within a source is by `Seq`; position is only a steady-state optimization.
*Why:* a consumer that falls behind and gets lapped learns from position only that
it's behind, not which/how many records were lost — `Seq 41 → 88` ⇒ 46 lost. There
is deliberately **no total order across sources** (cross-source is by `SourceTime`,
subject to skew).

### D3 — CRC-32C is mandatory on in-memory ring buffers
*Why:* a half-written record can satisfy the magic + length checks while carrying a
wrong value; only an integrity check rejects a plausible-but-wrong record. *Rejected:*
a "barrier" capability flag exempting memory buffers — it's unverifiable and an async
reader can still see `Head` ahead of the bytes.

### D4 — Loss accounting takes three signals, not one
Seq gaps (mid-stream loss) **+** authoritative `RecordsDropped` (known producer
evictions) **+** `SourceHighWater` checkpoints (the silent-source tail). *Why:* a
seq counter only reveals loss when a *later* record from that source arrives; a source
that goes quiet after dropping records would otherwise be invisible. See
[`source-high-water.md`](source-high-water.md).

## Concurrency

### D5 — Atomic publish = `HeadAbs` commit + a real seqlock + absolute-position overwrite check
The visibility commit is a single atomic `HeadAbs` store; a version-counter seqlock
brackets the multi-word control snapshot; an overwritten record is detected by comparing
**64-bit absolute byte offsets** — `OldestAbs > record_start_abs` (`carriage/src/control.rs`,
`overwrote_record_at`). *Why:* a consumer must never accept a torn or overwritten record;
monotonic 64-bit absolute offsets make the compare wrap-unambiguous. *Note:* the rev-6
proposal draft specifies the equivalent check in **seq-space** (via `OldestSeq`); the impl
realizes it with absolute offsets — one of the impl↔proposal divergences (open items below,
and [`spec-feedback.md`](spec-feedback.md)). *Rejected:* a hi/lo/hi pseudo-seqlock (could
accept a torn counter).

### D6 — The release/acquire fences are load-bearing and belong in the contract
*Why:* the unfenced publish/overwrite path can accept overwritten data on weakly
ordered hardware — yet **loom does not surface it and x86 is too strongly ordered to
expose it**, so it must be specified, not left for an implementer to discover by
testing. Proven on ARM (fenced airtight; the unfenced model is checked in as a
deliberately-broken control). See [`spec-feedback.md`](spec-feedback.md).

## ST ↔ runtime handoff

### D7 — Multi-record bursts handed off via an edge-persistent `ScanRecords` list
A cold/warm transition emits `LoggerStopped` + one `SourceHighWater` per source in a
single scan; the 256-byte staging ring can't hold the burst and only exposed the
*last* record. The producer now exposes the whole scan's records as a descriptor list,
**reset only on an Execute edge** (so the runtime, which reads outputs *after* the
scan, still sees it). The runtime enforces `delta == ScanRecordCount`. *Why:* the
runtime reads FB outputs post-scan; a per-call reset would zero the burst before it's
read. *Rejected:* a "(start,end) over the 256B ring" list (the burst self-evicts);
multi-scan drain (changes the proven single-edge transition semantics).

### D8 — One-record-per-scan first, bursts second; synthetic heartbeat before real records
*Why:* de-risk the live path in slices — prove the runtime→shm→consumer plumbing with
a synthetic heartbeat (S4b-2), then one real ST record/scan (S4b-3b), then bursts
(S4b-3c) — each with its own ARM proof rather than one big unproven jump.

## truST integration

### D9 — Isolated-unsafe shm crate + safe publisher; `trust-runtime` stays `forbid(unsafe)`
`open-ot-shm` owns the mmap + atomics + fences behind a safe API (the proven
`SharedConcurrentStore`, extracted from the Phase-4 harness); `trust-runtime` depends
on it and calls the safe publisher at the per-scan publish point. *Why:* reuse the
proven memory-ordering code instead of reimplementing it, and keep unsafe out of the
product runtime. *Rejected:* mmap atomics directly in `trust-runtime` (it forbids
unsafe); a truST-native protocol first (defer — make OpenOT work correctly first).

## Authoring (truST)

### D10 — Author by **attribute**, not pragma, function calls, or op-codes
`{attribute 'oot' := …}` on the variable declaration; the compiler emits the records.
*Why:* an attribute attaches to the declaration and survives rename/refactor, and the
compiler owns the id/key/enum space and the def-file hash — "the engineer never sees an
id." *Rejected:* a custom `{oot …}` pragma (truST could, but attribute was chosen and
is portable-tolerated by other IEC tools); hand-callable `OOT_Log*` functions / raw
producer op-codes (a lowering target, not a front door — uglier than the WG's own
example). The authoring *mechanism* is explicitly out of the standard's scope; this is
truST's choice. See [`authoring-attributes.md`](authoring-attributes.md).

### D11 — The compiler generates the definition file from the attributes
id→meaning + a content hash, from the AST. *Why:* ids derived from code (no spelling
drift, no codegen-vs-extract sync), and the hash binds a record stream to its
definition. *Rejected:* a hand-maintained definition file.

### D12 — id-only on the wire; names/units/templates live in the definition file
*Why:* no strings on the controller, tiny records; the consumer resolves ids→meaning
via the def file. A `ValueChanged valueId=2001 new=15.25` becomes `Level = 15.25 L`
only after resolution.

## Time

### D13 — `SourceTime` is host-injected (truST has no `CURRENT_DT`)
truST exposes only a monotonic `TIME()`; there is **no wall-clock ST builtin**. The
runtime feeds real Unix-ns into the producer's source-time input each scan. *Why:* this
is exactly the standard's model — the platform glue (RTC/NTP) supplies the clock, which
on a hosted build is the host clock. *Status:* a pure-ST `CURRENT_DT()` would need a new
truST builtin (deferred).

### D14 — Deterministic stamping for conformance vectors, real clock for the live example
*Why:* byte-exact ST↔Rust vectors need a fixed clock to stay reproducible; the showcase
log needs real timestamps. The producer takes a source-time input the vectors fill with
a constant and the example fills from the clock.

## Conformance

### D15 — Conformance = byte-exact ST↔Rust vectors **and** the live ARM capstone
Three evidence surfaces with distinct strengths: **(1)** byte-exact cross-language
vectors prove per-record **encoding**; **(2)** the **fenced** capstone
(`openot_capstone_fenced_cross_process` — a cross-process truST producer → mmap →
concurrent Rust consumer on ARM) proves the **composition** under real concurrency
(full reconciliation, `rejected=0`); **(3)** the **unfenced** runs (the capstone
contrast and the `live-harness` litmus) are **diagnostic** — on the Cortex-A76 the
weak-memory hazard is a documented **non-reproduction**, not a proof of safety. The
fences are established as load-bearing by the **fenced loom model + fence-hook tests**
(D6), *not* by an ARM leak; a non-reproduction is the model-expected outcome and does
not weaken that proof. *Why:* prose can't guarantee every interleaving, and the proof
boundary must be stated honestly — the reference impl is the ratification evidence the
proposal itself calls for.

---

## Open items / known divergences

- **BCB / record-header sizes and the overwrite-check space differ between impl and the
  rev-6 proposal draft** (BCB 88 vs 80; header layout; absolute-offset vs seq-space — D5).
  The impl is the ARM-proven one, so the intent is to update the *proposal* to match it —
  not yet reconciled. (D1, D5)
- **`SourceHighWater`** is our reconciliation aid, **in the vendor id range** (off core
  `0x0108`); propose it to the WG as a core addition or keep it vendor. (D4)
- **No model/state conformance check** — `model := 'ISA-88'` is not verified to match the
  canonical state set, and `unit` strings are not checked against a registry. (D10)
- **Ids default to declaration order** (first id 2001/7001/9001/10001) → def-hash drift on
  reorder; an explicit id can be pinned (provisional). (D11)
