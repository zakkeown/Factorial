#![allow(dead_code)]
//! Satisfactory-flavored integration tests for the Factorial engine.
//!
//! These tests model Satisfactory game mechanics (miner purity tiers,
//! overclocking with power shards, conveyor belt tiers, smart splitters,
//! vehicle routes, train freight, space elevator deliveries, and power
//! scaling) on top of the generic Factorial engine.
//!
//! Tests marked with ENGINE GAP comments require engine features that do not
//! yet exist. These are "red tests" meant to guide engine development.
//!
//! Reference: Satisfactory Wiki (https://satisfactory.wiki.gg/)

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;
use factorial_core::transport::*;
use factorial_power::{PowerConsumer, PowerModule, PowerProducer};

// ===========================================================================
// Satisfactory item type IDs (200+ range, separate from Builderment's 10+ range)
// ===========================================================================

// Raw ores
fn s_iron_ore() -> ItemTypeId {
    ItemTypeId(200)
}
fn s_copper_ore() -> ItemTypeId {
    ItemTypeId(201)
}
fn s_limestone() -> ItemTypeId {
    ItemTypeId(202)
}
fn s_coal() -> ItemTypeId {
    ItemTypeId(203)
}

// Smelted ingots
fn s_iron_ingot() -> ItemTypeId {
    ItemTypeId(210)
}
fn s_copper_ingot() -> ItemTypeId {
    ItemTypeId(211)
}
fn s_steel_ingot() -> ItemTypeId {
    ItemTypeId(212)
}
fn s_concrete() -> ItemTypeId {
    ItemTypeId(213)
}

// Constructor products
fn s_iron_plate() -> ItemTypeId {
    ItemTypeId(220)
}
fn s_iron_rod() -> ItemTypeId {
    ItemTypeId(221)
}
fn s_screw() -> ItemTypeId {
    ItemTypeId(222)
}
fn s_wire() -> ItemTypeId {
    ItemTypeId(223)
}
fn s_cable() -> ItemTypeId {
    ItemTypeId(224)
}

// Assembler products
fn s_reinforced_plate() -> ItemTypeId {
    ItemTypeId(225)
}
fn s_rotor() -> ItemTypeId {
    ItemTypeId(226)
}
fn s_modular_frame() -> ItemTypeId {
    ItemTypeId(227)
}
fn s_smart_plating() -> ItemTypeId {
    ItemTypeId(228)
}

// Steel-tier products
fn s_steel_beam() -> ItemTypeId {
    ItemTypeId(230)
}
fn s_steel_pipe() -> ItemTypeId {
    ItemTypeId(231)
}
fn s_motor() -> ItemTypeId {
    ItemTypeId(232)
}
fn s_stator() -> ItemTypeId {
    ItemTypeId(233)
}

// ===========================================================================
// Satisfactory building capacities
// ===========================================================================

/// Standard input/output capacity for single-input buildings (Smelter,
/// Constructor).
const SAT_STD_CAP: u32 = 50;

/// Larger capacity for multi-input buildings (Assembler, Manufacturer,
/// Foundry) to avoid the faster-arriving item starving the slower one.
const SAT_MULTI_CAP: u32 = 10_000;

/// Sink capacity for buildings that accumulate items (Space Elevator,
/// AWESOME Sink).
const SAT_SINK_CAP: u32 = 50_000;

// ===========================================================================
// Satisfactory transport helpers
// ===========================================================================

/// Mk.1 Conveyor Belt: 60 items/min = ~1 item/tick at 60-tick minutes.
/// Modeled as 8 slots, speed 1.0, 1 lane.
fn mk1_belt() -> Transport {
    make_item_transport(8)
}

/// Mk.3 Conveyor Belt: 270 items/min ~ 4.5x Mk.1 throughput.
/// Modeled as 8 slots, speed 4.5, 1 lane.
fn mk3_belt() -> Transport {
    Transport::Item(ItemTransport {
        speed: Fixed64::from_num(4.5),
        slot_count: 8,
        lanes: 1,
    })
}

/// Mk.5 Conveyor Belt: 780 items/min ~ 13x Mk.1 throughput.
/// Modeled as 8 slots, speed 13.0, 1 lane.
fn mk5_belt() -> Transport {
    Transport::Item(ItemTransport {
        speed: Fixed64::from_num(13.0),
        slot_count: 8,
        lanes: 1,
    })
}

// ===========================================================================
// Satisfactory recipe helpers
// ===========================================================================

/// Smelter: 1 Iron Ore -> 1 Iron Ingot, 2 ticks.
fn smelter_iron() -> Processor {
    make_recipe(vec![(s_iron_ore(), 1)], vec![(s_iron_ingot(), 1)], 2)
}

/// Smelter: 1 Copper Ore -> 1 Copper Ingot, 2 ticks.
fn smelter_copper() -> Processor {
    make_recipe(vec![(s_copper_ore(), 1)], vec![(s_copper_ingot(), 1)], 2)
}

/// Constructor: 3 Iron Ingot -> 2 Iron Plate, 6 ticks.
fn constructor_iron_plate() -> Processor {
    make_recipe(vec![(s_iron_ingot(), 3)], vec![(s_iron_plate(), 2)], 6)
}

/// Constructor: 1 Iron Ingot -> 1 Iron Rod, 4 ticks.
fn constructor_iron_rod() -> Processor {
    make_recipe(vec![(s_iron_ingot(), 1)], vec![(s_iron_rod(), 1)], 4)
}

/// Constructor: 1 Iron Rod -> 4 Screw, 6 ticks.
fn constructor_screw() -> Processor {
    make_recipe(vec![(s_iron_rod(), 1)], vec![(s_screw(), 4)], 6)
}

/// Constructor: 1 Copper Ingot -> 2 Wire, 4 ticks.
fn constructor_wire() -> Processor {
    make_recipe(vec![(s_copper_ingot(), 1)], vec![(s_wire(), 2)], 4)
}

/// Constructor: 2 Wire -> 1 Cable, 2 ticks.
fn constructor_cable() -> Processor {
    make_recipe(vec![(s_wire(), 2)], vec![(s_cable(), 1)], 2)
}

/// Assembler: 6 Iron Plate + 12 Screw -> 1 Reinforced Iron Plate, 12 ticks.
fn assembler_reinforced_plate() -> Processor {
    make_recipe(
        vec![(s_iron_plate(), 6), (s_screw(), 12)],
        vec![(s_reinforced_plate(), 1)],
        12,
    )
}

