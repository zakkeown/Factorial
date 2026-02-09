//! Engine lifecycle WASM exports.

use factorial_core::engine::Engine;
use factorial_core::sim::SimulationStrategy;

use crate::{
    EVENT_CACHE, EngineSlot, HANDLE_TABLE, RESULT_INVALID_HANDLE, RESULT_OK,
    register_event_listeners, with_engine,
};

/// Create a new engine with `Tick` simulation strategy.
///
/// Returns a handle (>= 0) on success, or -1 if no slot is available.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_create() -> i32 {
    HANDLE_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        for (i, slot) in table.iter_mut().enumerate() {
            if slot.is_none() {
                let mut engine = Engine::new(SimulationStrategy::Tick);
                register_event_listeners(&mut engine);
                *slot = Some(EngineSlot {
                    engine,
                    event_cache: Vec::new(),
                });
                return i as i32;
            }
        }
        -1
    })
}

/// Create a new engine with `Delta` simulation strategy.
///
/// `fixed_timestep` is the number of ticks per fixed simulation step.
/// Returns a handle (>= 0) on success, or -1 if no slot is available.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_create_delta(fixed_timestep: u64) -> i32 {
    HANDLE_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        for (i, slot) in table.iter_mut().enumerate() {
            if slot.is_none() {
                let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep });
                register_event_listeners(&mut engine);
                *slot = Some(EngineSlot {
                    engine,
                    event_cache: Vec::new(),
                });
                return i as i32;
            }
        }
        -1
    })
}

/// Destroy the engine at `handle` and free its slot.
///
/// Returns [`RESULT_OK`] on success, or [`RESULT_INVALID_HANDLE`] if the
/// handle is out of range or already destroyed.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_destroy(handle: i32) -> i32 {
    HANDLE_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        let idx = handle as usize;
        if idx >= table.len() {
            return RESULT_INVALID_HANDLE;
        }
        if table[idx].is_none() {
            return RESULT_INVALID_HANDLE;
        }
        table[idx] = None;
        EVENT_CACHE.with(|c| c.borrow_mut().clear());
        RESULT_OK
    })
}

/// Run a single simulation step on the engine at `handle`.
///
/// Returns [`RESULT_OK`] on success, or [`RESULT_INVALID_HANDLE`].
#[unsafe(no_mangle)]
pub extern "C" fn factorial_step(handle: i32) -> i32 {
    EVENT_CACHE.with(|c| c.borrow_mut().clear());
    with_engine(handle, |slot| {
        slot.engine.step();
        RESULT_OK
    })
}

/// Advance the simulation by `dt` ticks.
///
/// In tick mode `dt` is ignored and exactly one step runs.
/// In delta mode `dt` is accumulated and as many fixed steps as fit are run.
///
/// Returns [`RESULT_OK`] on success, or [`RESULT_INVALID_HANDLE`].
#[unsafe(no_mangle)]
pub extern "C" fn factorial_advance(handle: i32, dt: u64) -> i32 {
    EVENT_CACHE.with(|c| c.borrow_mut().clear());
    with_engine(handle, |slot| {
        slot.engine.advance(dt);
        RESULT_OK
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: destroy all engines so tests don't leak into each other.
    fn cleanup() {
        HANDLE_TABLE.with(|table| {
            let mut table = table.borrow_mut();
            for slot in table.iter_mut() {
                *slot = None;
            }
        });
        EVENT_CACHE.with(|c| c.borrow_mut().clear());
    }

    #[test]
    fn create_returns_valid_handle() {
        cleanup();
        let h = factorial_create();
        assert!(h >= 0);
        assert!((h as usize) < crate::MAX_ENGINES);
        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn destroy_invalid_handle_returns_error() {
        cleanup();
        assert_eq!(factorial_destroy(-1i32), RESULT_INVALID_HANDLE);
        assert_eq!(factorial_destroy(99), RESULT_INVALID_HANDLE);
        // Destroying an already-empty slot should also fail.
        assert_eq!(factorial_destroy(0), RESULT_INVALID_HANDLE);
        cleanup();
    }

    #[test]
    fn create_delta_returns_valid_handle() {
        cleanup();
        let h = factorial_create_delta(16);
        assert!(h >= 0);
        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn step_returns_ok() {
        cleanup();
        let h = factorial_create();
        assert_eq!(factorial_step(h), RESULT_OK);
        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn step_invalid_handle_returns_error() {
        cleanup();
        assert_eq!(factorial_step(99), RESULT_INVALID_HANDLE);
        assert_eq!(factorial_step(-1i32), RESULT_INVALID_HANDLE);
        cleanup();
    }

    #[test]
    fn advance_works() {
        cleanup();
        let h = factorial_create_delta(4);
        // Advance by 8 ticks => should run 2 internal steps.
        assert_eq!(factorial_advance(h, 8), RESULT_OK);
        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn create_reuses_destroyed_slot() {
        cleanup();
        let h1 = factorial_create();
        assert_eq!(h1, 0);
        factorial_destroy(h1);

        let h2 = factorial_create();
        // Should reuse slot 0.
        assert_eq!(h2, 0);
        factorial_destroy(h2);
        cleanup();
    }
}
