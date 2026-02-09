//! Serialization WASM exports.

use factorial_core::engine::Engine;

use crate::{
    EVENT_CACHE, EngineSlot, RESULT_DESERIALIZE_ERROR, RESULT_OK, RESULT_SERIALIZE_ERROR,
    register_event_listeners, with_engine, with_table,
};

/// Serialize engine state into the caller-provided buffer at `out_ptr`
/// (capacity `out_len` bytes). Writes the actual byte count to
/// `*out_written_ptr`.
///
/// Returns [`RESULT_OK`] on success, [`RESULT_SERIALIZE_ERROR`] if
/// serialization fails, or [`RESULT_INTERNAL_ERROR`] if the buffer is too
/// small.
///
/// # Safety
///
/// `out_ptr` must point to a valid byte buffer of at least `out_len` bytes.
/// `out_written_ptr` must be a valid, aligned pointer to an `i32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_serialize(
    handle: i32,
    out_ptr: *mut u8,
    out_len: i32,
    out_written_ptr: *mut i32,
) -> i32 {
    if out_ptr.is_null() || out_written_ptr.is_null() {
        return crate::RESULT_INTERNAL_ERROR;
    }
    with_engine(handle, |slot| match slot.engine.serialize() {
        Ok(data) => {
            if data.len() > out_len as usize {
                return crate::RESULT_INTERNAL_ERROR;
            }
            let buf = unsafe { std::slice::from_raw_parts_mut(out_ptr, data.len()) };
            buf.copy_from_slice(&data);
            unsafe { *out_written_ptr = data.len() as i32 };
            RESULT_OK
        }
        Err(_) => RESULT_SERIALIZE_ERROR,
    })
}

/// Deserialize engine state from the byte buffer at `data_ptr` (length
/// `data_len` bytes).
///
/// Returns a new engine handle (>= 0) on success, or a negative error code:
/// - `-RESULT_DESERIALIZE_ERROR` if deserialization fails
/// - `-RESULT_INTERNAL_ERROR` if no handle slot is available
///
/// # Safety
///
/// `data_ptr` must point to at least `data_len` valid bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_deserialize(data_ptr: *const u8, data_len: i32) -> i32 {
    if data_ptr.is_null() || data_len <= 0 {
        return -RESULT_DESERIALIZE_ERROR;
    }
    let slice = unsafe { std::slice::from_raw_parts(data_ptr, data_len as usize) };
    match Engine::deserialize(slice) {
        Ok(mut engine) => {
            register_event_listeners(&mut engine);
            EVENT_CACHE.with(|c| c.borrow_mut().clear());
            with_table(|table| {
                for (i, slot) in table.iter_mut().enumerate() {
                    if slot.is_none() {
                        *slot = Some(EngineSlot {
                            engine,
                            event_cache: Vec::new(),
                        });
                        return i as i32;
                    }
                }
                -crate::RESULT_INTERNAL_ERROR
            })
        }
        Err(_) => -RESULT_DESERIALIZE_ERROR,
    }
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
    use crate::query::{factorial_get_tick, factorial_node_count};
    use crate::transport::factorial_set_output_capacity;
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

    fn setup_source_node(h: i32) -> u64 {
        let mut pending: u64 = 0;
        unsafe { factorial_add_node(h, 0, &mut pending) };
        let mut buf = [0u8; 256];
        let mut written: i32 = 0;
        unsafe { factorial_apply_mutations(h, buf.as_mut_ptr(), 256, &mut written) };
        let real_id = u64::from_le_bytes(buf[16..24].try_into().unwrap());

        let rate = Fixed64::from_num(3).to_bits();
        factorial_set_source(h, real_id, 1, rate);
        factorial_set_output_capacity(h, real_id, 100);
        real_id
    }

    #[test]
    fn serialize_round_trip() {
        cleanup();
        let h = factorial_create();
        let _node_id = setup_source_node(h);

        // Step a few times so tick > 0
        factorial_step(h);
        factorial_step(h);

        let mut tick_before: u64 = 0;
        unsafe { factorial_get_tick(h, &mut tick_before) };
        assert_eq!(tick_before, 2);

        let mut node_count_before: u32 = 0;
        unsafe { factorial_node_count(h, &mut node_count_before) };
        assert_eq!(node_count_before, 1);

        // Serialize
        let mut ser_buf = vec![0u8; 64 * 1024];
        let mut written: i32 = 0;
        let rc = unsafe {
            factorial_serialize(h, ser_buf.as_mut_ptr(), ser_buf.len() as i32, &mut written)
        };
        assert_eq!(rc, RESULT_OK);
        assert!(written > 0);

        // Deserialize into a new engine
        let h2 = unsafe { factorial_deserialize(ser_buf.as_ptr(), written) };
        assert!(h2 >= 0, "deserialize returned error: {h2}");

        // Verify tick and node count match
        let mut tick_after: u64 = 0;
        unsafe { factorial_get_tick(h2, &mut tick_after) };
        assert_eq!(tick_after, tick_before);

        let mut node_count_after: u32 = 0;
        unsafe { factorial_node_count(h2, &mut node_count_after) };
        assert_eq!(node_count_after, node_count_before);

        factorial_destroy(h);
        factorial_destroy(h2);
        cleanup();
    }

    #[test]
    fn deserialize_bad_data() {
        cleanup();
        let bad = [0xFF, 0xFE, 0xFD, 0xFC];
        let h = unsafe { factorial_deserialize(bad.as_ptr(), bad.len() as i32) };
        assert!(
            h < 0,
            "deserialize of bad data should return negative error"
        );
        cleanup();
    }

    #[test]
    fn serialize_invalid_handle() {
        cleanup();
        let mut buf = [0u8; 1024];
        let mut written: i32 = 0;
        let rc = unsafe { factorial_serialize(99, buf.as_mut_ptr(), 1024, &mut written) };
        assert_eq!(rc, crate::RESULT_INVALID_HANDLE);
        cleanup();
    }
}
