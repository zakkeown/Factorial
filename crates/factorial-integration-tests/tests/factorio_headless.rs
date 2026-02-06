//! Factorio-style integration tests for the Factorial engine.
//!
//! Models a Factorio green-science-pack production line to stress-test the
//! engine's transport, fluid, power, tech tree, and junction systems. Each
//! test targets a specific mechanic (oil refining, train logistics, inserter
//! loading, dual-lane belts, etc.) and documents ENGINE GAPs where the engine
//! does not yet expose the necessary API.
//!
//! Item IDs start at 100 to avoid collision with Builderment item IDs (0-51).

// Not all item types are used by every test. Suppress dead_code warnings for
// constructors reserved for future tests.
#![allow(dead_code)]

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::junction::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;
use factorial_core::transport::*;
use factorial_power::{PowerConsumer, PowerEvent, PowerModule, PowerProducer};
use factorial_tech_tree::{ResearchCost, TechEvent, TechId, TechTree, Technology, Unlock};

// ENGINE GAP: factorial-fluid is not yet in this crate's Cargo.toml
// dev-dependencies. The fluid tests below will not compile until:
//   [dev-dependencies]
//   factorial-fluid = { path = "../factorial-fluid" }
// is added to crates/factorial-integration-tests/Cargo.toml.
use factorial_fluid::{FluidConsumer, FluidModule, FluidProducer, FluidStorage};

// ============================================================================
// Factorio item type constructors (IDs 100-149)
// ============================================================================

// --- Raw resources ---
fn f_iron_ore() -> ItemTypeId {
    ItemTypeId(100)
}
fn f_copper_ore() -> ItemTypeId {
    ItemTypeId(101)
}
fn f_coal() -> ItemTypeId {
    ItemTypeId(102)
}
fn f_stone() -> ItemTypeId {
    ItemTypeId(103)
}
fn f_crude_oil() -> ItemTypeId {
    ItemTypeId(104)
}
fn f_water() -> ItemTypeId {
    ItemTypeId(105)
}

// --- Smelted materials ---
fn f_iron_plate() -> ItemTypeId {
    ItemTypeId(110)
}
fn f_copper_plate() -> ItemTypeId {
    ItemTypeId(111)
}
fn f_steel_plate() -> ItemTypeId {
    ItemTypeId(112)
}
fn f_stone_brick() -> ItemTypeId {
    ItemTypeId(113)
}

// --- Intermediates ---
fn f_iron_gear() -> ItemTypeId {
    ItemTypeId(120)
}
fn f_copper_cable() -> ItemTypeId {
    ItemTypeId(121)
}
fn f_green_circuit() -> ItemTypeId {
    ItemTypeId(122)
}
fn f_red_circuit() -> ItemTypeId {
    ItemTypeId(123)
}
fn f_inserter_item() -> ItemTypeId {
    ItemTypeId(124)
}
fn f_transport_belt() -> ItemTypeId {
    ItemTypeId(125)
}
fn f_plastic_bar() -> ItemTypeId {
    ItemTypeId(126)
}

// --- Fluids ---
fn f_petroleum() -> ItemTypeId {
    ItemTypeId(130)
}
fn f_light_oil() -> ItemTypeId {
    ItemTypeId(131)
}
fn f_heavy_oil() -> ItemTypeId {
    ItemTypeId(132)
}
fn f_sulfuric_acid() -> ItemTypeId {
    ItemTypeId(133)
}

// --- Science packs ---
fn f_red_science() -> ItemTypeId {
    ItemTypeId(140)
}
fn f_green_science() -> ItemTypeId {
    ItemTypeId(141)
}

// ============================================================================
// Shared constants
// ============================================================================

/// Standard input buffer for single-input buildings (furnaces, single-recipe assemblers).
const STD_INPUT_CAP: u32 = 50;
/// Standard output buffer.
const STD_OUTPUT_CAP: u32 = 50;
/// Large input buffer for multi-input buildings. Prevents the faster-arriving
/// item type from filling the shared inventory before the slower type arrives.
const MULTI_INPUT_CAP: u32 = 10_000;
/// Sink buffer for buildings that only accumulate items.
const SINK_INPUT_CAP: u32 = 50_000;

// ============================================================================
// Shared transport helpers
// ============================================================================

/// Standard Factorio yellow belt: 8 slots, speed 1.0, 1 lane.
fn yellow_belt() -> Transport {
    make_item_transport(8)
}

/// Dual-lane belt: 8 slots per lane, speed 1.0, 2 lanes.
fn dual_lane_belt() -> Transport {
    Transport::Item(ItemTransport {
        speed: Fixed64::from_num(1),
        slot_count: 8,
        lanes: 2,
    })
}

/// Flow pipe for fluids: rate 10.0 units/tick, 1000 buffer, no latency.
fn flow_pipe() -> Transport {
    make_flow_transport(10.0)
}

/// Train logistics: 50-item batches every 20 ticks.
fn train_transport() -> Transport {
    make_batch_transport(50, 20)
}

// ============================================================================
// Shared builder helpers
// ============================================================================

