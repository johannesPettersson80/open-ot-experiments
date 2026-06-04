# Specification Feedback

These are implementation findings that should be considered when drafting a wire-format v2 and core semantic layer. They are intentionally phrased as technical deltas, not as final standard text.

1. **Record identity needs `RunId` in the wire header.** The implementation uses a 40-byte header: Sync, TotalRecordLength, Flags, SourceTime, RunId, Seq, SourceId, EventTypeId. Carrying `RunId` on every record separates cold-start identity from sequence reset and keeps late consumers from resolving records against the wrong run.

2. **Overwrite detection is buffer-global absolute byte position.** Physical overwrite is about reclaimed byte ranges in one interleaved ring, so `HeadAbs` and `OldestAbs` are the conformance-critical bounds. Per-source `Seq` is still needed for loss attribution, but it is not the mechanism that proves a physical cursor has not been overwritten.

3. **`RunId` and `epochId` should not mean the same thing.** A warm definition change keeps `RunId` stable and source `Seq` continuous. A cold start increments `RunId` and resets source `Seq`. Retained records resolve by absolute position against `EpochFirstAbs`, the start offset of the `LoggerStarted` record that opens the current epoch.

4. **The publish ordering must be normative.** The producer must advance `OldestAbs`, execute a Release fence, and only then clobber reclaimed bytes. The consumer must read candidate bytes, execute an Acquire fence, and then re-read `OldestAbs` before delivery. CRC catches mixed torn records; the fence-backed oldest check catches clean overwritten records at a stale absolute cursor.

5. **Sequence gaps need a silent-source completion signal.** A per-source sequence counter only exposes a gap after the source emits again. A per-source high-water checkpoint lets a consumer reconcile a source that produced records, was fully evicted, and then went silent.

6. **The shared-memory control block should publish absolute positions coherently.** A physical `HeadOffset` is ambiguous after wrap. The experiment uses an 88-byte control block with a 32-bit `SeqLock` guarding 64-bit `HeadAbs`, `OldestAbs`, run, epoch, and definition-hash fields. Consumers keep their own absolute cursor.
