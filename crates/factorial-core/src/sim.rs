//! Simulation strategy and state types.
//!
//! The engine is parameterized by a [`SimulationStrategy`] that determines how
//! time advances. All strategies execute the same six-phase pipeline; they
//! differ only in how many steps are run per `advance()` call.

use crate::fixed::{Fixed64, Ticks};
use crate::graph::MutationResult;

// ---------------------------------------------------------------------------
// Simulation strategy
// ---------------------------------------------------------------------------

/// How the engine advances time. Chosen at engine construction.
///
/// All strategies execute the same six-phase step internally. The strategy
/// only controls how many steps are run when `Engine::advance()` is called.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SimulationStrategy {
    /// Single step per call. The game calls `engine.step()` at a fixed rate.
    /// Deterministic by construction.
    Tick,

    /// Real-time mode. The game calls `engine.advance(dt)` with elapsed time.
    /// Internally accumulates time and runs as many fixed steps as fit,
    /// carrying the remainder forward.
    Delta {
        /// Duration of one fixed simulation step, in ticks.
        /// For example, if the sim runs at 60 UPS and the game runs at
        /// variable FPS, `fixed_timestep` would be 1 (one tick per step).
        fixed_timestep: Ticks,
    },
}

// ---------------------------------------------------------------------------
// Simulation state
// ---------------------------------------------------------------------------

/// Mutable simulation state tracked by the engine.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimState {
    /// Current tick counter. Incremented by 1 for each simulation step.
    pub tick: Ticks,

    /// Accumulated time remainder for delta mode. When the accumulator
    /// reaches `fixed_timestep`, a step is run and the accumulator is
    /// decremented. Unused in tick mode.
    pub accumulator: Ticks,
}

impl SimState {
    /// Create a new simulation state starting at tick 0.
    pub fn new() -> Self {
        Self {
            tick: 0,
            accumulator: 0,
        }
    }
}

impl Default for SimState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Advance result
// ---------------------------------------------------------------------------

/// Result of an `Engine::advance()` call.
#[derive(Debug, Default)]
pub struct AdvanceResult {
    /// Number of simulation steps actually executed.
    pub steps_run: u64,

    /// Mutation results from the pre-tick phase of each step.
    /// One entry per step that had pending mutations.
    pub mutation_results: Vec<MutationResult>,
}

// ---------------------------------------------------------------------------
// State hash
// ---------------------------------------------------------------------------

/// A simple deterministic hash of simulation state for desync detection.
///
/// Uses FNV-1a (64-bit) for speed and simplicity. Not cryptographic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateHash(pub u64);

impl StateHash {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    /// Start a new hash.
    pub fn new() -> Self {
        Self(Self::FNV_OFFSET)
    }

    /// Feed bytes into the hash.
    pub fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(Self::FNV_PRIME);
        }
    }

    /// Feed a u64 into the hash.
    pub fn write_u64(&mut self, v: u64) {
        self.write(&v.to_le_bytes());
    }

    /// Feed a u32 into the hash.
    pub fn write_u32(&mut self, v: u32) {
        self.write(&v.to_le_bytes());
    }

    /// Feed a Fixed64 into the hash.
    pub fn write_fixed64(&mut self, v: Fixed64) {
        self.write(&v.to_bits().to_le_bytes());
    }

    /// Finalize and return the hash value.
    pub fn finish(self) -> u64 {
        self.0
    }
}

impl Default for StateHash {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_state_starts_at_zero() {
        let state = SimState::new();
        assert_eq!(state.tick, 0);
        assert_eq!(state.accumulator, 0);
    }

    #[test]
    fn state_hash_deterministic() {
        let mut h1 = StateHash::new();
        h1.write_u64(42);
        h1.write_u32(7);

        let mut h2 = StateHash::new();
        h2.write_u64(42);
        h2.write_u32(7);

        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn state_hash_differs_for_different_inputs() {
        let mut h1 = StateHash::new();
        h1.write_u64(1);

        let mut h2 = StateHash::new();
        h2.write_u64(2);

        assert_ne!(h1.finish(), h2.finish());
    }

    #[test]
    fn state_hash_order_matters() {
        let mut h1 = StateHash::new();
        h1.write_u32(1);
        h1.write_u32(2);

        let mut h2 = StateHash::new();
        h2.write_u32(2);
        h2.write_u32(1);

        assert_ne!(h1.finish(), h2.finish());
    }
}
