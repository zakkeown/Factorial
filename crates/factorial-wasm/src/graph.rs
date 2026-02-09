//! Graph mutation WASM exports.

use factorial_core::id::BuildingTypeId;

use crate::{
    RESULT_OK, edge_id_to_ffi, ffi_to_edge_id, ffi_to_node_id, node_id_to_ffi, with_engine,
};

/// Queue a new node with the given building type. Writes the pending ID to
/// `*out_pending_ptr`. The node is not yet live -- call
/// [`factorial_apply_mutations`] to materialise it.
///
/// # Safety
///
/// `out_pending_ptr` must be a valid, aligned pointer to a `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_add_node(
    handle: i32,
    building_type: u32,
    out_pending_ptr: *mut u64,
) -> i32 {
    if out_pending_ptr.is_null() {
        return crate::RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| {
        let pending = slot
            .engine
            .graph
            .queue_add_node(BuildingTypeId(building_type));
        unsafe { *out_pending_ptr = pending.0 };
        RESULT_OK
    })
}

/// Queue a node for removal.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_remove_node(handle: i32, node_id: u64) -> i32 {
    with_engine(handle, |slot| {
        slot.engine.graph.queue_remove_node(ffi_to_node_id(node_id));
        RESULT_OK
    })
}

/// Queue an edge connecting `from_node` to `to_node`. Writes the pending edge
/// ID to `*out_pending_ptr`.
///
/// # Safety
///
/// `out_pending_ptr` must be a valid, aligned pointer to a `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_connect(
    handle: i32,
    from_node: u64,
    to_node: u64,
    out_pending_ptr: *mut u64,
) -> i32 {
    if out_pending_ptr.is_null() {
        return crate::RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| {
        let pending = slot
            .engine
            .graph
            .queue_connect(ffi_to_node_id(from_node), ffi_to_node_id(to_node));
        unsafe { *out_pending_ptr = pending.0 };
        RESULT_OK
    })
}

/// Queue an edge for removal.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_disconnect(handle: i32, edge_id: u64) -> i32 {
    with_engine(handle, |slot| {
        slot.engine.graph.queue_disconnect(ffi_to_edge_id(edge_id));
        RESULT_OK
    })
}

