//! Logic network WASM exports.

use factorial_core::fixed::Fixed64;
use factorial_core::id::ItemTypeId;

use factorial_logic::combinator::{ArithmeticCombinator, DeciderCombinator, DeciderOutput};
use factorial_logic::condition::{Condition, InventorySource};
use factorial_logic::{LogicModuleBridge, SignalSet, WireNetworkId};

use crate::{
    RESULT_INTERNAL_ERROR, RESULT_NODE_NOT_FOUND, RESULT_OK, ffi_to_arithmetic_op,
    ffi_to_comparison_op, ffi_to_node_id, ffi_to_selector, ffi_to_wire_color, with_engine,
};

/// Register the logic module on the engine at `handle`.
///
/// This must be called before any other `factorial_logic_*` function.
/// Returns [`RESULT_OK`] on success.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_logic_register(handle: i32) -> i32 {
    with_engine(handle, |slot| {
        slot.engine
            .register_module(Box::new(LogicModuleBridge::new()));
        RESULT_OK
    })
}

/// Create a new wire network of the given color.
///
/// Writes the new network ID to `*out_id_ptr`.
///
/// # Safety
///
/// `out_id_ptr` must be a valid, aligned pointer to a `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_logic_create_network(
    handle: i32,
    color: u32,
    out_id_ptr: *mut u32,
) -> i32 {
    if out_id_ptr.is_null() {
        return RESULT_INTERNAL_ERROR;
    }
    let wire_color = match ffi_to_wire_color(color) {
        Some(c) => c,
        None => return RESULT_INTERNAL_ERROR,
    };
    with_engine(handle, |slot| {
        let bridge = match slot.engine.find_module_mut::<LogicModuleBridge>() {
            Some(b) => b,
            None => return RESULT_INTERNAL_ERROR,
        };
        let id = bridge.logic_mut().create_network(wire_color);
        unsafe { *out_id_ptr = id.0 };
        RESULT_OK
    })
}

/// Remove a wire network by ID.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_logic_remove_network(handle: i32, network_id: u32) -> i32 {
    with_engine(handle, |slot| {
        let bridge = match slot.engine.find_module_mut::<LogicModuleBridge>() {
            Some(b) => b,
            None => return RESULT_INTERNAL_ERROR,
        };
        bridge.logic_mut().remove_network(WireNetworkId(network_id));
        RESULT_OK
    })
}

/// Add a node to a wire network.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_logic_add_to_network(
    handle: i32,
    network_id: u32,
    node_id: u64,
) -> i32 {
    with_engine(handle, |slot| {
        let bridge = match slot.engine.find_module_mut::<LogicModuleBridge>() {
            Some(b) => b,
            None => return RESULT_INTERNAL_ERROR,
        };
        bridge
            .logic_mut()
            .add_to_network(WireNetworkId(network_id), ffi_to_node_id(node_id));
        RESULT_OK
    })
}

/// Remove a node from a wire network.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_logic_remove_from_network(
    handle: i32,
    network_id: u32,
    node_id: u64,
) -> i32 {
    with_engine(handle, |slot| {
        let bridge = match slot.engine.find_module_mut::<LogicModuleBridge>() {
            Some(b) => b,
            None => return RESULT_INTERNAL_ERROR,
        };
        bridge
            .logic_mut()
            .remove_from_network(WireNetworkId(network_id), ffi_to_node_id(node_id));
        RESULT_OK
    })
}

