# WASM Build Target Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `factorial-wasm` crate that compiles the Factorial engine to `wasm32-unknown-unknown` with exported C-style functions callable from any WASM runtime.

**Architecture:** Thin wrapper crate around `factorial-core` and `factorial-logic`. Engine instances live in a handle table (`Vec<Option<Engine>>`), referenced by integer handles. No raw pointers, no wasm-bindgen. Includes WASM-specific `factorial_alloc`/`factorial_free` for cross-boundary memory management.

**Tech Stack:** Rust (`wasm32-unknown-unknown` target), `factorial-core`, `factorial-logic`, `slotmap`

**Design doc:** `docs/plans/2026-02-08-wasm-build-target-design.md`

---

### Task 1: Verify Profiling Feature Gate in Core

Before creating the WASM crate, confirm the `std::time::Instant` usage in `factorial-core` is fully gated behind the `profiling` feature so WASM compilation works.

**Files:**
- Read: `crates/factorial-core/src/engine.rs:512-572`
- Read: `crates/factorial-core/Cargo.toml`

**Step 1: Verify the Instant calls are feature-gated**

Check that all 6 `Instant::now()` calls in `engine.rs` are behind `#[cfg(feature = "profiling")]`. They should already be (confirmed during design). Also verify `profiling.rs` only uses `std::time::Duration` (which is WASM-safe) and not `Instant` outside the feature gate.

Run: `cargo build --package factorial-core --target wasm32-unknown-unknown`
Expected: Fails because the WASM target isn't installed yet. That's fine — we're just checking the code visually for now.

**Step 2: Install the WASM target**

Run: `rustup target add wasm32-unknown-unknown`
Expected: Target installed successfully.

**Step 3: Verify factorial-core compiles to WASM**

Run: `cargo build --package factorial-core --target wasm32-unknown-unknown`
Expected: Compiles successfully (no `profiling` feature enabled by default, so `Instant` is not compiled).

**Step 4: Verify factorial-logic compiles to WASM**

Run: `cargo build --package factorial-logic --target wasm32-unknown-unknown`
Expected: Compiles successfully.

**Step 5: Commit**

No code changes needed if everything compiles. If any fixes were required, commit them:
```bash
git add -A
git commit -m "fix: gate std::time::Instant behind profiling feature for WASM compat"
```

---

### Task 2: Scaffold the factorial-wasm Crate

Create the crate skeleton with Cargo.toml, module structure, and result codes.

**Files:**
- Create: `crates/factorial-wasm/Cargo.toml`
- Create: `crates/factorial-wasm/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

**Step 1: Create the crate directory**

Run: `mkdir -p crates/factorial-wasm/src`

**Step 2: Write Cargo.toml**

Create `crates/factorial-wasm/Cargo.toml`:
```toml
[package]
name = "factorial-wasm"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
factorial-core = { path = "../factorial-core" }
factorial-logic = { path = "../factorial-logic" }
slotmap = { workspace = true }

[dev-dependencies]
factorial-core = { path = "../factorial-core", features = ["test-utils"] }
```

Note: `cdylib` produces the `.wasm` file when targeting `wasm32-unknown-unknown`. `rlib` allows `cargo test` to link unit tests on native.

**Step 3: Write the initial lib.rs**

Create `crates/factorial-wasm/src/lib.rs` with:
- Module declarations for all planned submodules
- Result code enum (i32, not repr(C))
- Handle table type and its `static mut` instance (or a `std::sync::OnceLock`/simple static approach)
- `factorial_alloc` and `factorial_free` exports for WASM memory management
- Helper functions for handle validation

```rust
//! WASM bindings for the Factorial engine.
//!
//! Compiles to `wasm32-unknown-unknown` and exports plain functions
//! callable from any WASM runtime. No wasm-bindgen, no JS glue.
//!
//! Engine instances are stored in a handle table and referenced by
//! integer handle. Memory for cross-boundary data exchange is managed
//! via `factorial_alloc` / `factorial_free`.

mod engine;
mod event;
mod graph;
mod logic;
mod processor;
mod query;
mod serialize;
mod transport;

use factorial_core::engine::Engine;
use factorial_core::event::{Event, EventKind};
use factorial_core::fixed::Fixed64;
use factorial_core::id::{BuildingTypeId, EdgeId, ItemTypeId, NodeId};
use factorial_core::item::Inventory;
use factorial_core::processor::{
    Depletion, FixedRecipe, Processor, ProcessorState, RecipeInput, RecipeOutput, SourceProcessor,
    StallReason,
};
use factorial_core::sim::SimulationStrategy;
use factorial_core::transport::{
    BatchTransport, FlowTransport, ItemTransport, Transport, VehicleTransport,
};

