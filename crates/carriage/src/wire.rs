//! Binary wire record: encode and decode.
//!
//! A record is a 40-byte little-endian header (`Sync` = `OOT2`, `TotalRecordLength`,
//! `Flags`, `SourceTime`, `RunId`, `Seq`, `SourceId`, `EventTypeId`), zero or more
//! 4-byte-aligned TLV [`Slot`]s, and an optional CRC-32C trailer flagged by
//! [`FLAG_HAS_CRC`]. [`decode`] validates `Sync`, length, slot padding, and CRC.

use crate::crc::crc32c;

/// Fixed record header length in bytes.
pub const HEADER_LEN: usize = 40;
/// CRC-32C trailer length in bytes.
pub const CRC_LEN: usize = 4;
/// Sync marker at the start of every record: ASCII `OOT2`.
pub const SYNC: [u8; 4] = *b"OOT2";

/// Flag bit 0: `SourceTime` is not synchronized to a trusted clock.
pub const FLAG_TIME_UNSYNCED: u16 = 1 << 0;
/// Flag bit 1: record was synthesized downstream (e.g. an inferred loss marker).
pub const FLAG_SYNTHETIC: u16 = 1 << 1;
/// Flag bit 2: payload was truncated to fit.
pub const FLAG_PARTIAL_PAYLOAD: u16 = 1 << 2;
/// Flag bit 3: a CRC-32C trailer is present.
pub const FLAG_HAS_CRC: u16 = 1 << 3;

/// A TLV value slot: a `u16` key, a 1-byte type tag, and a length-prefixed payload.
///
/// On the wire each slot is `key (2) | ty (1) | len (1) | payload | zero-padding` to the
/// next 4-byte boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    /// Value key identifier.
    pub key: u16,
    /// Type tag for the payload (interpreted by the definition file, not by this crate).
    pub ty: u8,
    /// Raw little-endian payload bytes (at most 255).
    pub payload: Vec<u8>,
}

impl Slot {
    /// Builds a slot from a key, type tag, and payload.
    pub fn new(key: u16, ty: u8, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            key,
            ty,
            payload: payload.into(),
        }
    }

    fn encoded_len(&self) -> Result<usize, WireError> {
        if self.payload.len() > u8::MAX as usize {
            return Err(WireError::SlotTooLong {
                key: self.key,
                len: self.payload.len(),
            });
        }
        Ok(4 + self.payload.len() + padding_len(self.payload.len()))
    }

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), WireError> {
        let len = u8::try_from(self.payload.len()).map_err(|_| WireError::SlotTooLong {
            key: self.key,
            len: self.payload.len(),
        })?;
        write_u16(out, self.key);
        out.push(self.ty);
        out.push(len);
        out.extend_from_slice(&self.payload);
        out.extend(std::iter::repeat_n(0, padding_len(self.payload.len())));
        Ok(())
    }
}

/// A decoded or to-be-encoded event record: header fields plus TLV slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Record flags (see the `FLAG_*` constants).
    pub flags: u16,
    /// Source-local timestamp.
    pub source_time: u64,
    /// Run identifier; changes on a cold start.
    pub run_id: u64,
    /// Per-source sequence number.
    pub seq: u64,
    /// Source identifier (`0` is the system source).
    pub source_id: u32,
    /// Event type identifier.
    pub event_type_id: u32,
    /// TLV value slots.
    pub slots: Vec<Slot>,
}

impl Record {
    /// Creates a record with no flags and no slots.
    pub fn new(
        source_time: u64,
        run_id: u64,
        seq: u64,
        source_id: u32,
        event_type_id: u32,
    ) -> Self {
        Self {
            flags: 0,
            source_time,
            run_id,
            seq,
            source_id,
            event_type_id,
            slots: Vec::new(),
        }
    }

    /// Encodes the record to wire bytes, appending a CRC-32C trailer when `with_crc`.
    ///
    /// Sets or clears [`FLAG_HAS_CRC`] to match. Errors if a slot or the whole record
    /// exceeds its length field.
    pub fn encode(&self, with_crc: bool) -> Result<Vec<u8>, WireError> {
        let mut slots_len = 0usize;
        for slot in &self.slots {
            slots_len += slot.encoded_len()?;
        }

        let total_len = HEADER_LEN + slots_len + usize::from(with_crc) * CRC_LEN;
        if total_len > u16::MAX as usize {
            return Err(WireError::RecordTooLong { len: total_len });
        }

        let flags = if with_crc {
            self.flags | FLAG_HAS_CRC
        } else {
            self.flags & !FLAG_HAS_CRC
        };

        let mut out = Vec::with_capacity(total_len);
        out.extend_from_slice(&SYNC);
        write_u16(&mut out, total_len as u16);
        write_u16(&mut out, flags);
        write_u64(&mut out, self.source_time);
        write_u64(&mut out, self.run_id);
        write_u64(&mut out, self.seq);
        write_u32(&mut out, self.source_id);
        write_u32(&mut out, self.event_type_id);

        for slot in &self.slots {
            slot.encode_into(&mut out)?;
        }

        if with_crc {
            let crc = crc32c(&out);
            write_u32(&mut out, crc);
        }

        debug_assert_eq!(out.len(), total_len);
        Ok(out)
    }
}

