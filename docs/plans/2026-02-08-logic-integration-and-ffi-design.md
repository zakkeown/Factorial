# Logic Integration & FFI Bindings Design

**Date:** 2026-02-08
**Status:** Draft
**Scope:** Wire LogicModule into the engine's Module system; add C FFI bindings; add 3 integration tests

---

## Overview

The `factorial-logic` crate exists as a standalone module with wire networks, combinators, conditions, and a tick pipeline. This design covers three pieces of work:

1. **Module bridge** — Register LogicModule as a `Module` so it auto-ticks in phase 4 alongside junctions and other modules.
2. **FFI bindings** — Expose logic network operations through the existing `FactorialEngine` handle so C/C++/Godot/Unity can create wire networks, configure combinators, and query circuit control state.
3. **Integration tests** — Three cross-crate test scenarios exercising logic networks inside a running engine.

---

## Part 1: Module Trait Enhancement (factorial-core)

### Problem

The `Module` trait provides `on_tick()`, `serialize_state()`, and `load_state()`, but no way to downcast a `&dyn Module` back to a concrete type. This means callers who register a `LogicModuleBridge` as a module can't access its configuration API afterwards.

### Solution

Add `as_any()` / `as_any_mut()` methods to the Module trait:

```rust
pub trait Module: std::fmt::Debug {
    fn name(&self) -> &str;
    fn on_tick(&mut self, ctx: &mut ModuleContext<'_>) { let _ = ctx; }
    fn serialize_state(&self) -> Vec<u8> { Vec::new() }
    fn load_state(&mut self, _data: &[u8]) -> Result<(), ModuleError> { Ok(()) }

    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
```

Every Module impl adds two one-liner methods:

```rust
fn as_any(&self) -> &dyn std::any::Any { self }
fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
```

Add convenience methods on Engine:

```rust
impl Engine {
    /// Find a registered module by concrete type.
    pub fn find_module<T: Module + 'static>(&self) -> Option<&T> {
        self.modules.iter()
            .find_map(|m| m.as_any().downcast_ref::<T>())
    }

    /// Find a registered module by concrete type (mutable).
    pub fn find_module_mut<T: Module + 'static>(&mut self) -> Option<&mut T> {
        self.modules.iter_mut()
            .find_map(|m| m.as_any_mut().downcast_mut::<T>())
    }
}
```

### Impact

- Backward compatible: existing Module impls must add the two `as_any` methods, but they're trivial.
- All test modules in `module.rs` tests need updating (4 structs: CounterModule, GraphInspectorModule, InventoryModifierModule, StatefulModule).
- Standard Rust pattern used by Bevy, Axum, etc.

---

## Part 2: LogicModuleBridge (factorial-logic)

### New file: `crates/factorial-logic/src/bridge.rs`

A thin wrapper that holds a `LogicModule` and implements `factorial_core::module::Module`.

```rust
use factorial_core::module::{Module, ModuleContext, ModuleError};
use crate::LogicModule;

#[derive(Debug)]
pub struct LogicModuleBridge {
    logic: LogicModule,
}

impl LogicModuleBridge {
    pub fn new() -> Self {
        Self { logic: LogicModule::new() }
    }

    /// Access the inner LogicModule for configuration.
    pub fn logic(&self) -> &LogicModule { &self.logic }

    /// Access the inner LogicModule for configuration (mutable).
    pub fn logic_mut(&mut self) -> &mut LogicModule { &mut self.logic }
}

impl Module for LogicModuleBridge {
    fn name(&self) -> &str { "logic" }

    fn on_tick(&mut self, ctx: &mut ModuleContext<'_>) {
        let _events = self.logic.tick(ctx.inputs, ctx.outputs, ctx.tick);
        // Events are stored internally; callers query via logic().
        // Future: bridge LogicEvents into the core EventBus.
    }

    fn serialize_state(&self) -> Vec<u8> {
        bitcode::serialize(&self.logic).unwrap_or_default()
    }

    fn load_state(&mut self, data: &[u8]) -> Result<(), ModuleError> {
        self.logic = bitcode::deserialize(data)
            .map_err(|e| ModuleError::DeserializeFailed(e.to_string()))?;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
```

### Usage

```rust
// Register
engine.register_module(Box::new(LogicModuleBridge::new()));

// Configure
let bridge = engine.find_module_mut::<LogicModuleBridge>().unwrap();
let net = bridge.logic_mut().create_network(WireColor::Red);
bridge.logic_mut().add_to_network(net, chest_node);

// After engine.step(), logic is auto-ticked
engine.step();

// Query
let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
let active = bridge.logic().is_active(inserter_node);
```