use factorial_logic::combinator::{
    ArithmeticCombinator, ArithmeticOp, DeciderCombinator, DeciderOutput, SignalSelector,
};
use factorial_logic::condition::{ComparisonOp, Condition, InventorySource};
use factorial_logic::{LogicModuleBridge, SignalSet, WireColor, WireNetworkId};

use slotmap::{Key, KeyData};

use std::alloc::{alloc, dealloc, Layout};

// ---------------------------------------------------------------------------
// Result codes (i32 for WASM)
// ---------------------------------------------------------------------------

pub const RESULT_OK: i32 = 0;
pub const RESULT_INVALID_HANDLE: i32 = 1;
pub const RESULT_SERIALIZE_ERROR: i32 = 2;
pub const RESULT_DESERIALIZE_ERROR: i32 = 3;
pub const RESULT_NODE_NOT_FOUND: i32 = 4;
pub const RESULT_EDGE_NOT_FOUND: i32 = 5;
pub const RESULT_INTERNAL_ERROR: i32 = 6;
pub const RESULT_ALLOC_ERROR: i32 = 7;

// ---------------------------------------------------------------------------
// Handle table
// ---------------------------------------------------------------------------

/// Maximum number of simultaneous engine instances.
const MAX_ENGINES: usize = 16;

struct EngineSlot {
    engine: Engine,
    /// Cached events from the last step, stored as flat bytes for WASM.
    event_cache: Vec<FlatEvent>,
}

/// Flat event representation for WASM (no pointers, fixed-size).
/// 64 bytes total, all fields are simple integers.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FlatEvent {
    pub kind: u32,
    pub tick: u64,
    pub node: u64,
    pub edge: u64,
    pub item_type: u32,
    pub quantity: u32,
    pub building_type: u32,
    pub from_node: u64,
    pub to_node: u64,
}

static mut HANDLE_TABLE: Option<Vec<Option<EngineSlot>>> = None;

fn table() -> &'static mut Vec<Option<EngineSlot>> {
    unsafe {
        if HANDLE_TABLE.is_none() {
            let mut v = Vec::with_capacity(MAX_ENGINES);
            for _ in 0..MAX_ENGINES {
                v.push(None);
            }
            HANDLE_TABLE = Some(v);
        }
        HANDLE_TABLE.as_mut().unwrap()
    }
}

fn with_engine<F, T>(handle: i32, f: F) -> Result<T, i32>
where
    F: FnOnce(&mut EngineSlot) -> Result<T, i32>,
{
    let idx = handle as usize;
    let table = table();
    if idx >= table.len() {
        return Err(RESULT_INVALID_HANDLE);
    }
    match table[idx].as_mut() {
        Some(slot) => f(slot),
        None => Err(RESULT_INVALID_HANDLE),
    }
}

// ---------------------------------------------------------------------------
// ID conversion helpers
// ---------------------------------------------------------------------------

fn node_id_to_ffi(id: NodeId) -> u64 {
    id.data().as_ffi()
}

fn ffi_to_node_id(ffi: u64) -> NodeId {
    KeyData::from_ffi(ffi).into()
}

fn edge_id_to_ffi(id: EdgeId) -> u64 {
    id.data().as_ffi()
}

fn ffi_to_edge_id(ffi: u64) -> EdgeId {
    KeyData::from_ffi(ffi).into()
}

// ---------------------------------------------------------------------------
// Event conversion helper
// ---------------------------------------------------------------------------

