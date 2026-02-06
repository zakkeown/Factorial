//! C FFI layer for the Factorial engine.
//!
//! This crate exposes a C-compatible API for creating and controlling
//! the simulation engine from non-Rust code (Godot, Unity, etc.).
//!
//! # Safety
//!
//! Every `extern "C"` function wraps its body in `std::panic::catch_unwind`
//! to prevent Rust panics from crossing the FFI boundary. Null pointer inputs
//! are detected early and return appropriate error codes.
//!
//! # Pull-based events
//!
//! After each `factorial_step` call, the caller polls events via
//! `factorial_poll_events`. This returns an engine-owned buffer that is
//! valid until the next `factorial_step` or `factorial_destroy`.

use std::panic::catch_unwind;
use std::ptr;

use factorial_core::engine::Engine;
use factorial_core::event::{Event, EventKind};
use factorial_core::fixed::Fixed64;
use factorial_core::id::{BuildingTypeId, ItemTypeId, NodeId, EdgeId};
use factorial_core::item::Inventory;
use factorial_core::processor::{
    Depletion, FixedRecipe, Processor, ProcessorState, RecipeInput, RecipeOutput, SourceProcessor,
};
use factorial_core::sim::SimulationStrategy;
use factorial_core::transport::{
    BatchTransport, FlowTransport, ItemTransport, Transport, VehicleTransport,
};

use slotmap::{Key, KeyData};

// ---------------------------------------------------------------------------
// Result codes
// ---------------------------------------------------------------------------

/// Status codes returned by all FFI functions.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactorialResult {
    /// Success.
    Ok = 0,
    /// A required pointer argument was null.
    NullPointer = 1,
    /// The engine handle is invalid (null or dangling).
    InvalidHandle = 2,
    /// Serialization failed.
    SerializeError = 3,
    /// Deserialization failed.
    DeserializeError = 4,
    /// The requested node was not found.
    NodeNotFound = 5,
    /// The requested edge was not found.
    EdgeNotFound = 6,
    /// An internal panic was caught at the FFI boundary.
    InternalError = 7,
    /// The engine is poisoned (a previous panic left it in an inconsistent state).
    Poisoned = 8,
}

// ---------------------------------------------------------------------------
// Opaque handle
// ---------------------------------------------------------------------------

/// Opaque engine handle. Callers receive `*mut FactorialEngine` from
/// `factorial_create` and pass it to all subsequent calls.
#[repr(C)]
pub struct FactorialEngine {
    inner: Engine,
    poisoned: bool,
}

// ---------------------------------------------------------------------------
// FFI-safe ID types
// ---------------------------------------------------------------------------

/// C-compatible representation of a NodeId (u64 ffi key).
pub type FfiNodeId = u64;

/// C-compatible representation of an EdgeId (u64 ffi key).
pub type FfiEdgeId = u64;

/// C-compatible representation of a PendingNodeId.
pub type FfiPendingNodeId = u64;

/// C-compatible representation of a PendingEdgeId.
pub type FfiPendingEdgeId = u64;

// ---------------------------------------------------------------------------
// FFI-safe processor state
// ---------------------------------------------------------------------------

/// C-compatible processor state tag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiProcessorState {
    Idle = 0,
    Working = 1,
    StalledMissingInputs = 2,
    StalledOutputFull = 3,
    StalledNoPower = 4,
    StalledDepleted = 5,
}

/// C-compatible processor state with progress.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiProcessorInfo {
    pub state: FfiProcessorState,
    /// For `Working` state, the current progress tick. 0 otherwise.
    pub progress: u32,
}

// ---------------------------------------------------------------------------
// FFI-safe event types
// ---------------------------------------------------------------------------

/// C-compatible event tag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiEventKind {
    ItemProduced = 0,
    ItemConsumed = 1,
    RecipeStarted = 2,
    RecipeCompleted = 3,
    BuildingStalled = 4,
    BuildingResumed = 5,
    ItemDelivered = 6,
    TransportFull = 7,
    NodeAdded = 8,
    NodeRemoved = 9,
    EdgeAdded = 10,
    EdgeRemoved = 11,
}

/// C-compatible event data. Union fields are determined by `kind`.
/// We use a flat struct with all possible fields to keep it simple and
/// fully `repr(C)` without actual C unions.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiEvent {
    pub kind: FfiEventKind,
    pub tick: u64,
    /// Node ID (used by most events). 0 if not applicable.
    pub node: FfiNodeId,
    /// Edge ID (used by transport events). 0 if not applicable.
    pub edge: FfiEdgeId,
    /// Item type ID (used by item events). 0 if not applicable.
    pub item_type: u32,
    /// Quantity (used by item events). 0 if not applicable.
    pub quantity: u32,
    /// Building type ID (used by NodeAdded). 0 if not applicable.
    pub building_type: u32,
    /// Source node (used by EdgeAdded). 0 if not applicable.
    pub from_node: FfiNodeId,
    /// Dest node (used by EdgeAdded). 0 if not applicable.
    pub to_node: FfiNodeId,
}

/// Result of polling events: a pointer to engine-owned event buffer and count.
#[repr(C)]
#[derive(Debug)]
pub struct FfiEventBuffer {
    /// Pointer to an array of `FfiEvent`. Valid until next step/destroy.
    pub events: *const FfiEvent,
    /// Number of events in the buffer.
    pub count: u32,
}

// ---------------------------------------------------------------------------
// Serialization buffer
// ---------------------------------------------------------------------------

/// An engine-allocated byte buffer returned from serialization.
#[repr(C)]
#[derive(Debug)]
pub struct FfiByteBuffer {
    /// Pointer to the data. Null on error.
    pub data: *mut u8,
    /// Length in bytes.
    pub len: usize,
}

// ---------------------------------------------------------------------------
// Mutation result
// ---------------------------------------------------------------------------

/// Result of applying mutations. Maps pending IDs to real IDs.
#[repr(C)]
#[derive(Debug)]
pub struct FfiMutationResult {
    /// Array of (pending_node_id, real_node_id) pairs.
    pub added_nodes: *const FfiIdPair,
    pub added_node_count: u32,
    /// Array of (pending_edge_id, real_edge_id) pairs.
    pub added_edges: *const FfiIdPair,
    pub added_edge_count: u32,
}

/// A pair of (pending_id, real_id) for mutation results.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiIdPair {
    pub pending_id: u64,
    pub real_id: u64,
}

// ---------------------------------------------------------------------------
// Helper: convert between slotmap keys and FFI u64
// ---------------------------------------------------------------------------

fn node_id_to_ffi(id: NodeId) -> FfiNodeId {
    id.data().as_ffi()
}

fn ffi_to_node_id(ffi: FfiNodeId) -> NodeId {
    KeyData::from_ffi(ffi).into()
}

fn edge_id_to_ffi(id: EdgeId) -> FfiEdgeId {
    id.data().as_ffi()
}

fn ffi_to_edge_id(ffi: FfiEdgeId) -> EdgeId {
    KeyData::from_ffi(ffi).into()
}

fn convert_processor_state(state: &ProcessorState) -> FfiProcessorInfo {
    match state {
        ProcessorState::Idle => FfiProcessorInfo {
            state: FfiProcessorState::Idle,
            progress: 0,
        },
        ProcessorState::Working { progress } => FfiProcessorInfo {
            state: FfiProcessorState::Working,
            progress: *progress,
        },
        ProcessorState::Stalled { reason } => {
            use factorial_core::processor::StallReason;
            let s = match reason {
                StallReason::MissingInputs => FfiProcessorState::StalledMissingInputs,
                StallReason::OutputFull => FfiProcessorState::StalledOutputFull,
                StallReason::NoPower => FfiProcessorState::StalledNoPower,
                StallReason::Depleted => FfiProcessorState::StalledDepleted,
            };
            FfiProcessorInfo {
                state: s,
                progress: 0,
            }
        }
    }
}

