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
use crate::sim::SimulationStrategy;
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
pub fn iron_ore() -> ItemTypeId {
    ItemTypeId(10)
}
pub fn copper_ore() -> ItemTypeId {
    ItemTypeId(11)
}
pub fn coal() -> ItemTypeId {
    ItemTypeId(12)
}
pub fn stone() -> ItemTypeId {
    ItemTypeId(13)
}
pub fn wood() -> ItemTypeId {
    ItemTypeId(14)
}
pub fn tungsten_ore() -> ItemTypeId {
    ItemTypeId(15)
}

// Tier 1: Furnace/Workshop products
pub fn iron_ingot() -> ItemTypeId {
    ItemTypeId(20)
}
pub fn copper_ingot() -> ItemTypeId {
    ItemTypeId(21)
}
pub fn sand() -> ItemTypeId {
    ItemTypeId(22)
}
pub fn glass() -> ItemTypeId {
    ItemTypeId(23)
}
pub fn wood_plank() -> ItemTypeId {
    ItemTypeId(24)
}
pub fn iron_gear_b() -> ItemTypeId {
    ItemTypeId(25)
}
pub fn copper_wire() -> ItemTypeId {
    ItemTypeId(26)
}

// Tier 2: Machine Shop/Forge products
pub fn motor() -> ItemTypeId {
    ItemTypeId(30)
}
pub fn wood_frame() -> ItemTypeId {
    ItemTypeId(31)
}
pub fn light_bulb() -> ItemTypeId {
    ItemTypeId(32)
}
pub fn graphite() -> ItemTypeId {
    ItemTypeId(33)
}
pub fn steel() -> ItemTypeId {
    ItemTypeId(34)
}
pub fn tungsten_carbide() -> ItemTypeId {
    ItemTypeId(35)
}

// Tier 3: Industrial Factory products
pub fn electric_motor() -> ItemTypeId {
    ItemTypeId(40)
}
pub fn circuit_board() -> ItemTypeId {
    ItemTypeId(41)
}
pub fn basic_robot() -> ItemTypeId {
    ItemTypeId(42)
}

// Tier 4: Manufacturer products
pub fn computer() -> ItemTypeId {
    ItemTypeId(50)
}
pub fn super_computer() -> ItemTypeId {
    ItemTypeId(51)
}

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
                consumed: true,
            })
            .collect(),
        outputs: outputs
            .into_iter()
            .map(|(item_type, quantity)| RecipeOutput {
                item_type,
                quantity,
                bonus: None,
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

// ===========================================================================
// Factory builders (for benchmarks, stress tests, and proptests)
// ===========================================================================

/// Build a linear chain of N nodes: Source -> Assembler -> ... -> Assembler.
/// Deep graph with 1 node per topological level.
pub fn build_chain_factory(length: usize) -> Engine {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    if length == 0 {
        return engine;
    }

    // First node is a source.
    let mut prev = add_node(&mut engine, make_source(iron(), 2.0), 100, 100);

    // Remaining nodes are assemblers chained linearly.
    for _ in 1..length {
        let node = add_node(
            &mut engine,
            make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5),
            100,
            100,
        );
        connect(&mut engine, prev, node, make_flow_transport(10.0));
        prev = node;
    }

    engine
}

/// Build a wide factory: 1 source feeding N consumer assemblers.
/// 2 topological levels — best case for level parallelism.
pub fn build_wide_factory(fan_out: usize) -> Engine {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    let source = add_node(&mut engine, make_source(iron(), 100.0), 1000, 1000);

    for _ in 0..fan_out {
        let consumer = add_node(
            &mut engine,
            make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5),
            100,
            100,
        );
        connect(&mut engine, source, consumer, make_flow_transport(10.0));
    }

    engine
}

/// Build a grid factory: rows x cols mesh of assembler nodes.
/// Each node connects to the one to its right and the one below it.
pub fn build_grid_factory(rows: usize, cols: usize) -> Engine {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    if rows == 0 || cols == 0 {
        return engine;
    }

    // Create all nodes. First column are sources, rest are assemblers.
    let mut grid: Vec<Vec<NodeId>> = Vec::with_capacity(rows);
    for _ in 0..rows {
        let mut row = Vec::with_capacity(cols);
        // First node in each row is a source.
        row.push(add_node(&mut engine, make_source(iron(), 2.0), 100, 100));
        for _ in 1..cols {
            row.push(add_node(
                &mut engine,
                make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5),
                100,
                100,
            ));
        }
        grid.push(row);
    }

    // Connect horizontally (left to right).
    for row in &grid {
        for c in 0..cols - 1 {
            connect(&mut engine, row[c], row[c + 1], make_flow_transport(10.0));
        }
    }

    // Connect vertically (top to bottom).
    for pair in grid.windows(2) {
        for (&src, &dst) in pair[0].iter().zip(pair[1].iter()) {
            connect(&mut engine, src, dst, make_flow_transport(5.0));
        }
    }

    engine
}

/// Build a large factory with the given node count.
/// Chains of 20 nodes with cross-links between adjacent chains.
pub fn build_large_factory(node_count: usize) -> Engine {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    let chain_length = 20;
    let chain_count = node_count.div_ceil(chain_length);
    let mut chains: Vec<Vec<NodeId>> = Vec::with_capacity(chain_count);
    let mut total = 0;

    for _ in 0..chain_count {
        let mut chain = Vec::with_capacity(chain_length);

        // Source node.
        let source = add_node(&mut engine, make_source(iron(), 2.0), 100, 100);
        chain.push(source);
        total += 1;

        // Assembler chain.
        for _ in 1..chain_length {
            if total >= node_count {
                break;
            }
            let assembler = add_node(
                &mut engine,
                make_recipe(vec![(iron(), 1)], vec![(gear(), 1)], 5),
                100,
                100,
            );
            chain.push(assembler);
            total += 1;
        }

        // Connect chain linearly with alternating transports.
        for i in 0..chain.len() - 1 {
            let transport = match i % 3 {
                0 => make_flow_transport(10.0),
                1 => make_item_transport(10),
                _ => make_batch_transport(10, 5),
            };
            connect(&mut engine, chain[i], chain[i + 1], transport);
        }

        chains.push(chain);
    }

    // Cross-links between adjacent chains at every other depth.
    for i in 0..chains.len().saturating_sub(1) {
        let max_depth = chains[i].len().min(chains[i + 1].len());
        for depth in (1..max_depth).step_by(2) {
            connect(
                &mut engine,
                chains[i][depth],
                chains[i + 1][depth],
                make_flow_transport(5.0),
            );
        }
    }

    engine
}
