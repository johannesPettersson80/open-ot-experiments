//! CRC-32C (Castagnoli), reflected, used for the optional record trailer.
//!
//! Bit-reflected polynomial `0x82F63B78`, init and final-xor `0xFFFFFFFF`. The
//! known answer is `crc32c("123456789") == 0xE3069283`.

const REFLECTED_POLY: u32 = 0x82F6_3B78;

/// Computes the CRC-32C (Castagnoli) checksum of `bytes`.
///
/// Uses the reflected polynomial `0x82F63B78` with init and final-xor `0xFFFFFFFF`,
/// matching the optional record trailer. `crc32c("123456789") == 0xE3069283`.
pub fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (REFLECTED_POLY & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::crc32c;

    #[test]
    fn crc32c_known_answer() {
        assert_eq!(crc32c(b"123456789"), 0xE306_9283);
    }

    #[test]
    fn crc32c_empty_input() {
        assert_eq!(crc32c(b""), 0);
    }
}