fn convert_event(event: &Event) -> FfiEvent {
    match event {
        Event::ItemProduced {
            node,
            item_type,
            quantity,
            tick,
        } => FfiEvent {
            kind: FfiEventKind::ItemProduced,
            tick: *tick,
            node: node_id_to_ffi(*node),
            edge: 0,
            item_type: item_type.0,
            quantity: *quantity,
            building_type: 0,
            from_node: 0,
            to_node: 0,
        },
        Event::ItemConsumed {
            node,
            item_type,
            quantity,
            tick,
        } => FfiEvent {
            kind: FfiEventKind::ItemConsumed,
            tick: *tick,
            node: node_id_to_ffi(*node),
            edge: 0,
            item_type: item_type.0,
            quantity: *quantity,
            building_type: 0,
            from_node: 0,
            to_node: 0,
        },
        Event::RecipeStarted { node, tick } => FfiEvent {
            kind: FfiEventKind::RecipeStarted,
            tick: *tick,
            node: node_id_to_ffi(*node),
            edge: 0,
            item_type: 0,
            quantity: 0,
            building_type: 0,
            from_node: 0,
            to_node: 0,
        },
        Event::RecipeCompleted { node, tick } => FfiEvent {
            kind: FfiEventKind::RecipeCompleted,
            tick: *tick,
            node: node_id_to_ffi(*node),
            edge: 0,
            item_type: 0,
            quantity: 0,
            building_type: 0,
            from_node: 0,
            to_node: 0,
        },
        Event::BuildingStalled { node, reason: _, tick } => FfiEvent {
            kind: FfiEventKind::BuildingStalled,
            tick: *tick,
            node: node_id_to_ffi(*node),
            edge: 0,
            item_type: 0,
            quantity: 0,
            building_type: 0,
            from_node: 0,
            to_node: 0,
        },
        Event::BuildingResumed { node, tick } => FfiEvent {
            kind: FfiEventKind::BuildingResumed,
            tick: *tick,
            node: node_id_to_ffi(*node),
            edge: 0,
            item_type: 0,
            quantity: 0,
            building_type: 0,
            from_node: 0,
            to_node: 0,
        },
        Event::ItemDelivered {
            edge,
            quantity,
            tick,
        } => FfiEvent {
            kind: FfiEventKind::ItemDelivered,
            tick: *tick,
            node: 0,
            edge: edge_id_to_ffi(*edge),
            item_type: 0,
            quantity: *quantity,
            building_type: 0,
            from_node: 0,
            to_node: 0,
        },
        Event::TransportFull { edge, tick } => FfiEvent {
            kind: FfiEventKind::TransportFull,
            tick: *tick,
            node: 0,
            edge: edge_id_to_ffi(*edge),
            item_type: 0,
            quantity: 0,
            building_type: 0,
            from_node: 0,
            to_node: 0,
        },
        Event::NodeAdded {
            node,
            building_type,
            tick,
        } => FfiEvent {
            kind: FfiEventKind::NodeAdded,
            tick: *tick,
            node: node_id_to_ffi(*node),
            edge: 0,
            item_type: 0,
            quantity: 0,
            building_type: building_type.0,
            from_node: 0,
            to_node: 0,
        },
        Event::NodeRemoved { node, tick } => FfiEvent {
            kind: FfiEventKind::NodeRemoved,
            tick: *tick,
            node: node_id_to_ffi(*node),
            edge: 0,
            item_type: 0,
            quantity: 0,
            building_type: 0,
            from_node: 0,
            to_node: 0,
        },
        Event::EdgeAdded {
            edge,
            from,
            to,
            tick,
        } => FfiEvent {
            kind: FfiEventKind::EdgeAdded,
            tick: *tick,
            node: 0,
            edge: edge_id_to_ffi(*edge),
            item_type: 0,
            quantity: 0,
            building_type: 0,
            from_node: node_id_to_ffi(*from),
            to_node: node_id_to_ffi(*to),
        },
        Event::EdgeRemoved { edge, tick } => FfiEvent {
            kind: FfiEventKind::EdgeRemoved,
            tick: *tick,
            node: 0,
            edge: edge_id_to_ffi(*edge),
            item_type: 0,
            quantity: 0,
            building_type: 0,
            from_node: 0,
            to_node: 0,
        },
    }
}

// ---------------------------------------------------------------------------
// Engine-owned caches (thread-local)
// ---------------------------------------------------------------------------

// We store cached events and mutation results in thread-locals so the engine
// pointer remains a plain *mut Engine. The caches are valid until the next
// step/apply_mutations/destroy call.
//
// NOTE: For a real multi-engine scenario you'd use a side-table keyed by
// engine pointer. For simplicity we use a thread-local vec here since the
// C API is single-threaded per engine.
thread_local! {
    static EVENT_CACHE: std::cell::RefCell<Vec<FfiEvent>> = const { std::cell::RefCell::new(Vec::new()) };
    static MUTATION_NODE_CACHE: std::cell::RefCell<Vec<FfiIdPair>> = const { std::cell::RefCell::new(Vec::new()) };
    static MUTATION_EDGE_CACHE: std::cell::RefCell<Vec<FfiIdPair>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Register passive listeners on all event kinds that capture events into
/// the thread-local `EVENT_CACHE`. This must be called once after creating
/// an engine to enable pull-based event polling.
fn register_ffi_event_listeners(engine: &mut Engine) {
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

// ===========================================================================
// Extern "C" functions
// ===========================================================================

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Create a new engine with `Tick` simulation strategy.
/// Returns a heap-allocated engine pointer. The caller must eventually
/// call `factorial_destroy` to free the memory.
///
/// Returns null on internal error.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_create() -> *mut FactorialEngine {
    match catch_unwind(|| {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        register_ffi_event_listeners(&mut engine);
        Box::into_raw(Box::new(FactorialEngine { inner: engine, poisoned: false }))
    }) {
        Ok(ptr) => ptr,
        Err(_) => ptr::null_mut(),
    }
}

/// Create a new engine with `Delta` simulation strategy.
/// `fixed_timestep` is the number of ticks per fixed simulation step.
///
/// Returns null on internal error.
#[unsafe(no_mangle)]
pub extern "C" fn factorial_create_delta(fixed_timestep: u64) -> *mut FactorialEngine {
    match catch_unwind(|| {
        let mut engine = Engine::new(SimulationStrategy::Delta { fixed_timestep });
        register_ffi_event_listeners(&mut engine);
        Box::into_raw(Box::new(FactorialEngine { inner: engine, poisoned: false }))
    }) {
        Ok(ptr) => ptr,
        Err(_) => ptr::null_mut(),
    }
}

/// Destroy an engine and free its memory.
///
/// # Safety
///
/// `engine` must be a pointer returned by `factorial_create` that has not
/// yet been destroyed. After this call the pointer is invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_destroy(engine: *mut FactorialEngine) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: caller guarantees `engine` was returned by factorial_create.
        let _ = unsafe { Box::from_raw(engine) };
        // Clear event cache.
        EVENT_CACHE.with(|c| c.borrow_mut().clear());
    })) {
        Ok(()) => FactorialResult::Ok,
        Err(_) => FactorialResult::InternalError,
    }
}

// ---------------------------------------------------------------------------
// Simulation
// ---------------------------------------------------------------------------

/// Advance the simulation by one tick (tick mode) or by `dt` ticks (delta mode).
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_step(engine: *mut FactorialEngine) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        // Clear previous events before stepping so the cache only contains
        // events from this step.
        EVENT_CACHE.with(|c| c.borrow_mut().clear());
        engine.inner.step();
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Advance the simulation by `dt` ticks. In tick mode `dt` is ignored
/// and exactly one step runs. In delta mode, `dt` is accumulated.
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_advance(
    engine: *mut FactorialEngine,
    dt: u64,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        // Clear previous events before advancing.
        EVENT_CACHE.with(|c| c.borrow_mut().clear());
        engine.inner.advance(dt);
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

// ---------------------------------------------------------------------------
// Graph mutation: queue operations
// ---------------------------------------------------------------------------

/// Queue a node to be added to the graph. Returns a pending node ID
/// via `out_pending`. The real node ID is assigned after `factorial_apply_mutations`.
///
/// # Safety
///
/// `engine` and `out_pending` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_add_node(
    engine: *mut FactorialEngine,
    building_type: u32,
    out_pending: *mut FfiPendingNodeId,
) -> FactorialResult {
    if engine.is_null() || out_pending.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let pending = engine.inner.graph.queue_add_node(BuildingTypeId(building_type));
        unsafe { *out_pending = pending.0 };
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Queue a node for removal.
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_remove_node(
    engine: *mut FactorialEngine,
    node_id: FfiNodeId,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        engine.inner.graph.queue_remove_node(ffi_to_node_id(node_id));
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Queue an edge connecting two nodes. Returns a pending edge ID via `out_pending`.
///
/// # Safety
///
/// `engine` and `out_pending` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_connect(
    engine: *mut FactorialEngine,
    from_node: FfiNodeId,
    to_node: FfiNodeId,
    out_pending: *mut FfiPendingEdgeId,
) -> FactorialResult {
    if engine.is_null() || out_pending.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let pending = engine
            .inner
            .graph
            .queue_connect(ffi_to_node_id(from_node), ffi_to_node_id(to_node));
        unsafe { *out_pending = pending.0 };
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Queue an edge for removal (disconnect).
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_disconnect(
    engine: *mut FactorialEngine,
    edge_id: FfiEdgeId,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        engine.inner.graph.queue_disconnect(ffi_to_edge_id(edge_id));
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Apply all queued graph mutations atomically. Results (pending->real ID
/// mappings) are written to `out_result`.
///
/// # Safety
///
/// `engine` and `out_result` must be valid pointers. The pointers in the
/// returned `FfiMutationResult` are valid until the next call to
/// `factorial_apply_mutations` or `factorial_destroy`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_apply_mutations(
    engine: *mut FactorialEngine,
    out_result: *mut FfiMutationResult,
) -> FactorialResult {
    if engine.is_null() || out_result.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let result = engine.inner.graph.apply_mutations();

        // Convert to FFI-safe pairs and store in thread-local caches.
        let node_pairs: Vec<FfiIdPair> = result
            .added_nodes
            .iter()
            .map(|(pending, real)| FfiIdPair {
                pending_id: pending.0,
                real_id: node_id_to_ffi(*real),
            })
            .collect();

        let edge_pairs: Vec<FfiIdPair> = result
            .added_edges
            .iter()
            .map(|(pending, real)| FfiIdPair {
                pending_id: pending.0,
                real_id: edge_id_to_ffi(*real),
            })
            .collect();

        MUTATION_NODE_CACHE.with(|c| *c.borrow_mut() = node_pairs);
        MUTATION_EDGE_CACHE.with(|c| *c.borrow_mut() = edge_pairs);

        MUTATION_NODE_CACHE.with(|c| {
            let nodes = c.borrow();
            MUTATION_EDGE_CACHE.with(|e| {
                let edges = e.borrow();
                unsafe {
                    *out_result = FfiMutationResult {
                        added_nodes: if nodes.is_empty() {
                            ptr::null()
                        } else {
                            nodes.as_ptr()
                        },
                        added_node_count: nodes.len() as u32,
                        added_edges: if edges.is_empty() {
                            ptr::null()
                        } else {
                            edges.as_ptr()
                        },
                        added_edge_count: edges.len() as u32,
                    };
                }
            });
        });
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Get the number of nodes in the graph.
///
/// # Safety
///
/// `engine` and `out_count` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_node_count(
    engine: *const FactorialEngine,
    out_count: *mut u32,
) -> FactorialResult {
    if engine.is_null() || out_count.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &*engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        unsafe { *out_count = engine.inner.node_count() as u32 };
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => FactorialResult::InternalError,
    }
}

/// Get the number of edges in the graph.
///
/// # Safety
///
/// `engine` and `out_count` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_edge_count(
    engine: *const FactorialEngine,
    out_count: *mut u32,
) -> FactorialResult {
    if engine.is_null() || out_count.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &*engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        unsafe { *out_count = engine.inner.edge_count() as u32 };
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => FactorialResult::InternalError,
    }
}

