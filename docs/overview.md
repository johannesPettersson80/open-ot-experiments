# OpenOT — how the logging system works, end to end

A map from "engineer tags a variable" to "consumer reads a resolved, loss-accounted log."
Each stage links to its detailed doc. The OpenOT standard is three contracts —
**carriage** (wire), **definition** (id→meaning), **document** (resolved output); authoring and
transport sit around them.

```
 ENGINEER WRITES                            examples/reactor/Reactor.st
   Level : REAL {attribute 'oot' := 'value', 'unit' := 'L', 'deadband' := '0.5'};
   ...normal control logic; no log calls...
        │
        │  (1) AUTHORING  — tag meaning on the declaration         authoring-attributes.md
        ▼
 truST COMPILER
   reads {attribute 'oot'}, assigns ids, lowers to producer ops,
   AND generates the definition file (id→meaning + content hash)
        │ emits                                   │ generates
        ▼                                         ▼
 ST PRODUCER FB                            DEFINITION FILE         definition-file.md
   per-source Seq, RunId, epochs;            id → name/unit/        examples/reactor/
   id-only OOT2 records + CRC;               state/condition;       openot-definition.json
   bursts via ScanRecords                    content hash
        │
        │  (2) CARRIAGE  — byte-exact encode                       carriage-contract.md
        ▼
 SHARED-MEMORY RING                         crates/open-ot-shm
   control block + byte ring; release/acquire publish protocol
        │
        │  (3) TRANSPORT — concurrent, fenced, ARM-proven          decisions.md D5–D6
        ▼
 RUST REFERENCE CONSUMER                    crates/conformance
   walks bytes · CRC · seqlock overwrite check ·
   loss accounting (seq gaps + RecordsDropped + high-water)
        │
        │  (4) RESOLVE   — join id-only records to the def file    definition (resolver)
        ▼
 RESOLVED LOG / DOCUMENT                    document-format.md
   ValueChanged valueId=2001  →  "Level = 15.25 L"               examples/reactor/batch-log.*
```

## The four stages

**1 — Authoring (truST).** The engineer annotates a variable's *meaning* with
`{attribute 'oot' := 'value' | 'state' | 'alarm' | 'message', …}` and writes ordinary control
logic. No log calls, no ids. The truST compiler reads the attributes. → [`authoring-attributes.md`](authoring-attributes.md),
decisions [D10](decisions.md).

**2 — Carriage (the wire).** The compiler lowers each tagged change to the proven ST producer,
which emits an **id-only** `OOT2` record — per-source `Seq`, `RunId`, a CRC trailer, no strings.
A multi-record transition burst is handed off via the producer's edge-persistent `ScanRecords`
list. → [`carriage-contract.md`](carriage-contract.md), decisions [D1](decisions.md), [D7](decisions.md).

**3 — Transport (shared memory).** Records land in a shared-memory ring (`open-ot-shm`): a control
block + byte ring with a single-writer publish protocol, a version-counter seqlock, and
release/acquire fences proven correct under concurrency on ARM. The consumer reads it live, in a
separate process/thread, while the producer writes. → decisions [D5–D6](decisions.md).

**4 — Resolution (meaning).** The Rust reference consumer validates each record (CRC + seq-space
overwrite check), reconciles loss three ways (seq gaps + authoritative `RecordsDropped` +
source high-water), then **joins the id-only records against the definition file** to recover
names/units/states. `ValueChanged valueId=2001 new=15.25` becomes `Level = 15.25 L`. →
[`definition-file.md`](definition-file.md), [`document-format.md`](document-format.md),
[`source-high-water.md`](source-high-water.md).

## Why each guarantee exists

See [`decisions.md`](decisions.md) for the rationale behind every choice (id-only wire, per-source
seq, mandatory CRC, the fences, the burst handoff, attribute authoring, host-injected time, …) and
the open reconciliation items.

## Try it

`examples/reactor/` is the worked example: [`Reactor.st`](../examples/reactor) (attributes only) →
`batch-log.txt` / `batch-log.json` (the produced log) + `openot-definition.json` (the generated
id→meaning file). The Rust carriage walk-through (`cargo run -p open-ot-carriage --example
end_to_end`) shows produce → overflow → read back → reconcile without the truST runtime.
