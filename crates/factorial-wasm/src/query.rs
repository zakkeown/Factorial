//! Query WASM exports.

use factorial_core::processor::{ProcessorState, StallReason};

use crate::{RESULT_NODE_NOT_FOUND, RESULT_OK, ffi_to_node_id, with_engine};

/// Write the current node count to `*out_count`.
///
/// # Safety
///
/// `out_count` must be a valid, aligned pointer to a `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_node_count(handle: i32, out_count: *mut u32) -> i32 {
    if out_count.is_null() {
        return crate::RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| {
        unsafe { *out_count = slot.engine.node_count() as u32 };
        RESULT_OK
    })
}

/// Write the current edge count to `*out_count`.
///
/// # Safety
///
/// `out_count` must be a valid, aligned pointer to a `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_edge_count(handle: i32, out_count: *mut u32) -> i32 {
    if out_count.is_null() {
        return crate::RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| {
        unsafe { *out_count = slot.engine.edge_count() as u32 };
        RESULT_OK
    })
}

/// Write the current tick number to `*out_tick`.
///
/// # Safety
///
/// `out_tick` must be a valid, aligned pointer to a `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_get_tick(handle: i32, out_tick: *mut u64) -> i32 {
    if out_tick.is_null() {
        return crate::RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| {
        unsafe { *out_tick = slot.engine.sim_state.tick };
        RESULT_OK
    })
}

/// Write the most recent state hash to `*out_hash`.
///
/// # Safety
///
/// `out_hash` must be a valid, aligned pointer to a `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_get_state_hash(handle: i32, out_hash: *mut u64) -> i32 {
    if out_hash.is_null() {
        return crate::RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| {
        unsafe { *out_hash = slot.engine.state_hash() };
        RESULT_OK
    })
}

/// Query the processor state for `node_id`.
///
/// Writes the state discriminant to `*out_state`:
///   0 = Idle, 1 = Working, 2 = StalledMissingInputs, 3 = StalledOutputFull,
///   4 = StalledNoPower, 5 = StalledDepleted.
///
/// For `Working`, `*out_progress` receives the progress counter.
///
/// # Safety
///
/// `out_state` and `out_progress` must be valid, aligned pointers to `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_get_processor_state(
    handle: i32,
    node_id: u64,
    out_state: *mut u32,
    out_progress: *mut u32,
) -> i32 {
    if out_state.is_null() || out_progress.is_null() {
        return crate::RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| {
        let nid = ffi_to_node_id(node_id);
        match slot.engine.get_processor_state(nid) {
            Some(state) => {
                let (s, p) = match state {
                    ProcessorState::Idle => (0u32, 0u32),
                    ProcessorState::Working { progress } => (1, *progress),
                    ProcessorState::Stalled { reason } => {
                        let code = match reason {
                            StallReason::MissingInputs => 2u32,
                            StallReason::OutputFull => 3,
                            StallReason::NoPower => 4,
                            StallReason::Depleted => 5,
                        };
                        (code, 0)
                    }
                };
                unsafe {
                    *out_state = s;
                    *out_progress = p;
                }
                RESULT_OK
            }
            None => RESULT_NODE_NOT_FOUND,
        }
    })
}

/// Write the total input inventory item count for `node_id` to `*out_count`.
///
/// # Safety
///
/// `out_count` must be a valid, aligned pointer to a `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_get_input_inventory_count(
    handle: i32,
    node_id: u64,
    out_count: *mut u32,
) -> i32 {
    if out_count.is_null() {
        return crate::RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| {
        let nid = ffi_to_node_id(node_id);
        match slot.engine.get_input_inventory(nid) {
            Some(inv) => {
                let total: u32 = inv.input_slots.iter().map(|s| s.total()).sum();
                unsafe { *out_count = total };
                RESULT_OK
            }
            None => RESULT_NODE_NOT_FOUND,
        }
    })
}

