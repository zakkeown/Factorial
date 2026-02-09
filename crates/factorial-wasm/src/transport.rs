//! Transport configuration WASM exports.

use factorial_core::fixed::Fixed64;
use factorial_core::item::Inventory;
use factorial_core::transport::{
    BatchTransport, FlowTransport, ItemTransport, Transport, VehicleTransport,
};

use crate::{RESULT_OK, ffi_to_edge_id, ffi_to_node_id, with_engine};

/// Set the transport on `edge_id` to a continuous flow transport.
///
/// `rate` is the fixed-point bits representation of items per tick.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_set_flow_transport(handle: i32, edge_id: u64, rate: i64) -> i32 {
    with_engine(handle, |slot| {
        let eid = ffi_to_edge_id(edge_id);
        slot.engine.set_transport(
            eid,
            Transport::Flow(FlowTransport {
                rate: Fixed64::from_bits(rate),
                buffer_capacity: Fixed64::from_num(1000),
                latency: 0,
            }),
        );
        RESULT_OK
    })
}

/// Set the transport on `edge_id` to a discrete item (belt) transport.
///
/// `speed` is the fixed-point bits representation of slots advanced per tick.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_set_item_transport(
    handle: i32,
    edge_id: u64,
    speed: i64,
    slot_count: u32,
    lanes: u8,
) -> i32 {
    with_engine(handle, |slot| {
        let eid = ffi_to_edge_id(edge_id);
        slot.engine.set_transport(
            eid,
            Transport::Item(ItemTransport {
                speed: Fixed64::from_bits(speed),
                slot_count,
                lanes,
            }),
        );
        RESULT_OK
    })
}

/// Set the transport on `edge_id` to a batch transport.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_set_batch_transport(
    handle: i32,
    edge_id: u64,
    batch_size: u32,
    cycle_time: u32,
) -> i32 {
    with_engine(handle, |slot| {
        let eid = ffi_to_edge_id(edge_id);
        slot.engine.set_transport(
            eid,
            Transport::Batch(BatchTransport {
                batch_size,
                cycle_time,
            }),
        );
        RESULT_OK
    })
}

/// Set the transport on `edge_id` to a vehicle transport.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_set_vehicle_transport(
    handle: i32,
    edge_id: u64,
    capacity: u32,
    travel_time: u32,
) -> i32 {
    with_engine(handle, |slot| {
        let eid = ffi_to_edge_id(edge_id);
        slot.engine.set_transport(
            eid,
            Transport::Vehicle(VehicleTransport {
                capacity,
                travel_time,
            }),
        );
        RESULT_OK
    })
}

/// Set the input inventory capacity for `node_id`.
///
/// Creates an inventory with 1 input slot and 1 output slot, each with the
/// given capacity.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_set_input_capacity(handle: i32, node_id: u64, capacity: u32) -> i32 {
    with_engine(handle, |slot| {
        let nid = ffi_to_node_id(node_id);
        slot.engine
            .set_input_inventory(nid, Inventory::new(1, 1, capacity));
        RESULT_OK
    })
}

/// Set the output inventory capacity for `node_id`.
///
/// Creates an inventory with 1 input slot and 1 output slot, each with the
/// given capacity.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_set_output_capacity(handle: i32, node_id: u64, capacity: u32) -> i32 {
    with_engine(handle, |slot| {
        let nid = ffi_to_node_id(node_id);
        slot.engine
            .set_output_inventory(nid, Inventory::new(1, 1, capacity));
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
    use crate::graph::{factorial_add_node, factorial_apply_mutations, factorial_connect};
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

    /// Create two nodes and connect them, returning (handle, n1, n2, edge_id).
    fn create_engine_with_edge() -> (i32, u64, u64, u64) {
        let h = factorial_create();
        let mut p1: u64 = 0;
        let mut p2: u64 = 0;
        unsafe { factorial_add_node(h, 0, &mut p1) };
        unsafe { factorial_add_node(h, 0, &mut p2) };

        let mut buf = [0u8; 512];
        let mut written: i32 = 0;
        unsafe { factorial_apply_mutations(h, buf.as_mut_ptr(), 512, &mut written) };

        let n1 = u64::from_le_bytes(buf[16..24].try_into().unwrap());
        let n2 = u64::from_le_bytes(buf[32..40].try_into().unwrap());

        let mut edge_pending: u64 = 0;
        unsafe { factorial_connect(h, n1, n2, &mut edge_pending) };

        let mut buf2 = [0u8; 512];
        let mut written2: i32 = 0;
        unsafe { factorial_apply_mutations(h, buf2.as_mut_ptr(), 512, &mut written2) };

        let edge_id = u64::from_le_bytes(buf2[16..24].try_into().unwrap());

        (h, n1, n2, edge_id)
    }

    #[test]
    fn set_all_transport_types() {
        cleanup();
        let (h, _n1, _n2, edge_id) = create_engine_with_edge();

        // Flow
        let rate = Fixed64::from_num(10).to_bits();
        let rc = factorial_set_flow_transport(h, edge_id, rate);
        assert_eq!(rc, RESULT_OK);

        // Item
        let speed = Fixed64::from_num(1).to_bits();
        let rc = factorial_set_item_transport(h, edge_id, speed, 10, 1);
        assert_eq!(rc, RESULT_OK);

        // Batch
        let rc = factorial_set_batch_transport(h, edge_id, 5, 10);
        assert_eq!(rc, RESULT_OK);

        // Vehicle
        let rc = factorial_set_vehicle_transport(h, edge_id, 20, 30);
        assert_eq!(rc, RESULT_OK);

        // Step to verify engine is healthy after all transport configurations
        assert_eq!(factorial_step(h), crate::RESULT_OK);

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn set_capacities() {
        cleanup();
        let (h, node_id) = create_engine_with_node();

        let rc = factorial_set_input_capacity(h, node_id, 50);
        assert_eq!(rc, RESULT_OK);

        let rc = factorial_set_output_capacity(h, node_id, 100);
        assert_eq!(rc, RESULT_OK);

        // Verify by querying inventory counts (should be 0 items, but no error)
        let mut in_count: u32 = 99;
        let mut out_count: u32 = 99;
        unsafe { crate::query::factorial_get_input_inventory_count(h, node_id, &mut in_count) };
        unsafe { crate::query::factorial_get_output_inventory_count(h, node_id, &mut out_count) };
        assert_eq!(in_count, 0);
        assert_eq!(out_count, 0);

        factorial_destroy(h);
        cleanup();
    }
}