fn convert_event(event: &Event) -> FlatEvent {
    match event {
        Event::ItemProduced { node, item_type, quantity, tick } => FlatEvent {
            kind: 0, tick: *tick, node: node_id_to_ffi(*node),
            item_type: item_type.0, quantity: *quantity, ..Default::default()
        },
        Event::ItemConsumed { node, item_type, quantity, tick } => FlatEvent {
            kind: 1, tick: *tick, node: node_id_to_ffi(*node),
            item_type: item_type.0, quantity: *quantity, ..Default::default()
        },
        Event::RecipeStarted { node, tick } => FlatEvent {
            kind: 2, tick: *tick, node: node_id_to_ffi(*node), ..Default::default()
        },
        Event::RecipeCompleted { node, tick } => FlatEvent {
            kind: 3, tick: *tick, node: node_id_to_ffi(*node), ..Default::default()
        },
        Event::BuildingStalled { node, reason: _, tick } => FlatEvent {
            kind: 4, tick: *tick, node: node_id_to_ffi(*node), ..Default::default()
        },
        Event::BuildingResumed { node, tick } => FlatEvent {
            kind: 5, tick: *tick, node: node_id_to_ffi(*node), ..Default::default()
        },
        Event::ItemDelivered { edge, quantity, tick } => FlatEvent {
            kind: 6, tick: *tick, edge: edge_id_to_ffi(*edge),
            quantity: *quantity, ..Default::default()
        },
        Event::TransportFull { edge, tick } => FlatEvent {
            kind: 7, tick: *tick, edge: edge_id_to_ffi(*edge), ..Default::default()
        },
        Event::NodeAdded { node, building_type, tick } => FlatEvent {
            kind: 8, tick: *tick, node: node_id_to_ffi(*node),
            building_type: building_type.0, ..Default::default()
        },
        Event::NodeRemoved { node, tick } => FlatEvent {
            kind: 9, tick: *tick, node: node_id_to_ffi(*node), ..Default::default()
        },
        Event::EdgeAdded { edge, from, to, tick } => FlatEvent {
            kind: 10, tick: *tick, edge: edge_id_to_ffi(*edge),
            from_node: node_id_to_ffi(*from), to_node: node_id_to_ffi(*to),
            ..Default::default()
        },
        Event::EdgeRemoved { edge, tick } => FlatEvent {
            kind: 11, tick: *tick, edge: edge_id_to_ffi(*edge), ..Default::default()
        },
    }
}

// ---------------------------------------------------------------------------
// Logic helpers
// ---------------------------------------------------------------------------

fn ffi_to_wire_color(color: i32) -> WireColor {
    if color == 1 { WireColor::Green } else { WireColor::Red }
}

fn ffi_to_selector(kind: i32, value: u64) -> SignalSelector {
    match kind {
        1 => SignalSelector::Constant(Fixed64::from_bits(value as i64)),
        2 => SignalSelector::Each,
        _ => SignalSelector::Signal(ItemTypeId(value as u32)),
    }
}

fn ffi_to_arithmetic_op(op: i32) -> ArithmeticOp {
    match op {
        1 => ArithmeticOp::Subtract,
        2 => ArithmeticOp::Multiply,
        3 => ArithmeticOp::Divide,
        4 => ArithmeticOp::Modulo,
        _ => ArithmeticOp::Add,
    }
}

fn ffi_to_comparison_op(op: i32) -> ComparisonOp {
    match op {
        1 => ComparisonOp::Lt,
        2 => ComparisonOp::Eq,
        3 => ComparisonOp::Gte,
        4 => ComparisonOp::Lte,
        5 => ComparisonOp::Ne,
        _ => ComparisonOp::Gt,
    }
}

// ---------------------------------------------------------------------------
// WASM memory management
// ---------------------------------------------------------------------------

/// Allocate `size` bytes in WASM linear memory. Returns a pointer (as i32).
/// Returns 0 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_alloc(size: i32) -> i32 {
    if size <= 0 {
        return 0;
    }
    let layout = match Layout::from_size_align(size as usize, 8) {
        Ok(l) => l,
        Err(_) => return 0,
    };
    let ptr = unsafe { alloc(layout) };
    if ptr.is_null() {
        0
    } else {
        ptr as i32
    }
}

/// Free `size` bytes at `ptr` in WASM linear memory.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_free(ptr: i32, size: i32) {
    if ptr == 0 || size <= 0 {
        return;
    }
    let layout = match Layout::from_size_align(size as usize, 8) {
        Ok(l) => l,
        Err(_) => return,
    };
    unsafe { dealloc(ptr as *mut u8, layout) };
}

// ---------------------------------------------------------------------------
// Event listener registration
// ---------------------------------------------------------------------------