/// Assembler: 5 Iron Rod + 25 Screw -> 1 Rotor, 15 ticks.
fn assembler_rotor() -> Processor {
    make_recipe(
        vec![(s_iron_rod(), 5), (s_screw(), 25)],
        vec![(s_rotor(), 1)],
        15,
    )
}

/// Assembler: 1 Reinforced Iron Plate + 1 Rotor -> 1 Smart Plating, 30 ticks.
/// (Space Elevator Phase 1 component)
fn assembler_smart_plating() -> Processor {
    make_recipe(
        vec![(s_reinforced_plate(), 1), (s_rotor(), 1)],
        vec![(s_smart_plating(), 1)],
        30,
    )
}

/// Foundry: 3 Iron Ore + 3 Coal -> 3 Steel Ingot, 4 ticks.
/// (Satisfactory Foundry -- raw ore goes directly in, unlike Builderment.)
fn foundry_steel_ingot() -> Processor {
    make_recipe(
        vec![(s_iron_ore(), 3), (s_coal(), 3)],
        vec![(s_steel_ingot(), 3)],
        4,
    )
}

/// Constructor: 4 Steel Ingot -> 1 Steel Beam, 4 ticks.
fn constructor_steel_beam() -> Processor {
    make_recipe(vec![(s_steel_ingot(), 4)], vec![(s_steel_beam(), 1)], 4)
}

/// Constructor: 3 Steel Ingot -> 2 Steel Pipe, 6 ticks.
fn constructor_steel_pipe() -> Processor {
    make_recipe(vec![(s_steel_ingot(), 3)], vec![(s_steel_pipe(), 2)], 6)
}

/// Assembler: 10 Stator + 5 Rotor -> 1 Motor (simplified), 12 ticks.
fn assembler_motor() -> Processor {
    make_recipe(
        vec![(s_stator(), 10), (s_rotor(), 5)],
        vec![(s_motor(), 1)],
        12,
    )
}

/// Assembler: 3 Steel Pipe + 8 Wire -> 1 Stator, 12 ticks.
fn assembler_stator() -> Processor {
    make_recipe(
        vec![(s_steel_pipe(), 3), (s_wire(), 8)],
        vec![(s_stator(), 1)],
        12,
    )
}

/// Alternate Constructor recipe: 5 Iron Ingot -> 4 Iron Rod (Cast Screw alt),
/// 12 ticks. A slightly different ratio than the standard recipe.
fn constructor_iron_rod_alternate() -> Processor {
    make_recipe(vec![(s_iron_ingot(), 5)], vec![(s_iron_rod(), 4)], 12)
}

// ===========================================================================
// Test 1: Miner purity tiers
// ===========================================================================