/// A decoded record plus the number of bytes it consumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedRecord {
    /// The decoded record.
    pub record: Record,
    /// Total bytes consumed (the record's `TotalRecordLength`).
    pub consumed: usize,
}

/// Errors from wire-level decoding and encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// The trailer CRC did not match the recomputed CRC.
    CrcMismatch {
        /// CRC stored in the record trailer.
        expected: u32,
        /// CRC recomputed over the record bytes.
        actual: u32,
    },
    /// `TotalRecordLength` was smaller than a header (or smaller than header + CRC).
    InvalidLength {
        /// `TotalRecordLength` read from the header.
        total_len: usize,
        /// Bytes actually available to the decoder.
        available: usize,
    },
    /// Slot padding bytes were non-zero at `offset`.
    InvalidPadding {
        /// Byte offset of the first non-zero padding byte.
        offset: usize,
    },
    /// A slot was malformed at `offset`.
    InvalidSlot {
        /// Byte offset of the malformed slot.
        offset: usize,
    },
    /// The record's total length exceeds the `u16` length field.
    RecordTooLong {
        /// Encoded record length that overflowed `u16`.
        len: usize,
    },
    /// A slot payload exceeds the `u8` length field.
    SlotTooLong {
        /// Value key of the offending slot.
        key: u16,
        /// Payload length that overflowed `u8`.
        len: usize,
    },
    /// Fewer bytes were available than the record needs.
    Truncated {
        /// Bytes the record requires.
        needed: usize,
        /// Bytes actually available.
        available: usize,
    },
    /// The leading bytes were not [`SYNC`].
    WrongSync,
}

/// Decodes one record from the front of `bytes`, validating sync, length, padding, and CRC.
pub fn decode(bytes: &[u8]) -> Result<DecodedRecord, WireError> {
    if bytes.len() < HEADER_LEN {
        return Err(WireError::Truncated {
            needed: HEADER_LEN,
            available: bytes.len(),
        });
    }
    if bytes[..4] != SYNC {
        return Err(WireError::WrongSync);
    }

    let total_len = read_u16(bytes, 4) as usize;
    if total_len < HEADER_LEN {
        return Err(WireError::InvalidLength {
            total_len,
            available: bytes.len(),
        });
    }
    if total_len > bytes.len() {
        return Err(WireError::Truncated {
            needed: total_len,
            available: bytes.len(),
        });
    }

    let flags = read_u16(bytes, 6);
    let has_crc = flags & FLAG_HAS_CRC != 0;
    if has_crc && total_len < HEADER_LEN + CRC_LEN {
        return Err(WireError::InvalidLength {
            total_len,
            available: bytes.len(),
        });
    }

    let slots_end = if has_crc {
        let trailer = total_len - CRC_LEN;
        let expected = read_u32(bytes, trailer);
        let actual = crc32c(&bytes[..trailer]);
        if expected != actual {
            return Err(WireError::CrcMismatch { expected, actual });
        }
        trailer
    } else {
        total_len
    };

    let mut slots = Vec::new();
    let mut offset = HEADER_LEN;
    while offset < slots_end {
        if slots_end - offset < 4 {
            return Err(WireError::InvalidSlot { offset });
        }

        let key = read_u16(bytes, offset);
        let ty = bytes[offset + 2];
        let len = usize::from(bytes[offset + 3]);
        let payload_start = offset + 4;
        let payload_end = payload_start + len;
        if payload_end > slots_end {
            return Err(WireError::InvalidSlot { offset });
        }

        let padding = padding_len(len);
        let next = payload_end + padding;
        if next > slots_end {
            return Err(WireError::InvalidSlot { offset });
        }
        for (i, byte) in bytes[payload_end..next].iter().enumerate() {
            if *byte != 0 {
                return Err(WireError::InvalidPadding {
                    offset: payload_end + i,
                });
            }
        }

        slots.push(Slot {
            key,
            ty,
            payload: bytes[payload_start..payload_end].to_vec(),
        });
        offset = next;
    }

    Ok(DecodedRecord {
        record: Record {
            flags,
            source_time: read_u64(bytes, 8),
            run_id: read_u64(bytes, 16),
            seq: read_u64(bytes, 24),
            source_id: read_u32(bytes, 32),
            event_type_id: read_u32(bytes, 36),
            slots,
        },
        consumed: total_len,
    })
}

