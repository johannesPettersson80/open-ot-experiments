//! Shared-memory store for the live OpenOT carriage harness.
//!
//! The crate maps a `/dev/shm` file with a shared mutable mapping and exposes it as a
//! [`ConcurrentStore`]. It is a test harness, not the normative carriage API; the
//! mapping dependency stays here so `open-ot-carriage` remains dependency-light.

use std::fs::{File, OpenOptions};
use std::io;
use std::num::TryFromIntError;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering, fence};
use std::thread;
use std::time::Duration;

use memmap2::MmapMut;
use open_ot_carriage::concurrent::ConcurrentStore;
use open_ot_carriage::control::{
    CONTROL_BLOCK_LEN, CONTROL_BLOCK_SYNC, ControlBlockError, ControlBlockSnapshot,
    OFF_BUFFER_BYTES, OFF_BUFFER_ID, OFF_CAPS, OFF_DEFINITION_HASH, OFF_EPOCH_FIRST_ABS,
    OFF_EPOCH_ID, OFF_HEAD_ABS, OFF_LOST_COUNT, OFF_OLDEST_ABS, OFF_PREV_DEFINITION_HASH,
    OFF_RESERVED, OFF_RESERVED2, OFF_RUN_ID, OFF_SEQ_LOCK, OFF_SYNC, OFF_VERSION,
};
use open_ot_carriage::ring::DEFAULT_BUFFER_ID;

/// Byte offset where the ring byte pool begins.
pub const DATA_OFFSET: usize = align_up(CONTROL_BLOCK_LEN, 64);

const FILE_MODE: u32 = 0o600;
const CONTROL_SNAPSHOT_RETRIES: usize = 1_000;

/// Fence behavior selected for the shared-memory store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceMode {
    /// Use the release/acquire fences required by the carriage protocol.
    Fenced,
    /// Deliberately no-op the release/acquire fences for A/B stress testing.
    Unfenced,
}

impl FenceMode {
    /// Parses a command/report value.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "fenced" => Ok(Self::Fenced),
            "unfenced" => Ok(Self::Unfenced),
            _ => Err(format!(
                "invalid fence mode {value}; expected fenced or unfenced"
            )),
        }
    }

    /// Stable lowercase label for command/report output.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fenced => "fenced",
            Self::Unfenced => "unfenced",
        }
    }
}

/// A cloneable [`ConcurrentStore`] implementation over a shared mmap region.
#[derive(Debug, Clone)]
pub struct SharedConcurrentStore {
    region: Arc<SharedRegion>,
    capacity: usize,
    fence_mode: FenceMode,
    recheck_stall: Duration,
}

impl SharedConcurrentStore {
    /// Creates or truncates `path` and maps it as a zeroed shared ring of `capacity` bytes.
    pub fn create(path: impl AsRef<Path>, capacity: usize) -> io::Result<Self> {
        Self::create_with_mode(path, capacity, FenceMode::Fenced)
    }

    /// Creates or truncates `path` and maps it with the selected fence behavior.
    pub fn create_with_mode(
        path: impl AsRef<Path>,
        capacity: usize,
        fence_mode: FenceMode,
    ) -> io::Result<Self> {
        validate_capacity(capacity)?;
        validate_layout();
        let len = mapping_len(capacity)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .mode(FILE_MODE)
            .open(path)?;
        file.set_len(u64::try_from(len).map_err(int_error)?)?;
        let store = Self {
            region: Arc::new(SharedRegion::map(&file, len)?),
            capacity,
            fence_mode,
            recheck_stall: Duration::ZERO,
        };
        store.initialize_counters();
        Ok(store)
    }

    /// Opens an existing mapped ring at `path`, inferring capacity from the file length.
    pub fn open_existing(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::open_existing_with_mode(path, FenceMode::Fenced)
    }

