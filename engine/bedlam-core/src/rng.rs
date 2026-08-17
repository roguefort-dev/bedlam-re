//! Seeded PCG32 PRNG (XSH-RR 64/32 output, 64-bit LCG state).
//!
//! Charter (docs/PLAN.md sec 7): our own seeded PRNG, statistically matched
//! to the original later; the original bit-stream is deliberately NOT
//! mirrored (parity tier T3). Deterministic across OSes and Rust versions by
//! construction: only integer wrapping arithmetic.

/// PCG32 generator: 64-bit LCG state, XSH-RR 32-bit output.
///
/// Fields are private; use [`Pcg32::state`] / [`Pcg32::stream`] for
/// canonical serialization and [`Pcg32::from_raw_parts`] to rebuild.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

/// LCG multiplier used by the standard PCG32 (pcg32 in pcg-basic).
const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

impl Pcg32 {
    /// Create a generator from a `seed` and a `stream` selector.
    ///
    /// Both are passed through SplitMix64 (no seeding correlations), then:
    /// - `state = splitmix64(seed)` with one LCG step of decorrelation, and
    /// - `inc = splitmix64(stream) | 1` — the increment must be odd for a
    ///   full-period stream, and different `stream` values therefore select
    ///   independent PCG streams from the same seed.
    pub fn new(seed: u64, stream: u64) -> Self {
        let state = splitmix64(seed);
        let inc = splitmix64(stream) | 1;
        let mut rng = Pcg32 { state, inc };
        rng.advance_state();
        rng
    }

    /// Rebuild from canonical serialization parts (see [`Pcg32::state`] and
    /// [`Pcg32::stream`]).
    ///
    /// Engine invariant, NOT validated here: `inc` must be odd. This
    /// constructor exists for serialization round-trips of our own bytes,
    /// never for arbitrary user-chosen pairs.
    pub fn from_raw_parts(state: u64, inc: u64) -> Self {
        Pcg32 { state, inc }
    }

    /// Next 32 random bits (PCG XSH-RR: xorshift-high then random rotate).
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.advance_state();
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Next 64 random bits: exactly two [`Pcg32::next_u32`] calls, the FIRST
    /// call supplying the LOW word. The order is pinned here and by unit
    /// test; changing it changes every recorded bit-stream.
    pub fn next_u64(&mut self) -> u64 {
        let lo = u64::from(self.next_u32());
        let hi = u64::from(self.next_u32());
        (hi << 32) | lo
    }

    /// Uniform random value in `0..max_exclusive` using EXACT unbiased
    /// rejection sampling (the `arc4random_uniform` method): draw x uniform
    /// in `[0, 2^32)` and reject `x < 2^32 mod max_exclusive`, which leaves
    /// a multiple of `max_exclusive` accepted values so `x % max_exclusive`
    /// is exactly uniform. Rejected draws still advance the stream (they are
    /// not hidden from the state hash).
    ///
    /// Panics if `max_exclusive == 0` — an engine precondition violation,
    /// not user bytes.
    pub fn bounded(&mut self, max_exclusive: u32) -> u32 {
        assert!(max_exclusive > 0, "bounded: max_exclusive must be > 0");
        let m = u64::from(max_exclusive);
        let threshold = (u64::from(u32::MAX) + 1) % m;
        loop {
            let x = u64::from(self.next_u32());
            if x >= threshold {
                return (x % m) as u32;
            }
        }
    }

    /// Current LCG state (for canonical serialization / state hashing).
    pub fn state(&self) -> u64 {
        self.state
    }

    /// Current stream selector, i.e. the odd LCG increment (for canonical
    /// serialization / state hashing).
    pub fn stream(&self) -> u64 {
        self.inc
    }

    /// One raw LCG step (state = state * M + inc); output mixing happens in
    /// [`Pcg32::next_u32`] against the PRE-step state.
    fn advance_state(&mut self) {
        self.state = self.state.wrapping_mul(MULTIPLIER).wrapping_add(self.inc);
    }
}

/// SplitMix64 step function used only for seeding.
fn splitmix64(x: u64) -> u64 {
    const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut z = x.wrapping_add(GAMMA);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_always_in_range() {
        for &max in &[7u32, 1u32] {
            let mut rng = Pcg32::new(42, 7);
            for _ in 0..100_000 {
                let v = rng.bounded(max);
                assert!(v < max, "bounded({max}) returned {v}");
            }
        }
    }

    #[test]
    fn bounded_four_bucket_uniformity_smoke() {
        let mut rng = Pcg32::new(1234, 3);
        let mut buckets = [0usize; 4];
        for _ in 0..40_000 {
            buckets[rng.bounded(4) as usize] += 1;
        }
        // Each bucket within 20% of the 10_000 mean.
        for (i, &count) in buckets.iter().enumerate() {
            assert!(
                (8_000..=12_000).contains(&count),
                "bucket {i} count {count} outside 20% of mean"
            );
        }
    }

    #[test]
    fn same_seed_and_stream_reproduce() {
        let mut a = Pcg32::new(99, 5);
        let mut b = Pcg32::new(99, 5);
        for _ in 0..16 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        assert_eq!(a.state(), b.state());
        assert_eq!(a.stream(), b.stream());
    }

    #[test]
    fn different_stream_diverges() {
        let mut a = Pcg32::new(99, 5);
        let mut b = Pcg32::new(99, 6);
        let differs = (0..8).any(|_| a.next_u32() != b.next_u32());
        assert!(differs, "distinct streams produced identical output");
        // The stream selector is odd by construction.
        assert_eq!(a.stream() & 1, 1);
        assert_eq!(b.stream() & 1, 1);
    }

    #[test]
    fn next_u64_word_order_is_pinned() {
        let mut a = Pcg32::new(7, 11);
        let v = a.next_u64();
        let mut b = Pcg32::new(7, 11);
        let lo = u64::from(b.next_u32());
        let hi = u64::from(b.next_u32());
        assert_eq!(v, (hi << 32) | lo);
    }

    #[test]
    fn from_raw_parts_round_trips() {
        let mut rng = Pcg32::new(5, 9);
        rng.next_u32();
        rng.next_u32();
        let rebuilt = Pcg32::from_raw_parts(rng.state(), rng.stream());
        let mut rebuilt = rebuilt;
        assert_eq!(rng.next_u32(), rebuilt.next_u32());
    }
}