/// Register passive listeners on all event kinds. Captured events are
/// stored in the EngineSlot's event_cache during each step.
///
/// Because WASM is single-threaded, we use a raw pointer to write
/// events directly into the slot's cache. The pointer is valid for
/// the lifetime of the slot.
fn register_event_listeners(slot_ptr: *mut EngineSlot) {
    let all_kinds = [
        EventKind::ItemProduced,
        EventKind::ItemConsumed,
        EventKind::RecipeStarted,
        EventKind::RecipeCompleted,
        EventKind::BuildingStalled,
        EventKind::BuildingResumed,
        EventKind::ItemDelivered,
        EventKind::TransportFull,
        EventKind::NodeAdded,
        EventKind::NodeRemoved,
        EventKind::EdgeAdded,
        EventKind::EdgeRemoved,
    ];

    for kind in all_kinds {
        let ptr = slot_ptr;
        // SAFETY: WASM is single-threaded. The slot_ptr is valid as long
        // as the engine exists in the handle table. The listener is removed
        // when the engine is destroyed.
        let engine = unsafe { &mut *ptr };
        engine.engine.on_passive(
            kind,
            Box::new(move |event: &Event| {
                let slot = unsafe { &mut *ptr };
                slot.event_cache.push(convert_event(event));
            }),
        );
    }
}
```

**Step 4: Create empty submodule files**

Create empty files for each submodule (they'll be filled in subsequent tasks):
- `crates/factorial-wasm/src/engine.rs`
- `crates/factorial-wasm/src/graph.rs`
- `crates/factorial-wasm/src/query.rs`
- `crates/factorial-wasm/src/processor.rs`
- `crates/factorial-wasm/src/transport.rs`
- `crates/factorial-wasm/src/serialize.rs`
- `crates/factorial-wasm/src/event.rs`
- `crates/factorial-wasm/src/logic.rs`

Each file should just have a doc comment:
```rust
//! [Description] WASM exports.
```

**Step 5: Add to workspace**

In root `Cargo.toml`, add `"crates/factorial-wasm"` to the `members` list.

**Step 6: Verify it compiles (native)**

Run: `cargo build --package factorial-wasm`
Expected: Compiles (empty submodules, lib.rs has the types but no exported functions yet beyond alloc/free).

**Step 7: Verify it compiles (WASM)**

Run: `cargo build --package factorial-wasm --target wasm32-unknown-unknown`
Expected: Compiles to `.wasm`.

**Step 8: Commit**

```bash
git add crates/factorial-wasm/ Cargo.toml
git commit -m "feat(wasm): scaffold factorial-wasm crate with handle table and alloc"
```

---

### Task 3: Engine Lifecycle Exports

Implement create, create_delta, destroy, step, and advance.

**Files:**
- Modify: `crates/factorial-wasm/src/engine.rs`
- Test: inline `#[cfg(test)] mod tests`

**Step 1: Write failing tests**

Add to `crates/factorial-wasm/src/engine.rs`:
```rust
//! Engine lifecycle WASM exports.

use crate::*;

/// Create a new engine with Tick strategy. Returns handle (>= 0) or -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_create() -> i32 {
    todo!()
}

/// Create a new engine with Delta strategy. Returns handle (>= 0) or -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_create_delta(fixed_timestep: i64) -> i32 {
    todo!()
}

/// Destroy an engine and free its handle slot.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_destroy(handle: i32) -> i32 {
    todo!()
}

/// Step the engine by one tick.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_step(handle: i32) -> i32 {
    todo!()
}

/// Advance the engine by dt ticks (delta mode).
#[unsafe(no_mangle)]
pub extern "C" fn factorial_advance(handle: i32, dt: i64) -> i32 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_returns_valid_handle() {
        let h = factorial_create();
        assert!(h >= 0, "expected valid handle, got {h}");
        factorial_destroy(h);
    }

    #[test]
    fn destroy_invalid_handle_returns_error() {
        let result = factorial_destroy(999);
        assert_eq!(result, RESULT_INVALID_HANDLE);
    }

    #[test]
    fn create_delta_returns_valid_handle() {
        let h = factorial_create_delta(60);
        assert!(h >= 0);
        factorial_destroy(h);
    }

    #[test]
    fn step_increments_tick() {
        let h = factorial_create();
        factorial_step(h);
        // We'll verify via query in a later task; for now just check no error.
        let result = factorial_step(h);
        assert_eq!(result, RESULT_OK);
        factorial_destroy(h);
    }

    #[test]
    fn step_invalid_handle_returns_error() {
        let result = factorial_step(-1);
        assert_eq!(result, RESULT_INVALID_HANDLE);
    }

    #[test]
    fn advance_works() {
        let h = factorial_create_delta(60);
        let result = factorial_advance(h, 120);
        assert_eq!(result, RESULT_OK);
        factorial_destroy(h);
    }

    #[test]
    fn create_reuses_destroyed_slot() {
        let h1 = factorial_create();
        factorial_destroy(h1);
        let h2 = factorial_create();
        assert_eq!(h1, h2, "should reuse the freed slot");
        factorial_destroy(h2);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --package factorial-wasm -- engine`
Expected: FAIL (todo! panics).

**Step 3: Implement the functions**

Replace the `todo!()` calls with real implementations:

