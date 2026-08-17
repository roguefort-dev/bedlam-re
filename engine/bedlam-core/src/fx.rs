//! Q16.16 fixed-point arithmetic — the only numeric type allowed in sim
//! state (Determinism Charter: no floats in sim state; integer/fixed-point
//! only). All operations are plain integer arithmetic, identical on every
//! OS and Rust version.

/// Q16.16 fixed-point number: the inner `i32` counts 1/65536 units, so
/// `Fixed(0x0001_0000)` == 1.0. Range is roughly [-32768, 32768).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Fixed(i32);

impl Fixed {
    /// 0.0
    pub const ZERO: Fixed = Fixed(0);
    /// 1.0
    pub const ONE: Fixed = Fixed(1 << 16);
    /// -1.0
    pub const NEG_ONE: Fixed = Fixed(-(1i32 << 16));

    /// Exact integer value.
    ///
    /// Precondition (engine, debug-asserted): `|v| <= 0x7FFF` so `v << 16`
    /// cannot overflow `i32`. Larger game-world quantities are fixed-point
    /// by nature and must not pass through here.
    pub fn from_int(v: i32) -> Fixed {
        debug_assert!(v.unsigned_abs() <= 0x7FFF, "from_int: |{v}| > 0x7FFF");
        Fixed(v << 16)
    }

    /// Truncated integer part, floor toward negative infinity
    /// (arithmetic shift, e.g. -0.5 floors to -1).
    pub fn to_i32_floor(self) -> i32 {
        self.0 >> 16
    }

    /// `num / den` as fixed-point, or `None` if `den == 0` or the result
    /// does not fit Q16.16. 64-bit intermediate, truncating toward zero.
    pub fn from_ratio(num: i32, den: i32) -> Option<Fixed> {
        if den == 0 {
            return None;
        }
        let q = ((num as i64) << 16) / (den as i64);
        narrow_exact(q).map(Fixed)
    }

    /// Product via 64-bit intermediate, then SATURATING narrow to Q16.16:
    /// overflow pins at `i32::MAX` / `i32::MIN` raw rather than wrapping.
    /// Saturation (not `None`) keeps hot-path math branch-free and matches
    /// how the sim treats clamp boundaries elsewhere.
    // Inherent method per the crate spec (explicit call sites in sim code);
    // deliberately not the `std::ops` traits.
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, rhs: Fixed) -> Fixed {
        Fixed(saturate_i32((self.0 as i64 * rhs.0 as i64) >> 16))
    }

    /// Saturating addition.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtraction.
    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.saturating_sub(rhs.0))
    }

    /// Negation, saturating at `i32::MIN` (which negates to itself).
    #[allow(clippy::should_implement_trait)]
    pub fn neg(self) -> Fixed {
        Fixed(0i32.saturating_sub(self.0))
    }

    /// Absolute value, saturating at `i32::MIN` raw (=> `i32::MAX`).
    pub fn abs(self) -> Fixed {
        Fixed(self.0.saturating_abs())
    }

    /// Quotient via 64-bit intermediate (`(self << 16) / rhs`) with
    /// SATURATING narrow, or `None` if `rhs` is zero.
    // `div -> Option<Fixed>` cannot sanely be `std::ops::Div` (which would
    // make `a / b` return an Option); inherent method per the crate spec.
    #[allow(clippy::should_implement_trait)]
    pub fn div(self, rhs: Fixed) -> Option<Fixed> {
        if rhs.0 == 0 {
            return None;
        }
        let q = ((self.0 as i64) << 16) / (rhs.0 as i64);
        Some(Fixed(saturate_i32(q)))
    }

    /// Wrap a raw Q16.16 word. Escape hatch for serialization and tests;
    /// normal code builds values with the constructors above.
    pub const fn raw(v: i32) -> Fixed {
        Fixed(v)
    }

    /// Unwrap the raw Q16.16 word (serialization / tests).
    pub const fn to_raw(self) -> i32 {
        self.0
    }
}

/// Narrow an `i64` to `i32`, or `None` if out of range.
fn narrow_exact(v: i64) -> Option<i32> {
    if v > i64::from(i32::MAX) || v < i64::from(i32::MIN) {
        None
    } else {
        Some(v as i32)
    }
}

/// Narrow an `i64` to `i32`, saturating at the range bounds.
fn saturate_i32(v: i64) -> i32 {
    if v > i64::from(i32::MAX) {
        i32::MAX
    } else if v < i64::from(i32::MIN) {
        i32::MIN
    } else {
        v as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_is_from_int_one() {
        assert_eq!(Fixed::ONE, Fixed::from_int(1));
        assert_eq!(Fixed::ONE.to_raw(), 0x0001_0000);
        assert_eq!(Fixed::NEG_ONE, Fixed::from_int(-1));
        assert_eq!(Fixed::ZERO.to_i32_floor(), 0);
    }

    #[test]
    fn int_times_half_is_three_halves() {
        let three = Fixed::from_int(3);
        let half = Fixed::from_ratio(1, 2).unwrap();
        assert_eq!(three.mul(half), Fixed::from_ratio(3, 2).unwrap());
    }

    #[test]
    fn mul_saturates_at_i32_bounds() {
        let big = Fixed::raw(i32::MAX);
        let two = Fixed::from_int(2);
        assert_eq!(big.mul(two).to_raw(), i32::MAX);
        let small = Fixed::raw(i32::MIN);
        assert_eq!(small.mul(two).to_raw(), i32::MIN);
    }

    #[test]
    fn abs_saturates_at_min() {
        assert_eq!(Fixed::raw(i32::MIN).abs().to_raw(), i32::MAX);
        assert_eq!(Fixed::NEG_ONE.abs(), Fixed::ONE);
    }

    #[test]
    fn div_by_zero_is_none() {
        assert!(Fixed::ONE.div(Fixed::ZERO).is_none());
        // Saturating narrow on the way out, not a wrap:
        let huge = Fixed::raw(i32::MAX).div(Fixed::from_ratio(1, 2).unwrap());
        assert_eq!(huge.unwrap().to_raw(), i32::MAX);
    }

    #[test]
    fn third_times_three_is_within_one_lsb() {
        let third = Fixed::from_ratio(1, 3).unwrap();
        let product = third.mul(Fixed::from_int(3));
        let diff = (product.to_raw() as i64 - Fixed::ONE.to_raw() as i64).abs();
        assert!(diff <= 1, "raw diff {diff} > 1/65536");
    }

    #[test]
    fn floor_goes_toward_negative_infinity() {
        let neg_half = Fixed::from_ratio(1, 2).unwrap().neg();
        assert_eq!(neg_half.to_i32_floor(), -1);
        assert_eq!(Fixed::from_ratio(1, 2).unwrap().to_i32_floor(), 0);
    }

    #[test]
    fn from_ratio_rejects_zero_denominator_and_overflow() {
        assert!(Fixed::from_ratio(1, 0).is_none());
        assert!(Fixed::from_ratio(i32::MAX, 1).is_none());
        assert!(Fixed::from_ratio(i32::MIN + 1, 1).is_none());
        // Largest exactly representable integer ratio still fits.
        assert!(Fixed::from_ratio(0x7FFF, 1).is_some());
    }
}
