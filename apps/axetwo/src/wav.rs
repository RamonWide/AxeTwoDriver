//! Minimal WAV writer for 24-bit PCM.

use std::io::Write;

pub fn write(path: &str, rate: u32, channels: u16, audio: &[u8]) -> std::io::Result<()> {
    let frame = u32::from(channels) * 3;
    let data_len =
        u32::try_from(audio.len()).map_err(|_| std::io::Error::other("recording too long for a WAV file"))?;
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    f.write_all(b"RIFF")?;
    f.write_all(&data_len.saturating_add(36).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&channels.to_le_bytes())?;
    f.write_all(&rate.to_le_bytes())?;
    f.write_all(&(rate.saturating_mul(frame)).to_le_bytes())?;
    f.write_all(&(frame as u16).to_le_bytes())?;
    f.write_all(&24u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    f.write_all(audio)?;
    f.flush()
}
