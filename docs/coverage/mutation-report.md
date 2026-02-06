# Mutation Testing Report

**Date:** 2026-02-06
**Tool:** cargo-mutants 26.2.0
**Scope:** factorial-core (10 files), factorial-ffi (1 file)

## Summary

| Package | Total Mutants | Killed | Survived | Unviable | Timeout | Kill Rate* |
|---------|--------------|--------|----------|----------|---------|------------|
| factorial-core | 382 | 288 | 68 | 26 | 0 | 80.9% |
| factorial-ffi | 9 | 7 | 0 | 2 | 0 | 100.0% |
| **Combined** | **391** | **295** | **68** | **28** | **0** | **81.3%** |

*Kill rate excludes unviable mutants from the denominator.

### Files Tested (factorial-core)

| File | Mutants | Killed | Survived | Unviable |
|------|---------|--------|----------|----------|
| graph.rs | 75 | 64 | 5 | 6 |
| dirty.rs | 23 | 22 | 0 | 1 |
| fixed.rs | 12 | 10 | 2 | 0 |
| serialize.rs | 87 | 68 | 13 | 6 |
| validation.rs | 26 | 13 | 5 | 8 |
| sim.rs | 8 | 8 | 0 | 0 |
| processor.rs | 142 | 97 | 40 | 5 |
| data_loader.rs | 8 | 5 | 3 | 0 |

Note: engine.rs (283 mutants), transport.rs (80), event.rs (77), item.rs (54), and other files were not tested due to time constraints. Total factorial-core mutant population is 998.

## Notable Survivors (Worth Killing)

### processor.rs -- Production Logic (40 survivors)

These survivors are in core simulation tick functions and represent real behavioral risks:

**tick_source (effective_rate calculation):**
- `line 348: replace * with /` -- effective_rate uses `base_rate * speed * productivity`; mutating `*` to `/` survives because tests only check base rate or 2x speed (where division would also produce non-zero output).
- `line 373: replace > with ==/>=/< in state transition` -- boundary condition for deciding Working vs Idle based on `effective_rate > 0`.

**tick_fixed (duration ceiling):**
- `line 409: replace > with <` in fractional ceiling check
- `line 410: replace + with -/*` in ceiling calculation `raw + 1`
- `line 422: replace < with <=` in output space check
- `line 442-443: replace >/+ operators` in input availability check

**tick_demand (consumption logic):**
- `line 607: replace == with !=` in single-type mode available check
- `line 611: replace > with >=` in whole-items boundary
- `line 616: replace - with +` in accumulated subtraction
- `line 617: replace > with >=` in stall threshold
- `line 622: replace == with != / && with ||` in multi-type stall condition
- `line 624: replace != with ==` in state change comparison
- `line 643: replace > with >=` in available items check
- `line 666: delete ! / replace || with && / replace > with ==/</>= ` in Working/Idle state transition

**tick_passthrough:**
- `line 705: replace > with >=` in to_move boundary
- `line 708: replace -= with +=//=` in space_remaining decrement

**ResolvedModifiers::resolve:**
- `line 253: replace > with >=` in Capped stacking rule

### Targeted Tests Written

14 new tests were added to kill high-value survivors:

- `checked_mul_64_happy_path` / `checked_div_64_happy_path` (fixed.rs) -- kill "replace with None" mutants
- `source_speed_modifier_doubles_rate` -- kill effective_rate `*` to `/` mutant
- `source_zero_rate_goes_idle` -- kill state transition boundary mutants
- `fixed_recipe_fractional_speed_ceils_duration` -- kill ceiling `+ with -/*` mutants
- `fixed_recipe_exact_output_space_starts` -- kill `< with <=` boundary mutant
- `demand_consumed_total_tracks_correctly` -- kill consumed_total tracking mutants
- `demand_accumulated_decreases_after_consume` -- kill `- with +` accumulated mutant
- `passthrough_multiple_types_respects_total_space` -- kill `-= with +=` space mutant
- `passthrough_zero_qty_items_not_emitted` -- kill `> with >=` boundary mutant
- `demand_stalls_when_items_wanted_but_none_consumed` -- kill stall condition mutants
- `stacking_capped_equal_values` -- kill `> with >=` in Capped stacking
- `property_processor_respects_output_space` -- kill `!= with ==` mutants
- `connect_filtered_increments_pending_edge_id` -- kill `+= with *=` mutant
- `topological_order_with_feedback_respects_in_degree` -- kill in-degree `-= with +=//=` mutants
- `deserialized_graph_recomputes_topo` -- kill `default_dirty -> false` mutant

### serialize.rs -- Hash Functions (13 survivors)

- `hash_graph`, `hash_processors`, `hash_processor_states`, `hash_inventories`, `hash_transports` -- replacing these with constant 0 or 1 survives.
- `serde_json_key_bytes` -- replacing with `[0; 8]` or `[1; 8]` survives.
- `SnapshotRingBuffer::is_empty` -- replacing with `true` survives.

These hash functions are used for change detection and determinism validation. While they are production code, they are auxiliary (used for diagnostics, not simulation correctness). The existing tests for `serialize_state_hash_changes_with_state` and `serialize_subsystem_hashes_change_independently` test the composite hash but not individual subsystem hashes in isolation.

### validation.rs -- diff_engines (5 survivors)

- `lines 150-154: replace && with ||` in the `is_identical` compound boolean expression.

Each `&&` can be mutated to `||` individually without being caught because the test only checks the final `is_identical` result, not each individual subsystem comparison. The mutation makes the check more permissive, and existing tests don't exercise cases where exactly one subsystem differs.

### data_loader.rs (3 survivors)

- `line 98: replace load_registry_json_bytes -> Ok(Default::default())` -- no test calls `load_registry_json_bytes` directly.
- `lines 104-122: delete match arms for "fixed64", "fixed32", "u32", "u8"` -- property type parsing falls through to the default case.

## Accepted Survivors

The following survivors are considered acceptable (low risk, not worth the test investment):

1. **serialize.rs hash helpers** -- Individual subsystem hashes are internal implementation details. The composite `StateHash` is tested. Adding isolated tests would couple tests to hashing internals.

2. **validation.rs `&&` to `||`** -- The `diff_engines` function is a diagnostic tool, not simulation logic. The compound boolean is correct and tested for the common case.

3. **data_loader.rs property parsing** -- The match arm deletions fall to a default case that produces a valid `PropertyDef` (just with different type). This is a data loading concern, not simulation correctness.

## Recommendations

1. **processor.rs** remains the highest-priority area for additional test coverage. The 40 survivors represent real mutation opportunities in core simulation logic. The 14 targeted tests written in this pass address the most critical ones.

2. **engine.rs** (283 mutants, untested) should be the next target for mutation testing. It contains the main simulation loop and is likely to have similar survivor patterns.

3. The overall kill rate of 80.9% for factorial-core is reasonable for a first pass but could be improved to 85-90% with targeted tests for the remaining processor.rs boundary conditions.