```rust
//! Engine lifecycle WASM exports.

use crate::*;

/// Create a new engine with Tick strategy. Returns handle (>= 0) or -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_create() -> i32 {
    let table = table();
    for (i, slot) in table.iter_mut().enumerate() {
        if slot.is_none() {
            let mut engine_slot = EngineSlot {
                engine: Engine::new(SimulationStrategy::Tick),
                event_cache: Vec::new(),
            };
            let slot_ptr = &mut engine_slot as *mut EngineSlot;
            *slot = Some(engine_slot);
            // Re-borrow after insertion to get the stable pointer.
            let stable_ptr = slot.as_mut().unwrap() as *mut EngineSlot;
            register_event_listeners(stable_ptr);
            return i as i32;
        }
    }
    -1 // No free slots
}

/// Create a new engine with Delta strategy. Returns handle (>= 0) or -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_create_delta(fixed_timestep: i64) -> i32 {
    let table = table();
    for (i, slot) in table.iter_mut().enumerate() {
        if slot.is_none() {
            let engine_slot = EngineSlot {
                engine: Engine::new(SimulationStrategy::Delta {
                    fixed_timestep: fixed_timestep as u64,
                }),
                event_cache: Vec::new(),
            };
            *slot = Some(engine_slot);
            let stable_ptr = slot.as_mut().unwrap() as *mut EngineSlot;
            register_event_listeners(stable_ptr);
            return i as i32;
        }
    }
    -1
}

/// Destroy an engine and free its handle slot.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_destroy(handle: i32) -> i32 {
    if handle < 0 {
        return RESULT_INVALID_HANDLE;
    }
    let idx = handle as usize;
    let table = table();
    if idx >= table.len() || table[idx].is_none() {
        return RESULT_INVALID_HANDLE;
    }
    table[idx] = None;
    RESULT_OK
}

/// Step the engine by one tick.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_step(handle: i32) -> i32 {
    if handle < 0 {
        return RESULT_INVALID_HANDLE;
    }
    match with_engine(handle, |slot| {
        slot.event_cache.clear();
        slot.engine.step();
        Ok(())
    }) {
        Ok(()) => RESULT_OK,
        Err(e) => e,
    }
}

/// Advance the engine by dt ticks (delta mode).
#[unsafe(no_mangle)]
pub extern "C" fn factorial_advance(handle: i32, dt: i64) -> i32 {
    if handle < 0 {
        return RESULT_INVALID_HANDLE;
    }
    match with_engine(handle, |slot| {
        slot.event_cache.clear();
        slot.engine.advance(dt as u64);
        Ok(())
    }) {
        Ok(()) => RESULT_OK,
        Err(e) => e,
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --package factorial-wasm -- engine`
Expected: All PASS.

**Step 5: Verify WASM compile**

Run: `cargo build --package factorial-wasm --target wasm32-unknown-unknown`
Expected: Compiles.

**Step 6: Commit**

```bash
git add crates/factorial-wasm/src/engine.rs
git commit -m "feat(wasm): add engine lifecycle exports (create, destroy, step, advance)"
```

---

### Task 4: Graph Mutation Exports

Implement add_node, remove_node, connect, disconnect, and apply_mutations.

**Files:**
- Modify: `crates/factorial-wasm/src/graph.rs`

**Step 1: Write failing tests**

