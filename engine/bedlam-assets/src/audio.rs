//! Audio helpers: wrap raw 8-bit mono PCM in a canonical 44-byte WAV header.

/// Wrap `pcm` in a WAV container (44-byte RIFF header, 8-bit unsigned mono).
/// The sample rate is unverified (pending EXD HMI init check); callers pass
/// 11025 today, matching the legacy tool.
pub fn wav_wrap(pcm: &[u8], rate: u32) -> Vec<u8> {
    let mut wav: Vec<u8> = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&((36 + pcm.len()) as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // pcm
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes()); // byte rate
    wav.extend_from_slice(&1u16.to_le_bytes()); // block align
    wav.extend_from_slice(&8u16.to_le_bytes()); // bits
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    wav.extend_from_slice(pcm);
    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_layout() {
        let w = wav_wrap(&[1, 2, 3], 11025);
        assert_eq!(w.len(), 44 + 3);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[4..8], &39u32.to_le_bytes());
        assert_eq!(&w[8..12], b"WAVE");
        assert_eq!(&w[12..16], b"fmt ");
        assert_eq!(&w[16..20], &16u32.to_le_bytes());
        assert_eq!(&w[20..22], &1u16.to_le_bytes());
        assert_eq!(&w[22..24], &1u16.to_le_bytes());
        assert_eq!(&w[24..28], &11025u32.to_le_bytes());
        assert_eq!(&w[28..32], &11025u32.to_le_bytes());
        assert_eq!(&w[32..34], &1u16.to_le_bytes());
        assert_eq!(&w[34..36], &8u16.to_le_bytes());
        assert_eq!(&w[36..40], b"data");
        assert_eq!(&w[40..44], &3u32.to_le_bytes());
        assert_eq!(&w[44..], &[1, 2, 3]);
    }

    #[test]
    fn empty_pcm() {
        let w = wav_wrap(&[], 22050);
        assert_eq!(w.len(), 44);
        assert_eq!(&w[4..8], &36u32.to_le_bytes());
        assert_eq!(&w[40..44], &0u32.to_le_bytes());
    }
}