fn padding_len(payload_len: usize) -> usize {
    (4 - (payload_len % 4)) % 4
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{
        EVENT_RECORDS_DROPPED, KEY_DROPPED_COUNT, KEY_FIRST_LOST_SEQ, KEY_LAST_LOST_SEQ, TY_UDINT,
        TY_UINT, TY_ULINT,
    };

    #[test]
    fn wire_round_trip_without_crc() {
        let mut record = Record::new(1, 2, 3, 4, 5);
        record.flags = FLAG_TIME_UNSYNCED;
        record.slots.push(Slot::new(0x1001, TY_UINT, [0x34, 0x12]));

        let bytes = record.encode(false).unwrap();
        let decoded = decode(&bytes).unwrap();

        assert_eq!(decoded.consumed, bytes.len());
        assert_eq!(decoded.record.flags & FLAG_HAS_CRC, 0);
        assert_eq!(decoded.record, record);
    }

    #[test]
    fn wire_round_trip_with_crc() {
        let mut record = Record::new(10, 11, 12, 13, 14);
        record
            .slots
            .push(Slot::new(0x0001, TY_ULINT, 123u64.to_le_bytes()));
        record
            .slots
            .push(Slot::new(0x0002, TY_UDINT, 99u32.to_le_bytes()));

        let bytes = record.encode(true).unwrap();
        let decoded = decode(&bytes).unwrap();

        assert_eq!(decoded.consumed, bytes.len());
        assert_eq!(decoded.record.flags & FLAG_HAS_CRC, FLAG_HAS_CRC);
        assert_eq!(decoded.record.seq, record.seq);
        assert_eq!(decoded.record.slots, record.slots);
    }

    #[test]
    fn wire_crc_corruption_rejected() {
        let mut record = Record::new(10, 11, 12, 13, 14);
        record
            .slots
            .push(Slot::new(0x0001, TY_ULINT, 123u64.to_le_bytes()));

        let mut bytes = record.encode(true).unwrap();
        bytes[HEADER_LEN + 4] ^= 0x55;

        assert!(matches!(decode(&bytes), Err(WireError::CrcMismatch { .. })));
    }

    #[test]
    fn wire_length_bounds_rejected() {
        let record = Record::new(10, 11, 12, 13, 14);
        let mut too_short = record.encode(false).unwrap();
        too_short[4..6].copy_from_slice(&(HEADER_LEN as u16 - 1).to_le_bytes());
        assert!(matches!(
            decode(&too_short),
            Err(WireError::InvalidLength { .. })
        ));

        let mut too_long = record.encode(false).unwrap();
        too_long[4..6].copy_from_slice(&(1024u16).to_le_bytes());
        assert!(matches!(
            decode(&too_long),
            Err(WireError::Truncated { .. })
        ));
    }

    #[test]
    fn wire_state_transition_byte_vector() {
        let mut record = Record::new(0x0102_0304_0506_0708, 1, 2, 42, 0x0001);
        record
            .slots
            .push(Slot::new(0x0001, TY_UINT, 0x1234u16.to_le_bytes()));
        record
            .slots
            .push(Slot::new(0x0002, TY_UINT, 2u16.to_le_bytes()));

        let bytes = record.encode(true).unwrap();
        let expected = vec![
            0x4F, 0x4F, 0x54, 0x32, 0x3C, 0x00, 0x08, 0x00, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03,
            0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x03, 0x02, 0x34, 0x12, 0x00, 0x00, 0x02, 0x00, 0x03, 0x02, 0x02, 0x00, 0x00, 0x00,
            0xE1, 0x8E, 0xE0, 0x51,
        ];

        assert_eq!(bytes, expected);
        assert_eq!(decode(&bytes).unwrap().record, record_with_crc(record));
    }

    #[test]
    fn wire_records_dropped_byte_vector() {
        let mut record = Record::new(0, 9, 44, 42, EVENT_RECORDS_DROPPED);
        record
            .slots
            .push(Slot::new(KEY_DROPPED_COUNT, TY_UDINT, 5u32.to_le_bytes()));
        record.slots.push(Slot::new(
            KEY_FIRST_LOST_SEQ,
            TY_ULINT,
            100u64.to_le_bytes(),
        ));
        record
            .slots
            .push(Slot::new(KEY_LAST_LOST_SEQ, TY_ULINT, 104u64.to_le_bytes()));

        let bytes = record.encode(true).unwrap();
        let expected = vec![
            0x4F, 0x4F, 0x54, 0x32, 0x4C, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2C, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00, 0x16, 0x00,
            0x05, 0x04, 0x05, 0x00, 0x00, 0x00, 0x17, 0x00, 0x07, 0x08, 0x64, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x07, 0x08, 0x68, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x57, 0x48, 0xCC, 0xE3,
        ];

        assert_eq!(bytes, expected);
        assert_eq!(decode(&bytes).unwrap().record, record_with_crc(record));
    }

    fn record_with_crc(mut record: Record) -> Record {
        record.flags |= FLAG_HAS_CRC;
        record
    }
}
