//! Event polling WASM exports.

use crate::{EVENT_CACHE, FlatEvent, RESULT_OK, with_engine};

/// Poll events captured during the most recent `factorial_step` or
/// `factorial_advance` call. Copies [`FlatEvent`] structs into the
/// caller-provided buffer at `out_ptr` (capacity `out_len` bytes).
///
/// The number of events written is stored in `*out_count_ptr`.
///
/// Events are stored in the thread-local [`EVENT_CACHE`] and are valid
/// until the next step/advance/destroy call.
///
/// # Safety
///
/// `out_ptr` must point to a valid byte buffer of at least `out_len` bytes.
/// `out_count_ptr` must be a valid, aligned pointer to a `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_poll_events(
    handle: i32,
    out_ptr: *mut u8,
    out_len: i32,
    out_count_ptr: *mut u32,
) -> i32 {
    if out_ptr.is_null() || out_count_ptr.is_null() {
        return crate::RESULT_INTERNAL_ERROR;
    }
    // Validate the handle exists (ensures we don't poll for a dead engine).
    with_engine(handle, |_slot| {
        EVENT_CACHE.with(|c| {
            let cache = c.borrow();
            let event_size = std::mem::size_of::<FlatEvent>();
            let max_events = if event_size > 0 {
                out_len as usize / event_size
            } else {
                0
            };
            let count = cache.len().min(max_events);

            if count > 0 {
                let byte_count = count * event_size;
                let src = cache.as_ptr() as *const u8;
                unsafe {
                    std::ptr::copy_nonoverlapping(src, out_ptr, byte_count);
                }
            }

            unsafe { *out_count_ptr = count as u32 };
        });
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
    use crate::processor::factorial_set_source;
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
    fn poll_events_returns_data() {
        cleanup();
        let h = factorial_create();
        let _node_id = setup_source_node(h);

        // Step to produce events (source production should trigger events).
        factorial_step(h);

        let event_size = std::mem::size_of::<FlatEvent>();
        let buf_size = event_size * 64;
        let mut buf = vec![0u8; buf_size];
        let mut count: u32 = 0;
        let rc = unsafe { factorial_poll_events(h, buf.as_mut_ptr(), buf_size as i32, &mut count) };
        assert_eq!(rc, RESULT_OK);
        assert!(
            count > 0,
            "Expected at least one event after stepping a source node"
        );

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn poll_events_empty_before_step() {
        cleanup();
        let h = factorial_create();

        let event_size = std::mem::size_of::<FlatEvent>();
        let buf_size = event_size * 64;
        let mut buf = vec![0u8; buf_size];
        let mut count: u32 = 99;
        let rc = unsafe { factorial_poll_events(h, buf.as_mut_ptr(), buf_size as i32, &mut count) };
        assert_eq!(rc, RESULT_OK);
        assert_eq!(count, 0, "Expected zero events before any step");

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn poll_events_invalid_handle() {
        cleanup();
        let event_size = std::mem::size_of::<FlatEvent>();
        let buf_size = event_size * 64;
        let mut buf = vec![0u8; buf_size];
        let mut count: u32 = 0;
        let rc =
            unsafe { factorial_poll_events(99, buf.as_mut_ptr(), buf_size as i32, &mut count) };
        assert_eq!(rc, crate::RESULT_INVALID_HANDLE);
        cleanup();
    }
}
