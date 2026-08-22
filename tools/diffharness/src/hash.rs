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
}