/// Configure a constant combinator on a node.
///
/// `item_ids_ptr` and `values_ptr` are parallel arrays of length `count`.
/// `enabled` is 0 for disabled, non-zero for enabled.
///
/// # Safety
///
/// `item_ids_ptr` must point to at least `count` `u32` values.
/// `values_ptr` must point to at least `count` `i64` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_logic_set_constant(
    handle: i32,
    node_id: u64,
    item_ids_ptr: *const u32,
    values_ptr: *const i64,
    count: u32,
    enabled: u32,
) -> i32 {
    if count > 0 && (item_ids_ptr.is_null() || values_ptr.is_null()) {
        return RESULT_INTERNAL_ERROR;
    }
    let mut signals = SignalSet::new();
    for i in 0..count as usize {
        let item_id = unsafe { *item_ids_ptr.add(i) };
        let value = unsafe { *values_ptr.add(i) };
        signals.insert(ItemTypeId(item_id), Fixed64::from_bits(value));
    }
    with_engine(handle, |slot| {
        let bridge = match slot.engine.find_module_mut::<LogicModuleBridge>() {
            Some(b) => b,
            None => return RESULT_INTERNAL_ERROR,
        };
        bridge
            .logic_mut()
            .set_constant(ffi_to_node_id(node_id), signals, enabled != 0);
        RESULT_OK
    })
}

/// Configure an arithmetic combinator on a node.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_logic_set_arithmetic(
    handle: i32,
    node_id: u64,
    left_kind: u32,
    left_value: u64,
    op: u32,
    right_kind: u32,
    right_value: u64,
    output_item: u32,
) -> i32 {
    let left = match ffi_to_selector(left_kind, left_value) {
        Some(s) => s,
        None => return RESULT_INTERNAL_ERROR,
    };
    let right = match ffi_to_selector(right_kind, right_value) {
        Some(s) => s,
        None => return RESULT_INTERNAL_ERROR,
    };
    let arith_op = match ffi_to_arithmetic_op(op) {
        Some(o) => o,
        None => return RESULT_INTERNAL_ERROR,
    };
    with_engine(handle, |slot| {
        let bridge = match slot.engine.find_module_mut::<LogicModuleBridge>() {
            Some(b) => b,
            None => return RESULT_INTERNAL_ERROR,
        };
        bridge.logic_mut().set_arithmetic(
            ffi_to_node_id(node_id),
            ArithmeticCombinator {
                left,
                op: arith_op,
                right,
                output: ItemTypeId(output_item),
            },
        );
        RESULT_OK
    })
}

/// Configure a decider combinator on a node.
///
/// `output_kind`: 0 = One(output_item), 1 = InputCount(output_item),
/// 2 = Everything.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_logic_set_decider(
    handle: i32,
    node_id: u64,
    left_kind: u32,
    left_value: u64,
    cmp_op: u32,
    right_kind: u32,
    right_value: u64,
    output_kind: u32,
    output_item: u32,
) -> i32 {
    let left = match ffi_to_selector(left_kind, left_value) {
        Some(s) => s,
        None => return RESULT_INTERNAL_ERROR,
    };
    let right = match ffi_to_selector(right_kind, right_value) {
        Some(s) => s,
        None => return RESULT_INTERNAL_ERROR,
    };
    let op = match ffi_to_comparison_op(cmp_op) {
        Some(o) => o,
        None => return RESULT_INTERNAL_ERROR,
    };
    let output = match output_kind {
        0 => DeciderOutput::One(ItemTypeId(output_item)),
        1 => DeciderOutput::InputCount(ItemTypeId(output_item)),
        2 => DeciderOutput::Everything,
        _ => return RESULT_INTERNAL_ERROR,
    };
    with_engine(handle, |slot| {
        let bridge = match slot.engine.find_module_mut::<LogicModuleBridge>() {
            Some(b) => b,
            None => return RESULT_INTERNAL_ERROR,
        };
        bridge.logic_mut().set_decider(
            ffi_to_node_id(node_id),
            DeciderCombinator {
                condition: Condition { left, op, right },
                output,
            },
        );
        RESULT_OK
    })
}