```rust
//! Graph mutation WASM exports.

use crate::*;

/// Queue a node addition. Writes the pending node ID to the i32 at `out_pending_ptr`.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_add_node(handle: i32, building_type: u32, out_pending_ptr: i32) -> i32 {
    todo!()
}

/// Queue a node removal.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_remove_node(handle: i32, node_id: u64) -> i32 {
    todo!()
}

/// Queue an edge. Writes the pending edge ID to the i32 at `out_pending_ptr`.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_connect(handle: i32, from_node: u64, to_node: u64, out_pending_ptr: i32) -> i32 {
    todo!()
}

/// Queue an edge removal.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_disconnect(handle: i32, edge_id: u64) -> i32 {
    todo!()
}

/// Apply all queued mutations. Writes results (pending->real ID pairs) to
/// the buffer at `out_ptr`. Format: [node_count: u32, edge_count: u32,
/// then node_count pairs of (pending: u64, real: u64),
/// then edge_count pairs of (pending: u64, real: u64)].
///
/// `out_ptr` must point to a buffer allocated via `factorial_alloc` with
/// sufficient space. `out_len` is the buffer size in bytes.
///
/// Returns RESULT_OK on success. Writes actual byte count to `out_written_ptr`.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_apply_mutations(handle: i32, out_ptr: i32, out_len: i32, out_written_ptr: i32) -> i32 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::*;

    #[test]
    fn add_node_and_apply() {
        let h = factorial_create();

        let mut pending: u64 = 0;
        let result = factorial_add_node(h, 0, &mut pending as *mut u64 as i32);
        assert_eq!(result, RESULT_OK);

        // Allocate buffer for mutation result.
        let buf_size = 256;
        let buf = vec![0u8; buf_size];
        let buf_ptr = buf.as_ptr() as i32;
        let mut written: i32 = 0;
        let result = factorial_apply_mutations(h, buf_ptr, buf_size as i32, &mut written as *mut i32 as i32);
        assert_eq!(result, RESULT_OK);
        assert!(written > 0);

        // Parse: first 4 bytes = node_count, next 4 = edge_count.
        let node_count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(node_count, 1);

        factorial_destroy(h);
    }

    #[test]
    fn add_node_invalid_handle() {
        let mut pending: u64 = 0;
        let result = factorial_add_node(999, 0, &mut pending as *mut u64 as i32);
        assert_eq!(result, RESULT_INVALID_HANDLE);
    }

    #[test]
    fn connect_and_disconnect() {
        let h = factorial_create();

        // Add two nodes.
        let mut p1: u64 = 0;
        let mut p2: u64 = 0;
        factorial_add_node(h, 0, &mut p1 as *mut u64 as i32);
        factorial_add_node(h, 0, &mut p2 as *mut u64 as i32);

        let buf = vec![0u8; 512];
        let buf_ptr = buf.as_ptr() as i32;
        let mut written: i32 = 0;
        factorial_apply_mutations(h, buf_ptr, 512, &mut written as *mut i32 as i32);

        // Parse node IDs from mutation result.
        let node_count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(node_count, 2);

        // Node pairs start at offset 8. Each pair is (u64 pending, u64 real) = 16 bytes.
        let real_1 = u64::from_le_bytes(buf[16..24].try_into().unwrap());
        let real_2 = u64::from_le_bytes(buf[32..40].try_into().unwrap());

        // Connect them.
        let mut edge_pending: u64 = 0;
        let result = factorial_connect(h, real_1, real_2, &mut edge_pending as *mut u64 as i32);
        assert_eq!(result, RESULT_OK);

        let buf2 = vec![0u8; 512];
        let mut written2: i32 = 0;
        factorial_apply_mutations(h, buf2.as_ptr() as i32, 512, &mut written2 as *mut i32 as i32);
        let edge_count = u32::from_le_bytes([buf2[4], buf2[5], buf2[6], buf2[7]]);
        assert_eq!(edge_count, 1);

        factorial_destroy(h);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --package factorial-wasm -- graph`
Expected: FAIL.

**Step 3: Implement the functions**

Replace `todo!()` with implementations that call through to the core engine via `with_engine`. For `apply_mutations`, write the result as a flat byte buffer:
- Bytes 0-3: `node_count` (u32 LE)
- Bytes 4-7: `edge_count` (u32 LE)
- Then `node_count` pairs of `(pending_u64_le, real_u64_le)` (16 bytes each)
- Then `edge_count` pairs of `(pending_u64_le, real_u64_le)` (16 bytes each)

Write directly to the output pointer in WASM memory (or the raw pointer in native tests).

**Step 4: Run tests**

Run: `cargo test --package factorial-wasm -- graph`
Expected: PASS.

**Step 5: WASM compile check**

Run: `cargo build --package factorial-wasm --target wasm32-unknown-unknown`
Expected: Compiles.

**Step 6: Commit**

```bash
git add crates/factorial-wasm/src/graph.rs
git commit -m "feat(wasm): add graph mutation exports (add_node, connect, apply_mutations)"
```

---

### Task 5: Query Exports

Implement node_count, edge_count, get_tick, get_state_hash, get_processor_state, get_input/output_inventory_count.

**Files:**
- Modify: `crates/factorial-wasm/src/query.rs`

**Step 1: Write failing tests**

Tests should:
- Create an engine, add a node with a source processor, step, then query tick/node_count/state_hash/inventory counts.
- Test invalid handle returns `RESULT_INVALID_HANDLE`.

All query functions write their result to a pointer in WASM memory. Signature pattern:
```rust
pub extern "C" fn factorial_node_count(handle: i32, out_ptr: i32) -> i32
```
Where `out_ptr` points to a `u32` for counts, `u64` for tick/hash.

**Step 2: Run tests — expect FAIL**

**Step 3: Implement**

Each function follows the pattern:
```rust
with_engine(handle, |slot| {
    let value = slot.engine.some_query();
    unsafe { *(out_ptr as *mut T) = value };
    Ok(())
}).map_or_else(|e| e, |_| RESULT_OK)
```

For `get_processor_state`, write two values: state tag (u32) and progress (u32) = 8 bytes.

**Step 4: Run tests — expect PASS**

**Step 5: WASM compile check**

**Step 6: Commit**