    /// Opens an existing mapped ring at `path` with the selected fence behavior.
    pub fn open_existing_with_mode(
        path: impl AsRef<Path>,
        fence_mode: FenceMode,
    ) -> io::Result<Self> {
        validate_layout();
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let len = usize::try_from(file.metadata()?.len()).map_err(int_error)?;
        if len < DATA_OFFSET {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "shared mapping shorter than control block",
            ));
        }
        let capacity = len - DATA_OFFSET;
        validate_capacity(capacity)?;
        Ok(Self {
            region: Arc::new(SharedRegion::map(&file, len)?),
            capacity,
            fence_mode,
            recheck_stall: Duration::ZERO,
        })
    }

    /// Returns the total mmap length for this store.
    pub fn mapping_len(&self) -> usize {
        self.region.len
    }

    /// Returns a copy that stalls between byte copy and oldest-offset recheck.
    pub fn with_recheck_stall(mut self, stall: Duration) -> Self {
        self.recheck_stall = stall;
        self
    }

    /// Returns the selected fence behavior.
    pub fn fence_mode(&self) -> FenceMode {
        self.fence_mode
    }

    fn initialize_counters(&self) {
        for offset in 0..DATA_OFFSET {
            self.atomic_u8_at(offset).store(0, Ordering::Relaxed);
        }
        for (index, byte) in CONTROL_BLOCK_SYNC.iter().enumerate() {
            self.atomic_u8_at(OFF_SYNC + index)
                .store(*byte, Ordering::Relaxed);
        }
        self.atomic_u8_at(OFF_VERSION).store(2, Ordering::Relaxed);
        self.atomic_u8_at(OFF_CAPS).store(0, Ordering::Relaxed);
        self.atomic_u32(OFF_BUFFER_ID)
            .store(DEFAULT_BUFFER_ID, Ordering::Relaxed);
        self.atomic_u32(OFF_BUFFER_BYTES).store(
            u32::try_from(self.capacity).unwrap_or(u32::MAX),
            Ordering::Relaxed,
        );
        self.atomic_u32(OFF_SEQ_LOCK).store(0, Ordering::Relaxed);
        self.atomic_u64(OFF_HEAD_ABS).store(0, Ordering::Relaxed);
        self.atomic_u64(OFF_OLDEST_ABS).store(0, Ordering::Relaxed);
        self.atomic_u64(OFF_LOST_COUNT).store(0, Ordering::Relaxed);
        self.atomic_u64(OFF_RUN_ID).store(0, Ordering::Relaxed);
        self.atomic_u64(OFF_EPOCH_ID).store(0, Ordering::Relaxed);
        self.atomic_u64(OFF_EPOCH_FIRST_ABS)
            .store(0, Ordering::Relaxed);
        fence(Ordering::Release);
    }

    fn atomic_u8_at(&self, offset: usize) -> &AtomicU8 {
        debug_assert!(offset < self.region.len);
        // SAFETY: byte atomics require alignment 1; the offset is within the live
        // mmap; after initialization the harness accesses shared bytes only through
        // atomic views.
        unsafe { AtomicU8::from_ptr(self.region.ptr.as_ptr().add(offset)) }
    }

    fn atomic_u32(&self, offset: usize) -> &AtomicU32 {
        debug_assert_eq!(offset % align_of::<u32>(), 0);
        debug_assert!(offset + size_of::<u32>() <= self.region.len);
        // SAFETY: `SharedRegion` owns a live MAP_SHARED mapping for at least the
        // returned reference lifetime; the offsets are 4-aligned and in bounds; all
        // access to these bytes after initialization is atomic.
        unsafe { AtomicU32::from_ptr(self.region.ptr.as_ptr().add(offset).cast::<u32>()) }
    }

    fn atomic_u64(&self, offset: usize) -> &AtomicU64 {
        debug_assert_eq!(offset % align_of::<u64>(), 0);
        debug_assert!(offset + size_of::<u64>() <= self.region.len);
        // SAFETY: `SharedRegion` owns a live MAP_SHARED mapping for at least the
        // returned reference lifetime; the offsets are 8-aligned and within the
        // mapping; all access to these bytes after initialization goes through
        // atomic operations. On aarch64, aligned 64-bit atomics over shared physical
        // pages are real inter-process atomics on one machine. The harness is
        // single-writer/single-reader, matching the carriage protocol.
        unsafe { AtomicU64::from_ptr(self.region.ptr.as_ptr().add(offset).cast::<u64>()) }
    }

    fn atomic_u8(&self, phys: usize) -> &AtomicU8 {
        debug_assert!(phys < self.capacity);
        let offset = DATA_OFFSET + phys;
        self.atomic_u8_at(offset)
    }

    fn read_u64_bytes(&self, bytes: &mut [u8; CONTROL_BLOCK_LEN], offset: usize) {
        bytes[offset..offset + size_of::<u64>()].copy_from_slice(
            &self
                .atomic_u64(offset)
                .load(Ordering::Relaxed)
                .to_le_bytes(),
        );
    }

    fn read_u32_bytes(&self, bytes: &mut [u8; CONTROL_BLOCK_LEN], offset: usize) {
        bytes[offset..offset + size_of::<u32>()].copy_from_slice(
            &self
                .atomic_u32(offset)
                .load(Ordering::Relaxed)
                .to_le_bytes(),
        );
    }

    fn read_control_image(&self, seq_lock: u32) -> [u8; CONTROL_BLOCK_LEN] {
        let mut bytes = [0; CONTROL_BLOCK_LEN];
        for (offset, byte) in bytes.iter_mut().enumerate() {
            *byte = self.atomic_u8_at(offset).load(Ordering::Relaxed);
        }
        bytes[OFF_SEQ_LOCK..OFF_SEQ_LOCK + size_of::<u32>()]
            .copy_from_slice(&seq_lock.to_le_bytes());
        self.read_u32_bytes(&mut bytes, OFF_BUFFER_ID);
        self.read_u32_bytes(&mut bytes, OFF_BUFFER_BYTES);
        self.read_u64_bytes(&mut bytes, OFF_HEAD_ABS);
        self.read_u64_bytes(&mut bytes, OFF_OLDEST_ABS);
        self.read_u64_bytes(&mut bytes, OFF_LOST_COUNT);
        self.read_u64_bytes(&mut bytes, OFF_RUN_ID);
        self.read_u64_bytes(&mut bytes, OFF_EPOCH_ID);
        self.read_u64_bytes(&mut bytes, OFF_EPOCH_FIRST_ABS);
        bytes
    }
}