/// Set a circuit control condition on a node.
///
/// The condition is evaluated each tick against the signals on the specified
/// wire color.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_logic_set_circuit_control(
    handle: i32,
    node_id: u64,
    left_kind: u32,
    left_value: u64,
    cmp_op: u32,
    right_kind: u32,
    right_value: u64,
    wire_color: u32,
) -> i32 {
    let left = match ffi_to_selector(left_kind, left_value) {
        Some(s) => s,
        None => return RESULT_INTERNAL_ERROR,
    };
    let right = match ffi_to_selector(right_kind, right_value) {
        Some(s) => s,
        None => return RESULT_INTERNAL_ERROR,
    };
    let op = match ffi_to_comparison_op(cmp_op) {
        Some(o) => o,
        None => return RESULT_INTERNAL_ERROR,
    };
    let color = match ffi_to_wire_color(wire_color) {
        Some(c) => c,
        None => return RESULT_INTERNAL_ERROR,
    };
    with_engine(handle, |slot| {
        let bridge = match slot.engine.find_module_mut::<LogicModuleBridge>() {
            Some(b) => b,
            None => return RESULT_INTERNAL_ERROR,
        };
        bridge.logic_mut().set_circuit_control(
            ffi_to_node_id(node_id),
            Condition { left, op, right },
            color,
        );
        RESULT_OK
    })
}

/// Query whether a node's circuit control is active.
///
/// Writes 1 (active) or 0 (inactive) to `*out_active_ptr`.
/// Returns [`RESULT_NODE_NOT_FOUND`] if the node has no circuit control.
///
/// # Safety
///
/// `out_active_ptr` must be a valid, aligned pointer to a `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_logic_is_active(
    handle: i32,
    node_id: u64,
    out_active_ptr: *mut u32,
) -> i32 {
    if out_active_ptr.is_null() {
        return RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| {
        let bridge = match slot.engine.find_module::<LogicModuleBridge>() {
            Some(b) => b,
            None => return RESULT_INTERNAL_ERROR,
        };
        match bridge.logic().is_active(ffi_to_node_id(node_id)) {
            Some(active) => {
                unsafe { *out_active_ptr = if active { 1 } else { 0 } };
                RESULT_OK
            }
            None => RESULT_NODE_NOT_FOUND,
        }
    })
}

/// Get the signal value for a specific item on a wire network.
///
/// Writes the fixed-point bits to `*out_value_ptr`. Writes 0 if the item
/// is not present on the network.
///
/// # Safety
///
/// `out_value_ptr` must be a valid, aligned pointer to an `i64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_logic_get_network_signal(
    handle: i32,
    network_id: u32,
    item_id: u32,
    out_value_ptr: *mut i64,
) -> i32 {
    if out_value_ptr.is_null() {
        return RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| {
        let bridge = match slot.engine.find_module::<LogicModuleBridge>() {
            Some(b) => b,
            None => return RESULT_INTERNAL_ERROR,
        };
        match bridge.logic().network_signals(WireNetworkId(network_id)) {
            Some(signals) => {
                let value = signals
                    .get(&ItemTypeId(item_id))
                    .copied()
                    .unwrap_or(Fixed64::from_num(0));
                unsafe { *out_value_ptr = value.to_bits() };
                RESULT_OK
            }
            None => {
                unsafe { *out_value_ptr = 0 };
                RESULT_OK
            }
        }
    })
}

/// Remove all logic state associated with a node.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_logic_remove_node(handle: i32, node_id: u64) -> i32 {
    with_engine(handle, |slot| {
        let bridge = match slot.engine.find_module_mut::<LogicModuleBridge>() {
            Some(b) => b,
            None => return RESULT_INTERNAL_ERROR,
        };
        bridge.logic_mut().remove_node(ffi_to_node_id(node_id));
        RESULT_OK
    })
}