/// Get the current tick counter.
///
/// # Safety
///
/// `engine` and `out_tick` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_get_tick(
    engine: *const FactorialEngine,
    out_tick: *mut u64,
) -> FactorialResult {
    if engine.is_null() || out_tick.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &*engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        unsafe { *out_tick = engine.inner.sim_state.tick };
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => FactorialResult::InternalError,
    }
}

/// Get the state hash of the engine (for desync detection).
///
/// # Safety
///
/// `engine` and `out_hash` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_get_state_hash(
    engine: *const FactorialEngine,
    out_hash: *mut u64,
) -> FactorialResult {
    if engine.is_null() || out_hash.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &*engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        unsafe { *out_hash = engine.inner.state_hash() };
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => FactorialResult::InternalError,
    }
}

/// Get the processor state for a node.
///
/// # Safety
///
/// `engine` and `out_info` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_get_processor_state(
    engine: *const FactorialEngine,
    node_id: FfiNodeId,
    out_info: *mut FfiProcessorInfo,
) -> FactorialResult {
    if engine.is_null() || out_info.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &*engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let nid = ffi_to_node_id(node_id);
        match engine.inner.get_processor_state(nid) {
            Some(state) => {
                unsafe { *out_info = convert_processor_state(state) };
                FactorialResult::Ok
            }
            None => FactorialResult::NodeNotFound,
        }
    })) {
        Ok(result) => result,
        Err(_) => FactorialResult::InternalError,
    }
}

/// Get the total item count in a node's input inventory.
///
/// # Safety
///
/// `engine` and `out_count` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_get_input_inventory_count(
    engine: *const FactorialEngine,
    node_id: FfiNodeId,
    out_count: *mut u32,
) -> FactorialResult {
    if engine.is_null() || out_count.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &*engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let nid = ffi_to_node_id(node_id);
        match engine.inner.get_input_inventory(nid) {
            Some(inv) => {
                let total: u32 = inv.input_slots.iter().map(|s| s.total()).sum();
                unsafe { *out_count = total };
                FactorialResult::Ok
            }
            None => FactorialResult::NodeNotFound,
        }
    })) {
        Ok(result) => result,
        Err(_) => FactorialResult::InternalError,
    }
}

/// Get the total item count in a node's output inventory.
///
/// # Safety
///
/// `engine` and `out_count` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_get_output_inventory_count(
    engine: *const FactorialEngine,
    node_id: FfiNodeId,
    out_count: *mut u32,
) -> FactorialResult {
    if engine.is_null() || out_count.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &*engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let nid = ffi_to_node_id(node_id);
        match engine.inner.get_output_inventory(nid) {
            Some(inv) => {
                let total: u32 = inv.output_slots.iter().map(|s| s.total()).sum();
                unsafe { *out_count = total };
                FactorialResult::Ok
            }
            None => FactorialResult::NodeNotFound,
        }
    })) {
        Ok(result) => result,
        Err(_) => FactorialResult::InternalError,
    }
}

// ---------------------------------------------------------------------------
// Events (pull-based)
// ---------------------------------------------------------------------------

/// Poll all buffered events since the last step. Returns a pointer to an
/// engine-owned buffer of `FfiEvent` structs. The buffer is valid until the
/// next `factorial_step`, `factorial_advance`, or `factorial_destroy`.
///
/// # Safety
///
/// `engine` and `out_buffer` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_poll_events(
    engine: *const FactorialEngine,
    out_buffer: *mut FfiEventBuffer,
) -> FactorialResult {
    if engine.is_null() || out_buffer.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &*engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        // Events were captured into EVENT_CACHE by passive listeners during
        // the most recent step/advance call. Return a pointer into the cache.
        EVENT_CACHE.with(|c| {
            let cache = c.borrow();
            unsafe {
                *out_buffer = FfiEventBuffer {
                    events: if cache.is_empty() {
                        ptr::null()
                    } else {
                        cache.as_ptr()
                    },
                    count: cache.len() as u32,
                };
            }
        });
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => FactorialResult::InternalError,
    }
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

/// Serialize the engine state to a binary buffer. The returned
/// `FfiByteBuffer` contains a pointer and length. The caller must free the
/// buffer with `factorial_free_buffer` when done.
///
/// # Safety
///
/// `engine` and `out_buffer` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_serialize(
    engine: *const FactorialEngine,
    out_buffer: *mut FfiByteBuffer,
) -> FactorialResult {
    if engine.is_null() || out_buffer.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &*engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        match engine.inner.serialize() {
            Ok(data) => {
                let len = data.len();
                let mut boxed = data.into_boxed_slice();
                let ptr = boxed.as_mut_ptr();
                std::mem::forget(boxed);
                unsafe {
                    *out_buffer = FfiByteBuffer { data: ptr, len };
                }
                FactorialResult::Ok
            }
            Err(_) => {
                unsafe {
                    *out_buffer = FfiByteBuffer {
                        data: ptr::null_mut(),
                        len: 0,
                    };
                }
                FactorialResult::SerializeError
            }
        }
    })) {
        Ok(result) => result,
        Err(_) => FactorialResult::InternalError,
    }
}

/// Deserialize an engine from a binary buffer. Returns a new engine pointer
/// via `out_engine`. The caller takes ownership.
///
/// # Safety
///
/// `data` must point to `len` valid bytes. `out_engine` must be a valid pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_deserialize(
    data: *const u8,
    len: usize,
    out_engine: *mut *mut FactorialEngine,
) -> FactorialResult {
    if data.is_null() || out_engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let slice = unsafe { std::slice::from_raw_parts(data, len) };
        match Engine::deserialize(slice) {
            Ok(mut engine) => {
                register_ffi_event_listeners(&mut engine);
                unsafe { *out_engine = Box::into_raw(Box::new(FactorialEngine { inner: engine, poisoned: false })) };
                FactorialResult::Ok
            }
            Err(_) => {
                unsafe { *out_engine = ptr::null_mut() };
                FactorialResult::DeserializeError
            }
        }
    })) {
        Ok(result) => result,
        Err(_) => FactorialResult::InternalError,
    }
}

/// Free a byte buffer returned by `factorial_serialize`.
///
/// # Safety
///
/// `buffer` must be a buffer originally returned by `factorial_serialize`.
/// After this call the buffer's data pointer is invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_free_buffer(buffer: FfiByteBuffer) -> FactorialResult {
    if buffer.data.is_null() {
        // Null data is a no-op (not an error).
        return FactorialResult::Ok;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Reconstruct the boxed slice and let it drop.
        let _ = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(buffer.data, buffer.len)) };
    })) {
        Ok(()) => FactorialResult::Ok,
        Err(_) => FactorialResult::InternalError,
    }
}

// ---------------------------------------------------------------------------
// FFI-safe configuration structs
// ---------------------------------------------------------------------------

/// C-compatible item stack (item type + quantity).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiItemStack {
    pub item_type: u32,
    pub quantity: u32,
}