### Dependencies

`bridge.rs` needs `bitcode` promoted from dev-dependency to regular dependency in factorial-logic's Cargo.toml (for serialize_state/load_state).

### Module re-export

Add `pub mod bridge;` to `lib.rs` and re-export: `pub use bridge::LogicModuleBridge;`

---

## Part 3: FFI Bindings (factorial-ffi)

### Approach

All logic functions operate on the existing `*mut FactorialEngine` handle. Internally, each function:

1. Validates the handle (null check, poison check)
2. Finds the `LogicModuleBridge` via `find_module_mut::<LogicModuleBridge>()`
3. Returns `FactorialResult::InternalError` if logic module not registered
4. Calls the appropriate LogicModule method
5. Wraps in `catch_unwind` for panic safety

### New FFI types

```c
// Wire color enum
typedef enum { WIRE_RED = 0, WIRE_GREEN = 1 } FfiWireColor;

// Signal selector kind
typedef enum { SELECTOR_SIGNAL = 0, SELECTOR_CONSTANT = 1, SELECTOR_EACH = 2 } FfiSelectorKind;

// Arithmetic operation
typedef enum { ARITH_ADD = 0, ARITH_SUB = 1, ARITH_MUL = 2, ARITH_DIV = 3, ARITH_MOD = 4 } FfiArithmeticOp;

// Comparison operation
typedef enum { CMP_GT = 0, CMP_LT = 1, CMP_EQ = 2, CMP_GTE = 3, CMP_LTE = 4, CMP_NE = 5 } FfiComparisonOp;

// Decider output kind
typedef enum { DECIDER_ONE = 0, DECIDER_INPUT_COUNT = 1, DECIDER_EVERYTHING = 2 } FfiDeciderOutputKind;

// Wire network ID (u32)
typedef uint32_t FfiWireNetworkId;
```

### New FFI functions

```c
// Registration
FactorialResult factorial_logic_register(FactorialEngine* engine);

// Network management
FactorialResult factorial_logic_create_network(FactorialEngine* engine, FfiWireColor color, FfiWireNetworkId* out_id);
FactorialResult factorial_logic_remove_network(FactorialEngine* engine, FfiWireNetworkId network_id);
FactorialResult factorial_logic_add_to_network(FactorialEngine* engine, FfiWireNetworkId network_id, FfiNodeId node_id);
FactorialResult factorial_logic_remove_from_network(FactorialEngine* engine, FfiWireNetworkId network_id, FfiNodeId node_id);

// Signal sources
FactorialResult factorial_logic_set_constant(FactorialEngine* engine, FfiNodeId node_id, const uint64_t* item_ids, const int64_t* values, uint32_t count, bool enabled);
FactorialResult factorial_logic_set_arithmetic(FactorialEngine* engine, FfiNodeId node_id, FfiSelectorKind left_kind, uint64_t left_value, FfiArithmeticOp op, FfiSelectorKind right_kind, uint64_t right_value, uint64_t output_item);
FactorialResult factorial_logic_set_decider(FactorialEngine* engine, FfiNodeId node_id, FfiSelectorKind left_kind, uint64_t left_value, FfiComparisonOp cmp_op, FfiSelectorKind right_kind, uint64_t right_value, FfiDeciderOutputKind output_kind, uint64_t output_item);

// Circuit control
FactorialResult factorial_logic_set_circuit_control(FactorialEngine* engine, FfiNodeId node_id, FfiSelectorKind left_kind, uint64_t left_value, FfiComparisonOp cmp_op, FfiSelectorKind right_kind, uint64_t right_value, FfiWireColor wire_color);

// Queries
FactorialResult factorial_logic_is_active(FactorialEngine* engine, FfiNodeId node_id, bool* out_active);
FactorialResult factorial_logic_get_network_signal(FactorialEngine* engine, FfiWireNetworkId network_id, uint64_t item_id, int64_t* out_value);

// Cleanup
FactorialResult factorial_logic_remove_node(FactorialEngine* engine, FfiNodeId node_id);
```

### Cargo.toml change

`factorial-ffi` adds `factorial-logic` as a dependency.

---

## Part 4: Integration Tests

### New file: `crates/factorial-integration-tests/tests/logic_networks.rs`

Three test scenarios exercising logic networks inside a running engine.

### Test 1: Circuit-controlled inserter (Factorio-style)