/// Configure an inventory reader on a node.
///
/// `source`: 0 = Input, otherwise Output.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_logic_set_inventory_reader(
    handle: i32,
    node_id: u64,
    target_node_id: u64,
    source: u32,
) -> i32 {
    let inv_source = if source == 0 {
        InventorySource::Input
    } else {
        InventorySource::Output
    };
    with_engine(handle, |slot| {
        let bridge = match slot.engine.find_module_mut::<LogicModuleBridge>() {
            Some(b) => b,
            None => return RESULT_INTERNAL_ERROR,
        };
        bridge.logic_mut().set_inventory_reader(
            ffi_to_node_id(node_id),
            ffi_to_node_id(target_node_id),
            inv_source,
        );
        RESULT_OK
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
    use crate::{EVENT_CACHE, HANDLE_TABLE};

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
    fn register_and_create_network() {
        cleanup();
        let h = factorial_create();

        // Register the logic module.
        let rc = factorial_logic_register(h);
        assert_eq!(rc, RESULT_OK);

        // Create a red network.
        let mut net_id: u32 = 99;
        let rc = unsafe { factorial_logic_create_network(h, 0, &mut net_id) };
        assert_eq!(rc, RESULT_OK);
        assert_eq!(net_id, 0); // First network gets ID 0.

        // Create a green network.
        let mut net_id2: u32 = 99;
        let rc = unsafe { factorial_logic_create_network(h, 1, &mut net_id2) };
        assert_eq!(rc, RESULT_OK);
        assert_eq!(net_id2, 1);

        // Remove the first network.
        let rc = factorial_logic_remove_network(h, net_id);
        assert_eq!(rc, RESULT_OK);

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn constant_combinator_signal() {
        cleanup();
        let (h, node_id) = create_engine_with_node();

        // Register logic module.
        factorial_logic_register(h);

        // Create a network and add the node.
        let mut net_id: u32 = 0;
        unsafe { factorial_logic_create_network(h, 0, &mut net_id) };
        factorial_logic_add_to_network(h, net_id, node_id);

        // Set constant combinator: item 1 = Fixed64(42)
        let item_ids: [u32; 1] = [1];
        let values: [i64; 1] = [Fixed64::from_num(42).to_bits()];
        let rc = unsafe {
            factorial_logic_set_constant(
                h,
                node_id,
                item_ids.as_ptr(),
                values.as_ptr(),
                1,
                1, // enabled
            )
        };
        assert_eq!(rc, RESULT_OK);

        // Step the engine so the logic module ticks.
        factorial_step(h);

        // Read the network signal for item 1.
        let mut value: i64 = 0;
        let rc = unsafe { factorial_logic_get_network_signal(h, net_id, 1, &mut value) };
        assert_eq!(rc, RESULT_OK);
        assert_eq!(
            Fixed64::from_bits(value),
            Fixed64::from_num(42),
            "Expected signal value 42 for item 1"
        );

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn logic_register_invalid_handle() {
        cleanup();
        let rc = factorial_logic_register(99);
        assert_eq!(rc, crate::RESULT_INVALID_HANDLE);
        cleanup();
    }

    #[test]
    fn logic_create_network_without_register() {
        cleanup();
        let h = factorial_create();
        // Don't register the logic module -- should get INTERNAL_ERROR.
        let mut net_id: u32 = 0;
        let rc = unsafe { factorial_logic_create_network(h, 0, &mut net_id) };
        assert_eq!(rc, RESULT_INTERNAL_ERROR);
        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn add_and_remove_from_network() {
        cleanup();
        let (h, node_id) = create_engine_with_node();
        factorial_logic_register(h);

        let mut net_id: u32 = 0;
        unsafe { factorial_logic_create_network(h, 0, &mut net_id) };

        let rc = factorial_logic_add_to_network(h, net_id, node_id);
        assert_eq!(rc, RESULT_OK);

        let rc = factorial_logic_remove_from_network(h, net_id, node_id);
        assert_eq!(rc, RESULT_OK);

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn set_arithmetic_combinator() {
        cleanup();
        let (h, node_id) = create_engine_with_node();
        factorial_logic_register(h);

        // left = Signal(item 0), op = Add, right = Constant(10), output = item 1
        let rc = factorial_logic_set_arithmetic(
            h,
            node_id,
            0,                                      // left_kind: Signal
            0,                                      // left_value: ItemTypeId(0)
            0,                                      // op: Add
            1,                                      // right_kind: Constant
            Fixed64::from_num(10).to_bits() as u64, // right_value
            1,                                      // output_item
        );
        assert_eq!(rc, RESULT_OK);

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn set_decider_combinator() {
        cleanup();
        let (h, node_id) = create_engine_with_node();
        factorial_logic_register(h);

        // left = Signal(0), cmp = Gt, right = Constant(50), output = One(item 1)
        let rc = factorial_logic_set_decider(
            h,
            node_id,
            0,                                      // left_kind: Signal
            0,                                      // left_value: ItemTypeId(0)
            0,                                      // cmp_op: Gt
            1,                                      // right_kind: Constant
            Fixed64::from_num(50).to_bits() as u64, // right_value
            0,                                      // output_kind: One
            1,                                      // output_item
        );
        assert_eq!(rc, RESULT_OK);

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn set_circuit_control_and_query_active() {
        cleanup();
        let (h, node_id) = create_engine_with_node();
        factorial_logic_register(h);

        // Create network, add node, set constant with high value
        let mut net_id: u32 = 0;
        unsafe { factorial_logic_create_network(h, 0, &mut net_id) };
        factorial_logic_add_to_network(h, net_id, node_id);

        // Add a second node with the constant
        let mut pending2: u64 = 0;
        unsafe { factorial_add_node(h, 0, &mut pending2) };
        let mut buf = [0u8; 256];
        let mut written: i32 = 0;
        unsafe { factorial_apply_mutations(h, buf.as_mut_ptr(), 256, &mut written) };
        let node2 = u64::from_le_bytes(buf[16..24].try_into().unwrap());

        factorial_logic_add_to_network(h, net_id, node2);

        // Set constant on node2: item 0 = 100
        let item_ids: [u32; 1] = [0];
        let values: [i64; 1] = [Fixed64::from_num(100).to_bits()];
        unsafe {
            factorial_logic_set_constant(h, node2, item_ids.as_ptr(), values.as_ptr(), 1, 1);
        }

        // Set circuit control on node_id: Signal(0) > Constant(50), Red wire
        let rc = factorial_logic_set_circuit_control(
            h,
            node_id,
            0,                                      // left_kind: Signal
            0,                                      // left_value: ItemTypeId(0)
            0,                                      // cmp_op: Gt
            1,                                      // right_kind: Constant
            Fixed64::from_num(50).to_bits() as u64, // right_value
            0,                                      // wire_color: Red
        );
        assert_eq!(rc, RESULT_OK);

        // Step to evaluate
        factorial_step(h);

        // Query active state
        let mut active: u32 = 99;
        let rc = unsafe { factorial_logic_is_active(h, node_id, &mut active) };
        assert_eq!(rc, RESULT_OK);
        assert_eq!(active, 1, "Circuit control should be active");

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn remove_logic_node() {
        cleanup();
        let (h, node_id) = create_engine_with_node();
        factorial_logic_register(h);

        let item_ids: [u32; 1] = [0];
        let values: [i64; 1] = [Fixed64::from_num(10).to_bits()];
        unsafe {
            factorial_logic_set_constant(h, node_id, item_ids.as_ptr(), values.as_ptr(), 1, 1);
        }

        let rc = factorial_logic_remove_node(h, node_id);
        assert_eq!(rc, RESULT_OK);

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn set_inventory_reader_test() {
        cleanup();
        let (h, node_id) = create_engine_with_node();
        factorial_logic_register(h);

        // Set inventory reader: read Input inventory of same node
        let rc = factorial_logic_set_inventory_reader(h, node_id, node_id, 0);
        assert_eq!(rc, RESULT_OK);

        // Set inventory reader: read Output inventory
        let rc = factorial_logic_set_inventory_reader(h, node_id, node_id, 1);
        assert_eq!(rc, RESULT_OK);

        factorial_destroy(h);
        cleanup();
    }
}