```bash
git add crates/factorial-wasm/src/query.rs
git commit -m "feat(wasm): add query exports (node_count, tick, state_hash, inventories)"
```

---

### Task 6: Processor Configuration Exports

Implement set_source, set_fixed_processor.

**Files:**
- Modify: `crates/factorial-wasm/src/processor.rs`

**Step 1: Write failing tests**

Test that after setting a source processor and stepping, output inventory increases.

Signatures:
```rust
pub extern "C" fn factorial_set_source(handle: i32, node_id: u64, item_type: u32, rate: i64) -> i32
pub extern "C" fn factorial_set_fixed_processor(handle: i32, node_id: u64, recipe_ptr: i32, recipe_len: i32) -> i32
```

For `set_fixed_processor`, the recipe is passed as a flat byte buffer:
- Bytes 0-3: `input_count` (u32 LE)
- Bytes 4-7: `output_count` (u32 LE)
- Bytes 8-11: `duration` (u32 LE)
- Then `input_count` pairs of `(item_type: u32, quantity: u32)` (8 bytes each)
- Then `output_count` pairs of `(item_type: u32, quantity: u32)` (8 bytes each)

**Step 2-5: Standard TDD cycle**

**Step 6: Commit**

```bash
git add crates/factorial-wasm/src/processor.rs
git commit -m "feat(wasm): add processor config exports (set_source, set_fixed_processor)"
```

---

### Task 7: Transport Configuration Exports

Implement set_flow_transport, set_item_transport, set_batch_transport, set_vehicle_transport, set_input_capacity, set_output_capacity.

**Files:**
- Modify: `crates/factorial-wasm/src/transport.rs`

**Step 1: Write failing tests**

Test: create two nodes, connect them, set transport, set inventories, step, verify items move.

Signatures (simple scalar args, no pointers needed):
```rust
pub extern "C" fn factorial_set_flow_transport(handle: i32, edge_id: u64, rate: i64) -> i32
pub extern "C" fn factorial_set_item_transport(handle: i32, edge_id: u64, speed: i64, slot_count: u32, lanes: u32) -> i32
pub extern "C" fn factorial_set_batch_transport(handle: i32, edge_id: u64, batch_size: u32, cycle_time: u32) -> i32
pub extern "C" fn factorial_set_vehicle_transport(handle: i32, edge_id: u64, capacity: u32, travel_time: u32) -> i32
pub extern "C" fn factorial_set_input_capacity(handle: i32, node_id: u64, capacity: u32) -> i32
pub extern "C" fn factorial_set_output_capacity(handle: i32, node_id: u64, capacity: u32) -> i32
```

**Step 2-5: Standard TDD cycle**

**Step 6: Commit**

```bash
git add crates/factorial-wasm/src/transport.rs
git commit -m "feat(wasm): add transport and inventory config exports"
```

---

### Task 8: Serialization Exports

Implement serialize and deserialize.

**Files:**
- Modify: `crates/factorial-wasm/src/serialize.rs`

**Step 1: Write failing tests**

Test round-trip: create engine, add node, step, serialize, deserialize into new handle, verify tick and node_count match.

Signatures:
```rust
/// Serialize engine state. Writes serialized bytes into the buffer at `out_ptr`.
/// Writes actual byte count to `out_written_ptr`. Returns RESULT_OK or error.
pub extern "C" fn factorial_serialize(handle: i32, out_ptr: i32, out_len: i32, out_written_ptr: i32) -> i32

/// Deserialize engine state from `data_ptr` (length `data_len`).
/// Returns a new engine handle (>= 0) or a negative error code.
pub extern "C" fn factorial_deserialize(data_ptr: i32, data_len: i32) -> i32
```

**Step 2-5: Standard TDD cycle**

**Step 6: Commit**

```bash
git add crates/factorial-wasm/src/serialize.rs
git commit -m "feat(wasm): add serialization exports (serialize, deserialize)"
```

---

### Task 9: Event Polling Export

Implement poll_events.

**Files:**
- Modify: `crates/factorial-wasm/src/event.rs`

**Step 1: Write failing tests**

Test: create engine, add source node, step, poll events, verify at least one event returned.

Signature:
```rust
/// Poll events from the last step. Writes flat FlatEvent structs into the
/// buffer at `out_ptr`. Each event is `size_of::<FlatEvent>()` bytes.
/// Writes event count to `out_count_ptr`.
pub extern "C" fn factorial_poll_events(handle: i32, out_ptr: i32, out_len: i32, out_count_ptr: i32) -> i32
```

Events were already captured during `step` via the passive listeners registered in `register_event_listeners`. This function just copies them to the caller's buffer.

**Step 2-5: Standard TDD cycle**

