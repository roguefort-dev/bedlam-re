//! Shop secondary RNG, EXW 0x4029b6 + bounded transform 0x41ec29.
//! Isolated from the mission's charter-controlled simulation RNG.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShopRandom {
    state: u32,
}
impl ShopRandom {
    /// Explicit state corresponding to low/high words 0x4ede4c/0x4ede4e.
    pub fn from_state(state: u32) -> Self {
        Self { state }
    }
    pub fn state(&self) -> u32 {
        self.state
    }
    pub fn bounded(&mut self, bound: u32) -> u32 {
        // All shop bounds are 1..=9. The original division is undefined for
        // larger values that make the second divisor zero.
        assert!((1..=9).contains(&bound), "shop random bound");
        self.state = self.state.wrapping_mul(129).wrapping_add(0x3619_62e9);
        let sample = (self.state >> 16) & 0x7fff;
        (sample / (32768 / bound - 1)).min(bound - 1)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn instruction_arithmetic_matches_carry_propagation() {
        // Independent word-level evaluation of the original byte shuffles,
        // rotate-through-carry sequence, and ADD/ADC pair.
        for seed in [0, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 0x1234_abcd] {
            let lo = seed as u16;
            let hi = (seed >> 16) as u16;
            let shifted_lo = (lo & 511) << 7;
            let shifted_hi = ((hi << 8) | (lo >> 8)) >> 1 | ((hi & 0x100) << 7);
            let sum_lo = u32::from(shifted_lo) + u32::from(lo);
            let sum_hi = u32::from(shifted_hi) + u32::from(hi) + (sum_lo >> 16);
            let add_lo = (sum_lo & 65535) + 0x62e9;
            let add_hi = (sum_hi & 65535) + 0x3619 + (add_lo >> 16);
            let expected = ((add_hi & 65535) << 16) | (add_lo & 65535);
            let mut rng = ShopRandom::from_state(seed);
            rng.bounded(9);
            assert_eq!(rng.state(), expected);
        }
    }
    #[test]
    fn bounded_draws_preserve_original_division_and_clamp() {
        let mut rng = ShopRandom::from_state(0);
        assert_eq!(
            (0..5).map(|_| rng.bounded(9)).collect::<Vec<_>>(),
            [3, 8, 2, 7, 7]
        );
        let before = rng.state();
        assert_eq!(rng.bounded(1), 0);
        assert_ne!(rng.state(), before);
    }
}
