//! Mirrored FNV-1a 64 — byte-identical to `bedlam-core`'s hash util.
//!
//! Why a mirror and not a dependency: `bedlam-core` pulls `thiserror`,
//! and this crate is deliberately ZERO-dependency so the registry/schema
//! guard builds and runs in any environment (incl. offline CI) without
//! touching the lock file's dependency graph (W2 crate charter, W3
//! ticket fallback clause). The mirror is pinned to the engine's public
//! expected outputs by `tests/dump_schema.rs::engine_hash_vectors` — if
//! either side drifts, that test fails loudly.
//!
//! Algorithm + the canonical little-endian multi-byte write rule are
//! copied verbatim from `engine/bedlam-core/src/hash.rs` (its `public_
//! vectors` / `write_u32_equals_four_le_bytes` tests are the anchor).

/// Incremental FNV-1a 64 hasher.
///
/// Multi-byte values are fed as canonical LITTLE-ENDIAN bytes (see the
/// `write_*` methods), matching the engine crate-wide serialization rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fnv1a64(u64);

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

impl Fnv1a64 {
    /// New hasher at the FNV-1a offset basis.
    pub fn new() -> Self {
        Fnv1a64(FNV_OFFSET_BASIS)
    }

    /// Mix one byte.
    pub fn write_u8(&mut self, v: u8) {
        self.0 = (self.0 ^ u64::from(v)).wrapping_mul(FNV_PRIME);
    }

    /// Mix a `u16` as its 2 canonical LE bytes.
    pub fn write_u16(&mut self, v: u16) {
        self.write_bytes(&v.to_le_bytes());
    }

    /// Mix a `u32` as its 4 canonical LE bytes.
    pub fn write_u32(&mut self, v: u32) {
        self.write_bytes(&v.to_le_bytes());
    }

    /// Mix a `u64` as its 8 canonical LE bytes.
    pub fn write_u64(&mut self, v: u64) {
        self.write_bytes(&v.to_le_bytes());
    }

    /// Mix a byte slice in order.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u8(b);
        }
    }

    /// Final hash value.
    pub fn finish(&self) -> u64 {
        self.0
    }
}

impl Default for Fnv1a64 {
    fn default() -> Self {
        Self::new()
    }
}

/// One-shot FNV-1a 64 over `bytes`.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = Fnv1a64::new();
    h.write_bytes(bytes);
    h.finish()
}

/// A dump digest value (per-frame digest or chain digest), formatted as
/// 16 lowercase hex digits — the same surface as the engine's `StateHash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DumpDigest(pub u64);

impl std::fmt::Display for DumpDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Self-contained SHA-256 (W4). Same zero-dependency charter as the FNV
/// mirror above: needed for the dump header's `build_sha256` (the watched
/// binary) and the per-dump manifest fingerprints, so it lives here rather
/// than pulling a hash crate into the lock file. Pinned to the standard
/// FIPS 180-4 vectors by the tests below.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    // Message padding: 0x80, zeros, then the 64-bit BIG-endian bit length.
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut msg = bytes.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 64];
    for block in msg.chunks_exact(64) {
        // First 16 words come from the block; 16..64 are the expansion.
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let s = i * 4;
            *word = u32::from_be_bytes([block[s], block[s + 1], block[s + 2], block[s + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Lowercase hex encoding (used for sha256 fingerprints in the manifest).
pub fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_vectors() {
        assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
        assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x85944171f73967e8);
    }

    #[test]
    fn write_u32_equals_four_le_bytes() {
        let mut a = Fnv1a64::new();
        a.write_u32(0xDEAD_BEEF);
        assert_eq!(a.finish(), fnv1a64(&[0xEF, 0xBE, 0xAD, 0xDE]));

        let mut b = Fnv1a64::new();
        b.write_u64(0x0102_0304_0506_0708);
        assert_eq!(
            b.finish(),
            fnv1a64(&[0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01])
        );
    }

    #[test]
    fn incremental_equals_one_shot() {
        let mut h = Fnv1a64::new();
        h.write_bytes(b"foo");
        h.write_bytes(b"bar");
        assert_eq!(h.finish(), fnv1a64(b"foobar"));
    }

    #[test]
    fn digest_formatting() {
        let d = DumpDigest(0xDEAD_BEEF);
        assert_eq!(d.to_string(), "00000000deadbeef");
    }

    #[test]
    fn sha256_fips_vectors() {
        assert_eq!(
            hex_lower(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex_lower(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex_lower(&sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // > one block (pads to 2 blocks): 'a' * 1000
        let long = vec![b'a'; 1000];
        assert_eq!(
            hex_lower(&sha256(&long)),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
    }

    #[test]
    fn hex_lower_matches_dump_bytes() {
        assert_eq!(hex_lower(&[0x00, 0x0f, 0xff]), "000fff");
    }
}
