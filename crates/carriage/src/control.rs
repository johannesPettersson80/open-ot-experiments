//! Shared-memory control block layout and snapshot validation.

/// Byte length of the Phase-0 shared-memory control block.
pub const CONTROL_BLOCK_LEN: usize = 88;

/// Sync bytes for the Phase-0 shared-memory control block.
pub const CONTROL_BLOCK_SYNC: [u8; 4] = *b"OOT2";

/// Offset of the `Sync` bytes.
pub const OFF_SYNC: usize = 0;
/// Offset of the one-byte control-block version.
pub const OFF_VERSION: usize = 4;
/// Offset of the one-byte capabilities bitset.
pub const OFF_CAPS: usize = 5;
/// Offset of the first reserved byte range.
pub const OFF_RESERVED: usize = 6;
/// Offset of the logical buffer id.
pub const OFF_BUFFER_ID: usize = 8;
/// Offset of the ring byte capacity.
pub const OFF_BUFFER_BYTES: usize = 12;
/// Offset of the 32-bit seqlock word.
pub const OFF_SEQ_LOCK: usize = 16;
/// Offset of the second reserved byte range.
pub const OFF_RESERVED2: usize = 20;
/// Offset of the published absolute head.
pub const OFF_HEAD_ABS: usize = 24;
/// Offset of the oldest retained absolute byte.
pub const OFF_OLDEST_ABS: usize = 32;
/// Offset of the persisted lost-record count.
pub const OFF_LOST_COUNT: usize = 40;
/// Offset of the current run id.
pub const OFF_RUN_ID: usize = 48;
/// Offset of the current epoch id.
pub const OFF_EPOCH_ID: usize = 56;
/// Offset of the current epoch's first absolute byte.
pub const OFF_EPOCH_FIRST_ABS: usize = 64;
/// Offset of the current definition hash prefix.
pub const OFF_DEFINITION_HASH: usize = 72;
/// Offset of the previous definition hash prefix.
pub const OFF_PREV_DEFINITION_HASH: usize = 80;

/// A coherent snapshot of the producer-owned control block fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlBlockSnapshot {
    /// Wire/control-block version.
    pub version: u8,
    /// Producer capabilities bitset.
    pub caps: u8,
    /// Logical buffer id.
    pub buffer_id: u32,
    /// Ring capacity in bytes.
    pub buffer_bytes: u32,
    /// Published absolute head, one past the last committed byte.
    pub head_abs: u64,
    /// Oldest retained absolute byte offset.
    pub oldest_abs: u64,
    /// Persisted non-wrapping lost-record count.
    pub lost_count: u64,
    /// Current run id.
    pub run_id: u64,
    /// Current epoch id.
    pub epoch_id: u64,
    /// Absolute start of the current epoch's `LoggerStarted` record.
    pub epoch_first_abs: u64,
    /// Current definition hash prefix.
    pub definition_hash: [u8; 8],
    /// Previous definition hash prefix.
    pub prev_definition_hash: [u8; 8],
}

impl ControlBlockSnapshot {
    /// Encodes this snapshot as an 88-byte control-block image with a stable even `SeqLock`.
    pub fn encode(&self, seq_lock: u32) -> [u8; CONTROL_BLOCK_LEN] {
        assert!(
            seq_lock.is_multiple_of(2),
            "stable control-block image needs even SeqLock"
        );
        let mut bytes = [0u8; CONTROL_BLOCK_LEN];
        bytes[OFF_SYNC..OFF_SYNC + 4].copy_from_slice(&CONTROL_BLOCK_SYNC);
        bytes[OFF_VERSION] = self.version;
        bytes[OFF_CAPS] = self.caps;
        write_u32(&mut bytes, OFF_BUFFER_ID, self.buffer_id);
        write_u32(&mut bytes, OFF_BUFFER_BYTES, self.buffer_bytes);
        write_u32(&mut bytes, OFF_SEQ_LOCK, seq_lock);
        write_u64(&mut bytes, OFF_HEAD_ABS, self.head_abs);
        write_u64(&mut bytes, OFF_OLDEST_ABS, self.oldest_abs);
        write_u64(&mut bytes, OFF_LOST_COUNT, self.lost_count);
        write_u64(&mut bytes, OFF_RUN_ID, self.run_id);
        write_u64(&mut bytes, OFF_EPOCH_ID, self.epoch_id);
        write_u64(&mut bytes, OFF_EPOCH_FIRST_ABS, self.epoch_first_abs);
        bytes[OFF_DEFINITION_HASH..OFF_DEFINITION_HASH + 8].copy_from_slice(&self.definition_hash);
        bytes[OFF_PREV_DEFINITION_HASH..OFF_PREV_DEFINITION_HASH + 8]
            .copy_from_slice(&self.prev_definition_hash);
        bytes
    }

