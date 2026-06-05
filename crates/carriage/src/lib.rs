//! # OpenOT carriage prototype
//!
//! An experimental, dependency-light Rust crate that makes the proposed OpenOT
//! ring-buffer event-logging rules executable. It is **not** a normative standard;
//! its job is to turn the wire format, loss accounting, epoch model, and concurrent
//! publish/read protocol into code that can be tested, and to emit byte-exact
//! fixtures a future specification can be checked against.
//!
//! ## Modules
//!
//! - [`wire`] — the binary record: 40-byte header, TLV slots, optional CRC-32C trailer.
//! - [`ring`] — the single-buffer byte-pool ring: absolute offsets, wrap markers,
//!   overwrite detection, and producer-side loss tracking.
//! - [`loss`] — downstream loss accounting (seq gaps, authoritative `RecordsDropped`,
//!   source high-water checkpoints) and the reading consumers.
//! - [`control`] — 88-byte shared-memory control block and seqlock snapshot validation.
//! - [`epoch`] — run, epoch, definition-change, and source high-water producer logic.
//! - [`concurrent`] — the release/acquire publish/read protocol for a ring whose
//!   bytes a consumer may observe asynchronously.
//! - [`crc`] — CRC-32C (Castagnoli).
//! - [`vectors`] — deterministic conformance-vector generation.
//!
//! ## Loss model
//!
//! Completeness rests on three complementary signals, none sufficient alone: seq gaps
//! catch mid-stream loss, [`records_dropped_record`] reports producer-known evictions,
//! and source high-water (see [`epoch::EpochProducer::checkpoint_high_water`]) closes
//! the tail of a source that was dropped and then went silent.

pub mod concurrent;
pub mod consumer;
pub mod control;
pub mod crc;
pub mod epoch;
pub mod loss;
pub mod registry;
pub mod ring;
pub mod vectors;
pub mod wire;

pub use concurrent::{
    ConcurrentProducer, ConcurrentRawConsumer, ConcurrentRing, ConcurrentStore,
    OwnedConcurrentStore,
};
pub use consumer::{LossAccountingConsumer, RawByteConsumer};
pub use control::{CONTROL_BLOCK_LEN, ControlBlockError, ControlBlockSnapshot};
pub use crc::crc32c;
pub use epoch::{EpochError, EpochProducer, EpochResolver};
pub use loss::{LossEvent, records_dropped_record};
pub use ring::{LossRange, ReadBatch, ReadRecord, RingBuffer, RingError};
pub use wire::{DecodedRecord, FLAG_HAS_CRC, Record, Slot, WireError};
