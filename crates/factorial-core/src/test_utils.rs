//! Shared test helpers for integration tests and benchmarks.
//!
//! Gated behind `#[cfg(any(test, feature = "test-utils"))]` so these helpers
//! are available in unit tests, integration tests, and benchmarks (via the
//! `test-utils` feature).

use crate::engine::Engine;
use crate::fixed::Fixed64;
use crate::id::*;
use crate::item::Inventory;
use crate::processor::*;
use crate::transport::*;

// ===========================================================================
// Fixed-point helper
// ===========================================================================

pub fn fixed(v: f64) -> Fixed64 {
    Fixed64::from_num(v)
}

// ===========================================================================
// Item constructors
// ===========================================================================

pub fn iron() -> ItemTypeId {
    ItemTypeId(0)
}
pub fn copper() -> ItemTypeId {
    ItemTypeId(1)
}
pub fn gear() -> ItemTypeId {
    ItemTypeId(2)
}
pub fn water() -> ItemTypeId {
    ItemTypeId(3)
}
pub fn oxygen() -> ItemTypeId {
    ItemTypeId(4)
}
pub fn hydrogen() -> ItemTypeId {
    ItemTypeId(5)
}

// ===========================================================================
// Builderment item types
// ===========================================================================

// Raw resources
pub fn iron_ore() -> ItemTypeId { ItemTypeId(10) }
pub fn copper_ore() -> ItemTypeId { ItemTypeId(11) }
pub fn coal() -> ItemTypeId { ItemTypeId(12) }
pub fn stone() -> ItemTypeId { ItemTypeId(13) }
pub fn wood() -> ItemTypeId { ItemTypeId(14) }
pub fn tungsten_ore() -> ItemTypeId { ItemTypeId(15) }

// Tier 1: Furnace/Workshop products
pub fn iron_ingot() -> ItemTypeId { ItemTypeId(20) }
pub fn copper_ingot() -> ItemTypeId { ItemTypeId(21) }
pub fn sand() -> ItemTypeId { ItemTypeId(22) }
pub fn glass() -> ItemTypeId { ItemTypeId(23) }
pub fn wood_plank() -> ItemTypeId { ItemTypeId(24) }
pub fn iron_gear_b() -> ItemTypeId { ItemTypeId(25) }
pub fn copper_wire() -> ItemTypeId { ItemTypeId(26) }

// Tier 2: Machine Shop/Forge products
pub fn motor() -> ItemTypeId { ItemTypeId(30) }
pub fn wood_frame() -> ItemTypeId { ItemTypeId(31) }
pub fn light_bulb() -> ItemTypeId { ItemTypeId(32) }
pub fn graphite() -> ItemTypeId { ItemTypeId(33) }
pub fn steel() -> ItemTypeId { ItemTypeId(34) }
pub fn tungsten_carbide() -> ItemTypeId { ItemTypeId(35) }

// Tier 3: Industrial Factory products
pub fn electric_motor() -> ItemTypeId { ItemTypeId(40) }
pub fn circuit_board() -> ItemTypeId { ItemTypeId(41) }
pub fn basic_robot() -> ItemTypeId { ItemTypeId(42) }

// Tier 4: Manufacturer products
pub fn computer() -> ItemTypeId { ItemTypeId(50) }
pub fn super_computer() -> ItemTypeId { ItemTypeId(51) }

// ===========================================================================
// Building constructor
// ===========================================================================

pub fn building() -> BuildingTypeId {
    BuildingTypeId(0)
}

// ===========================================================================
// Inventory helper
// ===========================================================================

pub fn simple_inventory(capacity: u32) -> Inventory {
    Inventory::new(1, 1, capacity)
}

// ===========================================================================
// Processor constructors
// ===========================================================================

pub fn make_source(item: ItemTypeId, rate: f64) -> Processor {
    Processor::Source(SourceProcessor {
        output_type: item,
        base_rate: fixed(rate),
        depletion: Depletion::Infinite,
        accumulated: fixed(0.0),
        initial_properties: None,
    })
}