/// C-compatible recipe for FixedRecipe processor.
#[repr(C)]
#[derive(Debug)]
pub struct FfiRecipe {
    pub input_count: u32,
    pub inputs: *const FfiItemStack,
    pub output_count: u32,
    pub outputs: *const FfiItemStack,
    pub duration: u32,
}

// ---------------------------------------------------------------------------
// Configuration: Processors
// ---------------------------------------------------------------------------

/// Set a node's processor to Source.
///
/// `rate` is raw Fixed64 bits (Q32.32). Use `Fixed64::to_bits()` on the Rust
/// side or shift an integer left by 32 on the C side to construct it.
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_source(
    engine: *mut FactorialEngine,
    node_id: FfiNodeId,
    item_type: u32,
    rate: i64,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let nid = ffi_to_node_id(node_id);
        let processor = Processor::Source(SourceProcessor {
            output_type: ItemTypeId(item_type),
            base_rate: Fixed64::from_bits(rate),
            depletion: Depletion::Infinite,
            accumulated: Fixed64::from_num(0),
            initial_properties: None,
        });
        engine.inner.set_processor(nid, processor);
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Set a node's processor to FixedRecipe.
///
/// The `recipe` pointer must point to a valid `FfiRecipe` whose `inputs` and
/// `outputs` arrays have the declared counts.
///
/// # Safety
///
/// `engine` and `recipe` must be valid pointers. The arrays referenced by
/// `recipe.inputs` and `recipe.outputs` must be valid for the declared counts.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_fixed_processor(
    engine: *mut FactorialEngine,
    node_id: FfiNodeId,
    recipe: *const FfiRecipe,
) -> FactorialResult {
    if engine.is_null() || recipe.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let recipe = unsafe { &*recipe };
        let inputs = if recipe.input_count > 0 && !recipe.inputs.is_null() {
            let ffi_inputs =
                unsafe { std::slice::from_raw_parts(recipe.inputs, recipe.input_count as usize) };
            ffi_inputs
                .iter()
                .map(|fi| RecipeInput {
                    item_type: ItemTypeId(fi.item_type),
                    quantity: fi.quantity,
                })
                .collect()
        } else {
            Vec::new()
        };
        let outputs = if recipe.output_count > 0 && !recipe.outputs.is_null() {
            let ffi_outputs =
                unsafe { std::slice::from_raw_parts(recipe.outputs, recipe.output_count as usize) };
            ffi_outputs
                .iter()
                .map(|fo| RecipeOutput {
                    item_type: ItemTypeId(fo.item_type),
                    quantity: fo.quantity,
                })
                .collect()
        } else {
            Vec::new()
        };
        let nid = ffi_to_node_id(node_id);
        let processor = Processor::Fixed(FixedRecipe {
            inputs,
            outputs,
            duration: recipe.duration,
        });
        engine.inner.set_processor(nid, processor);
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration: Transports
// ---------------------------------------------------------------------------

/// Set an edge's transport to FlowTransport with default buffer/latency.
///
/// `rate` is raw Fixed64 bits (Q32.32).
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_flow_transport(
    engine: *mut FactorialEngine,
    edge_id: FfiEdgeId,
    rate: i64,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let eid = ffi_to_edge_id(edge_id);
        let transport = Transport::Flow(FlowTransport {
            rate: Fixed64::from_bits(rate),
            buffer_capacity: Fixed64::from_num(1000),
            latency: 0,
        });
        engine.inner.set_transport(eid, transport);
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Set an edge's transport to ItemTransport.
///
/// `speed` is raw Fixed64 bits (Q32.32).
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_item_transport(
    engine: *mut FactorialEngine,
    edge_id: FfiEdgeId,
    speed: i64,
    slot_count: u32,
    lanes: u8,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let eid = ffi_to_edge_id(edge_id);
        let transport = Transport::Item(ItemTransport {
            speed: Fixed64::from_bits(speed),
            slot_count,
            lanes,
        });
        engine.inner.set_transport(eid, transport);
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Set an edge's transport to BatchTransport.
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_batch_transport(
    engine: *mut FactorialEngine,
    edge_id: FfiEdgeId,
    batch_size: u32,
    cycle_time: u32,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let eid = ffi_to_edge_id(edge_id);
        let transport = Transport::Batch(BatchTransport {
            batch_size,
            cycle_time,
        });
        engine.inner.set_transport(eid, transport);
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Set an edge's transport to VehicleTransport.
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_vehicle_transport(
    engine: *mut FactorialEngine,
    edge_id: FfiEdgeId,
    capacity: u32,
    travel_time: u32,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let eid = ffi_to_edge_id(edge_id);
        let transport = Transport::Vehicle(VehicleTransport {
            capacity,
            travel_time,
        });
        engine.inner.set_transport(eid, transport);
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration: Inventories
// ---------------------------------------------------------------------------

/// Set the input inventory for a node with the given capacity.
///
/// Creates an inventory with 1 input slot and 1 output slot.
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_input_capacity(
    engine: *mut FactorialEngine,
    node_id: FfiNodeId,
    capacity: u32,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let nid = ffi_to_node_id(node_id);
        let inventory = Inventory::new(1, 1, capacity);
        engine.inner.set_input_inventory(nid, inventory);
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

/// Set the output inventory for a node with the given capacity.
///
/// Creates an inventory with 1 input slot and 1 output slot.
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_set_output_capacity(
    engine: *mut FactorialEngine,
    node_id: FfiNodeId,
    capacity: u32,
) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let engine = unsafe { &mut *engine };
        if engine.poisoned {
            return FactorialResult::Poisoned;
        }
        let nid = ffi_to_node_id(node_id);
        let inventory = Inventory::new(1, 1, capacity);
        engine.inner.set_output_inventory(nid, inventory);
        FactorialResult::Ok
    })) {
        Ok(result) => result,
        Err(_) => {
            let engine = unsafe { &mut *engine };
            engine.poisoned = true;
            FactorialResult::InternalError
        }
    }
}

// ---------------------------------------------------------------------------
// Poison checking
// ---------------------------------------------------------------------------

/// Check whether the engine is poisoned (a previous panic left it in an
/// inconsistent state). Returns `false` if the engine pointer is null.
///
/// # Safety
///
/// `engine` must be a valid engine pointer or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_is_poisoned(engine: *const FactorialEngine) -> bool {
    if engine.is_null() {
        return false;
    }
    let engine = unsafe { &*engine };
    engine.poisoned
}

/// Clear the poisoned flag on an engine, allowing it to be used again.
///
/// # Safety
///
/// `engine` must be a valid engine pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn factorial_clear_poison(engine: *mut FactorialEngine) -> FactorialResult {
    if engine.is_null() {
        return FactorialResult::NullPointer;
    }
    let engine = unsafe { &mut *engine };
    engine.poisoned = false;
    FactorialResult::Ok
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers (imported from core test_utils where possible)
    // -----------------------------------------------------------------------

    use factorial_core::test_utils::{iron, gear, make_source, make_recipe, simple_inventory};

    // -----------------------------------------------------------------------
    // Test 1: Create and destroy engine lifecycle
    // -----------------------------------------------------------------------
    #[test]
    fn create_and_destroy() {
        let engine = factorial_create();
        assert!(!engine.is_null());

        let result = unsafe { factorial_destroy(engine) };
        assert_eq!(result, FactorialResult::Ok);
    }

    // -----------------------------------------------------------------------
    // Test 2: Destroy null pointer returns NullPointer
    // -----------------------------------------------------------------------
    #[test]
    fn destroy_null_returns_null_pointer() {
        let result = unsafe { factorial_destroy(ptr::null_mut()) };
        assert_eq!(result, FactorialResult::NullPointer);
    }

    // -----------------------------------------------------------------------
    // Test 3: Step null pointer returns NullPointer
    // -----------------------------------------------------------------------
    #[test]
    fn step_null_returns_null_pointer() {
        let result = unsafe { factorial_step(ptr::null_mut()) };
        assert_eq!(result, FactorialResult::NullPointer);
    }

    // -----------------------------------------------------------------------
    // Test 4: Add node and apply mutations
    // -----------------------------------------------------------------------
    #[test]
    fn add_node_and_apply_mutations() {
        let engine = factorial_create();
        assert!(!engine.is_null());

        // Add a node.
        let mut pending: FfiPendingNodeId = 0;
        let result = unsafe { factorial_add_node(engine, 0, &mut pending) };
        assert_eq!(result, FactorialResult::Ok);

        // Apply mutations.
        let mut mutation_result = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        let result = unsafe { factorial_apply_mutations(engine, &mut mutation_result) };
        assert_eq!(result, FactorialResult::Ok);
        assert_eq!(mutation_result.added_node_count, 1);

        // Read back the real node ID.
        let pairs =
            unsafe { std::slice::from_raw_parts(mutation_result.added_nodes, 1) };
        assert_eq!(pairs[0].pending_id, pending);
        assert_ne!(pairs[0].real_id, 0);

        // Verify node count.
        let mut count: u32 = 0;
        let result = unsafe { factorial_node_count(engine, &mut count) };
        assert_eq!(result, FactorialResult::Ok);
        assert_eq!(count, 1);

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 5: Add node, step, check tick incremented
    // -----------------------------------------------------------------------
    #[test]
    fn step_increments_tick() {
        let engine = factorial_create();
        assert!(!engine.is_null());

        let mut tick: u64 = 0;
        unsafe { factorial_get_tick(engine, &mut tick) };
        assert_eq!(tick, 0);

        unsafe { factorial_step(engine) };

        unsafe { factorial_get_tick(engine, &mut tick) };
        assert_eq!(tick, 1);

        unsafe { factorial_step(engine) };
        unsafe { factorial_step(engine) };

        unsafe { factorial_get_tick(engine, &mut tick) };
        assert_eq!(tick, 3);

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 6: Add node with source processor, step, check output inventory
    // -----------------------------------------------------------------------
    #[test]
    fn add_node_step_produces_output() {
        let engine_ptr = factorial_create();
        assert!(!engine_ptr.is_null());

        // Add a node via FFI.
        let mut pending: FfiPendingNodeId = 0;
        unsafe { factorial_add_node(engine_ptr, 0, &mut pending) };

        let mut mutation_result = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine_ptr, &mut mutation_result) };
        let pairs =
            unsafe { std::slice::from_raw_parts(mutation_result.added_nodes, 1) };
        let node_ffi_id = pairs[0].real_id;

        // Set up the processor and inventories on the Rust side (direct access
        // for test setup -- real users would have dedicated FFI for this).
        let engine = unsafe { &mut *engine_ptr };
        let node_id = ffi_to_node_id(node_ffi_id);
        engine.inner.set_processor(node_id, make_source(iron(), 3.0));
        engine.inner.set_input_inventory(node_id, simple_inventory(100));
        engine.inner.set_output_inventory(node_id, simple_inventory(100));

        // Step once via FFI.
        let result = unsafe { factorial_step(engine_ptr) };
        assert_eq!(result, FactorialResult::Ok);

        // Query output inventory count.
        let mut count: u32 = 0;
        let result =
            unsafe { factorial_get_output_inventory_count(engine_ptr, node_ffi_id, &mut count) };
        assert_eq!(result, FactorialResult::Ok);
        assert_eq!(count, 3, "source should produce 3 items per tick");

        // Query processor state.
        let mut info = FfiProcessorInfo {
            state: FfiProcessorState::Idle,
            progress: 0,
        };
        let result =
            unsafe { factorial_get_processor_state(engine_ptr, node_ffi_id, &mut info) };
        assert_eq!(result, FactorialResult::Ok);
        assert_eq!(info.state, FfiProcessorState::Working);

        unsafe { factorial_destroy(engine_ptr) };
    }

    // -----------------------------------------------------------------------
    // Test 7: Serialize and deserialize through C API
    // -----------------------------------------------------------------------
    #[test]
    fn serialize_deserialize_round_trip() {
        let engine_ptr = factorial_create();

        // Add a node and step.
        let mut pending: FfiPendingNodeId = 0;
        unsafe { factorial_add_node(engine_ptr, 42, &mut pending) };
        let mut mutation_result = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine_ptr, &mut mutation_result) };

        // Set up processor and inventories.
        let pairs =
            unsafe { std::slice::from_raw_parts(mutation_result.added_nodes, 1) };
        let node_ffi_id = pairs[0].real_id;
        let engine = unsafe { &mut *engine_ptr };
        let node_id = ffi_to_node_id(node_ffi_id);
        engine.inner.set_processor(node_id, make_source(iron(), 2.0));
        engine.inner.set_input_inventory(node_id, simple_inventory(100));
        engine.inner.set_output_inventory(node_id, simple_inventory(100));

        // Step a few times.
        for _ in 0..5 {
            unsafe { factorial_step(engine_ptr) };
        }

        // Get state hash before serialize.
        let mut hash_before: u64 = 0;
        unsafe { factorial_get_state_hash(engine_ptr, &mut hash_before) };

        // Serialize.
        let mut buffer = FfiByteBuffer {
            data: ptr::null_mut(),
            len: 0,
        };
        let result = unsafe { factorial_serialize(engine_ptr, &mut buffer) };
        assert_eq!(result, FactorialResult::Ok);
        assert!(!buffer.data.is_null());
        assert!(buffer.len > 0);

        // Deserialize into a new engine.
        let mut restored_ptr: *mut FactorialEngine = ptr::null_mut();
        let result = unsafe { factorial_deserialize(buffer.data, buffer.len, &mut restored_ptr) };
        assert_eq!(result, FactorialResult::Ok);
        assert!(!restored_ptr.is_null());

        // Verify state hash matches.
        let mut hash_after: u64 = 0;
        unsafe { factorial_get_state_hash(restored_ptr, &mut hash_after) };
        assert_eq!(hash_before, hash_after);

        // Verify node count.
        let mut count: u32 = 0;
        unsafe { factorial_node_count(restored_ptr, &mut count) };
        assert_eq!(count, 1);

        // Verify tick.
        let mut tick: u64 = 0;
        unsafe { factorial_get_tick(restored_ptr, &mut tick) };
        assert_eq!(tick, 5);

        // Clean up.
        let result = unsafe { factorial_free_buffer(buffer) };
        assert_eq!(result, FactorialResult::Ok);
        unsafe { factorial_destroy(restored_ptr) };
        unsafe { factorial_destroy(engine_ptr) };
    }

    // -----------------------------------------------------------------------
    // Test 8: Poll events returns correct data
    // -----------------------------------------------------------------------
    #[test]
    fn poll_events_returns_data() {
        let engine_ptr = factorial_create();

        // Add a source node that produces items.
        let mut pending: FfiPendingNodeId = 0;
        unsafe { factorial_add_node(engine_ptr, 7, &mut pending) };
        let mut mutation_result = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine_ptr, &mut mutation_result) };
        let pairs =
            unsafe { std::slice::from_raw_parts(mutation_result.added_nodes, 1) };
        let node_ffi_id = pairs[0].real_id;

        let engine = unsafe { &mut *engine_ptr };
        let node_id = ffi_to_node_id(node_ffi_id);
        engine.inner.set_processor(node_id, make_source(iron(), 2.0));
        engine.inner.set_input_inventory(node_id, simple_inventory(100));
        engine.inner.set_output_inventory(node_id, simple_inventory(100));

        // Step.
        unsafe { factorial_step(engine_ptr) };

        // Poll events.
        let mut event_buffer = FfiEventBuffer {
            events: ptr::null(),
            count: 0,
        };
        let result = unsafe { factorial_poll_events(engine_ptr, &mut event_buffer) };
        assert_eq!(result, FactorialResult::Ok);

        // The source produces items, so we should see at least an ItemProduced event.
        assert!(event_buffer.count > 0, "expected at least one event");

        let events =
            unsafe { std::slice::from_raw_parts(event_buffer.events, event_buffer.count as usize) };

        // Find the ItemProduced event.
        let produced = events
            .iter()
            .find(|e| e.kind == FfiEventKind::ItemProduced);
        assert!(produced.is_some(), "expected an ItemProduced event");

        let produced = produced.unwrap();
        assert_eq!(produced.node, node_ffi_id);
        assert_eq!(produced.item_type, 0); // iron = ItemTypeId(0)
        assert_eq!(produced.quantity, 2);

        unsafe { factorial_destroy(engine_ptr) };
    }

    // -----------------------------------------------------------------------
    // Test 9: Null pointer inputs handled gracefully across all functions
    // -----------------------------------------------------------------------
    #[test]
    fn null_pointer_handling() {
        // All functions that take engine should return NullPointer when null.
        assert_eq!(
            unsafe { factorial_step(ptr::null_mut()) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_advance(ptr::null_mut(), 0) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_add_node(ptr::null_mut(), 0, &mut 0u64) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_remove_node(ptr::null_mut(), 0) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_connect(ptr::null_mut(), 0, 0, &mut 0u64) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_disconnect(ptr::null_mut(), 0) },
            FactorialResult::NullPointer
        );

        let mut mr = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        assert_eq!(
            unsafe { factorial_apply_mutations(ptr::null_mut(), &mut mr) },
            FactorialResult::NullPointer
        );

        assert_eq!(
            unsafe { factorial_node_count(ptr::null(), &mut 0u32) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_edge_count(ptr::null(), &mut 0u32) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_get_tick(ptr::null(), &mut 0u64) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_get_state_hash(ptr::null(), &mut 0u64) },
            FactorialResult::NullPointer
        );

        let mut info = FfiProcessorInfo {
            state: FfiProcessorState::Idle,
            progress: 0,
        };
        assert_eq!(
            unsafe { factorial_get_processor_state(ptr::null(), 0, &mut info) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_get_input_inventory_count(ptr::null(), 0, &mut 0u32) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_get_output_inventory_count(ptr::null(), 0, &mut 0u32) },
            FactorialResult::NullPointer
        );

        let mut eb = FfiEventBuffer {
            events: ptr::null(),
            count: 0,
        };
        assert_eq!(
            unsafe { factorial_poll_events(ptr::null(), &mut eb) },
            FactorialResult::NullPointer
        );

        let mut buf = FfiByteBuffer {
            data: ptr::null_mut(),
            len: 0,
        };
        assert_eq!(
            unsafe { factorial_serialize(ptr::null(), &mut buf) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_deserialize(ptr::null(), 0, &mut ptr::null_mut()) },
            FactorialResult::NullPointer
        );

        // Also test null output pointer variants.
        let engine = factorial_create();
        assert_eq!(
            unsafe { factorial_add_node(engine, 0, ptr::null_mut()) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_connect(engine, 0, 0, ptr::null_mut()) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_node_count(engine, ptr::null_mut()) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_edge_count(engine, ptr::null_mut()) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_apply_mutations(engine, ptr::null_mut()) },
            FactorialResult::NullPointer
        );
        assert_eq!(
            unsafe { factorial_serialize(engine, ptr::null_mut()) },
            FactorialResult::NullPointer
        );

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 10: Connect two nodes and query edge count
    // -----------------------------------------------------------------------
    #[test]
    fn connect_nodes_and_query_edge_count() {
        let engine = factorial_create();

        // Add two nodes.
        let mut pending_a: FfiPendingNodeId = 0;
        let mut pending_b: FfiPendingNodeId = 0;
        unsafe { factorial_add_node(engine, 0, &mut pending_a) };
        unsafe { factorial_add_node(engine, 0, &mut pending_b) };

        let mut mr = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine, &mut mr) };
        assert_eq!(mr.added_node_count, 2);

        let node_pairs = unsafe { std::slice::from_raw_parts(mr.added_nodes, 2) };
        let node_a = node_pairs[0].real_id;
        let node_b = node_pairs[1].real_id;

        // Connect A -> B.
        let mut pending_edge: FfiPendingEdgeId = 0;
        let result = unsafe { factorial_connect(engine, node_a, node_b, &mut pending_edge) };
        assert_eq!(result, FactorialResult::Ok);

        let mut mr2 = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine, &mut mr2) };
        assert_eq!(mr2.added_edge_count, 1);
        assert_eq!(mr2.added_node_count, 0);

        // Verify edge count.
        let mut edge_count: u32 = 0;
        unsafe { factorial_edge_count(engine, &mut edge_count) };
        assert_eq!(edge_count, 1);

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 11: Remove node via FFI
    // -----------------------------------------------------------------------
    #[test]
    fn remove_node_via_ffi() {
        let engine = factorial_create();

        // Add two nodes.
        let mut pending_a: FfiPendingNodeId = 0;
        let mut pending_b: FfiPendingNodeId = 0;
        unsafe { factorial_add_node(engine, 0, &mut pending_a) };
        unsafe { factorial_add_node(engine, 0, &mut pending_b) };

        let mut mr = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine, &mut mr) };
        let node_pairs = unsafe { std::slice::from_raw_parts(mr.added_nodes, 2) };
        let node_a = node_pairs[0].real_id;

        // Verify 2 nodes.
        let mut count: u32 = 0;
        unsafe { factorial_node_count(engine, &mut count) };
        assert_eq!(count, 2);

        // Remove node A.
        let result = unsafe { factorial_remove_node(engine, node_a) };
        assert_eq!(result, FactorialResult::Ok);

        let mut mr2 = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine, &mut mr2) };

        // Verify 1 node remaining.
        unsafe { factorial_node_count(engine, &mut count) };
        assert_eq!(count, 1);

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 12: Disconnect edge via FFI
    // -----------------------------------------------------------------------
    #[test]
    fn disconnect_edge_via_ffi() {
        let engine = factorial_create();

        // Add two nodes and connect them.
        let mut pending_a: FfiPendingNodeId = 0;
        let mut pending_b: FfiPendingNodeId = 0;
        unsafe { factorial_add_node(engine, 0, &mut pending_a) };
        unsafe { factorial_add_node(engine, 0, &mut pending_b) };

        let mut mr = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine, &mut mr) };
        let node_pairs = unsafe { std::slice::from_raw_parts(mr.added_nodes, 2) };
        let node_a = node_pairs[0].real_id;
        let node_b = node_pairs[1].real_id;

        let mut pending_edge: FfiPendingEdgeId = 0;
        unsafe { factorial_connect(engine, node_a, node_b, &mut pending_edge) };
        let mut mr2 = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine, &mut mr2) };
        let edge_pairs = unsafe { std::slice::from_raw_parts(mr2.added_edges, 1) };
        let edge_id = edge_pairs[0].real_id;

        // Verify edge exists.
        let mut edge_count: u32 = 0;
        unsafe { factorial_edge_count(engine, &mut edge_count) };
        assert_eq!(edge_count, 1);

        // Disconnect.
        let result = unsafe { factorial_disconnect(engine, edge_id) };
        assert_eq!(result, FactorialResult::Ok);
        let mut mr3 = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine, &mut mr3) };

        // Verify edge removed.
        unsafe { factorial_edge_count(engine, &mut edge_count) };
        assert_eq!(edge_count, 0);

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 13: Deserialize invalid data returns DeserializeError
    // -----------------------------------------------------------------------
    #[test]
    fn deserialize_invalid_data() {
        let garbage = [0u8; 10];
        let mut engine_ptr: *mut FactorialEngine = ptr::null_mut();
        let result =
            unsafe { factorial_deserialize(garbage.as_ptr(), garbage.len(), &mut engine_ptr) };
        assert_eq!(result, FactorialResult::DeserializeError);
        assert!(engine_ptr.is_null());
    }

    // -----------------------------------------------------------------------
    // Test 14: Free null buffer is a no-op (not an error)
    // -----------------------------------------------------------------------
    #[test]
    fn free_null_buffer_is_noop() {
        let buffer = FfiByteBuffer {
            data: ptr::null_mut(),
            len: 0,
        };
        let result = unsafe { factorial_free_buffer(buffer) };
        assert_eq!(result, FactorialResult::Ok);
    }

    // -----------------------------------------------------------------------
    // Test 15: Create delta engine
    // -----------------------------------------------------------------------
    #[test]
    fn create_delta_engine() {
        let engine = factorial_create_delta(2);
        assert!(!engine.is_null());

        // Advance by 5 ticks with fixed_timestep=2 should run 2 steps.
        let result = unsafe { factorial_advance(engine, 5) };
        assert_eq!(result, FactorialResult::Ok);

        let mut tick: u64 = 0;
        unsafe { factorial_get_tick(engine, &mut tick) };
        assert_eq!(tick, 2); // 5 / 2 = 2 full steps

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 16: Multiple steps accumulate state correctly
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_steps_accumulate() {
        let engine_ptr = factorial_create();

        // Add a source node.
        let mut pending: FfiPendingNodeId = 0;
        unsafe { factorial_add_node(engine_ptr, 0, &mut pending) };
        let mut mr = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine_ptr, &mut mr) };
        let pairs = unsafe { std::slice::from_raw_parts(mr.added_nodes, 1) };
        let node_ffi_id = pairs[0].real_id;

        let engine = unsafe { &mut *engine_ptr };
        let node_id = ffi_to_node_id(node_ffi_id);
        engine.inner.set_processor(node_id, make_source(iron(), 1.0));
        engine.inner.set_input_inventory(node_id, simple_inventory(100));
        engine.inner.set_output_inventory(node_id, simple_inventory(100));

        // Step 10 times.
        for _ in 0..10 {
            unsafe { factorial_step(engine_ptr) };
        }

        // Output should have 10 items.
        let mut count: u32 = 0;
        unsafe { factorial_get_output_inventory_count(engine_ptr, node_ffi_id, &mut count) };
        assert_eq!(count, 10);

        unsafe { factorial_destroy(engine_ptr) };
    }

    // -----------------------------------------------------------------------
    // Test 17: Processor state for non-existent node returns NodeNotFound
    // -----------------------------------------------------------------------
    #[test]
    fn processor_state_node_not_found() {
        let engine = factorial_create();

        let mut info = FfiProcessorInfo {
            state: FfiProcessorState::Idle,
            progress: 0,
        };
        // Use a bogus node ID.
        let result = unsafe { factorial_get_processor_state(engine, 9999, &mut info) };
        assert_eq!(result, FactorialResult::NodeNotFound);

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 18: Inventory count for non-existent node returns NodeNotFound
    // -----------------------------------------------------------------------
    #[test]
    fn inventory_count_node_not_found() {
        let engine = factorial_create();

        let mut count: u32 = 0;
        let result =
            unsafe { factorial_get_input_inventory_count(engine, 9999, &mut count) };
        assert_eq!(result, FactorialResult::NodeNotFound);

        let result =
            unsafe { factorial_get_output_inventory_count(engine, 9999, &mut count) };
        assert_eq!(result, FactorialResult::NodeNotFound);

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 19: Full lifecycle -- add nodes, connect, step, serialize, restore
    // -----------------------------------------------------------------------
    #[test]
    fn full_lifecycle_integration() {
        let engine_ptr = factorial_create();

        // Add source and consumer nodes.
        let mut pending_src: FfiPendingNodeId = 0;
        let mut pending_consumer: FfiPendingNodeId = 0;
        unsafe { factorial_add_node(engine_ptr, 0, &mut pending_src) };
        unsafe { factorial_add_node(engine_ptr, 1, &mut pending_consumer) };

        let mut mr = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine_ptr, &mut mr) };
        assert_eq!(mr.added_node_count, 2);

        let pairs = unsafe { std::slice::from_raw_parts(mr.added_nodes, 2) };
        let src_ffi = pairs[0].real_id;
        let consumer_ffi = pairs[1].real_id;

        // Connect source -> consumer.
        let mut pending_edge: FfiPendingEdgeId = 0;
        unsafe { factorial_connect(engine_ptr, src_ffi, consumer_ffi, &mut pending_edge) };
        let mut mr2 = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine_ptr, &mut mr2) };
        assert_eq!(mr2.added_edge_count, 1);

        // Set up processors via direct access.
        let engine = unsafe { &mut *engine_ptr };
        let src_id = ffi_to_node_id(src_ffi);
        let consumer_id = ffi_to_node_id(consumer_ffi);

        engine.inner.set_processor(src_id, make_source(iron(), 5.0));
        engine.inner.set_input_inventory(src_id, simple_inventory(100));
        engine.inner.set_output_inventory(src_id, simple_inventory(100));

        engine.inner.set_processor(
            consumer_id,
            make_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 3),
        );
        let mut consumer_input = simple_inventory(100);
        let _ = consumer_input.input_slots[0].add(iron(), 10);
        engine.inner.set_input_inventory(consumer_id, consumer_input);
        engine.inner.set_output_inventory(consumer_id, simple_inventory(100));

        // Step 10 times.
        for _ in 0..10 {
            unsafe { factorial_step(engine_ptr) };
        }

        // Verify counts.
        let mut node_count: u32 = 0;
        let mut edge_count: u32 = 0;
        unsafe { factorial_node_count(engine_ptr, &mut node_count) };
        unsafe { factorial_edge_count(engine_ptr, &mut edge_count) };
        assert_eq!(node_count, 2);
        assert_eq!(edge_count, 1);

        // Serialize.
        let mut buffer = FfiByteBuffer {
            data: ptr::null_mut(),
            len: 0,
        };
        let result = unsafe { factorial_serialize(engine_ptr, &mut buffer) };
        assert_eq!(result, FactorialResult::Ok);
        assert!(buffer.len > 0);

        // Get original hash.
        let mut original_hash: u64 = 0;
        unsafe { factorial_get_state_hash(engine_ptr, &mut original_hash) };

        // Deserialize.
        let mut restored_ptr: *mut FactorialEngine = ptr::null_mut();
        let result = unsafe { factorial_deserialize(buffer.data, buffer.len, &mut restored_ptr) };
        assert_eq!(result, FactorialResult::Ok);

        // Verify restored state.
        let mut restored_hash: u64 = 0;
        unsafe { factorial_get_state_hash(restored_ptr, &mut restored_hash) };
        assert_eq!(original_hash, restored_hash);

        let mut restored_tick: u64 = 0;
        unsafe { factorial_get_tick(restored_ptr, &mut restored_tick) };
        assert_eq!(restored_tick, 10);

        // Clean up.
        unsafe { factorial_free_buffer(buffer) };
        unsafe { factorial_destroy(restored_ptr) };
        unsafe { factorial_destroy(engine_ptr) };
    }

    // -----------------------------------------------------------------------
    // Test 20: Event polling for empty engine returns zero events
    // -----------------------------------------------------------------------
    #[test]
    fn poll_events_empty_engine() {
        let engine = factorial_create();

        // Step without any nodes.
        unsafe { factorial_step(engine) };

        let mut eb = FfiEventBuffer {
            events: ptr::null(),
            count: 0,
        };
        let result = unsafe { factorial_poll_events(engine, &mut eb) };
        assert_eq!(result, FactorialResult::Ok);
        assert_eq!(eb.count, 0);

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 21: Poisoned engine blocks calls
    // -----------------------------------------------------------------------
    #[test]
    fn poisoned_engine_blocks_calls() {
        let engine = factorial_create();
        assert!(!engine.is_null());
        let engine_ref = unsafe { &mut *engine };
        engine_ref.poisoned = true;
        assert!(unsafe { factorial_is_poisoned(engine) });
        let result = unsafe { factorial_step(engine) };
        assert_eq!(result, FactorialResult::Poisoned);
        let mut tick: u64 = 0;
        let result = unsafe { factorial_get_tick(engine, &mut tick) };
        assert_eq!(result, FactorialResult::Poisoned);
        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 22: Clear poison allows resume
    // -----------------------------------------------------------------------
    #[test]
    fn clear_poison_allows_resume() {
        let engine = factorial_create();
        let engine_ref = unsafe { &mut *engine };
        engine_ref.poisoned = true;
        assert!(unsafe { factorial_is_poisoned(engine) });
        let result = unsafe { factorial_clear_poison(engine) };
        assert_eq!(result, FactorialResult::Ok);
        assert!(!unsafe { factorial_is_poisoned(engine) });
        let result = unsafe { factorial_step(engine) };
        assert_eq!(result, FactorialResult::Ok);
        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 23: New engine not poisoned
    // -----------------------------------------------------------------------
    #[test]
    fn new_engine_not_poisoned() {
        let engine = factorial_create();
        assert!(!unsafe { factorial_is_poisoned(engine) });
        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Helper: add a node via FFI, apply mutations, return real FfiNodeId
    // -----------------------------------------------------------------------
    fn ffi_add_node_and_apply(engine: *mut FactorialEngine, building_type: u32) -> FfiNodeId {
        let mut pending: FfiPendingNodeId = 0;
        unsafe { factorial_add_node(engine, building_type, &mut pending) };
        let mut mr = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine, &mut mr) };
        assert_eq!(mr.added_node_count, 1);
        let pairs = unsafe { std::slice::from_raw_parts(mr.added_nodes, 1) };
        pairs[0].real_id
    }

    // -----------------------------------------------------------------------
    // Helper: add two nodes via FFI, connect them, apply, return (node_a, node_b, edge)
    // -----------------------------------------------------------------------
    fn ffi_add_two_nodes_and_connect(
        engine: *mut FactorialEngine,
    ) -> (FfiNodeId, FfiNodeId, FfiEdgeId) {
        // Add two nodes.
        let mut pending_a: FfiPendingNodeId = 0;
        let mut pending_b: FfiPendingNodeId = 0;
        unsafe { factorial_add_node(engine, 0, &mut pending_a) };
        unsafe { factorial_add_node(engine, 1, &mut pending_b) };
        let mut mr = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine, &mut mr) };
        assert_eq!(mr.added_node_count, 2);
        let node_pairs = unsafe { std::slice::from_raw_parts(mr.added_nodes, 2) };
        let node_a = node_pairs[0].real_id;
        let node_b = node_pairs[1].real_id;

        // Connect A -> B.
        let mut pending_edge: FfiPendingEdgeId = 0;
        unsafe { factorial_connect(engine, node_a, node_b, &mut pending_edge) };
        let mut mr2 = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine, &mut mr2) };
        assert_eq!(mr2.added_edge_count, 1);
        let edge_pairs = unsafe { std::slice::from_raw_parts(mr2.added_edges, 1) };
        let edge_id = edge_pairs[0].real_id;

        (node_a, node_b, edge_id)
    }

    // -----------------------------------------------------------------------
    // Test 24: Set source via FFI
    // -----------------------------------------------------------------------
    #[test]
    fn set_source_via_ffi() {
        let engine = factorial_create();
        let node_id = ffi_add_node_and_apply(engine, 0);

        // Set source: item_type=0 (iron), rate=3.0 as Fixed64 bits.
        let rate_bits = Fixed64::from_num(3).to_bits();
        let result = unsafe { factorial_set_source(engine, node_id, 0, rate_bits) };
        assert_eq!(result, FactorialResult::Ok);

        // Set inventories.
        let result = unsafe { factorial_set_input_capacity(engine, node_id, 100) };
        assert_eq!(result, FactorialResult::Ok);
        let result = unsafe { factorial_set_output_capacity(engine, node_id, 100) };
        assert_eq!(result, FactorialResult::Ok);

        // Step once.
        let result = unsafe { factorial_step(engine) };
        assert_eq!(result, FactorialResult::Ok);

        // Check output: source at rate 3 should produce 3 items.
        let mut count: u32 = 0;
        let result =
            unsafe { factorial_get_output_inventory_count(engine, node_id, &mut count) };
        assert_eq!(result, FactorialResult::Ok);
        assert_eq!(count, 3, "source should produce 3 items per tick");

        // Verify processor state is Working.
        let mut info = FfiProcessorInfo {
            state: FfiProcessorState::Idle,
            progress: 0,
        };
        let result =
            unsafe { factorial_get_processor_state(engine, node_id, &mut info) };
        assert_eq!(result, FactorialResult::Ok);
        assert_eq!(info.state, FfiProcessorState::Working);

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 25: Set fixed processor via FFI
    // -----------------------------------------------------------------------
    #[test]
    fn set_fixed_processor_via_ffi() {
        let engine = factorial_create();
        let node_id = ffi_add_node_and_apply(engine, 0);

        // Build a recipe: 2 iron(0) -> 1 gear(2), duration 5.
        let inputs = [
            FfiItemStack { item_type: 0, quantity: 2 },
        ];
        let outputs = [
            FfiItemStack { item_type: 2, quantity: 1 },
        ];
        let recipe = FfiRecipe {
            input_count: 1,
            inputs: inputs.as_ptr(),
            output_count: 1,
            outputs: outputs.as_ptr(),
            duration: 5,
        };

        let result = unsafe { factorial_set_fixed_processor(engine, node_id, &recipe) };
        assert_eq!(result, FactorialResult::Ok);

        // Set inventories.
        unsafe { factorial_set_input_capacity(engine, node_id, 100) };
        unsafe { factorial_set_output_capacity(engine, node_id, 100) };

        // Verify processor was set by checking state (should be Idle initially).
        let mut info = FfiProcessorInfo {
            state: FfiProcessorState::Working,
            progress: 99,
        };
        let result =
            unsafe { factorial_get_processor_state(engine, node_id, &mut info) };
        assert_eq!(result, FactorialResult::Ok);
        assert_eq!(info.state, FfiProcessorState::Idle);

        // Step once -- should stall on missing inputs since input inventory is empty.
        unsafe { factorial_step(engine) };

        let result =
            unsafe { factorial_get_processor_state(engine, node_id, &mut info) };
        assert_eq!(result, FactorialResult::Ok);
        assert_eq!(info.state, FfiProcessorState::StalledMissingInputs);

        // Null checks.
        let result = unsafe { factorial_set_fixed_processor(ptr::null_mut(), node_id, &recipe) };
        assert_eq!(result, FactorialResult::NullPointer);
        let result = unsafe { factorial_set_fixed_processor(engine, node_id, ptr::null()) };
        assert_eq!(result, FactorialResult::NullPointer);

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 26: Set transport types via FFI
    // -----------------------------------------------------------------------
    #[test]
    fn set_transport_types_via_ffi() {
        let engine = factorial_create();

        // We need 4 separate edges for 4 transport types.
        // Add 5 nodes and 4 edges: A->B, B->C, C->D, D->E.
        let mut pending_ids: [FfiPendingNodeId; 5] = [0; 5];
        for pid in &mut pending_ids {
            unsafe { factorial_add_node(engine, 0, pid) };
        }
        let mut mr = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine, &mut mr) };
        assert_eq!(mr.added_node_count, 5);
        let nodes = unsafe { std::slice::from_raw_parts(mr.added_nodes, 5) };
        let node_ids: Vec<FfiNodeId> = nodes.iter().map(|p| p.real_id).collect();

        // Connect 4 edges.
        let mut pending_edges: [FfiPendingEdgeId; 4] = [0; 4];
        for i in 0..4 {
            unsafe { factorial_connect(engine, node_ids[i], node_ids[i + 1], &mut pending_edges[i]) };
        }
        let mut mr2 = FfiMutationResult {
            added_nodes: ptr::null(),
            added_node_count: 0,
            added_edges: ptr::null(),
            added_edge_count: 0,
        };
        unsafe { factorial_apply_mutations(engine, &mut mr2) };
        assert_eq!(mr2.added_edge_count, 4);
        let edges = unsafe { std::slice::from_raw_parts(mr2.added_edges, 4) };
        let edge_ids: Vec<FfiEdgeId> = edges.iter().map(|p| p.real_id).collect();

        // Set FlowTransport on edge 0.
        let flow_rate_bits = Fixed64::from_num(5).to_bits();
        let result = unsafe { factorial_set_flow_transport(engine, edge_ids[0], flow_rate_bits) };
        assert_eq!(result, FactorialResult::Ok);

        // Set ItemTransport on edge 1.
        let speed_bits = Fixed64::from_num(1).to_bits();
        let result = unsafe { factorial_set_item_transport(engine, edge_ids[1], speed_bits, 10, 1) };
        assert_eq!(result, FactorialResult::Ok);

        // Set BatchTransport on edge 2.
        let result = unsafe { factorial_set_batch_transport(engine, edge_ids[2], 20, 5) };
        assert_eq!(result, FactorialResult::Ok);

        // Set VehicleTransport on edge 3.
        let result = unsafe { factorial_set_vehicle_transport(engine, edge_ids[3], 50, 10) };
        assert_eq!(result, FactorialResult::Ok);

        // Verify edge count is still 4.
        let mut edge_count: u32 = 0;
        unsafe { factorial_edge_count(engine, &mut edge_count) };
        assert_eq!(edge_count, 4);

        // Step should not panic with the configured transports.
        let result = unsafe { factorial_step(engine) };
        assert_eq!(result, FactorialResult::Ok);

        unsafe { factorial_destroy(engine) };
    }

    // -----------------------------------------------------------------------
    // Test 27: Full FFI lifecycle -- no direct access
    // -----------------------------------------------------------------------
    #[test]
    fn full_ffi_lifecycle_no_direct_access() {
        let engine = factorial_create();

        // Add source node and consumer node.
        let (src_id, consumer_id, edge_id) = ffi_add_two_nodes_and_connect(engine);

        // Configure source: item_type=0 (iron), rate=5.0.
        let rate_bits = Fixed64::from_num(5).to_bits();
        assert_eq!(
            unsafe { factorial_set_source(engine, src_id, 0, rate_bits) },
            FactorialResult::Ok
        );

        // Configure consumer: 2 iron -> 1 gear, 3 ticks.
        let inputs = [FfiItemStack { item_type: 0, quantity: 2 }];
        let outputs = [FfiItemStack { item_type: 2, quantity: 1 }];
        let recipe = FfiRecipe {
            input_count: 1,
            inputs: inputs.as_ptr(),
            output_count: 1,
            outputs: outputs.as_ptr(),
            duration: 3,
        };
        assert_eq!(
            unsafe { factorial_set_fixed_processor(engine, consumer_id, &recipe) },
            FactorialResult::Ok
        );

        // Set inventories for both nodes.
        assert_eq!(
            unsafe { factorial_set_input_capacity(engine, src_id, 100) },
            FactorialResult::Ok
        );
        assert_eq!(
            unsafe { factorial_set_output_capacity(engine, src_id, 100) },
            FactorialResult::Ok
        );
        assert_eq!(
            unsafe { factorial_set_input_capacity(engine, consumer_id, 100) },
            FactorialResult::Ok
        );
        assert_eq!(
            unsafe { factorial_set_output_capacity(engine, consumer_id, 100) },
            FactorialResult::Ok
        );

        // Set transport: flow at rate 10.
        let transport_rate_bits = Fixed64::from_num(10).to_bits();
        assert_eq!(
            unsafe { factorial_set_flow_transport(engine, edge_id, transport_rate_bits) },
            FactorialResult::Ok
        );

        // Step 20 times.
        for _ in 0..20 {
            let result = unsafe { factorial_step(engine) };
            assert_eq!(result, FactorialResult::Ok);
        }

        // Verify tick is 20.
        let mut tick: u64 = 0;
        unsafe { factorial_get_tick(engine, &mut tick) };
        assert_eq!(tick, 20);

        // The source produces 5 iron/tick. Over 20 ticks that's 100 iron produced.
        // Some will be in the source output, some transported, some consumed.
        // We just verify the system ran without errors and something happened.

        // Check source output: it should have items (source produces 5/tick, transport
        // moves up to 10/tick, so source output shouldn't be full).
        let mut src_output: u32 = 0;
        unsafe { factorial_get_output_inventory_count(engine, src_id, &mut src_output) };

        // Check consumer output: after ~20 ticks with a 3-tick recipe consuming 2 iron
        // and receiving a flow of iron, the consumer should have produced some gears.
        let mut consumer_output: u32 = 0;
        unsafe { factorial_get_output_inventory_count(engine, consumer_id, &mut consumer_output) };

        // The consumer should have produced at least some gears by now.
        // With 5 iron/tick flowing in and the recipe consuming 2 iron every 3 ticks,
        // there should be meaningful output.
        assert!(
            consumer_output > 0,
            "consumer should have produced gears after 20 ticks, got {consumer_output}"
        );

        // Verify processor states are not stalled.
        let mut src_info = FfiProcessorInfo {
            state: FfiProcessorState::Idle,
            progress: 0,
        };
        unsafe { factorial_get_processor_state(engine, src_id, &mut src_info) };
        assert_eq!(src_info.state, FfiProcessorState::Working);

        unsafe { factorial_destroy(engine) };
    }
}