/// Apply all queued graph mutations and write the results into a flat byte
/// buffer at `out_ptr` (capacity `out_len` bytes).
///
/// # Buffer layout
///
/// | Offset | Size | Description |
/// |--------|------|-------------|
/// | 0      | 4    | `node_count` (u32 LE) |
/// | 4      | 4    | `edge_count` (u32 LE) |
/// | 8      | 16 * node_count | (pending_id: u64 LE, real_id: u64 LE) pairs |
/// | ...    | 16 * edge_count | (pending_id: u64 LE, real_id: u64 LE) pairs |
///
/// The actual number of bytes written is stored in `*out_written_ptr`.
///
/// # Safety
///
/// `out_ptr` must point to a valid byte buffer of at least `out_len` bytes.
/// `out_written_ptr` must be a valid, aligned pointer to an `i32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_apply_mutations(
    handle: i32,
    out_ptr: *mut u8,
    out_len: i32,
    out_written_ptr: *mut i32,
) -> i32 {
    if out_ptr.is_null() || out_written_ptr.is_null() {
        return crate::RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| {
        let result = slot.engine.graph.apply_mutations();

        let node_count = result.added_nodes.len() as u32;
        let edge_count = result.added_edges.len() as u32;
        let needed = 8 + (node_count as usize + edge_count as usize) * 16;

        if (out_len as usize) < needed {
            return crate::RESULT_INTERNAL_ERROR;
        }

        let buf = unsafe { std::slice::from_raw_parts_mut(out_ptr, out_len as usize) };
        let mut offset = 0;

        // Header: node_count, edge_count
        buf[offset..offset + 4].copy_from_slice(&node_count.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&edge_count.to_le_bytes());
        offset += 4;

        // Node pairs
        for (pending, real) in &result.added_nodes {
            buf[offset..offset + 8].copy_from_slice(&pending.0.to_le_bytes());
            offset += 8;
            buf[offset..offset + 8].copy_from_slice(&node_id_to_ffi(*real).to_le_bytes());
            offset += 8;
        }

        // Edge pairs
        for (pending, real) in &result.added_edges {
            buf[offset..offset + 8].copy_from_slice(&pending.0.to_le_bytes());
            offset += 8;
            buf[offset..offset + 8].copy_from_slice(&edge_id_to_ffi(*real).to_le_bytes());
            offset += 8;
        }

        unsafe { *out_written_ptr = needed as i32 };
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
    use crate::{EVENT_CACHE, HANDLE_TABLE};

    fn cleanup() {
        HANDLE_TABLE.with(|t| {
            for s in t.borrow_mut().iter_mut() {
                *s = None;
            }
        });
        EVENT_CACHE.with(|c| c.borrow_mut().clear());
    }

    /// Helper: create an engine, add one node, apply mutations, return
    /// (handle, real_node_id).
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
    fn add_node_and_apply() {
        cleanup();
        let h = factorial_create();
        let mut pending: u64 = 0;
        let rc = unsafe { factorial_add_node(h, 0, &mut pending) };
        assert_eq!(rc, RESULT_OK);

        let mut buf = [0u8; 256];
        let mut written: i32 = 0;
        let rc = unsafe { factorial_apply_mutations(h, buf.as_mut_ptr(), 256, &mut written) };
        assert_eq!(rc, RESULT_OK);
        assert!(written > 0);

        // Header: 1 node, 0 edges
        let node_count = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let edge_count = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        assert_eq!(node_count, 1);
        assert_eq!(edge_count, 0);

        // Pending ID should match what we got back
        let pending_back = u64::from_le_bytes(buf[8..16].try_into().unwrap());
        assert_eq!(pending_back, pending);

        // Real ID should be non-zero
        let real_id = u64::from_le_bytes(buf[16..24].try_into().unwrap());
        assert_ne!(real_id, 0);

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn add_node_invalid_handle() {
        cleanup();
        let mut pending: u64 = 0;
        let rc = unsafe { factorial_add_node(99, 0, &mut pending) };
        assert_eq!(rc, crate::RESULT_INVALID_HANDLE);
        cleanup();
    }

    #[test]
    fn connect_and_disconnect() {
        cleanup();
        let h = factorial_create();

        // Add two nodes
        let mut p1: u64 = 0;
        let mut p2: u64 = 0;
        unsafe { factorial_add_node(h, 0, &mut p1) };
        unsafe { factorial_add_node(h, 0, &mut p2) };

        let mut buf = [0u8; 512];
        let mut written: i32 = 0;
        unsafe { factorial_apply_mutations(h, buf.as_mut_ptr(), 512, &mut written) };

        let n1 = u64::from_le_bytes(buf[16..24].try_into().unwrap());
        let n2 = u64::from_le_bytes(buf[32..40].try_into().unwrap());

        // Connect them
        let mut edge_pending: u64 = 0;
        let rc = unsafe { factorial_connect(h, n1, n2, &mut edge_pending) };
        assert_eq!(rc, RESULT_OK);

        let mut buf2 = [0u8; 512];
        let mut written2: i32 = 0;
        unsafe { factorial_apply_mutations(h, buf2.as_mut_ptr(), 512, &mut written2) };

        let edge_count = u32::from_le_bytes(buf2[4..8].try_into().unwrap());
        assert_eq!(edge_count, 1);

        let real_edge = u64::from_le_bytes(buf2[16..24].try_into().unwrap());

        // Disconnect
        let rc = factorial_disconnect(h, real_edge);
        assert_eq!(rc, RESULT_OK);

        // Apply the disconnect
        let mut buf3 = [0u8; 512];
        let mut written3: i32 = 0;
        unsafe { factorial_apply_mutations(h, buf3.as_mut_ptr(), 512, &mut written3) };
        assert_eq!(written3, 8); // header only, no new nodes/edges

        // Step to verify engine is still healthy
        assert_eq!(factorial_step(h), RESULT_OK);

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn remove_node() {
        cleanup();
        let (h, node_id) = create_engine_with_node();

        let rc = factorial_remove_node(h, node_id);
        assert_eq!(rc, RESULT_OK);

        // Apply the removal
        let mut buf = [0u8; 256];
        let mut written: i32 = 0;
        unsafe { factorial_apply_mutations(h, buf.as_mut_ptr(), 256, &mut written) };

        // Step to verify engine is still healthy
        assert_eq!(factorial_step(h), RESULT_OK);

        factorial_destroy(h);
        cleanup();
    }
}
