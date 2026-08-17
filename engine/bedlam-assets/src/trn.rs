//! 256-byte palette remap LUTs (.trn).

use crate::AssetsError;

/// Palette remap table: output index per input index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trn {
    pub lut: [u8; 256],
}

impl Trn {
    /// Byte-identical rebuild of the original file.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.lut.to_vec()
    }
}

/// Parse a 256-byte remap LUT. The buffer must be exactly 256 bytes.
pub fn parse_trn(data: &[u8]) -> Result<Trn, AssetsError> {
    if data.len() != 256 {
        return Err(AssetsError::WrongSize { len: data.len() });
    }
    let mut lut = [0u8; 256];
    lut.copy_from_slice(data);
    Ok(Trn { lut })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_rebuild() {
        let mut d = vec![0u8; 256];
        for (i, v) in d.iter_mut().enumerate() {
            *v = (255 - i) as u8;
        }
        let t = parse_trn(&d).unwrap();
        assert_eq!(t.lut[0], 255);
        assert_eq!(t.lut[255], 0);
        assert_eq!(t.to_bytes(), d);
    }

    #[test]
    fn rejects_wrong_sizes() {
        assert_eq!(
            parse_trn(&vec![0u8; 255]),
            Err(AssetsError::WrongSize { len: 255 })
        );
        assert_eq!(
            parse_trn(&vec![0u8; 257]),
            Err(AssetsError::WrongSize { len: 257 })
        );
        assert_eq!(parse_trn(&[]), Err(AssetsError::WrongSize { len: 0 }));
    }

    #[test]
    fn no_panic_on_randomish_input() {
        let mut s = 7u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        for len in [0usize, 1, 100, 256, 1000] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_trn(&d);
        }
    }
}