A source node produces iron ore. An inserter (junction) moves iron into a chest. The chest has an inventory reader broadcasting on a red wire network. A second inserter downstream has a circuit control: "iron ore > 100 → enable."

Steps:
1. Build the production graph: source → inserter → chest → inserter → sink
2. Register LogicModuleBridge, create red wire network
3. Connect chest and downstream inserter to the network
4. Set inventory reader on chest (output inventory)
5. Set circuit control on downstream inserter: iron > 100
6. Run engine for N ticks until chest accumulates > 100 iron
7. Assert: downstream inserter `is_active() == Some(true)`
8. Assert: CircuitActivated event fired at the transition tick
9. Stop source, drain chest below 100
10. Assert: CircuitDeactivated event fires

### Test 2: Conditional production shutdown

A smelter (FixedRecipe processor) produces steel plates. A constant combinator outputs "steel threshold = 50." A decider combinator compares the smelter's output inventory against the threshold. When output > 50, circuit control deactivates the smelter. The test verifies production stops when the threshold is exceeded and resumes when plates are consumed.

Steps:
1. Build: ore source → smelter → output chest
2. Register logic, create red network connecting smelter and a combinator node
3. Set constant combinator with threshold signal
4. Set inventory reader on smelter (output)
5. Set decider combinator: steel > threshold → output signal
6. Set circuit control on smelter
7. Run until output > 50, verify deactivation
8. Verify one-tick delay behavior on combinator output

### Test 3: Dual-network arithmetic feedback

Two wire colors on overlapping nodes. Red carries raw resource counts from inventory readers. Green carries a computed ratio from an arithmetic combinator (iron / copper). A circuit control on green enables a balancer node only when the ratio drops below 2.0.

Steps:
1. Build: iron source + copper source → storage nodes
2. Create red network (both storage nodes) and green network (combinator + balancer)
3. Inventory readers on red for both storage nodes
4. Arithmetic combinator reads red signals, outputs iron/copper ratio on green
5. Circuit control on balancer: ratio < 2.0 on green → enable
6. Run ticks, verify red and green carry independent signals
7. Verify balancer activates/deactivates based on ratio changes
8. Verify multi-tick stabilization (one-tick delay on combinator)

### Cargo.toml change

`factorial-integration-tests` adds `factorial-logic` as a dependency.

---

## Implementation Order

```
1. Module trait enhancement (factorial-core)
   ├── Add as_any / as_any_mut to Module trait
   ├── Add find_module / find_module_mut to Engine
   ├── Update 4 test module impls in module.rs
   └── Verify: cargo test --package factorial-core

2. LogicModuleBridge (factorial-logic)
   ├── Promote bitcode to regular dependency
   ├── Create bridge.rs with LogicModuleBridge
   ├── Add pub mod bridge + re-export to lib.rs
   ├── Unit tests: bridge ticks, serialize/load round-trip
   └── Verify: cargo test --package factorial-logic

3. Integration tests (factorial-integration-tests)
   ├── Add factorial-logic dependency
   ├── Create logic_networks.rs with 3 test scenarios
   └── Verify: cargo test --package factorial-integration-tests

4. FFI bindings (factorial-ffi)
   ├── Add factorial-logic dependency
   ├── Add FFI types (FfiWireColor, FfiSelectorKind, etc.)
   ├── Add ~13 factorial_logic_* functions
   ├── FFI tests for each function
   └── Verify: cargo test --package factorial-ffi

5. Final verification
   ├── cargo clippy --workspace --all-targets -- -D warnings
   ├── cargo fmt --all -- --check
   └── cargo test --workspace
```

---

## Files Changed

| Crate | File | Change |
|---|---|---|
| factorial-core | `src/module.rs` | Add `as_any` / `as_any_mut` to Module trait |
| factorial-core | `src/engine.rs` | Add `find_module` / `find_module_mut` methods |
| factorial-logic | `Cargo.toml` | Promote bitcode to regular dep |
| factorial-logic | `src/lib.rs` | Add `pub mod bridge` + re-export |
| factorial-logic | `src/bridge.rs` | New: LogicModuleBridge |
| factorial-ffi | `Cargo.toml` | Add factorial-logic dependency |
| factorial-ffi | `src/lib.rs` | Add FFI types + ~13 functions |
| factorial-integration-tests | `Cargo.toml` | Add factorial-logic dependency |
| factorial-integration-tests | `tests/logic_networks.rs` | New: 3 integration test scenarios |
