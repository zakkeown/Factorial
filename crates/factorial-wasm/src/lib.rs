//! WASM bindings for the Factorial engine.
//!
//! This crate exposes a C-compatible, integer-handle-based API suitable for
//! consumption from JavaScript/TypeScript via `wasm-bindgen` or raw WASM
//! imports. The same API also works as a plain `cdylib` on native targets.
//!
//! # Handle Table
//!
//! Instead of opaque pointers, engines are referenced by integer handles
//! (indices into a fixed-size handle table). Up to [`MAX_ENGINES`] engines
//! may exist simultaneously.
//!
//! # Pull-based Events
//!
//! After each `factorial_step` / `factorial_advance` call the host polls
//! events via the event module. Events are cached in a thread-local buffer
//! that is valid until the next step or destroy call.

pub mod engine;
pub mod event;
pub mod graph;
pub mod logic;
pub mod processor;
pub mod query;
pub mod serialize;
pub mod transport;

use std::cell::RefCell;

use factorial_core::engine::Engine;
use factorial_core::event::{Event, EventKind};
use factorial_core::fixed::Fixed64;
use factorial_core::id::{EdgeId, ItemTypeId, NodeId};

use factorial_logic::WireColor;
use factorial_logic::combinator::{ArithmeticOp, SignalSelector};
use factorial_logic::condition::ComparisonOp;

use slotmap::{Key, KeyData};

// ---------------------------------------------------------------------------
// Result codes
// ---------------------------------------------------------------------------

/// Success.
pub const RESULT_OK: i32 = 0;
/// The engine handle is invalid.
pub const RESULT_INVALID_HANDLE: i32 = 1;
/// Serialization failed.
pub const RESULT_SERIALIZE_ERROR: i32 = 2;
/// Deserialization failed.
pub const RESULT_DESERIALIZE_ERROR: i32 = 3;
/// The requested node was not found.
pub const RESULT_NODE_NOT_FOUND: i32 = 4;
/// The requested edge was not found.
pub const RESULT_EDGE_NOT_FOUND: i32 = 5;
/// An internal error occurred.
pub const RESULT_INTERNAL_ERROR: i32 = 6;
/// Memory allocation failed.
pub const RESULT_ALLOC_ERROR: i32 = 7;

// ---------------------------------------------------------------------------
// Handle table
// ---------------------------------------------------------------------------

/// Maximum number of simultaneous engine instances.
pub const MAX_ENGINES: usize = 16;

/// Per-engine state held in the handle table.
pub struct EngineSlot {
    pub engine: Engine,
    pub event_cache: Vec<FlatEvent>,
}

/// Flat, `repr(C)` event representation for WASM consumers.
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

thread_local! {
    static HANDLE_TABLE: RefCell<Vec<Option<EngineSlot>>> = RefCell::new({
        let mut v = Vec::with_capacity(MAX_ENGINES);
        v.resize_with(MAX_ENGINES, || None);
        v
    });
    static EVENT_CACHE: RefCell<Vec<FlatEvent>> = const { RefCell::new(Vec::new()) };
}

/// Run a closure with mutable access to the handle table.
fn with_table<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vec<Option<EngineSlot>>) -> R,
{
    HANDLE_TABLE.with(|table| f(&mut table.borrow_mut()))
}

/// Run a closure with mutable access to the [`EngineSlot`] at `handle`.
/// Returns [`RESULT_INVALID_HANDLE`] if the handle is out of range or empty.
fn with_engine<F>(handle: i32, f: F) -> i32
where
    F: FnOnce(&mut EngineSlot) -> i32,
{
    HANDLE_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        let idx = handle as usize;
        if idx >= table.len() {
            return RESULT_INVALID_HANDLE;
        }
        match table[idx].as_mut() {
            Some(slot) => f(slot),
            None => RESULT_INVALID_HANDLE,
        }
    })
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
// Event conversion
// ---------------------------------------------------------------------------