    /// Decodes a snapshot when the caller observed `SeqLock` before and after the byte read.
    pub fn decode_with_locks(
        bytes: &[u8],
        first_seq_lock: u32,
        second_seq_lock: u32,
    ) -> Result<Self, ControlBlockError> {
        if bytes.len() < CONTROL_BLOCK_LEN {
            return Err(ControlBlockError::Truncated {
                needed: CONTROL_BLOCK_LEN,
                available: bytes.len(),
            });
        }
        if bytes[OFF_SYNC..OFF_SYNC + 4] != CONTROL_BLOCK_SYNC {
            return Err(ControlBlockError::WrongSync);
        }
        if !first_seq_lock.is_multiple_of(2) {
            return Err(ControlBlockError::Updating {
                seq_lock: first_seq_lock,
            });
        }
        if first_seq_lock != second_seq_lock {
            return Err(ControlBlockError::StaleSnapshot {
                first: first_seq_lock,
                second: second_seq_lock,
            });
        }
        if read_u32(bytes, OFF_SEQ_LOCK) != first_seq_lock {
            return Err(ControlBlockError::StaleSnapshot {
                first: first_seq_lock,
                second: read_u32(bytes, OFF_SEQ_LOCK),
            });
        }
        if bytes[OFF_RESERVED..OFF_RESERVED + 2] != [0, 0]
            || bytes[OFF_RESERVED2..OFF_RESERVED2 + 4] != [0, 0, 0, 0]
        {
            return Err(ControlBlockError::ReservedNonZero);
        }

        let mut definition_hash = [0; 8];
        definition_hash.copy_from_slice(&bytes[OFF_DEFINITION_HASH..OFF_DEFINITION_HASH + 8]);
        let mut prev_definition_hash = [0; 8];
        prev_definition_hash
            .copy_from_slice(&bytes[OFF_PREV_DEFINITION_HASH..OFF_PREV_DEFINITION_HASH + 8]);

        Ok(Self {
            version: bytes[OFF_VERSION],
            caps: bytes[OFF_CAPS],
            buffer_id: read_u32(bytes, OFF_BUFFER_ID),
            buffer_bytes: read_u32(bytes, OFF_BUFFER_BYTES),
            head_abs: read_u64(bytes, OFF_HEAD_ABS),
            oldest_abs: read_u64(bytes, OFF_OLDEST_ABS),
            lost_count: read_u64(bytes, OFF_LOST_COUNT),
            run_id: read_u64(bytes, OFF_RUN_ID),
            epoch_id: read_u64(bytes, OFF_EPOCH_ID),
            epoch_first_abs: read_u64(bytes, OFF_EPOCH_FIRST_ABS),
            definition_hash,
            prev_definition_hash,
        })
    }

    /// True if the absolute record start has already fallen behind `OldestAbs`.
    pub fn overwrote_record_at(&self, record_abs: u64) -> bool {
        self.oldest_abs > record_abs
    }
}

/// Errors from reading the shared-memory control block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlBlockError {
    /// Fewer than 88 bytes were available.
    Truncated {
        /// Number of bytes a control block requires (88).
        needed: usize,
        /// Number of bytes actually available.
        available: usize,
    },
    /// The sync bytes were not `OOT2`.
    WrongSync,
    /// The producer was updating the snapshot.
    Updating {
        /// The odd seqlock value observed; an odd value means a write is in progress.
        seq_lock: u32,
    },
    /// The two observed seqlock values differed.
    StaleSnapshot {
        /// The seqlock value read before the snapshot fields.
        first: u32,
        /// The seqlock value read after the snapshot fields; a write landed in between.
        second: u32,
    },
    /// Reserved bytes were non-zero.
    ReservedNonZero,
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("u32 field"))
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("u64 field"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_block_positive_image_round_trips() {
        let snapshot = fixture_snapshot();
        let bytes = snapshot.encode(2);
        assert_eq!(bytes.len(), CONTROL_BLOCK_LEN);
        assert_eq!(
            ControlBlockSnapshot::decode_with_locks(&bytes, 2, 2).unwrap(),
            snapshot
        );
    }

    #[test]
    fn control_block_rejects_torn_or_stale_snapshots() {
        let snapshot = fixture_snapshot();
        let mut updating = snapshot.encode(2);
        write_u32(&mut updating, OFF_SEQ_LOCK, 3);
        assert_eq!(
            ControlBlockSnapshot::decode_with_locks(&updating, 3, 3),
            Err(ControlBlockError::Updating { seq_lock: 3 })
        );

        let stable = snapshot.encode(4);
        assert_eq!(
            ControlBlockSnapshot::decode_with_locks(&stable, 4, 6),
            Err(ControlBlockError::StaleSnapshot {
                first: 4,
                second: 6
            })
        );
    }

    #[test]
    fn control_block_reports_overwritten_absolute_record() {
        let mut snapshot = fixture_snapshot();
        snapshot.oldest_abs = 256;
        assert!(snapshot.overwrote_record_at(128));
        assert!(!snapshot.overwrote_record_at(256));
    }

    fn fixture_snapshot() -> ControlBlockSnapshot {
        ControlBlockSnapshot {
            version: 2,
            caps: 0b0000_0011,
            buffer_id: 7,
            buffer_bytes: 4096,
            head_abs: 8192,
            oldest_abs: 6144,
            lost_count: 12,
            run_id: 3,
            epoch_id: 5,
            epoch_first_abs: 7000,
            definition_hash: *b"defhash1",
            prev_definition_hash: *b"oldhash1",
        }
    }
}