impl ConcurrentStore for SharedConcurrentStore {
    fn capacity(&self) -> usize {
        self.capacity
    }

    fn load_byte_relaxed(&self, phys: usize) -> u8 {
        self.atomic_u8(phys).load(Ordering::Relaxed)
    }

    fn store_byte_relaxed(&self, phys: usize, value: u8) {
        self.atomic_u8(phys).store(value, Ordering::Relaxed);
    }

    fn load_head_acquire(&self) -> u64 {
        self.atomic_u64(OFF_HEAD_ABS).load(Ordering::Acquire)
    }

    fn store_head_release(&self, value: u64) {
        self.atomic_u64(OFF_HEAD_ABS)
            .store(value, Ordering::Release);
    }

    fn load_oldest_acquire(&self) -> u64 {
        self.atomic_u64(OFF_OLDEST_ABS).load(Ordering::Acquire)
    }

    fn load_oldest_relaxed(&self) -> u64 {
        self.atomic_u64(OFF_OLDEST_ABS).load(Ordering::Relaxed)
    }

    fn store_oldest_relaxed(&self, value: u64) {
        self.atomic_u64(OFF_OLDEST_ABS)
            .store(value, Ordering::Relaxed);
    }

    fn fetch_add_lost_release(&self, value: u64) {
        self.atomic_u64(OFF_LOST_COUNT)
            .fetch_add(value, Ordering::Release);
    }

    fn load_lost_acquire(&self) -> u64 {
        self.atomic_u64(OFF_LOST_COUNT).load(Ordering::Acquire)
    }

    fn begin_control_update(&self) {
        let previous = self
            .atomic_u32(OFF_SEQ_LOCK)
            .fetch_add(1, Ordering::Relaxed);
        debug_assert!(previous.is_multiple_of(2));
        fence(Ordering::Release);
    }

    fn end_control_update_release(&self) {
        let previous = self
            .atomic_u32(OFF_SEQ_LOCK)
            .fetch_add(1, Ordering::Release);
        debug_assert!(!previous.is_multiple_of(2));
    }

