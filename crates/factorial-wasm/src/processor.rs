//! Processor configuration WASM exports.

use factorial_core::fixed::Fixed64;
use factorial_core::id::ItemTypeId;
use factorial_core::processor::{
    Depletion, FixedRecipe, Processor, RecipeInput, RecipeOutput, SourceProcessor,
};

use crate::{RESULT_OK, ffi_to_node_id, with_engine};

/// Configure a node as a source processor (mine, extractor, well).
///
/// `rate` is the fixed-point bits representation of the base production rate
/// per tick.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_set_source(
    handle: i32,
    node_id: u64,
    item_type: u32,
    rate: i64,
) -> i32 {
    with_engine(handle, |slot| {
        let nid = ffi_to_node_id(node_id);
        slot.engine.set_processor(
            nid,
            Processor::Source(SourceProcessor {
                output_type: ItemTypeId(item_type),
                base_rate: Fixed64::from_bits(rate),
                depletion: Depletion::Infinite,
                accumulated: Fixed64::from_num(0),
                initial_properties: None,
            }),
        );
        RESULT_OK
    })
}

/// Configure a node with a fixed recipe processor (assembler, smelter).
///
/// `recipe_ptr` points to a flat buffer with the following layout:
///
/// | Offset | Size | Description |
/// |--------|------|-------------|
/// | 0      | 4    | `input_count` (u32 LE) |
/// | 4      | 4    | `output_count` (u32 LE) |
/// | 8      | 4    | `duration` (u32 LE) |
/// | 12     | 8 * input_count  | (item_type: u32 LE, quantity: u32 LE) pairs |
/// | ...    | 8 * output_count | (item_type: u32 LE, quantity: u32 LE) pairs |
///
/// # Safety
///
/// `recipe_ptr` must point to a valid byte buffer of at least `recipe_len`
/// bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_fixed_processor(
    handle: i32,
    node_id: u64,
    recipe_ptr: *const u8,
    recipe_len: i32,
) -> i32 {
    if recipe_ptr.is_null() || recipe_len < 12 {
        return crate::RESULT_INTERNAL_ERROR;
    }

    let data = unsafe { std::slice::from_raw_parts(recipe_ptr, recipe_len as usize) };

    let input_count = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    let output_count = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let duration = u32::from_le_bytes(data[8..12].try_into().unwrap());

    let expected_len = 12 + (input_count + output_count) * 8;
    if (recipe_len as usize) < expected_len {
        return crate::RESULT_INTERNAL_ERROR;
    }

    let mut offset = 12;
    let mut inputs = Vec::with_capacity(input_count);
    for _ in 0..input_count {
        let item_type = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        let quantity = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
        inputs.push(RecipeInput {
            item_type: ItemTypeId(item_type),
            quantity,
            consumed: true,
        });
        offset += 8;
    }

    let mut outputs = Vec::with_capacity(output_count);
    for _ in 0..output_count {
        let item_type = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        let quantity = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
        outputs.push(RecipeOutput {
            item_type: ItemTypeId(item_type),
            quantity,
            bonus: None,
        });
        offset += 8;
    }

    with_engine(handle, |slot| {
        let nid = ffi_to_node_id(node_id);
        slot.engine.set_processor(
            nid,
            Processor::Fixed(FixedRecipe {
                inputs,
                outputs,
                duration,
            }),
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
    use crate::query::factorial_get_output_inventory_count;
    use crate::transport::factorial_set_output_capacity;
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
    fn set_source_and_produce() {
        cleanup();
        let (h, node_id) = create_engine_with_node();

        // Set source: 1 item/tick of type 0
        let rate = Fixed64::from_num(1).to_bits();
        let rc = factorial_set_source(h, node_id, 0, rate);
        assert_eq!(rc, RESULT_OK);

        // Give it an output inventory
        factorial_set_output_capacity(h, node_id, 100);

        // Run a few ticks
        for _ in 0..5 {
            factorial_step(h);
        }

        // Should have produced items into the output inventory
        let mut count: u32 = 0;
        unsafe { factorial_get_output_inventory_count(h, node_id, &mut count) };
        assert!(count > 0, "Source should have produced items");

        factorial_destroy(h);
        cleanup();
    }

    #[test]
    fn set_fixed_processor_and_run() {
        cleanup();
        let (h, node_id) = create_engine_with_node();

        // Build recipe buffer: 1 input (type 0, qty 1), 1 output (type 1, qty 1), duration 2
        let mut recipe = Vec::new();
        recipe.extend_from_slice(&1u32.to_le_bytes()); // input_count
        recipe.extend_from_slice(&1u32.to_le_bytes()); // output_count
        recipe.extend_from_slice(&2u32.to_le_bytes()); // duration
        recipe.extend_from_slice(&0u32.to_le_bytes()); // input item_type
        recipe.extend_from_slice(&1u32.to_le_bytes()); // input quantity
        recipe.extend_from_slice(&1u32.to_le_bytes()); // output item_type
        recipe.extend_from_slice(&1u32.to_le_bytes()); // output quantity

        let rc = unsafe {
            factorial_set_fixed_processor(h, node_id, recipe.as_ptr(), recipe.len() as i32)
        };
        assert_eq!(rc, RESULT_OK);

        // Give it inventories
        crate::transport::factorial_set_input_capacity(h, node_id, 100);
        factorial_set_output_capacity(h, node_id, 100);

        // Step — it will stall on missing inputs but shouldn't crash.
        factorial_step(h);

        // Verify processor state is stalled (missing inputs = 2)
        let mut state: u32 = 99;
        let mut progress: u32 = 99;
        unsafe {
            crate::query::factorial_get_processor_state(h, node_id, &mut state, &mut progress)
        };
        assert_eq!(state, 2); // StalledMissingInputs

        factorial_destroy(h);
        cleanup();
    }
}