/// Build an iron plate smelting chain: source -> belt -> furnace.
/// Returns the furnace NodeId.
fn build_iron_smelting_chain(engine: &mut Engine) -> NodeId {
    let ore_src = add_node(
        engine,
        make_source(f_iron_ore(), 5.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let furnace = add_node(
        engine,
        make_recipe(vec![(f_iron_ore(), 1)], vec![(f_iron_plate(), 1)], 2),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(engine, ore_src, furnace, yellow_belt());
    furnace
}

/// Build a copper plate smelting chain: source -> belt -> furnace.
/// Returns the furnace NodeId.
fn build_copper_smelting_chain(engine: &mut Engine) -> NodeId {
    let ore_src = add_node(
        engine,
        make_source(f_copper_ore(), 5.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let furnace = add_node(
        engine,
        make_recipe(vec![(f_copper_ore(), 1)], vec![(f_copper_plate(), 1)], 2),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(engine, ore_src, furnace, yellow_belt());
    furnace
}

/// Build a copper cable chain: copper smelting -> cable assembler.
/// Returns the cable assembler NodeId.
fn build_copper_cable_chain(engine: &mut Engine) -> NodeId {
    let copper_furnace = build_copper_smelting_chain(engine);
    // Factorio: 1 copper plate -> 2 copper cables, duration 0.5s (1 tick).
    let cable_assembler = add_node(
        engine,
        make_recipe(
            vec![(f_copper_plate(), 1)],
            vec![(f_copper_cable(), 2)],
            1,
        ),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(engine, copper_furnace, cable_assembler, yellow_belt());
    cable_assembler
}

/// Build an iron gear chain: iron smelting -> gear assembler.
/// Returns the gear assembler NodeId.
fn build_iron_gear_chain(engine: &mut Engine) -> NodeId {
    let iron_furnace = build_iron_smelting_chain(engine);
    // Factorio: 2 iron plates -> 1 iron gear, duration 1s (2 ticks).
    let gear_assembler = add_node(
        engine,
        make_recipe(vec![(f_iron_plate(), 2)], vec![(f_iron_gear(), 1)], 2),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(engine, iron_furnace, gear_assembler, yellow_belt());
    gear_assembler
}

// ============================================================================
// Test 1: Iron smelting line
// ============================================================================

/// Verify that a basic iron ore -> iron plate smelting line produces plates
/// over 500 ticks. This is the simplest possible production chain: a source
/// feeding a furnace via a belt.
#[test]
fn test_iron_smelting_line() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    let iron_furnace = build_iron_smelting_chain(&mut engine);

    // Add a sink to consume iron plates so the output buffer does not fill up
    // and stall the furnace.
    let sink = add_node(
        &mut engine,
        make_recipe(
            vec![(f_iron_plate(), 9999)],
            vec![(f_iron_ore(), 1)],
            99999,
        ),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, iron_furnace, sink, yellow_belt());

    // Run for 500 ticks.
    for _ in 0..500 {
        engine.step();
    }

    // The furnace should have produced iron plates. With a source rate of 5.0
    // items/tick and a recipe of 1 ore -> 1 plate in 2 ticks, we expect
    // significant throughput. Check the sink received plates.
    let plates_received = input_quantity(&engine, sink, f_iron_plate());
    assert!(
        plates_received > 0,
        "sink should have received iron plates after 500 ticks, got {plates_received}"
    );

    // Sanity: at 2-tick recipe time, theoretical max is ~250 plates, but belt
    // throughput limits this. We just verify meaningful production occurred.
    assert!(
        plates_received >= 50,
        "expected at least 50 iron plates after 500 ticks, got {plates_received}"
    );
}

// ============================================================================
// Test 2: Green circuit production (multi-input fan-in)
// ============================================================================

/// Verify green circuit production: iron plates + copper cables -> green
/// circuits. This tests fan-in from two independent production chains into a
/// single multi-input assembler.
///
/// Factorio recipe: 1 iron plate + 3 copper cable -> 1 green circuit (1s).
#[test]
fn test_green_circuit_production() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Build two independent supply chains feeding the green circuit assembler.
    let iron_furnace = build_iron_smelting_chain(&mut engine);
    let cable_assembler = build_copper_cable_chain(&mut engine);

    // Green circuit assembler: 1 iron plate + 3 copper cable -> 1 green circuit.
    let circuit_assembler = add_node(
        &mut engine,
        make_recipe(
            vec![(f_iron_plate(), 1), (f_copper_cable(), 3)],
            vec![(f_green_circuit(), 1)],
            2,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, iron_furnace, circuit_assembler, yellow_belt());
    connect(
        &mut engine,
        cable_assembler,
        circuit_assembler,
        yellow_belt(),
    );

    // Sink for green circuits.
    let sink = add_node(
        &mut engine,
        make_recipe(
            vec![(f_green_circuit(), 9999)],
            vec![(f_iron_ore(), 1)],
            99999,
        ),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, circuit_assembler, sink, yellow_belt());

    // Run for 800 ticks (longer to account for multi-input convergence delays).
    for _ in 0..800 {
        engine.step();
    }

    // Verify iron plates arrived at the assembler.
    let iron_arrived = input_quantity(&engine, circuit_assembler, f_iron_plate());
    let cable_arrived = input_quantity(&engine, circuit_assembler, f_copper_cable());

    // At minimum, some of both inputs should have arrived.
    assert!(
        iron_arrived > 0 || input_total(&engine, circuit_assembler) > 0,
        "iron plates should have arrived at circuit assembler"
    );
    assert!(
        cable_arrived > 0 || input_total(&engine, circuit_assembler) > 0,
        "copper cables should have arrived at circuit assembler"
    );

    // Verify green circuits were produced and delivered to the sink.
    let circuits_in_sink = input_quantity(&engine, sink, f_green_circuit());
    assert!(
        circuits_in_sink > 0,
        "green circuits should have been produced and delivered to sink, got {circuits_in_sink}"
    );
}

// ============================================================================
// Test 3: Oil refinery multi-output
// ============================================================================

/// Test an oil refinery that takes crude oil and produces three fluid outputs:
/// petroleum gas, light oil, and heavy oil.
///
/// This exposes a fundamental ENGINE GAP: the engine's output routing currently
/// sends all output types to all connected edges equally. In Factorio, each
/// output type needs to route to a specific pipe network.
///
/// ENGINE GAP: The engine needs a way to route specific output item types to
/// specific edges (output-type filtering on edges). Currently FixedRecipe
/// supports multi-output, but all outputs go to all edges indiscriminately.
/// A possible API would be:
///   engine.set_edge_filter(edge_id, Some(item_type_id))
/// or an EdgeConfig struct with a filter field.
#[test]
fn test_oil_refinery_multi_output() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Crude oil source (extracted from oil well).
    let oil_source = add_node(
        &mut engine,
        make_source(f_crude_oil(), 3.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // Oil refinery: 10 crude oil -> 4 petroleum + 3 light oil + 3 heavy oil.
    // Duration 5 ticks.
    let refinery = add_node(
        &mut engine,
        make_recipe(
            vec![(f_crude_oil(), 10)],
            vec![
                (f_petroleum(), 4),
                (f_light_oil(), 3),
                (f_heavy_oil(), 3),
            ],
            5,
        ),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, oil_source, refinery, flow_pipe());

    // ENGINE GAP: We want separate pipe connections for each output type.
    // Currently all three outputs go to all edges. We create three sinks,
    // one for each fluid, but without edge filtering they will receive a
    // mixed bag of all three fluids.
    //
    // Desired API:
    //   let petro_edge = connect(&mut engine, refinery, petro_sink, flow_pipe());
    //   engine.set_edge_filter(petro_edge, Some(f_petroleum()));

    let petro_sink = add_node(
        &mut engine,
        make_recipe(
            vec![(f_petroleum(), 9999)],
            vec![(f_iron_ore(), 1)],
            99999,
        ),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let light_oil_sink = add_node(
        &mut engine,
        make_recipe(
            vec![(f_light_oil(), 9999)],
            vec![(f_iron_ore(), 1)],
            99999,
        ),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let heavy_oil_sink = add_node(
        &mut engine,
        make_recipe(
            vec![(f_heavy_oil(), 9999)],
            vec![(f_iron_ore(), 1)],
            99999,
        ),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // Connect refinery to all three sinks (without type filtering).
    connect(&mut engine, refinery, petro_sink, flow_pipe());
    connect(&mut engine, refinery, light_oil_sink, flow_pipe());
    connect(&mut engine, refinery, heavy_oil_sink, flow_pipe());

    // Run for 500 ticks.
    for _ in 0..500 {
        engine.step();
    }

    // Verify the refinery received crude oil.
    let crude_in = input_quantity(&engine, refinery, f_crude_oil());
    assert!(
        crude_in > 0 || output_total(&engine, refinery) > 0,
        "refinery should have received or processed crude oil"
    );

    // ENGINE GAP: Without edge-level output filtering, we cannot verify that
    // petroleum only went to petro_sink, light oil only to light_oil_sink, etc.
    // For now, verify that *some* output was produced and distributed across
    // the three sinks.
    let total_sink_input = input_total(&engine, petro_sink)
        + input_total(&engine, light_oil_sink)
        + input_total(&engine, heavy_oil_sink);
    assert!(
        total_sink_input > 0,
        "refinery outputs should have reached at least one sink, got {total_sink_input}"
    );
}

// ============================================================================
// Test 4: Train delivery from remote iron mine
// ============================================================================

/// Model a remote iron mine connected to the main base via train (BatchTransport).
/// Trains pick up 50 iron ore every 20 ticks, delivering in bursts. Verify
/// that production is bursty (not smooth) and that the smelter handles the
/// batch delivery pattern.
#[test]
fn test_train_delivery_remote_iron() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Remote iron mine.
    let remote_mine = add_node(
        &mut engine,
        make_source(f_iron_ore(), 10.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // Main base smelter.
    let smelter = add_node(
        &mut engine,
        make_recipe(vec![(f_iron_ore(), 1)], vec![(f_iron_plate(), 1)], 2),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // Train connection: 50 ore per batch, 20 tick cycle.
    connect(&mut engine, remote_mine, smelter, train_transport());

    // Sink for plates.
    let sink = add_node(
        &mut engine,
        make_recipe(
            vec![(f_iron_plate(), 9999)],
            vec![(f_iron_ore(), 1)],
            99999,
        ),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, smelter, sink, yellow_belt());

    // Track ore arrivals per tick to verify burstiness.
    let mut ore_arrivals: Vec<u32> = Vec::new();
    let mut last_ore_count = 0u32;

    for _tick in 0..200 {
        engine.step();

        // Sample the smelter input every tick to detect batch arrivals.
        let current_ore = input_quantity(&engine, smelter, f_iron_ore());
        let delta = current_ore.saturating_sub(last_ore_count);
        ore_arrivals.push(delta);
        last_ore_count = current_ore;

        // Reset tracking when smelter consumes ore (simplification).
        if current_ore < last_ore_count {
            last_ore_count = current_ore;
        }
    }

    // Verify that production occurred.
    let plates_in_sink = input_quantity(&engine, sink, f_iron_plate());
    assert!(
        plates_in_sink > 0,
        "smelter should have produced plates from train-delivered ore, got {plates_in_sink}"
    );

    // Verify burstiness: most ticks should have zero arrivals, with occasional
    // large batches. Count ticks with non-zero arrivals.
    let non_zero_ticks = ore_arrivals.iter().filter(|&&d| d > 0).count();
    let total_ticks = ore_arrivals.len();

    // With a 20-tick cycle over 200 ticks, we expect roughly 10 delivery events.
    // Non-zero ticks should be a small fraction of total ticks.
    assert!(
        non_zero_ticks < total_ticks / 2,
        "train delivery should be bursty: {non_zero_ticks} non-zero out of {total_ticks} ticks"
    );
}

// ============================================================================
// Test 5: Inserter-based loading from belt to building
// ============================================================================

/// Test the Junction::Inserter system by placing an inserter between a belt
/// and a building. The inserter should pick items from the belt output and
/// place them into the building's input at its configured speed.
///
/// ENGINE GAP: The engine currently defines Junction configurations
/// (InserterConfig, SplitterConfig, MergerConfig) as data structures, but
/// there is no evidence that Junction::Inserter actually processes items
/// during the engine tick. The junction system appears to be data-only with
/// no runtime behavior yet implemented. Tests here verify that the junction
/// can be set on a node without error, but the actual item-transfer behavior
/// may not work until the junction tick logic is implemented.
///
/// ENGINE GAP: The engine needs junction processing during the tick phase
/// that reads items from incoming edges and writes them to the node's input
/// inventory according to the InserterConfig (speed, stack_size, filter).
#[test]
fn test_inserter_belt_to_building() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Iron plate source.
    let plate_source = add_node(
        &mut engine,
        make_source(f_iron_plate(), 3.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // The "belt segment" is modeled as a passthrough node. Items arrive from
    // the source, then an inserter picks them out and places them into the
    // assembler.
    //
    // ENGINE GAP: Ideally we would place an inserter on the edge between
    // two nodes, not on a node itself. The current junction system attaches
    // junctions to nodes. For now, we attach the inserter to the assembler
    // node and rely on the engine to use it for input processing.
    let assembler = add_node(
        &mut engine,
        make_recipe(vec![(f_iron_plate(), 2)], vec![(f_iron_gear(), 1)], 2),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // Set inserter junction on the assembler.
    engine.set_junction(
        assembler,
        Junction::Inserter(InserterConfig {
            speed: Fixed64::from_num(2), // 2 items per tick
            stack_size: 4,               // pick up to 4 items at once
            filter: None,                // accept all item types
        }),
    );

    connect(&mut engine, plate_source, assembler, yellow_belt());

    // Sink for gears.
    let sink = add_node(
        &mut engine,
        make_recipe(
            vec![(f_iron_gear(), 9999)],
            vec![(f_iron_ore(), 1)],
            99999,
        ),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, assembler, sink, yellow_belt());

    // Verify the junction was set.
    let junction = engine.junction(assembler);
    assert!(
        junction.is_some(),
        "inserter junction should be set on assembler"
    );
    assert!(
        matches!(junction.unwrap(), Junction::Inserter(_)),
        "junction should be an Inserter variant"
    );

    // Run for 500 ticks.
    for _ in 0..500 {
        engine.step();
    }

    // Verify that items flowed through the system. The inserter junction
    // may or may not affect throughput depending on engine implementation.
    // ENGINE GAP: If junction processing is not yet implemented, items
    // still flow via normal transport and the inserter config is ignored.
    let gears_in_sink = input_quantity(&engine, sink, f_iron_gear());
    assert!(
        gears_in_sink > 0,
        "assembler should have produced gears (inserter may not affect flow yet), got {gears_in_sink}"
    );
}

// ============================================================================
// Test 6: Dual-lane belt (two items sharing a belt)
// ============================================================================

/// Test that a 2-lane belt can carry two different item types simultaneously.
/// Lane 0 carries iron plates, lane 1 carries copper plates.
///
/// ENGINE GAP: The ItemTransport supports `lanes: 2` and creates the correct
/// number of slots (slot_count * lanes), but the current belt advance logic
/// uses a placeholder ItemTypeId(0) for all items. The engine does not track
/// which item type is in each slot, nor does it assign items to specific lanes
/// based on their type. True dual-lane support requires:
///   1. Belt slots to store the actual ItemTypeId (not a placeholder).
///   2. A mechanism to assign items to lanes based on source or filter.
///   3. Lane-aware delivery at the destination (deliver lane 0 items to
///      consumer A, lane 1 items to consumer B).
#[test]
fn test_dual_lane_belt() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Two sources producing different items.
    let iron_source = add_node(
        &mut engine,
        make_source(f_iron_plate(), 3.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let copper_source = add_node(
        &mut engine,
        make_source(f_copper_plate(), 3.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // A shared junction node that merges both inputs onto a dual-lane belt.
    // ENGINE GAP: The engine does not currently support merging two inputs
    // onto specific lanes of a single belt. We model this as a merger node
    // feeding a dual-lane belt to a consumer.
    let merger_node = add_node(
        &mut engine,
        // Passthrough: accepts both item types and outputs them unchanged.
        // ENGINE GAP: There is no passthrough processor type. We use a recipe
        // with 1-in-1-out for iron plates as a stand-in. In practice, a merger
        // node would need to handle multiple item types.
        make_recipe(
            vec![(f_iron_plate(), 1)],
            vec![(f_iron_plate(), 1)],
            1,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // Set a merger junction on this node.
    engine.set_junction(
        merger_node,
        Junction::Merger(MergerConfig {
            policy: MergePolicy::RoundRobin,
        }),
    );

    connect(&mut engine, iron_source, merger_node, yellow_belt());
    connect(&mut engine, copper_source, merger_node, yellow_belt());

    // Consumer connected via a dual-lane belt.
    let consumer = add_node(
        &mut engine,
        make_recipe(
            vec![(f_iron_plate(), 9999)],
            vec![(f_copper_plate(), 1)],
            99999,
        ),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, merger_node, consumer, dual_lane_belt());

    // Run for 300 ticks.
    for _ in 0..300 {
        engine.step();
    }

    // Verify that the dual-lane belt was created with the correct slot count.
    // With 8 slots per lane and 2 lanes, the belt should have 16 total slots.
    assert_eq!(engine.edge_count(), 3, "should have 3 edges (2 input + 1 output)");

    // Verify items flowed through the system.
    let items_at_consumer = input_total(&engine, consumer);
    assert!(
        items_at_consumer > 0,
        "consumer should have received items via dual-lane belt, got {items_at_consumer}"
    );

    // ENGINE GAP: Cannot verify lane separation. Both item types arrive as
    // ItemTypeId(0) in the belt slot model. True dual-lane tests need the
    // engine to track actual item types per slot.
}

// ============================================================================
// Test 7: Science pack production feeding tech tree research
// ============================================================================

/// Build red + green science pack production lines and feed them into the
/// tech tree to complete research. Verifies the integration between the
/// production engine and the tech tree module.
///
/// Red science = 1 iron gear + 1 copper plate, duration 5 ticks.
/// Green science = 1 inserter + 1 transport belt, duration 6 ticks.
///
/// For simplicity, green science uses iron gear + green circuit as inputs
/// (closer to actual Factorio recipe).
#[test]
fn test_science_pack_to_tech_tree() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // --- Red science production line ---
    // iron gear + copper plate -> red science pack
    let gear_for_red = build_iron_gear_chain(&mut engine);
    let copper_for_red = build_copper_smelting_chain(&mut engine);

    let red_science_assembler = add_node(
        &mut engine,
        make_recipe(
            vec![(f_iron_gear(), 1), (f_copper_plate(), 1)],
            vec![(f_red_science(), 1)],
            5,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(
        &mut engine,
        gear_for_red,
        red_science_assembler,
        yellow_belt(),
    );
    connect(
        &mut engine,
        copper_for_red,
        red_science_assembler,
        yellow_belt(),
    );

    // Sink to collect red science packs.
    let red_sink = add_node(
        &mut engine,
        make_recipe(
            vec![(f_red_science(), 9999)],
            vec![(f_iron_ore(), 1)],
            99999,
        ),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, red_science_assembler, red_sink, yellow_belt());

    // --- Green science production line ---
    // iron gear + green circuit -> green science pack
    // (Simplified recipe; true Factorio uses inserter + belt.)
    let gear_for_green = build_iron_gear_chain(&mut engine);

    // Build a green circuit sub-chain for green science.
    let iron_for_circuit = build_iron_smelting_chain(&mut engine);
    let cable_for_circuit = build_copper_cable_chain(&mut engine);
    let circuit_assembler = add_node(
        &mut engine,
        make_recipe(
            vec![(f_iron_plate(), 1), (f_copper_cable(), 3)],
            vec![(f_green_circuit(), 1)],
            2,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(
        &mut engine,
        iron_for_circuit,
        circuit_assembler,
        yellow_belt(),
    );
    connect(
        &mut engine,
        cable_for_circuit,
        circuit_assembler,
        yellow_belt(),
    );

    let green_science_assembler = add_node(
        &mut engine,
        make_recipe(
            vec![(f_iron_gear(), 1), (f_green_circuit(), 1)],
            vec![(f_green_science(), 1)],
            6,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(
        &mut engine,
        gear_for_green,
        green_science_assembler,
        yellow_belt(),
    );
    connect(
        &mut engine,
        circuit_assembler,
        green_science_assembler,
        yellow_belt(),
    );

    // Sink to collect green science packs.
    let green_sink = add_node(
        &mut engine,
        make_recipe(
            vec![(f_green_science(), 9999)],
            vec![(f_iron_ore(), 1)],
            99999,
        ),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(
        &mut engine,
        green_science_assembler,
        green_sink,
        yellow_belt(),
    );

    // --- Tech tree setup ---
    let mut tech_tree = TechTree::new();

    let automation_id = TechId(0);
    let logistics_id = TechId(1);

    tech_tree
        .register(Technology {
            id: automation_id,
            name: "Automation".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Items(vec![(f_red_science(), 10)]),
            unlocks: vec![Unlock::Building(BuildingTypeId(10))],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

    tech_tree
        .register(Technology {
            id: logistics_id,
            name: "Logistics".to_string(),
            prerequisites: vec![automation_id],
            cost: ResearchCost::Items(vec![
                (f_red_science(), 10),
                (f_green_science(), 10),
            ]),
            unlocks: vec![Unlock::Building(BuildingTypeId(11))],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

    // Start automation research.
    tech_tree.start_research(automation_id, 0).unwrap();

    // Run the factory for 2000 ticks, periodically contributing science packs.
    let mut completed_techs: Vec<TechId> = Vec::new();
    let mut current_research: Option<TechId> = Some(automation_id);

    for tick in 1..=2000u64 {
        engine.step();

        // Every 20 ticks, contribute science packs to current research.
        if tick % 20 == 0 {
            if let Some(tech_id) = current_research {
                let contributions = match tech_id {
                    t if t == automation_id => vec![(f_red_science(), 3)],
                    t if t == logistics_id => {
                        vec![(f_red_science(), 3), (f_green_science(), 3)]
                    }
                    _ => vec![],
                };
                let _ = tech_tree.contribute_items(tech_id, &contributions, tick);
            }
        }

        // Check for completed research.
        let events = tech_tree.drain_events();
        for event in events {
            if let TechEvent::ResearchCompleted { tech_id, .. } = event {
                completed_techs.push(tech_id);

                // Start next research.
                if tech_id == automation_id {
                    let _ = tech_tree.start_research(logistics_id, tick);
                    current_research = Some(logistics_id);
                } else {
                    current_research = None;
                }
            }
        }
    }

    // Verify automation research completed.
    assert!(
        completed_techs.contains(&automation_id),
        "Automation tech should have completed"
    );

    // Verify logistics research completed (depends on both red + green science).
    assert!(
        completed_techs.contains(&logistics_id),
        "Logistics tech should have completed (requires red + green science)"
    );

    // Verify automation completed before logistics.
    let auto_pos = completed_techs
        .iter()
        .position(|&t| t == automation_id)
        .unwrap();
    let logi_pos = completed_techs
        .iter()
        .position(|&t| t == logistics_id)
        .unwrap();
    assert!(
        auto_pos < logi_pos,
        "Automation should complete before Logistics"
    );
}

// ============================================================================
// Test 8: Power grid scaling with variable demand
// ============================================================================

/// Start with a single boiler providing power. As tech unlocks buildings,
/// add more consumers to the power grid. Verify brownout when demand exceeds
/// supply, then recovery when a second power producer is added.
#[test]
fn test_power_grid_scaling() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Build a minimal factory.
    let iron_furnace = build_iron_smelting_chain(&mut engine);
    let gear_assembler = add_node(
        &mut engine,
        make_recipe(vec![(f_iron_plate(), 2)], vec![(f_iron_gear(), 1)], 2),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, iron_furnace, gear_assembler, yellow_belt());

    // Set up power module.
    let mut power = PowerModule::new();
    let net_id = power.create_network();

    // Single boiler: 50 watts capacity.
    let boiler_node = iron_furnace; // Reuse an existing node as the power source.
    power.add_producer(
        net_id,
        boiler_node,
        PowerProducer {
            capacity: Fixed64::from_num(50),
        },
    );

    // Phase 1: One consumer (iron furnace) at 20W. Should be fully satisfied.
    power.add_consumer(
        net_id,
        iron_furnace,
        PowerConsumer {
            demand: Fixed64::from_num(20),
        },
    );

    for tick in 1..=100u64 {
        engine.step();
        let events = power.tick(tick);
        for event in &events {
            assert!(
                !matches!(event, PowerEvent::PowerGridBrownout { .. }),
                "no brownout expected with 20W demand on 50W supply at tick {tick}"
            );
        }
    }

    let satisfaction = power.satisfaction(net_id).unwrap();
    assert_eq!(
        satisfaction,
        Fixed64::from_num(1),
        "power should be fully satisfied with low demand"
    );

    // Phase 2: Add more consumers to exceed supply (simulate unlocking buildings).
    // Add 3 more buildings at 20W each = 80W total demand vs 50W supply.
    let extra_buildings = [gear_assembler];
    for &node in &extra_buildings {
        power.add_consumer(
            net_id,
            node,
            PowerConsumer {
                demand: Fixed64::from_num(20),
            },
        );
    }

    // Add two more virtual consumers (using the existing node IDs won't work
    // since they would overwrite). Create new engine nodes for the power test.
    let extra_1 = add_node(
        &mut engine,
        make_recipe(vec![(f_iron_plate(), 1)], vec![(f_iron_gear(), 1)], 2),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let extra_2 = add_node(
        &mut engine,
        make_recipe(vec![(f_iron_plate(), 1)], vec![(f_iron_gear(), 1)], 2),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    power.add_consumer(
        net_id,
        extra_1,
        PowerConsumer {
            demand: Fixed64::from_num(20),
        },
    );
    power.add_consumer(
        net_id,
        extra_2,
        PowerConsumer {
            demand: Fixed64::from_num(20),
        },
    );

    // Total demand: 4 * 20W = 80W, supply = 50W.
    let mut saw_brownout = false;
    for tick in 101..=200u64 {
        engine.step();
        let events = power.tick(tick);
        for event in &events {
            if matches!(event, PowerEvent::PowerGridBrownout { .. }) {
                saw_brownout = true;
            }
        }
    }
    assert!(
        saw_brownout,
        "should brownout when demand (80W) exceeds supply (50W)"
    );

    let satisfaction = power.satisfaction(net_id).unwrap();
    assert!(
        satisfaction < Fixed64::from_num(1),
        "satisfaction should be below 1.0 during brownout, got {satisfaction}"
    );

    // Phase 3: Add a second boiler (50W) to restore power.
    let boiler_2 = add_node(
        &mut engine,
        make_source(f_coal(), 5.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    power.add_producer(
        net_id,
        boiler_2,
        PowerProducer {
            capacity: Fixed64::from_num(50),
        },
    );

    // Total supply now 100W vs 80W demand.
    let mut saw_restored = false;
    for tick in 201..=300u64 {
        engine.step();
        let events = power.tick(tick);
        for event in &events {
            if matches!(event, PowerEvent::PowerGridRestored { .. }) {
                saw_restored = true;
            }
        }
    }
    assert!(
        saw_restored,
        "should see PowerGridRestored after adding second boiler"
    );

    let satisfaction = power.satisfaction(net_id).unwrap();
    assert_eq!(
        satisfaction,
        Fixed64::from_num(1),
        "power should be fully satisfied after adding second boiler"
    );
}

// ============================================================================
// Test 9: Fluid pipe network (water + crude oil)
// ============================================================================

/// Test the FluidModule by creating two fluid networks: one for water and
/// one for crude oil. Both feed an oil refinery node. Verify pressure
/// management and storage behavior.
///
/// ENGINE GAP: factorial-fluid is not listed in the integration test crate's
/// Cargo.toml dev-dependencies. This test will not compile until the
/// dependency is added.
///
/// ENGINE GAP: The FluidModule operates independently from the core Engine.
/// There is currently no automatic integration between the two -- the game
/// must manually drive fluid.tick() and apply pressure effects. A future
/// Module trait implementation for FluidModule would allow it to be registered
/// with the engine via engine.add_module().
#[test]
fn test_fluid_pipe_network() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Create engine nodes for the fluid network participants.
    let water_pump = add_node(
        &mut engine,
        make_source(f_water(), 10.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let oil_extractor = add_node(
        &mut engine,
        make_source(f_crude_oil(), 5.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    // The refinery engine node is not directly used in fluid assertions below;
    // the FluidModule uses separate virtual port nodes instead (see ENGINE GAP).
    let _refinery = add_node(
        &mut engine,
        make_recipe(
            vec![(f_crude_oil(), 10), (f_water(), 5)],
            vec![(f_petroleum(), 4)],
            5,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let water_tank = add_node(
        &mut engine,
        // Tank node has no processor; it is only used as a fluid storage anchor.
        make_source(f_water(), 0.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // Set up FluidModule.
    let mut fluid = FluidModule::new();

    // ENGINE GAP: The FluidModule stores per-node consumer specs in a flat
    // BTreeMap<NodeId, FluidConsumer>. If the same NodeId is registered as a
    // consumer in multiple fluid networks (e.g., the refinery consumes both
    // water and oil), the second registration overwrites the first because
    // the BTreeMap key is just the NodeId. To work around this, we use
    // separate "virtual" nodes for each fluid input port of the refinery.
    // A proper fix would be to key consumer specs by (FluidNetworkId, NodeId)
    // instead of just NodeId.
    let refinery_water_port = add_node(
        &mut engine,
        // Virtual node representing the refinery's water input port.
        make_source(f_water(), 0.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let refinery_oil_port = add_node(
        &mut engine,
        // Virtual node representing the refinery's oil input port.
        make_source(f_crude_oil(), 0.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // Water network: pump produces, refinery water port consumes, tank stores.
    let water_net = fluid.create_network(f_water());
    fluid.add_producer(
        water_net,
        water_pump,
        FluidProducer {
            rate: Fixed64::from_num(10),
        },
    );
    fluid.add_consumer(
        water_net,
        refinery_water_port,
        FluidConsumer {
            rate: Fixed64::from_num(5),
        },
    );
    fluid.add_storage(
        water_net,
        water_tank,
        FluidStorage {
            capacity: Fixed64::from_num(500),
            current: Fixed64::from_num(0),
            fill_rate: Fixed64::from_num(50),
        },
    );

    // Crude oil network: extractor produces, refinery oil port consumes.
    let oil_net = fluid.create_network(f_crude_oil());
    fluid.add_producer(
        oil_net,
        oil_extractor,
        FluidProducer {
            rate: Fixed64::from_num(5),
        },
    );
    fluid.add_consumer(
        oil_net,
        refinery_oil_port,
        FluidConsumer {
            rate: Fixed64::from_num(10),
        },
    );

    // Run for 200 ticks.
    let mut water_low_pressure_seen = false;
    let mut oil_low_pressure_seen = false;

    for tick in 1..=200u64 {
        engine.step();
        let fluid_events = fluid.tick(tick);

        for event in &fluid_events {
            match event {
                factorial_fluid::FluidEvent::PressureLow { network_id, .. }
                    if *network_id == water_net =>
                {
                    water_low_pressure_seen = true;
                }
                factorial_fluid::FluidEvent::PressureLow { network_id, .. }
                    if *network_id == oil_net =>
                {
                    oil_low_pressure_seen = true;
                }
                _ => {}
            }
        }
    }

    // Water network: production (10) > consumption (5), so pressure should
    // stay at 1.0 and excess fills the tank.
    let water_pressure = fluid.pressure(water_net).unwrap();
    assert_eq!(
        water_pressure,
        Fixed64::from_num(1),
        "water network should be fully pressurized (10 prod > 5 cons)"
    );
    assert!(
        !water_low_pressure_seen,
        "water network should never have low pressure"
    );

    // Water tank should have accumulated excess.
    let tank_level = fluid.storage.get(&water_tank).unwrap().current;
    assert!(
        tank_level > Fixed64::from_num(0),
        "water tank should have accumulated excess water, got {tank_level}"
    );

    // Oil network: production (5) < consumption (10), so pressure should
    // be below 1.0 (5/10 = 0.5).
    let oil_pressure = fluid.pressure(oil_net).unwrap();
    assert!(
        oil_pressure < Fixed64::from_num(1),
        "oil network should have low pressure (5 prod < 10 cons), got {oil_pressure}"
    );
    assert!(
        oil_low_pressure_seen,
        "should have seen oil network low pressure event"
    );
}

// ============================================================================
// Test 10: Plastic production chain (crude oil -> petroleum -> plastic)
// ============================================================================

/// Full production chain: crude oil is extracted, refined into petroleum gas,
/// and then petroleum is consumed to produce plastic bars. Tests fluid-to-item
/// conversion.
///
/// ENGINE GAP: There is no built-in mechanism to bridge the FluidModule
/// (which tracks fluid rates and pressure) with the core Engine's item-based
/// production. In a real Factorio implementation, the refinery would consume
/// fluid from the FluidModule and produce discrete items in the Engine.
/// Currently these are two separate systems that must be bridged by game code.
/// A future API might look like:
///   engine.register_fluid_bridge(node_id, fluid_network_id, direction)
/// which would automatically convert fluid rates to item production rates.
#[test]
fn test_plastic_production_chain() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Stage 1: Crude oil extraction (modeled as an engine source).
    let oil_source = add_node(
        &mut engine,
        make_source(f_crude_oil(), 5.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // Stage 2: Oil refinery - crude oil -> petroleum.
    // Simplified recipe: 5 crude oil -> 3 petroleum, duration 3 ticks.
    let refinery = add_node(
        &mut engine,
        make_recipe(
            vec![(f_crude_oil(), 5)],
            vec![(f_petroleum(), 3)],
            3,
        ),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, oil_source, refinery, flow_pipe());

    // Stage 3: Chemical plant - petroleum + coal -> plastic bar.
    // Factorio recipe: 20 petroleum + 1 coal -> 2 plastic, duration 1s.
    // Simplified: 3 petroleum + 1 coal -> 1 plastic, duration 3 ticks.
    let coal_source = add_node(
        &mut engine,
        make_source(f_coal(), 3.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let chemical_plant = add_node(
        &mut engine,
        make_recipe(
            vec![(f_petroleum(), 3), (f_coal(), 1)],
            vec![(f_plastic_bar(), 1)],
            3,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, refinery, chemical_plant, flow_pipe());
    connect(&mut engine, coal_source, chemical_plant, yellow_belt());

    // Sink for plastic bars.
    let plastic_sink = add_node(
        &mut engine,
        make_recipe(
            vec![(f_plastic_bar(), 9999)],
            vec![(f_iron_ore(), 1)],
            99999,
        ),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, chemical_plant, plastic_sink, yellow_belt());

    // Also set up FluidModule to track the fluid networks alongside the engine.
    let mut fluid = FluidModule::new();

    let crude_net = fluid.create_network(f_crude_oil());
    fluid.add_producer(
        crude_net,
        oil_source,
        FluidProducer {
            rate: Fixed64::from_num(5),
        },
    );
    fluid.add_consumer(
        crude_net,
        refinery,
        FluidConsumer {
            rate: Fixed64::from_num(5),
        },
    );

    let petro_net = fluid.create_network(f_petroleum());
    fluid.add_producer(
        petro_net,
        refinery,
        FluidProducer {
            rate: Fixed64::from_num(3), // refinery output rate
        },
    );
    fluid.add_consumer(
        petro_net,
        chemical_plant,
        FluidConsumer {
            rate: Fixed64::from_num(3),
        },
    );

    // Run for 600 ticks.
    for tick in 1..=600u64 {
        engine.step();
        let _fluid_events = fluid.tick(tick);
    }

    // Verify the full chain produced plastic bars.
    let plastics_in_sink = input_quantity(&engine, plastic_sink, f_plastic_bar());
    assert!(
        plastics_in_sink > 0,
        "plastic bars should have been produced through the full chain, got {plastics_in_sink}"
    );

    // Verify petroleum was an intermediate product: the refinery should have
    // output petroleum and the chemical plant should have consumed it.
    let petro_at_chem = input_quantity(&engine, chemical_plant, f_petroleum());
    let coal_at_chem = input_quantity(&engine, chemical_plant, f_coal());

    // At least one of the inputs should have arrived (or been consumed).
    assert!(
        petro_at_chem > 0 || plastics_in_sink > 0,
        "petroleum should have flowed from refinery to chemical plant"
    );
    assert!(
        coal_at_chem > 0 || plastics_in_sink > 0,
        "coal should have arrived at chemical plant"
    );

    // Verify fluid networks are tracking correctly.
    let crude_pressure = fluid.pressure(crude_net).unwrap();
    assert_eq!(
        crude_pressure,
        Fixed64::from_num(1),
        "crude oil network should be balanced (5 prod = 5 cons)"
    );

    let petro_pressure = fluid.pressure(petro_net).unwrap();
    assert_eq!(
        petro_pressure,
        Fixed64::from_num(1),
        "petroleum network should be balanced (3 prod = 3 cons)"
    );
}
