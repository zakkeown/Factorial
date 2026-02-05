use fixed::types::{I16F16, I32F32};

/// Q32.32 fixed-point: 32 integer bits, 32 fractional bits.
pub type Fixed64 = I32F32;

/// Q16.16 fixed-point for compact storage (item properties, etc.).
pub type Fixed32 = I16F16;

/// Ticks are the atomic unit of simulation time.
pub type Ticks = u64;

/// Convert an f64 to Fixed64. Use only for initialization, never in sim loop.
#[inline]
pub fn f64_to_fixed64(v: f64) -> Fixed64 {
    Fixed64::from_num(v)
}

/// Convert Fixed64 to f64. Use only for display/FFI, never in sim loop.
#[inline]
pub fn fixed64_to_f64(v: Fixed64) -> f64 {
    v.to_num::<f64>()
}

/// Convert an f64 to Fixed32. Use only for initialization.
#[inline]
pub fn f64_to_fixed32(v: f64) -> Fixed32 {
    Fixed32::from_num(v)
}

/// Convert Fixed32 to f64. Use only for display/FFI.
#[inline]
pub fn fixed32_to_f64(v: Fixed32) -> f64 {
    v.to_num::<f64>()
}

/// Checked multiplication for Fixed64 that returns None on overflow.
#[inline]
pub fn checked_mul_64(a: Fixed64, b: Fixed64) -> Option<Fixed64> {
    a.checked_mul(b)
}

/// Checked division for Fixed64 that returns None on zero divisor.
#[inline]
pub fn checked_div_64(a: Fixed64, b: Fixed64) -> Option<Fixed64> {
    a.checked_div(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed64_basic_arithmetic() {
        let a = f64_to_fixed64(1.5);
        let b = f64_to_fixed64(2.0);
        let sum = a + b;
        assert_eq!(fixed64_to_f64(sum), 3.5);
    }

    #[test]
    fn fixed64_multiplication() {
        let a = f64_to_fixed64(3.0);
        let b = f64_to_fixed64(4.0);
        let product = a * b;
        assert_eq!(fixed64_to_f64(product), 12.0);
    }

    #[test]
    fn fixed64_checked_mul_overflow() {
        let big = Fixed64::MAX;
        let two = f64_to_fixed64(2.0);
        assert!(checked_mul_64(big, two).is_none());
    }

    #[test]
    fn fixed64_checked_div_by_zero() {
        let a = f64_to_fixed64(1.0);
        let zero = f64_to_fixed64(0.0);
        assert!(checked_div_64(a, zero).is_none());
    }

    #[test]
    fn fixed32_basic_arithmetic() {
        let a = f64_to_fixed32(10.5);
        let b = f64_to_fixed32(3.25);
        let diff = a - b;
        assert_eq!(fixed32_to_f64(diff), 7.25);
    }

    #[test]
    fn fixed64_determinism() {
        let a = f64_to_fixed64(1.0 / 3.0);
        let b = f64_to_fixed64(1.0 / 3.0);
        assert_eq!(a, b);
        assert_eq!(a * f64_to_fixed64(3.0), b * f64_to_fixed64(3.0));
    }

    #[test]
    fn fixed64_ordering() {
        let a = f64_to_fixed64(1.0);
        let b = f64_to_fixed64(2.0);
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn ticks_type() {
        let t: Ticks = 60;
        assert_eq!(t, 60u64);
    }
}
