//! 24-bit little-endian PCM, three bytes per sample, as the unit sends it.

/// Bytes one sample occupies on the wire.
pub const SAMPLE_BYTES: usize = 3;
/// The value of full scale.
pub const FULL_SCALE: f64 = 8_388_608.0;

/// Decodes one sample. Returns 0 when fewer than three bytes are given.
pub fn decode(bytes: &[u8]) -> i32 {
    match bytes {
        [a, b, c, ..] => (i32::from(*c as i8) << 16) | (i32::from(*b) << 8) | i32::from(*a),
        _ => 0,
    }
}

/// Encodes one sample, clamped to the 24-bit range. Does nothing when `out`
/// is shorter than three bytes.
pub fn encode(value: i32, out: &mut [u8]) {
    let v = value.clamp(-8_388_608, 8_388_607).to_le_bytes();
    if let Some(dst) = out.get_mut(..SAMPLE_BYTES) {
        dst.copy_from_slice(&v[..SAMPLE_BYTES]);
    }
}

/// Peak level in dB relative to full scale; negative infinity for silence.
pub fn dbfs(peak: u32) -> f64 {
    if peak == 0 {
        f64::NEG_INFINITY
    } else {
        20.0 * (f64::from(peak) / FULL_SCALE).log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_the_extremes() {
        for v in [0, 1, -1, 8_388_607, -8_388_608, 0x12_3456, -0x12_3456] {
            let mut b = [0u8; 3];
            encode(v, &mut b);
            assert_eq!(decode(&b), v);
        }
    }

    #[test]
    fn clamps_and_tolerates_short_buffers() {
        let mut b = [0u8; 3];
        encode(i32::MAX, &mut b);
        assert_eq!(decode(&b), 8_388_607);
        encode(i32::MIN, &mut b);
        assert_eq!(decode(&b), -8_388_608);
        encode(5, &mut [0u8; 2]);
        assert_eq!(decode(&[1, 2]), 0);
    }

    #[test]
    fn full_scale_is_zero_db() {
        assert!(dbfs(8_388_608).abs() < 1e-9);
        assert!(dbfs(0).is_infinite());
    }
}