#[cfg(all(test, loom))]
mod loom_tests {
    use loom::sync::Arc;
    use loom::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering, fence};
    use loom::thread;

    #[test]
    fn seqlock_publish_rejects_overwritten_old_cursor() {
        loom::model(|| {
            let model = Arc::new(Model::new());
            model.seed_old_record();

            let writer = model.clone();
            let writer_thread = thread::spawn(move || {
                writer.seq_lock.fetch_add(1, Ordering::Relaxed);
                fence(Ordering::Release);
                writer.oldest_abs.store(4, Ordering::Relaxed);
                fence(Ordering::Release);
                writer.write_new_record();
                writer.head_abs.store(8, Ordering::Relaxed);
                writer.seq_lock.fetch_add(1, Ordering::Release);
            });

            let reader = model.clone();
            let reader_thread = thread::spawn(move || {
                if let Some(bytes) = reader.try_read_cursor_zero() {
                    assert_eq!(bytes, OLD);
                }
            });

            writer_thread.join().unwrap();
            reader_thread.join().unwrap();
        });
    }

    const OLD: [u8; 4] = [0xA5, 4, 0x10, 0xB1];
    const NEW: [u8; 4] = [0xA5, 4, 0x20, 0x81];

    struct Model {
        seq_lock: AtomicU32,
        head_abs: AtomicU64,
        oldest_abs: AtomicU64,
        bytes: [AtomicU8; 4],
    }

    impl Model {
        fn new() -> Self {
            Self {
                seq_lock: AtomicU32::new(0),
                head_abs: AtomicU64::new(0),
                oldest_abs: AtomicU64::new(0),
                bytes: std::array::from_fn(|_| AtomicU8::new(0)),
            }
        }

        fn seed_old_record(&self) {
            for (index, byte) in OLD.iter().enumerate() {
                self.bytes[index].store(*byte, Ordering::Relaxed);
            }
            self.head_abs.store(4, Ordering::Release);
        }

        fn write_new_record(&self) {
            for (index, byte) in NEW.iter().enumerate() {
                self.bytes[index].store(*byte, Ordering::Relaxed);
                thread::yield_now();
            }
        }

        fn try_read_cursor_zero(&self) -> Option<[u8; 4]> {
            let v0 = self.seq_lock.load(Ordering::Acquire);
            if !v0.is_multiple_of(2) {
                return None;
            }
            let head = self.head_abs.load(Ordering::Relaxed);
            let oldest_before = self.oldest_abs.load(Ordering::Relaxed);
            let v1 = self.seq_lock.load(Ordering::Acquire);
            if v0 != v1 || head == 0 || oldest_before > 0 {
                return None;
            }

            let mut bytes = [0; 4];
            for (index, byte) in bytes.iter_mut().enumerate() {
                *byte = self.bytes[index].load(Ordering::Relaxed);
                thread::yield_now();
            }
            fence(Ordering::Acquire);
            let oldest_after = self.oldest_abs.load(Ordering::Relaxed);
            if oldest_after > 0 || !valid_record(bytes) {
                return None;
            }
            Some(bytes)
        }
    }

    fn valid_record(bytes: [u8; 4]) -> bool {
        bytes[0] == 0xA5 && bytes[1] == 4 && bytes[3] == (bytes[0] ^ bytes[1] ^ bytes[2])
    }
}