    fn read_control_snapshot(&self) -> Result<ControlBlockSnapshot, ControlBlockError> {
        let mut last_error = ControlBlockError::Updating { seq_lock: 1 };
        for _ in 0..CONTROL_SNAPSHOT_RETRIES {
            let first = self.atomic_u32(OFF_SEQ_LOCK).load(Ordering::Acquire);
            if !first.is_multiple_of(2) {
                last_error = ControlBlockError::Updating { seq_lock: first };
                std::hint::spin_loop();
                continue;
            }

            let bytes = self.read_control_image(first);
            let second = self.atomic_u32(OFF_SEQ_LOCK).load(Ordering::Acquire);
            match ControlBlockSnapshot::decode_with_locks(&bytes, first, second) {
                Ok(snapshot) => return Ok(snapshot),
                Err(error @ ControlBlockError::Updating { .. })
                | Err(error @ ControlBlockError::StaleSnapshot { .. }) => {
                    last_error = error;
                    std::hint::spin_loop();
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_error)
    }

    fn read_oldest_after_record(&self) -> Result<u64, ControlBlockError> {
        Ok(self.load_oldest_relaxed())
    }

    fn release_before_clobber(&self) {
        if self.fence_mode == FenceMode::Fenced {
            fence(Ordering::Release);
        }
    }

    fn acquire_before_recheck(&self) {
        if !self.recheck_stall.is_zero() {
            thread::sleep(self.recheck_stall);
        }
        if self.fence_mode == FenceMode::Fenced {
            fence(Ordering::Acquire);
        }
    }
}

/// Owns a live shared-memory mapping.
#[derive(Debug)]
pub struct SharedRegion {
    _mapping: MmapMut,
    ptr: NonNull<u8>,
    len: usize,
}

// SAFETY: `SharedRegion` only exposes checked atomic operations through
// `SharedConcurrentStore`; the raw mapping pointer is not exposed for non-atomic access.
unsafe impl Send for SharedRegion {}
// SAFETY: shared access is restricted to atomic operations and immutable metadata.
unsafe impl Sync for SharedRegion {}

impl SharedRegion {
    fn map(file: &File, len: usize) -> io::Result<Self> {
        if len == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot mmap zero bytes",
            ));
        }
        // SAFETY: the file is opened read/write and sized before mapping. `MmapMut`
        // creates a shared mutable file mapping on Unix; the mapping is owned by
        // `SharedRegion` and unmapped by `MmapMut` on drop.
        let mut mapping = unsafe { MmapMut::map_mut(file)? };
        let ptr = NonNull::new(mapping.as_mut_ptr())
            .ok_or_else(|| io::Error::other("mmap returned null"))?;
        Ok(Self {
            _mapping: mapping,
            ptr,
            len,
        })
    }
}

fn validate_capacity(capacity: usize) -> io::Result<()> {
    if capacity == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ring capacity must be non-zero",
        ));
    }
    Ok(())
}

fn validate_layout() {
    assert_eq!(CONTROL_BLOCK_LEN, 88);
    assert_eq!(OFF_SYNC, 0);
    assert_eq!(OFF_VERSION, 4);
    assert_eq!(OFF_CAPS, 5);
    assert_eq!(OFF_RESERVED, 6);
    assert_eq!(OFF_BUFFER_ID, 8);
    assert_eq!(OFF_BUFFER_BYTES, 12);
    assert_eq!(OFF_SEQ_LOCK, 16);
    assert_eq!(OFF_RESERVED2, 20);
    assert_eq!(OFF_HEAD_ABS % align_of::<u64>(), 0);
    assert_eq!(OFF_OLDEST_ABS % align_of::<u64>(), 0);
    assert_eq!(OFF_LOST_COUNT % align_of::<u64>(), 0);
    assert_eq!(OFF_RUN_ID % align_of::<u64>(), 0);
    assert_eq!(OFF_EPOCH_ID % align_of::<u64>(), 0);
    assert_eq!(OFF_EPOCH_FIRST_ABS % align_of::<u64>(), 0);
    assert_eq!(OFF_DEFINITION_HASH, 72);
    assert_eq!(OFF_PREV_DEFINITION_HASH, 80);
    assert_eq!(DATA_OFFSET % 64, 0);
}

fn mapping_len(capacity: usize) -> io::Result<usize> {
    DATA_OFFSET
        .checked_add(capacity)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "mapping length overflow"))
}

fn int_error(error: TryFromIntError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

const fn align_up(value: usize, alignment: usize) -> usize {
    value.div_ceil(alignment) * alignment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_layout_uses_control_block_offsets_and_aligned_data_pool() {
        validate_layout();
        assert_eq!(CONTROL_BLOCK_LEN, 88);
        assert_eq!(OFF_SEQ_LOCK, 16);
        assert_eq!(OFF_HEAD_ABS, 24);
        assert_eq!(OFF_OLDEST_ABS, 32);
        assert_eq!(OFF_LOST_COUNT, 40);
        assert_eq!(DATA_OFFSET, 128);
    }
}