fn convert_event(event: &Event) -> FlatEvent {
    match event {
        Event::ItemProduced {
            node,
            item_type,
            quantity,
            tick,
        } => FlatEvent {
            kind: 0,
            tick: *tick,
            node: node_id_to_ffi(*node),
            item_type: item_type.0,
            quantity: *quantity,
            ..Default::default()
        },
        Event::ItemConsumed {
            node,
            item_type,
            quantity,
            tick,
        } => FlatEvent {
            kind: 1,
            tick: *tick,
            node: node_id_to_ffi(*node),
            item_type: item_type.0,
            quantity: *quantity,
            ..Default::default()
        },
        Event::RecipeStarted { node, tick } => FlatEvent {
            kind: 2,
            tick: *tick,
            node: node_id_to_ffi(*node),
            ..Default::default()
        },
        Event::RecipeCompleted { node, tick } => FlatEvent {
            kind: 3,
            tick: *tick,
            node: node_id_to_ffi(*node),
            ..Default::default()
        },
        Event::BuildingStalled {
            node,
            reason: _,
            tick,
        } => FlatEvent {
            kind: 4,
            tick: *tick,
            node: node_id_to_ffi(*node),
            ..Default::default()
        },
        Event::BuildingResumed { node, tick } => FlatEvent {
            kind: 5,
            tick: *tick,
            node: node_id_to_ffi(*node),
            ..Default::default()
        },
        Event::ItemDelivered {
            edge,
            quantity,
            tick,
        } => FlatEvent {
            kind: 6,
            tick: *tick,
            edge: edge_id_to_ffi(*edge),
            quantity: *quantity,
            ..Default::default()
        },
        Event::TransportFull { edge, tick } => FlatEvent {
            kind: 7,
            tick: *tick,
            edge: edge_id_to_ffi(*edge),
            ..Default::default()
        },
        Event::NodeAdded {
            node,
            building_type,
            tick,
        } => FlatEvent {
            kind: 8,
            tick: *tick,
            node: node_id_to_ffi(*node),
            building_type: building_type.0,
            ..Default::default()
        },
        Event::NodeRemoved { node, tick } => FlatEvent {
            kind: 9,
            tick: *tick,
            node: node_id_to_ffi(*node),
            ..Default::default()
        },
        Event::EdgeAdded {
            edge,
            from,
            to,
            tick,
        } => FlatEvent {
            kind: 10,
            tick: *tick,
            edge: edge_id_to_ffi(*edge),
            from_node: node_id_to_ffi(*from),
            to_node: node_id_to_ffi(*to),
            ..Default::default()
        },
        Event::EdgeRemoved { edge, tick } => FlatEvent {
            kind: 11,
            tick: *tick,
            edge: edge_id_to_ffi(*edge),
            ..Default::default()
        },
        Event::RecipeSwitched {
            node,
            old_recipe_index,
            new_recipe_index,
            tick,
        } => FlatEvent {
            kind: 12,
            tick: *tick,
            node: node_id_to_ffi(*node),
            item_type: *old_recipe_index as u32,
            quantity: *new_recipe_index as u32,
            ..Default::default()
        },
    }
}

// ---------------------------------------------------------------------------
// Logic helpers
// ---------------------------------------------------------------------------

fn ffi_to_wire_color(color: u32) -> Option<WireColor> {
    match color {
        0 => Some(WireColor::Red),
        1 => Some(WireColor::Green),
        _ => None,
    }
}

fn ffi_to_selector(kind: u32, value: u64) -> Option<SignalSelector> {
    match kind {
        0 => Some(SignalSelector::Signal(ItemTypeId(value as u32))),
        1 => Some(SignalSelector::Constant(Fixed64::from_bits(value as i64))),
        2 => Some(SignalSelector::Each),
        _ => None,
    }
}

fn ffi_to_arithmetic_op(op: u32) -> Option<ArithmeticOp> {
    match op {
        0 => Some(ArithmeticOp::Add),
        1 => Some(ArithmeticOp::Subtract),
        2 => Some(ArithmeticOp::Multiply),
        3 => Some(ArithmeticOp::Divide),
        4 => Some(ArithmeticOp::Modulo),
        _ => None,
    }
}

fn ffi_to_comparison_op(op: u32) -> Option<ComparisonOp> {
    match op {
        0 => Some(ComparisonOp::Gt),
        1 => Some(ComparisonOp::Lt),
        2 => Some(ComparisonOp::Eq),
        3 => Some(ComparisonOp::Gte),
        4 => Some(ComparisonOp::Lte),
        5 => Some(ComparisonOp::Ne),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Event listener registration
// ---------------------------------------------------------------------------

/// Register passive listeners on all event kinds that capture events into
/// the thread-local [`EVENT_CACHE`].
fn register_event_listeners(engine: &mut Engine) {
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
        EventKind::RecipeSwitched,
    ];

    for kind in all_kinds {
        engine.on_passive(
            kind,
            Box::new(move |event: &Event| {
                EVENT_CACHE.with(|c| {
                    c.borrow_mut().push(convert_event(event));
                });
            }),
        );
    }
}

// ---------------------------------------------------------------------------
// Linear-memory allocator exports
// ---------------------------------------------------------------------------

/// Allocate `size` bytes aligned to `align` in the WASM linear memory.
/// Returns a pointer to the allocated region, or null on failure.
///
/// # Safety
///
/// The caller must ensure `size` and `align` are valid (align must be a
/// power of two and non-zero).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_alloc(size: usize, align: usize) -> *mut u8 {
    if size == 0 || align == 0 || !align.is_power_of_two() {
        return std::ptr::null_mut();
    }
    // SAFETY: align is checked to be a power of two and non-zero above;
    // size is non-zero.
    let layout = match std::alloc::Layout::from_size_align(size, align) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };
    let ptr = unsafe { std::alloc::alloc(layout) };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    ptr
}

/// Free a region previously allocated by [`factorial_alloc`].
///
/// # Safety
///
/// `ptr` must have been returned by `factorial_alloc` with the same `size`
/// and `align` values. Calling this with invalid arguments is undefined
/// behaviour.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_free(ptr: *mut u8, size: usize, align: usize) {
    if ptr.is_null() || size == 0 || align == 0 || !align.is_power_of_two() {
        return;
    }
    // SAFETY: caller guarantees ptr/size/align match a previous alloc call.
    if let Ok(layout) = std::alloc::Layout::from_size_align(size, align) {
        unsafe { std::alloc::dealloc(ptr, layout) };
    }
}