**Step 6: Commit**

```bash
git add crates/factorial-wasm/src/event.rs
git commit -m "feat(wasm): add event polling export"
```

---

### Task 10: Logic Network Exports

Implement all logic functions: register, create/remove network, add/remove from network, set_constant, set_arithmetic, set_decider, set_circuit_control, is_active, get_network_signal, remove_node, set_inventory_reader.

**Files:**
- Modify: `crates/factorial-wasm/src/logic.rs`

**Step 1: Write failing tests**

Test: register logic module, create network, add node to network, set constant combinator, step, query signal.

Signatures — all use simple scalar arguments (i32/i64/u32/u64). For `set_constant` which needs parallel arrays, pass them as pointers into WASM memory:
```rust
pub extern "C" fn factorial_logic_register(handle: i32) -> i32
pub extern "C" fn factorial_logic_create_network(handle: i32, color: i32, out_id_ptr: i32) -> i32
pub extern "C" fn factorial_logic_remove_network(handle: i32, network_id: u32) -> i32
pub extern "C" fn factorial_logic_add_to_network(handle: i32, network_id: u32, node_id: u64) -> i32
pub extern "C" fn factorial_logic_remove_from_network(handle: i32, network_id: u32, node_id: u64) -> i32
pub extern "C" fn factorial_logic_set_constant(handle: i32, node_id: u64, item_ids_ptr: i32, values_ptr: i32, count: u32, enabled: u32) -> i32
pub extern "C" fn factorial_logic_set_arithmetic(handle: i32, node_id: u64, left_kind: i32, left_value: u64, op: i32, right_kind: i32, right_value: u64, output_item: u32) -> i32
pub extern "C" fn factorial_logic_set_decider(handle: i32, node_id: u64, left_kind: i32, left_value: u64, cmp_op: i32, right_kind: i32, right_value: u64, output_kind: i32, output_item: u32) -> i32
pub extern "C" fn factorial_logic_set_circuit_control(handle: i32, node_id: u64, left_kind: i32, left_value: u64, cmp_op: i32, right_kind: i32, right_value: u64, wire_color: i32) -> i32
pub extern "C" fn factorial_logic_is_active(handle: i32, node_id: u64, out_active_ptr: i32) -> i32
pub extern "C" fn factorial_logic_get_network_signal(handle: i32, network_id: u32, item_id: u32, out_value_ptr: i32) -> i32
pub extern "C" fn factorial_logic_remove_node(handle: i32, node_id: u64) -> i32
pub extern "C" fn factorial_logic_set_inventory_reader(handle: i32, node_id: u64, target_node_id: u64, source: u32) -> i32
```

**Step 2-5: Standard TDD cycle**

**Step 6: Commit**

```bash
git add crates/factorial-wasm/src/logic.rs
git commit -m "feat(wasm): add logic network exports"
```

---

### Task 11: CI Integration

Add a WASM build step to the CI workflow.

**Files:**
- Modify: `.github/workflows/test.yml`

**Step 1: Add WASM build job**

Add a new job to `.github/workflows/test.yml`:

```yaml
  wasm:
    name: WASM Build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - name: Build WASM
        run: cargo build --package factorial-wasm --target wasm32-unknown-unknown --release
```

**Step 2: Verify native tests still pass**

Run: `cargo test --workspace`
Expected: All pass (factorial-wasm tests run on native).

**Step 3: Verify WASM build**

Run: `cargo build --package factorial-wasm --target wasm32-unknown-unknown --release`
Expected: Produces `target/wasm32-unknown-unknown/release/factorial_wasm.wasm`.

**Step 4: Verify clippy passes**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: No warnings.

**Step 5: Verify fmt**

Run: `cargo fmt --all -- --check`
Expected: No formatting issues.

**Step 6: Commit**

```bash
git add .github/workflows/test.yml
git commit -m "ci: add WASM build check for factorial-wasm"
```

---

### Task 12: Final Verification and Cleanup

Run full CI suite locally, check WASM binary size, clean up any dead code warnings.

**Files:**
- Possibly any file with clippy warnings

**Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: All pass.

**Step 2: Run clippy**

Run: `RUSTFLAGS=-Dwarnings cargo clippy --workspace --all-targets -- -D warnings`
Expected: Clean.

**Step 3: Check WASM binary size**

Run: `ls -lh target/wasm32-unknown-unknown/release/factorial_wasm.wasm`
Expected: Report the size. No action needed unless it's unreasonably large (> 5 MB would be surprising for this crate).

**Step 4: Run fmt**

Run: `cargo fmt --all -- --check`
Expected: Clean.

**Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore(wasm): cleanup and final verification"
```