pub fn make_recipe(
    inputs: Vec<(ItemTypeId, u32)>,
    outputs: Vec<(ItemTypeId, u32)>,
    duration: u32,
) -> Processor {
    Processor::Fixed(FixedRecipe {
        inputs: inputs
            .into_iter()
            .map(|(item_type, quantity)| RecipeInput {
                item_type,
                quantity,
            })
            .collect(),
        outputs: outputs
            .into_iter()
            .map(|(item_type, quantity)| RecipeOutput {
                item_type,
                quantity,
            })
            .collect(),
        duration,
    })
}

// ===========================================================================
// Transport constructors
// ===========================================================================

pub fn make_flow_transport(rate: f64) -> Transport {
    Transport::Flow(FlowTransport {
        rate: fixed(rate),
        buffer_capacity: fixed(1000.0),
        latency: 0,
    })
}

pub fn make_item_transport(slot_count: u32) -> Transport {
    Transport::Item(ItemTransport {
        speed: fixed(1.0),
        slot_count,
        lanes: 1,
    })
}

pub fn make_batch_transport(batch_size: u32, cycle_time: u32) -> Transport {
    Transport::Batch(BatchTransport {
        batch_size,
        cycle_time,
    })
}

pub fn make_vehicle_transport(capacity: u32, travel_time: u32) -> Transport {
    Transport::Vehicle(VehicleTransport {
        capacity,
        travel_time,
    })
}

// ===========================================================================
// Engine helpers
// ===========================================================================

/// Add a node to the engine with the given processor and inventories.
/// Returns the assigned NodeId.
pub fn add_node(
    engine: &mut Engine,
    processor: Processor,
    input_capacity: u32,
    output_capacity: u32,
) -> NodeId {
    let pending = engine.graph.queue_add_node(building());
    let result = engine.graph.apply_mutations();
    let node = result.resolve_node(pending).unwrap();

    engine.set_processor(node, processor);
    engine.set_input_inventory(node, simple_inventory(input_capacity));
    engine.set_output_inventory(node, simple_inventory(output_capacity));

    node
}

/// Connect two nodes and set transport. Returns the EdgeId.
pub fn connect(engine: &mut Engine, from: NodeId, to: NodeId, transport: Transport) -> EdgeId {
    let pending = engine.graph.queue_connect(from, to);
    let result = engine.graph.apply_mutations();
    let edge = result.resolve_edge(pending).unwrap();
    engine.set_transport(edge, transport);
    edge
}

/// Connect two nodes with an item type filter on the edge.
pub fn connect_filtered(
    engine: &mut Engine,
    from: NodeId,
    to: NodeId,
    transport: Transport,
    item_filter: Option<ItemTypeId>,
) -> EdgeId {
    let pending = engine.graph.queue_connect_filtered(from, to, item_filter);
    let result = engine.graph.apply_mutations();
    let edge = result.resolve_edge(pending).unwrap();
    engine.set_transport(edge, transport);
    edge
}

// ===========================================================================
// Query helpers
// ===========================================================================

/// Get the total quantity of a specific item in a node's output inventory.
pub fn output_quantity(engine: &Engine, node: NodeId, item: ItemTypeId) -> u32 {
    engine
        .get_output_inventory(node)
        .map(|inv| {
            inv.output_slots
                .iter()
                .map(|s| s.quantity(item))
                .sum::<u32>()
        })
        .unwrap_or(0)
}

/// Get the total quantity of a specific item in a node's input inventory.
pub fn input_quantity(engine: &Engine, node: NodeId, item: ItemTypeId) -> u32 {
    engine
        .get_input_inventory(node)
        .map(|inv| {
            inv.input_slots
                .iter()
                .map(|s| s.quantity(item))
                .sum::<u32>()
        })
        .unwrap_or(0)
}

/// Total items across all types in a node's input inventory.
pub fn input_total(engine: &Engine, node: NodeId) -> u32 {
    engine
        .get_input_inventory(node)
        .map(|inv| inv.input_slots.iter().map(|s| s.total()).sum::<u32>())
        .unwrap_or(0)
}

/// Total items across all types in a node's output inventory.
pub fn output_total(engine: &Engine, node: NodeId) -> u32 {
    engine
        .get_output_inventory(node)
        .map(|inv| inv.output_slots.iter().map(|s| s.total()).sum::<u32>())
        .unwrap_or(0)
}