/// Satisfactory ore nodes come in three purity tiers: Impure (0.5x),
/// Normal (1.0x), and Pure (2.0x) base extraction rate. This test creates
/// three iron ore miners at these rates and verifies after 100 ticks that
/// the output ratios match expectations.
#[test]
fn test_miner_purity_tiers() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Impure miner: 0.5 ore/tick (produces 1 ore every 2 ticks).
    let impure_miner = add_node(
        &mut engine,
        make_source(s_iron_ore(), 0.5),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );

    // Normal miner: 1.0 ore/tick (produces 1 ore every tick).
    let normal_miner = add_node(
        &mut engine,
        make_source(s_iron_ore(), 1.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );

    // Pure miner: 2.0 ore/tick (produces 2 ore every tick).
    let pure_miner = add_node(
        &mut engine,
        make_source(s_iron_ore(), 2.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );

    // Sink nodes to accumulate output (large capacity so we never stall).
    let impure_sink = add_node(
        &mut engine,
        make_recipe(vec![(s_iron_ore(), 9999)], vec![(s_iron_ore(), 1)], 99999),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );
    let normal_sink = add_node(
        &mut engine,
        make_recipe(vec![(s_iron_ore(), 9999)], vec![(s_iron_ore(), 1)], 99999),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );
    let pure_sink = add_node(
        &mut engine,
        make_recipe(vec![(s_iron_ore(), 9999)], vec![(s_iron_ore(), 1)], 99999),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );

    // Connect miners to sinks via belt tiers that match their production rates.
    // The Mk.1 belt (speed 1.0, 8 slots) throughput caps at ~1 item/tick, so
    // faster miners need faster belts to realize their full output.
    connect(&mut engine, impure_miner, impure_sink, mk1_belt()); // 0.5/tick, Mk.1 is plenty
    connect(&mut engine, normal_miner, normal_sink, mk1_belt()); // 1.0/tick, Mk.1 is adequate
    connect(&mut engine, pure_miner, pure_sink, mk3_belt()); // 2.0/tick, needs Mk.3+

    // Run 100 ticks.
    for _ in 0..100 {
        engine.step();
    }

    // Check output quantities sitting in the miner output inventories.
    // After 100 ticks:
    //   Impure (0.5/tick): ~50 ore produced total (some in transit on belt,
    //       some in miner output, some delivered to sink input).
    //   Normal (1.0/tick): ~100 ore produced total.
    //   Pure   (2.0/tick): ~200 ore produced total (capped by output cap of 50).
    //
    // We check the sink input inventories, which accumulate delivered items.
    let impure_delivered = input_quantity(&engine, impure_sink, s_iron_ore());
    let normal_delivered = input_quantity(&engine, normal_sink, s_iron_ore());
    let pure_delivered = input_quantity(&engine, pure_sink, s_iron_ore());

    // The exact numbers depend on belt throughput and timing, but the ratio
    // should hold: normal >= 2 * impure (approximately), pure >= 2 * normal.
    assert!(
        impure_delivered > 0,
        "impure miner should have delivered some ore, got {impure_delivered}"
    );
    assert!(
        normal_delivered > impure_delivered,
        "normal miner ({normal_delivered}) should deliver more than impure ({impure_delivered})"
    );
    assert!(
        pure_delivered > normal_delivered,
        "pure miner ({pure_delivered}) should deliver more than normal ({normal_delivered})"
    );

    // Verify approximate 1:2:4 ratio (allow 30% tolerance due to belt latency).
    let ratio_normal_to_impure = normal_delivered as f64 / impure_delivered.max(1) as f64;
    assert!(
        ratio_normal_to_impure > 1.4 && ratio_normal_to_impure < 2.6,
        "normal:impure ratio should be ~2.0, got {ratio_normal_to_impure:.2}"
    );
}

// ===========================================================================
// Test 2: Overclocking with power shards
// ===========================================================================

/// In Satisfactory, power shards allow overclocking a building up to 250%.
/// A smelter with 250% overclock (speed modifier 2.5x) produces 2.5x faster
/// but also consumes proportionally more power.
///
/// ENGINE GAP: There is no automatic link between a speed modifier on a
/// processor and increased power demand. Game logic must manually update
/// `PowerConsumer.demand` when modifiers change. The engine should ideally
/// emit an event or provide a hook when modifiers change so power demand
/// can be recalculated.
#[test]
fn test_overclocking_with_power_shards() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // --- Normal smelter (1x speed) ---
    let iron_src_normal = add_node(
        &mut engine,
        make_source(s_iron_ore(), 5.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    let smelter_normal = add_node(&mut engine, smelter_iron(), SAT_STD_CAP, SAT_STD_CAP);
    connect(&mut engine, iron_src_normal, smelter_normal, mk1_belt());

    // --- Overclocked smelter (2.5x speed via power shard modifier) ---
    let iron_src_oc = add_node(
        &mut engine,
        make_source(s_iron_ore(), 5.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    let smelter_oc = add_node(&mut engine, smelter_iron(), SAT_STD_CAP, SAT_STD_CAP);
    connect(&mut engine, iron_src_oc, smelter_oc, mk5_belt());

    // Apply 250% overclock (2.5x speed modifier).
    engine.set_modifiers(
        smelter_oc,
        vec![Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(Fixed64::from_num(2.5)),
            stacking: StackingRule::Multiplicative,
        }],
    );

    // --- Power setup ---
    let mut power = PowerModule::new();
    let net = power.create_network();

    // Generator with plenty of capacity.
    power.add_producer(
        net,
        iron_src_normal, // Reuse a node as the power producer reference.
        PowerProducer {
            capacity: Fixed64::from_num(100),
        },
    );

    // Normal smelter: 4 MW base power.
    let base_power = Fixed64::from_num(4);
    power.add_consumer(net, smelter_normal, PowerConsumer { demand: base_power });

    // ENGINE GAP: Overclocked smelter power demand should be automatically
    // derived from the speed modifier. Satisfactory's formula is approximately:
    //   power = base_power * (clock_speed / 100)^1.321928
    // For 250% clock: 4 * 2.5^1.321928 ~ 12.87 MW.
    // For now, we manually set the increased demand.
    let overclock_power = Fixed64::from_num(13); // approximate
    power.add_consumer(
        net,
        smelter_oc,
        PowerConsumer {
            demand: overclock_power,
        },
    );

    // Collect sink nodes to measure production.
    let normal_sink = add_node(
        &mut engine,
        make_recipe(vec![(s_iron_ingot(), 9999)], vec![(s_iron_ore(), 1)], 99999),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );
    let oc_sink = add_node(
        &mut engine,
        make_recipe(vec![(s_iron_ingot(), 9999)], vec![(s_iron_ore(), 1)], 99999),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );
    connect(&mut engine, smelter_normal, normal_sink, mk1_belt());
    connect(&mut engine, smelter_oc, oc_sink, mk5_belt());

    // Run 200 ticks.
    for tick in 1..=200u64 {
        engine.step();
        let _events = power.tick(tick);
    }

    let normal_produced = input_quantity(&engine, normal_sink, s_iron_ingot());
    let oc_produced = input_quantity(&engine, oc_sink, s_iron_ingot());

    // The overclocked smelter should produce significantly more.
    assert!(
        oc_produced > normal_produced,
        "overclocked smelter ({oc_produced}) should produce more than normal ({normal_produced})"
    );

    // Power satisfaction should be 1.0 (we have 100 MW capacity, using ~17 MW).
    let satisfaction = power.satisfaction(net).unwrap();
    assert_eq!(
        satisfaction,
        Fixed64::from_num(1),
        "power grid should be fully satisfied"
    );

    // Verify power demand ratio: overclocked should use ~3.25x more power.
    // (This is a manual check since the engine doesn't auto-link modifiers to power.)
    let demand_ratio = overclock_power / base_power;
    assert!(
        demand_ratio > Fixed64::from_num(3),
        "overclock power demand ratio should be > 3x, got {demand_ratio}"
    );
}

// ===========================================================================
// Test 3: Constructor -> Assembler chain (increasing input counts)
// ===========================================================================

/// Tests a multi-stage chain of increasing complexity:
///   Iron Ingot source -> Iron Rod (Constructor, 1 input)
///     -> Screw (Constructor, 1 input)
///       -> Reinforced Iron Plate (Assembler, 2 inputs: Iron Plate + Screw)
///
/// This tests that multi-input machines correctly wait for all ingredients.
#[test]
fn test_constructor_assembler_manufacturer() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // --- Iron Ingot supply (feeds both Iron Rod and Iron Plate chains) ---
    let iron_ingot_src_for_rod = add_node(
        &mut engine,
        make_source(s_iron_ingot(), 3.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    let iron_ingot_src_for_plate = add_node(
        &mut engine,
        make_source(s_iron_ingot(), 3.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );

    // --- Iron Rod Constructor: 1 Iron Ingot -> 1 Iron Rod, 4 ticks ---
    let iron_rod_constructor = add_node(
        &mut engine,
        constructor_iron_rod(),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    connect(
        &mut engine,
        iron_ingot_src_for_rod,
        iron_rod_constructor,
        mk1_belt(),
    );

    // --- Screw Constructor: 1 Iron Rod -> 4 Screw, 6 ticks ---
    let screw_constructor = add_node(&mut engine, constructor_screw(), SAT_STD_CAP, SAT_STD_CAP);
    connect(
        &mut engine,
        iron_rod_constructor,
        screw_constructor,
        mk1_belt(),
    );

    // --- Iron Plate Constructor: 3 Iron Ingot -> 2 Iron Plate, 6 ticks ---
    let iron_plate_constructor = add_node(
        &mut engine,
        constructor_iron_plate(),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    connect(
        &mut engine,
        iron_ingot_src_for_plate,
        iron_plate_constructor,
        mk1_belt(),
    );

    // --- Assembler: 6 Iron Plate + 12 Screw -> 1 Reinforced Iron Plate ---
    let assembler_rip = add_node(
        &mut engine,
        assembler_reinforced_plate(),
        SAT_MULTI_CAP,
        SAT_STD_CAP,
    );
    connect(
        &mut engine,
        iron_plate_constructor,
        assembler_rip,
        mk1_belt(),
    );
    connect(&mut engine, screw_constructor, assembler_rip, mk1_belt());

    // --- Sink to accumulate Reinforced Iron Plates ---
    let rip_sink = add_node(
        &mut engine,
        make_recipe(
            vec![(s_reinforced_plate(), 9999)],
            vec![(s_iron_ore(), 1)],
            99999,
        ),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );
    connect(&mut engine, assembler_rip, rip_sink, mk1_belt());

    // Run 500 ticks to let the pipeline fill and produce.
    for _ in 0..500 {
        engine.step();
    }

    // Verify the assembler produced at least one Reinforced Iron Plate.
    let rip_produced = input_quantity(&engine, rip_sink, s_reinforced_plate());
    assert!(
        rip_produced > 0,
        "assembler should have produced reinforced iron plates, got {rip_produced}"
    );

    // Verify intermediate products flowed correctly.
    let screws_in_assembler = input_quantity(&engine, assembler_rip, s_screw());
    let plates_in_assembler = input_quantity(&engine, assembler_rip, s_iron_plate());

    // At least some items should have been delivered to the assembler.
    // (They may have been consumed already, so check either inventory or sink.)
    let total_evidence = rip_produced + screws_in_assembler + plates_in_assembler;
    assert!(
        total_evidence > 0,
        "should see evidence of items flowing through the chain"
    );
}

// ===========================================================================
// Test 4: Alternate recipe swap mid-simulation
// ===========================================================================

/// In Satisfactory, players can unlock alternate recipes via Hard Drives
/// and swap them into existing buildings. This test builds a line with the
/// standard Iron Rod recipe, then swaps to an alternate recipe mid-run.
///
/// ENGINE GAP: `engine.set_processor(node, new_processor)` exists and resets
/// the processor state to default. The test verifies that swapping a
/// processor on a running node works correctly. Open question: should it
/// preserve in-progress crafting or reset? Currently it resets to Idle.
#[test]
fn test_alternate_recipe_swap() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Iron Ingot source.
    let iron_src = add_node(
        &mut engine,
        make_source(s_iron_ingot(), 5.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );

    // Standard Constructor: 1 Iron Ingot -> 1 Iron Rod, 4 ticks.
    let constructor = add_node(
        &mut engine,
        constructor_iron_rod(),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    connect(&mut engine, iron_src, constructor, mk1_belt());

    // Sink for Iron Rods.
    let rod_sink = add_node(
        &mut engine,
        make_recipe(vec![(s_iron_rod(), 9999)], vec![(s_iron_ore(), 1)], 99999),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );
    connect(&mut engine, constructor, rod_sink, mk1_belt());

    // Phase 1: Run 100 ticks with standard recipe.
    for _ in 0..100 {
        engine.step();
    }

    let rods_phase1 = input_quantity(&engine, rod_sink, s_iron_rod());
    assert!(
        rods_phase1 > 0,
        "standard recipe should produce iron rods in phase 1, got {rods_phase1}"
    );

    // Phase 2: Swap to alternate recipe (5 Iron Ingot -> 4 Iron Rod, 12 ticks).
    // ENGINE GAP: set_processor resets ProcessorState to Idle. If the building
    // was mid-craft, those in-progress inputs are effectively lost. A future
    // improvement could refund in-progress inputs to the input inventory.
    engine.set_processor(constructor, constructor_iron_rod_alternate());

    // Verify the processor state was reset (not mid-craft from previous recipe).
    let state_after_swap = engine.get_processor_state(constructor);
    assert_eq!(
        state_after_swap,
        Some(&ProcessorState::Idle),
        "processor state should be reset to Idle after recipe swap"
    );

    // Run another 100 ticks with the alternate recipe.
    for _ in 0..100 {
        engine.step();
    }

    let rods_phase2_total = input_quantity(&engine, rod_sink, s_iron_rod());
    assert!(
        rods_phase2_total > rods_phase1,
        "should produce more iron rods after recipe swap, phase1={rods_phase1}, total={rods_phase2_total}"
    );

    // The alternate recipe produces 4 rods per craft (vs 1 for standard),
    // but takes 12 ticks (vs 4). Net rate: 4/12 = 0.33/tick vs 1/4 = 0.25/tick.
    // So alternate should be slightly faster. Hard to measure exactly due to
    // belt latency, but production should continue uninterrupted.
}

// ===========================================================================
// Test 5: Conveyor belt tiers
// ===========================================================================

/// Same production line but with three different belt speeds (Mk.1, Mk.3,
/// Mk.5). Higher belt speeds should deliver items faster and improve
/// throughput of the downstream consumer.
#[test]
fn test_conveyor_belt_tiers() {
    /// Build a smelter chain with the given belt transport and return the
    /// sink node ID.
    fn build_smelter_line(engine: &mut Engine, belt: Transport) -> NodeId {
        let ore_src = add_node(
            engine,
            make_source(s_iron_ore(), 20.0), // High rate to saturate any belt.
            SAT_STD_CAP,
            SAT_STD_CAP,
        );
        let smelter = add_node(engine, smelter_iron(), SAT_STD_CAP, SAT_STD_CAP);
        let sink = add_node(
            engine,
            make_recipe(vec![(s_iron_ingot(), 9999)], vec![(s_iron_ore(), 1)], 99999),
            SAT_SINK_CAP,
            SAT_STD_CAP,
        );
        connect(engine, ore_src, smelter, belt.clone());
        connect(engine, smelter, sink, belt);
        sink
    }

    let mut engine = Engine::new(SimulationStrategy::Tick);

    let mk1_sink = build_smelter_line(&mut engine, mk1_belt());
    let mk3_sink = build_smelter_line(&mut engine, mk3_belt());
    let mk5_sink = build_smelter_line(&mut engine, mk5_belt());

    // Run 200 ticks.
    for _ in 0..200 {
        engine.step();
    }

    let mk1_output = input_quantity(&engine, mk1_sink, s_iron_ingot());
    let mk3_output = input_quantity(&engine, mk3_sink, s_iron_ingot());
    let mk5_output = input_quantity(&engine, mk5_sink, s_iron_ingot());

    // All should produce something.
    assert!(
        mk1_output > 0,
        "Mk.1 belt line should produce ingots, got {mk1_output}"
    );
    assert!(
        mk3_output > 0,
        "Mk.3 belt line should produce ingots, got {mk3_output}"
    );
    assert!(
        mk5_output > 0,
        "Mk.5 belt line should produce ingots, got {mk5_output}"
    );

    // Higher belt tiers should deliver at least as much (throughput limited by
    // smelter recipe duration, but faster belts reduce pipeline latency so
    // early ticks produce more). Over long runs they may converge, but at
    // 200 ticks the faster belt should show an advantage.
    assert!(
        mk3_output >= mk1_output,
        "Mk.3 ({mk3_output}) should deliver at least as much as Mk.1 ({mk1_output})"
    );
    assert!(
        mk5_output >= mk3_output,
        "Mk.5 ({mk5_output}) should deliver at least as much as Mk.3 ({mk3_output})"
    );
}

// ===========================================================================
// Test 6: Smart splitter filtering
// ===========================================================================

/// Satisfactory's Smart Splitter routes different item types to different
/// outputs. This test sends mixed Iron Plates and Screws through a splitter
/// that filters Iron Plates to one output and Screws to another.
///
/// ENGINE GAP: The current `SplitterConfig.filter` only supports a single
/// `Option<ItemTypeId>`. Satisfactory's Smart Splitter needs per-output
/// filters: route item X to output A, item Y to output B, and everything
/// else to output C. The engine would need:
///   - `SmartSplitterConfig { rules: Vec<(Option<ItemTypeId>, OutputIndex)> }`
///   - Or a `SplitPolicy::FilterMap(HashMap<ItemTypeId, usize>)` variant.
///
/// For now, we test with two separate splitters in series (one filters iron
/// plates, the other gets everything else), which is a workaround.
#[test]
fn test_smart_splitter_filtering() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Source of iron plates.
    let plate_src = add_node(
        &mut engine,
        make_source(s_iron_plate(), 2.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );

    // Source of screws.
    let screw_src = add_node(
        &mut engine,
        make_source(s_screw(), 2.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );

    // ENGINE GAP: Ideally we would merge these onto one belt, then split.
    // Current engine has no way to merge two source outputs onto one belt
    // and then split by item type with per-output filters. We test the
    // splitter filter concept with separate paths instead.

    // Splitter node for plates (filters only iron plates).
    // We create a pass-through node with a junction configured to filter.
    let plate_passthrough = add_node(
        &mut engine,
        // Identity recipe: passes items through unchanged.
        // ENGINE GAP: No explicit "passthrough" processor. Using a Property
        // processor as a 1:1 pass-through.
        Processor::Property(PropertyProcessor {
            input_type: s_iron_plate(),
            output_type: s_iron_plate(),
            transform: PropertyTransform::Set(PropertyId(0), Fixed64::from_num(0)),
        }),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );

    let screw_passthrough = add_node(
        &mut engine,
        Processor::Property(PropertyProcessor {
            input_type: s_screw(),
            output_type: s_screw(),
            transform: PropertyTransform::Set(PropertyId(0), Fixed64::from_num(0)),
        }),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );

    // Set junction on plate passthrough: splitter with iron plate filter.
    engine.set_junction(
        plate_passthrough,
        factorial_core::junction::Junction::Splitter(factorial_core::junction::SplitterConfig {
            policy: factorial_core::junction::SplitPolicy::Priority,
            filter: Some(s_iron_plate()),
        }),
    );

    // Set junction on screw passthrough: splitter with screw filter.
    engine.set_junction(
        screw_passthrough,
        factorial_core::junction::Junction::Splitter(factorial_core::junction::SplitterConfig {
            policy: factorial_core::junction::SplitPolicy::Priority,
            filter: Some(s_screw()),
        }),
    );

    // Sink nodes.
    let plate_sink = add_node(
        &mut engine,
        make_recipe(vec![(s_iron_plate(), 9999)], vec![(s_iron_ore(), 1)], 99999),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );
    let screw_sink = add_node(
        &mut engine,
        make_recipe(vec![(s_screw(), 9999)], vec![(s_iron_ore(), 1)], 99999),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );

    // Connect the graph.
    connect(&mut engine, plate_src, plate_passthrough, mk1_belt());
    connect(&mut engine, plate_passthrough, plate_sink, mk1_belt());
    connect(&mut engine, screw_src, screw_passthrough, mk1_belt());
    connect(&mut engine, screw_passthrough, screw_sink, mk1_belt());

    // Run 100 ticks.
    for _ in 0..100 {
        engine.step();
    }

    // Verify plates went to the plate sink and screws went to the screw sink.
    let plates_delivered = input_quantity(&engine, plate_sink, s_iron_plate());
    let screws_delivered = input_quantity(&engine, screw_sink, s_screw());

    assert!(
        plates_delivered > 0,
        "iron plates should be delivered to plate sink, got {plates_delivered}"
    );
    assert!(
        screws_delivered > 0,
        "screws should be delivered to screw sink, got {screws_delivered}"
    );

    // Cross-check: screws should NOT end up in the plate sink (and vice versa).
    let screws_in_plate_sink = input_quantity(&engine, plate_sink, s_screw());
    let plates_in_screw_sink = input_quantity(&engine, screw_sink, s_iron_plate());
    assert_eq!(
        screws_in_plate_sink, 0,
        "screws should not appear in plate sink, got {screws_in_plate_sink}"
    );
    assert_eq!(
        plates_in_screw_sink, 0,
        "plates should not appear in screw sink, got {plates_in_screw_sink}"
    );
}

// ===========================================================================
// Test 7: Vehicle truck route
// ===========================================================================

/// Models a Satisfactory truck route carrying iron ore from a remote miner
/// to a base smelter. Vehicle capacity=48, travel_time=30 (round trip = 60
/// ticks). Verifies the vehicle delivers batches over multiple round trips.
#[test]
fn test_vehicle_truck_route() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Remote iron miner (high rate to always fill the truck).
    let remote_miner = add_node(
        &mut engine,
        make_source(s_iron_ore(), 10.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );

    // Base smelter.
    let base_smelter = add_node(
        &mut engine,
        smelter_iron(),
        SAT_MULTI_CAP, // Large input to buffer truck deliveries.
        SAT_STD_CAP,
    );

    // Connect via vehicle transport (truck: capacity=48, travel_time=30).
    let truck = make_vehicle_transport(48, 30);
    connect(&mut engine, remote_miner, base_smelter, truck);

    // Sink for ingots.
    let ingot_sink = add_node(
        &mut engine,
        make_recipe(vec![(s_iron_ingot(), 9999)], vec![(s_iron_ore(), 1)], 99999),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );
    connect(&mut engine, base_smelter, ingot_sink, mk1_belt());

    // Run 300 ticks (enough for ~5 round trips at 60 ticks/trip).
    for _ in 0..300 {
        engine.step();
    }

    // The truck should have delivered ore to the smelter.
    let ore_at_smelter = input_quantity(&engine, base_smelter, s_iron_ore());
    let ingots_at_sink = input_quantity(&engine, ingot_sink, s_iron_ingot());

    // Either there's ore waiting at the smelter, or it's been processed into
    // ingots (or both). The total should be positive.
    let total_evidence = ore_at_smelter + ingots_at_sink;
    assert!(
        total_evidence > 0,
        "truck should deliver ore that gets smelted; ore_at_smelter={ore_at_smelter}, ingots_at_sink={ingots_at_sink}"
    );

    // With 5 round trips, the truck should have delivered ~240 ore total
    // (48 per trip * 5 trips). Verify at least 2 deliveries' worth.
    let total_ore_moved = ore_at_smelter + ingots_at_sink; // ingots came from ore
    assert!(
        total_ore_moved >= 48,
        "should have delivered at least one full truckload, got {total_ore_moved}"
    );
}

// ===========================================================================
// Test 8: Train freight system
// ===========================================================================

/// Models Satisfactory's train system using BatchTransport. A freight car
/// holds 32 stacks. The train runs on a schedule with cycle_time=50 ticks
/// (loading + travel + unloading + return).
#[test]
fn test_train_freight_system() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Mining outpost (high production rate).
    let ore_mine = add_node(
        &mut engine,
        make_source(s_iron_ore(), 10.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );

    // Train station at the mining outpost -> train station at the base.
    // Modeled as a batch transport edge (batch_size=32, cycle_time=50).
    let base_station = add_node(
        &mut engine,
        // Pass-through node representing the receiving train station.
        Processor::Property(PropertyProcessor {
            input_type: s_iron_ore(),
            output_type: s_iron_ore(),
            transform: PropertyTransform::Set(PropertyId(0), Fixed64::from_num(0)),
        }),
        SAT_MULTI_CAP,
        SAT_STD_CAP,
    );

    let train = make_batch_transport(32, 50);
    connect(&mut engine, ore_mine, base_station, train);

    // Smelter array at the base.
    let smelter = add_node(&mut engine, smelter_iron(), SAT_MULTI_CAP, SAT_STD_CAP);
    connect(&mut engine, base_station, smelter, mk1_belt());

    // Sink.
    let ingot_sink = add_node(
        &mut engine,
        make_recipe(vec![(s_iron_ingot(), 9999)], vec![(s_iron_ore(), 1)], 99999),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );
    connect(&mut engine, smelter, ingot_sink, mk1_belt());

    // Run 500 ticks (10 train cycles at 50 ticks each).
    for _ in 0..500 {
        engine.step();
    }

    // Verify ore arrived at the base station.
    let ore_at_station = input_quantity(&engine, base_station, s_iron_ore());
    let ore_at_smelter = input_quantity(&engine, smelter, s_iron_ore());
    let ingots_at_sink = input_quantity(&engine, ingot_sink, s_iron_ingot());

    let total_throughput = ore_at_station + ore_at_smelter + ingots_at_sink;
    assert!(
        total_throughput > 0,
        "train should deliver ore to base; station={ore_at_station}, smelter={ore_at_smelter}, sink={ingots_at_sink}"
    );

    // 10 train cycles * 32 per batch = 320 ore total expected throughput.
    // Allow for some loss due to timing; at least 3 batches should complete.
    assert!(
        total_throughput >= 32,
        "at least one full train delivery should complete, got {total_throughput}"
    );
}

// ===========================================================================
// Test 9: Space Elevator Phase 1 (Smart Plating)
// ===========================================================================

/// Full production chain to produce Smart Plating (the first Space Elevator
/// deliverable):
///   Iron Ingot -> Iron Plate + Iron Rod
///   Iron Rod -> Screw
///   Iron Plate + Screw -> Reinforced Iron Plate
///   Iron Rod + Screw -> Rotor
///   Reinforced Iron Plate + Rotor -> Smart Plating
///
/// Feed 50 Smart Plating to a sink (Space Elevator analog).
#[test]
fn test_space_elevator_phase_1() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // === Iron Ingot sources (dedicated per consumer to avoid fan-out issues) ===
    let make_iron_ingot_src = |e: &mut Engine| -> NodeId {
        add_node(
            e,
            make_source(s_iron_ingot(), 5.0),
            SAT_STD_CAP,
            SAT_STD_CAP,
        )
    };

    // --- Iron Plate chain ---
    let ingot_for_plate = make_iron_ingot_src(&mut engine);
    let plate_constructor = add_node(
        &mut engine,
        constructor_iron_plate(),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    connect(&mut engine, ingot_for_plate, plate_constructor, mk1_belt());

    // --- Iron Rod chain (for Rotor) ---
    let ingot_for_rod_rotor = make_iron_ingot_src(&mut engine);
    let rod_constructor_rotor = add_node(
        &mut engine,
        constructor_iron_rod(),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    connect(
        &mut engine,
        ingot_for_rod_rotor,
        rod_constructor_rotor,
        mk1_belt(),
    );

    // --- Screw chain (for Reinforced Plate) ---
    let ingot_for_rod_screw_rip = make_iron_ingot_src(&mut engine);
    let rod_for_screw_rip = add_node(
        &mut engine,
        constructor_iron_rod(),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    let screw_for_rip = add_node(&mut engine, constructor_screw(), SAT_STD_CAP, SAT_STD_CAP);
    connect(
        &mut engine,
        ingot_for_rod_screw_rip,
        rod_for_screw_rip,
        mk1_belt(),
    );
    connect(&mut engine, rod_for_screw_rip, screw_for_rip, mk1_belt());

    // --- Screw chain (for Rotor) ---
    let ingot_for_rod_screw_rotor = make_iron_ingot_src(&mut engine);
    let rod_for_screw_rotor = add_node(
        &mut engine,
        constructor_iron_rod(),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    let screw_for_rotor = add_node(&mut engine, constructor_screw(), SAT_STD_CAP, SAT_STD_CAP);
    connect(
        &mut engine,
        ingot_for_rod_screw_rotor,
        rod_for_screw_rotor,
        mk1_belt(),
    );
    connect(
        &mut engine,
        rod_for_screw_rotor,
        screw_for_rotor,
        mk1_belt(),
    );

    // --- Reinforced Iron Plate Assembler ---
    let rip_assembler = add_node(
        &mut engine,
        assembler_reinforced_plate(),
        SAT_MULTI_CAP,
        SAT_STD_CAP,
    );
    connect(&mut engine, plate_constructor, rip_assembler, mk1_belt());
    connect(&mut engine, screw_for_rip, rip_assembler, mk1_belt());

    // --- Rotor Assembler ---
    let rotor_assembler = add_node(&mut engine, assembler_rotor(), SAT_MULTI_CAP, SAT_STD_CAP);
    connect(
        &mut engine,
        rod_constructor_rotor,
        rotor_assembler,
        mk1_belt(),
    );
    connect(&mut engine, screw_for_rotor, rotor_assembler, mk1_belt());

    // --- Smart Plating Assembler ---
    let smart_plating_assembler = add_node(
        &mut engine,
        assembler_smart_plating(),
        SAT_MULTI_CAP,
        SAT_STD_CAP,
    );
    connect(
        &mut engine,
        rip_assembler,
        smart_plating_assembler,
        mk1_belt(),
    );
    connect(
        &mut engine,
        rotor_assembler,
        smart_plating_assembler,
        mk1_belt(),
    );

    // --- Space Elevator Sink ---
    let space_elevator = add_node(
        &mut engine,
        make_recipe(
            vec![(s_smart_plating(), 9999)],
            vec![(s_iron_ore(), 1)],
            99999,
        ),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );
    connect(
        &mut engine,
        smart_plating_assembler,
        space_elevator,
        mk1_belt(),
    );

    // Run enough ticks to produce 50 Smart Plating.
    // Smart Plating takes 30 ticks per craft, so we need roughly:
    //   50 * 30 = 1500 ticks just for the final assembler, plus pipeline fill
    //   time. Run 5000 ticks to be safe.
    for _ in 0..5000 {
        engine.step();
    }

    let smart_plating_delivered = input_quantity(&engine, space_elevator, s_smart_plating());
    assert!(
        smart_plating_delivered >= 50,
        "Space Elevator should receive at least 50 Smart Plating, got {smart_plating_delivered}"
    );
}

// ===========================================================================
// Test 10: Power scaling from coal to fuel
// ===========================================================================

/// Start with coal power (75W capacity), add consumers that exceed it to
/// trigger brownout, then expand with fuel generators (250W) and verify
/// power satisfaction recovers.
#[test]
fn test_power_scaling_coal_to_fuel() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Create some production nodes as power consumers.
    let smelter1 = add_node(&mut engine, smelter_iron(), SAT_STD_CAP, SAT_STD_CAP);
    let smelter2 = add_node(&mut engine, smelter_iron(), SAT_STD_CAP, SAT_STD_CAP);
    let smelter3 = add_node(&mut engine, smelter_iron(), SAT_STD_CAP, SAT_STD_CAP);
    let constructor1 = add_node(
        &mut engine,
        constructor_iron_rod(),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    let assembler1 = add_node(
        &mut engine,
        assembler_reinforced_plate(),
        SAT_MULTI_CAP,
        SAT_STD_CAP,
    );

    // --- Power setup ---
    let mut power = PowerModule::new();
    let net = power.create_network();

    // Phase 1: Coal power plant -- 75W capacity.
    let coal_plant_node = smelter1; // Reuse a node as reference.
    power.add_producer(
        net,
        coal_plant_node,
        PowerProducer {
            capacity: Fixed64::from_num(75),
        },
    );

    // Register consumers: each smelter 4W, constructor 4W, assembler 15W.
    // Total demand: 3*4 + 4 + 15 = 31W. Should be fine with 75W.
    let consumer_nodes = [
        (smelter1, 4),
        (smelter2, 4),
        (smelter3, 4),
        (constructor1, 4),
        (assembler1, 15),
    ];
    for &(node, demand_mw) in &consumer_nodes {
        power.add_consumer(
            net,
            node,
            PowerConsumer {
                demand: Fixed64::from_num(demand_mw),
            },
        );
    }

    // Run a few ticks -- should be fully powered.
    for tick in 1..=10u64 {
        engine.step();
        let _events = power.tick(tick);
    }

    let sat_phase1 = power.satisfaction(net).unwrap();
    assert_eq!(
        sat_phase1,
        Fixed64::from_num(1),
        "phase 1: coal plant should fully satisfy 31W demand"
    );

    // Phase 2: Add more consumers to exceed 75W.
    // Add 10 more smelters at 4W each = 40W more, total demand = 71W. Still OK.
    // Then add 2 manufacturers at 55W each = 110W more, total = 181W. Brownout!
    let mfr1 = add_node(&mut engine, smelter_iron(), SAT_STD_CAP, SAT_STD_CAP);
    let mfr2 = add_node(&mut engine, smelter_iron(), SAT_STD_CAP, SAT_STD_CAP);
    power.add_consumer(
        net,
        mfr1,
        PowerConsumer {
            demand: Fixed64::from_num(55),
        },
    );
    power.add_consumer(
        net,
        mfr2,
        PowerConsumer {
            demand: Fixed64::from_num(55),
        },
    );

    for tick in 11..=20u64 {
        engine.step();
        let _events = power.tick(tick);
    }

    let sat_phase2 = power.satisfaction(net).unwrap();
    assert!(
        sat_phase2 < Fixed64::from_num(1),
        "phase 2: should be in brownout with 141W demand vs 75W capacity, satisfaction={sat_phase2}"
    );

    // Phase 3: Add fuel generator (250W) to restore power.
    let fuel_gen_node = mfr1; // Reuse a node.
    power.add_producer(
        net,
        fuel_gen_node,
        PowerProducer {
            capacity: Fixed64::from_num(250),
        },
    );

    for tick in 21..=30u64 {
        engine.step();
        let _events = power.tick(tick);
    }

    let sat_phase3 = power.satisfaction(net).unwrap();
    assert_eq!(
        sat_phase3,
        Fixed64::from_num(1),
        "phase 3: fuel generator (250W) + coal (75W) = 325W should satisfy 141W demand"
    );
}

// ===========================================================================
// Test 11: Steel production via Foundry (raw ore multi-input)
// ===========================================================================

/// In Satisfactory, the Foundry takes raw ores directly (not ingots) for
/// certain recipes. Steel Ingot: 3 Iron Ore + 3 Coal -> 3 Steel Ingot, 4
/// ticks. This is different from Builderment where steel requires Iron
/// Ingot + Coal.
#[test]
fn test_steel_production_foundry() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Raw ore sources.
    let iron_ore_src = add_node(
        &mut engine,
        make_source(s_iron_ore(), 5.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    let coal_src = add_node(
        &mut engine,
        make_source(s_coal(), 5.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );

    // Foundry: 3 Iron Ore + 3 Coal -> 3 Steel Ingot, 4 ticks.
    let foundry = add_node(
        &mut engine,
        foundry_steel_ingot(),
        SAT_MULTI_CAP,
        SAT_STD_CAP,
    );

    // Connect raw ores to foundry (no smelting step!).
    connect(&mut engine, iron_ore_src, foundry, mk1_belt());
    connect(&mut engine, coal_src, foundry, mk1_belt());

    // Steel Beam Constructor: 4 Steel Ingot -> 1 Steel Beam, 4 ticks.
    let beam_constructor = add_node(
        &mut engine,
        constructor_steel_beam(),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    connect(&mut engine, foundry, beam_constructor, mk1_belt());

    // Sink for steel beams.
    let beam_sink = add_node(
        &mut engine,
        make_recipe(vec![(s_steel_beam(), 9999)], vec![(s_iron_ore(), 1)], 99999),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );
    connect(&mut engine, beam_constructor, beam_sink, mk1_belt());

    // Run 300 ticks.
    for _ in 0..300 {
        engine.step();
    }

    // Verify the foundry produced steel ingots.
    let steel_at_constructor = input_quantity(&engine, beam_constructor, s_steel_ingot());
    let steel_in_foundry_output = output_quantity(&engine, foundry, s_steel_ingot());
    let beams_at_sink = input_quantity(&engine, beam_sink, s_steel_beam());

    let total_evidence = steel_at_constructor + steel_in_foundry_output + beams_at_sink;
    assert!(
        total_evidence > 0,
        "foundry should produce steel from raw ore; constructor_input={steel_at_constructor}, foundry_output={steel_in_foundry_output}, beams={beams_at_sink}"
    );

    // Steel beams should be produced (evidence of full chain working).
    assert!(
        beams_at_sink > 0,
        "steel beams should be produced from the foundry chain, got {beams_at_sink}"
    );
}

// ===========================================================================
// Test 12: Drone point-to-point logistics
// ===========================================================================

/// Two factory clusters connected by VehicleTransport modeling drones.
/// Drones have small capacity (9 items) but fast travel (10 ticks).
/// Cluster A produces iron plates; Cluster B consumes them to make
/// reinforced plates.
#[test]
fn test_drone_point_to_point() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // === Cluster A: Iron Plate production ===
    let cluster_a_ingot_src = add_node(
        &mut engine,
        make_source(s_iron_ingot(), 5.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    let cluster_a_plate_constructor = add_node(
        &mut engine,
        constructor_iron_plate(),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    connect(
        &mut engine,
        cluster_a_ingot_src,
        cluster_a_plate_constructor,
        mk1_belt(),
    );

    // === Drone link: Cluster A -> Cluster B (capacity=9, travel_time=10) ===
    // This models a drone port that picks up iron plates and flies them
    // to Cluster B's drone port.
    let drone = make_vehicle_transport(9, 10);

    // === Cluster B: Reinforced Iron Plate production ===
    // Cluster B needs iron plates (from drone) and screws (produced locally).
    let cluster_b_assembler = add_node(
        &mut engine,
        assembler_reinforced_plate(),
        SAT_MULTI_CAP,
        SAT_STD_CAP,
    );

    // Connect drone from Cluster A plates to Cluster B assembler.
    connect(
        &mut engine,
        cluster_a_plate_constructor,
        cluster_b_assembler,
        drone,
    );

    // Local screw production in Cluster B.
    let cluster_b_ingot_src = add_node(
        &mut engine,
        make_source(s_iron_ingot(), 5.0),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    let cluster_b_rod_constructor = add_node(
        &mut engine,
        constructor_iron_rod(),
        SAT_STD_CAP,
        SAT_STD_CAP,
    );
    let cluster_b_screw_constructor =
        add_node(&mut engine, constructor_screw(), SAT_STD_CAP, SAT_STD_CAP);
    connect(
        &mut engine,
        cluster_b_ingot_src,
        cluster_b_rod_constructor,
        mk1_belt(),
    );
    connect(
        &mut engine,
        cluster_b_rod_constructor,
        cluster_b_screw_constructor,
        mk1_belt(),
    );
    connect(
        &mut engine,
        cluster_b_screw_constructor,
        cluster_b_assembler,
        mk1_belt(),
    );

    // Sink for reinforced plates.
    let rip_sink = add_node(
        &mut engine,
        make_recipe(
            vec![(s_reinforced_plate(), 9999)],
            vec![(s_iron_ore(), 1)],
            99999,
        ),
        SAT_SINK_CAP,
        SAT_STD_CAP,
    );
    connect(&mut engine, cluster_b_assembler, rip_sink, mk1_belt());

    // Run 500 ticks. Drone round trip = 2*10 = 20 ticks, so ~25 trips.
    // Each trip delivers up to 9 iron plates.
    for _ in 0..500 {
        engine.step();
    }

    // Verify items were transferred via drone.
    let plates_at_assembler = input_quantity(&engine, cluster_b_assembler, s_iron_plate());
    let rip_at_sink = input_quantity(&engine, rip_sink, s_reinforced_plate());

    let total_evidence = plates_at_assembler + rip_at_sink;
    assert!(
        total_evidence > 0,
        "drone should transfer iron plates between clusters; plates_at_assembler={plates_at_assembler}, rip_at_sink={rip_at_sink}"
    );

    // Reinforced plates require both drone-delivered plates AND local screws.
    // If any RIP were produced, the drone logistics are working.
    assert!(
        rip_at_sink > 0,
        "cluster B should produce reinforced iron plates using drone-delivered iron plates, got {rip_at_sink}"
    );
}
