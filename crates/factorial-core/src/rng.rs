//! Deterministic PRNG for simulation use (bonus outputs, etc.).
//!
//! Uses the SplitMix64 algorithm: fast, 8 bytes of state, excellent
//! statistical properties, and trivially serializable for snapshots.

use crate::fixed::Fixed64;

/// SplitMix64 pseudo-random number generator.
///
/// Deterministic across platforms — critical for lockstep multiplayer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SimRng {
    state: u64,
}

impl SimRng {
    /// Create a new RNG with the given seed.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Generate the next `u64` in the sequence.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Returns `true` with the given probability (Fixed64 in [0, 1]).
    ///
    /// - probability <= 0 always returns false
    /// - probability >= 1 always returns true
    pub fn chance(&mut self, probability: Fixed64) -> bool {
        if probability <= Fixed64::ZERO {
            return false;
        }
        if probability >= Fixed64::from_num(1) {
            return true;
        }
        // Fixed64 is Q32.32 (I32F32). For p in (0,1), the raw bits hold
        // the fractional part in the lower 32 bits (integer part = 0).
        // Generate a uniform u32 from the PRNG and compare against the
        // lower 32 bits of the fixed-point representation.
        let r = self.next_u64();
        let upper = (r >> 32) as u32;
        let raw = probability.to_bits() as u64;
        // raw is Q32.32: for p in (0,1) it equals the fractional value
        // scaled to [0, 2^32). upper is uniform in [0, 2^32).
        (upper as u64) < raw
    }

    /// Get the internal state (for hashing/serialization).
    pub fn state(&self) -> u64 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a = SimRng::new(42);
        let mut b = SimRng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = SimRng::new(1);
        let mut b = SimRng::new(2);
        // Extremely unlikely to match.
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn chance_zero_always_false() {
        let mut rng = SimRng::new(999);
        for _ in 0..100 {
            assert!(!rng.chance(Fixed64::ZERO));
        }
    }

    #[test]
    fn chance_one_always_true() {
        let mut rng = SimRng::new(999);
        for _ in 0..100 {
            assert!(rng.chance(Fixed64::from_num(1)));
        }
    }

    #[test]
    fn chance_negative_always_false() {
        let mut rng = SimRng::new(999);
        assert!(!rng.chance(Fixed64::from_num(-1)));
    }

    #[test]
    fn chance_above_one_always_true() {
        let mut rng = SimRng::new(999);
        assert!(rng.chance(Fixed64::from_num(2)));
    }

    #[test]
    fn chance_half_roughly_balanced() {
        let mut rng = SimRng::new(12345);
        let trials = 10_000;
        let mut hits = 0u32;
        let half = Fixed64::from_num(0.5);
        for _ in 0..trials {
            if rng.chance(half) {
                hits += 1;
            }
        }
        // Expect ~5000 +/- 300 (very generous tolerance).
        assert!((4000..=6000).contains(&hits), "expected ~5000, got {hits}");
    }

    #[test]
    fn serialization_round_trip() {
        let mut rng = SimRng::new(42);
        // Advance state.
        for _ in 0..50 {
            rng.next_u64();
        }

        let json = serde_json::to_string(&rng).unwrap();
        let restored: SimRng = serde_json::from_str(&json).unwrap();
        assert_eq!(rng, restored);

        // Continue sequence — should match.
        let mut rng2 = restored;
        for _ in 0..10 {
            assert_eq!(rng.next_u64(), rng2.next_u64());
        }
    }
}