/// Write the total output inventory item count for `node_id` to `*out_count`.
///
/// # Safety
///
/// `out_count` must be a valid, aligned pointer to a `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_get_output_inventory_count(
    handle: i32,
    node_id: u64,
    out_count: *mut u32,
) -> i32 {
    if out_count.is_null() {
        return crate::RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| {
        let nid = ffi_to_node_id(node_id);
        match slot.engine.get_output_inventory(nid) {
            Some(inv) => {
                let total: u32 = inv.output_slots.iter().map(|s| s.total()).sum();
                unsafe { *out_count = total };
                RESULT_OK
            }
            None => RESULT_NODE_NOT_FOUND,
        }
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{factorial_create, factorial_destroy, factorial_step};
    use crate::graph::{factorial_add_node, factorial_apply_mutations};
    use crate::processor::factorial_set_source;
    use crate::{EVENT_CACHE, HANDLE_TABLE};

    use factorial_core::fixed::Fixed64;

    fn cleanup() {
        HANDLE_TABLE.with(|t| {
            for s in t.borrow_mut().iter_mut() {
                *s = None;
            }
        });
        EVENT_CACHE.with(|c| c.borrow_mut().clear());
    }

    fn create_engine_with_node() -> (i32, u64) {
        let h = factorial_create();
        let mut pending: u64 = 0;
        unsafe { factorial_add_node(h, 0, &mut pending) };
        let mut buf = [0u8; 256];
        let mut written: i32 = 0;
        unsafe { factorial_apply_mutations(h, buf.as_mut_ptr(), 256, &mut written) };
        let real_id = u64::from_le_bytes(buf[16..24].try_into().unwrap());
        (h, real_id)
    }

    #[test]
    fn node_count() {
        cleanup();
        let (h, _) = create_engine_with_node();
        let mut count: u32 = 0;
        let rc = unsafe { factorial_node_count(h, &mut count) };
        assert_eq!(rc, RESULT_OK);
        assert_eq!(count, 1);
        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn edge_count() {
        cleanup();
        let h = factorial_create();
        let mut count: u32 = 0;
        let rc = unsafe { factorial_edge_count(h, &mut count) };
        assert_eq!(rc, RESULT_OK);
        assert_eq!(count, 0);
        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn tick_increments() {
        cleanup();
        let h = factorial_create();
        let mut tick: u64 = 99;
        unsafe { factorial_get_tick(h, &mut tick) };
        assert_eq!(tick, 0);

        factorial_step(h);

        unsafe { factorial_get_tick(h, &mut tick) };
        assert_eq!(tick, 1);

        factorial_step(h);
        unsafe { factorial_get_tick(h, &mut tick) };
        assert_eq!(tick, 2);

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn state_hash() {
        cleanup();
        let h = factorial_create();
        let mut hash: u64 = 0;
        let rc = unsafe { factorial_get_state_hash(h, &mut hash) };
        assert_eq!(rc, RESULT_OK);
        // Hash is deterministic but its value depends on engine internals;
        // just verify the call succeeds.

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn processor_state_after_step() {
        cleanup();
        let (h, node_id) = create_engine_with_node();

        // Set up a source processor so the node has processor state.
        let rate = Fixed64::from_num(1).to_bits();
        factorial_set_source(h, node_id, 0, rate);

        // Set output inventory so production has somewhere to go.
        crate::transport::factorial_set_output_capacity(h, node_id, 100);

        factorial_step(h);

        let mut state: u32 = 99;
        let mut progress: u32 = 99;
        let rc = unsafe { factorial_get_processor_state(h, node_id, &mut state, &mut progress) };
        assert_eq!(rc, RESULT_OK);
        // After one step with a source processor, the state should be valid
        // (Idle=0 or Working=1). Sources may remain in Working state.
        assert!(state <= 1, "Expected Idle(0) or Working(1), got {state}");

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn inventory_counts() {
        cleanup();
        let (h, node_id) = create_engine_with_node();

        // Set up inventories
        crate::transport::factorial_set_input_capacity(h, node_id, 100);
        crate::transport::factorial_set_output_capacity(h, node_id, 100);

        let mut in_count: u32 = 99;
        let mut out_count: u32 = 99;

        let rc = unsafe { factorial_get_input_inventory_count(h, node_id, &mut in_count) };
        assert_eq!(rc, RESULT_OK);
        assert_eq!(in_count, 0);

        let rc = unsafe { factorial_get_output_inventory_count(h, node_id, &mut out_count) };
        assert_eq!(rc, RESULT_OK);
        assert_eq!(out_count, 0);

        factorial_destroy(h);
        cleanup();
    }
}
