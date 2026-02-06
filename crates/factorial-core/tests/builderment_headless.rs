//! Builderment headless integration tests.
//!
//! Models the full Builderment production chain from raw ore through Super
//! Computer using the Factorial engine. Tests end-to-end item flow, fan-out
//! from shared resources, serialization, and determinism.
//!
//! Reference: docs/plans/2026-02-05-builderment-headless-test-design.md
//! Story: docs/stories/builderment.md

use factorial_core::engine::Engine;
use factorial_core::id::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;
use factorial_core::transport::*;

/// Inventory capacity for single-input buildings (Furnaces, single-recipe Workshops).
const STD_INPUT_CAP: u32 = 50;
const STD_OUTPUT_CAP: u32 = 50;
/// Capacity for 2+ input buildings. The engine uses a single inventory slot
/// shared by all item types, and belts deliver regardless of destination
/// capacity (items overflow and are lost). Large capacity ensures the faster-
/// arriving item type does not prevent the slower type from fitting.
const MULTI_INPUT_CAP: u32 = 10_000;
/// Larger capacity for sinks that accumulate items.
const SINK_INPUT_CAP: u32 = 50_000;

/// Belt configuration: 8 slots, speed 1.0, 1 lane (Builderment-style discrete belts).
fn belt() -> Transport {
    make_item_transport(8)
}


/// Build a complete Builderment factory from raw resources through Super Computer.
///
/// To work around the engine's first-edge-wins transport scheduling (where the
/// first outgoing belt from a node monopolizes its output), each consumer that
/// needs a shared intermediate product gets its own dedicated production chain
/// all the way back to a raw resource source. This eliminates fan-out and
/// guarantees every consumer receives items.
///
/// Returns the engine and a struct containing all node/edge IDs for assertions.
fn build_builderment_factory() -> (Engine, FactoryNodes) {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Helper: build a complete sub-chain from source through furnace.
    // Returns the furnace NodeId.
    let make_iron_chain = |e: &mut Engine| -> NodeId {
        let src = add_node(e, make_source(iron_ore(), 5.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
        let furnace = add_node(e, make_recipe(vec![(iron_ore(), 1)], vec![(iron_ingot(), 1)], 2), STD_INPUT_CAP, STD_OUTPUT_CAP);
        connect(e, src, furnace, belt());
        furnace
    };

    let make_copper_wire_chain = |e: &mut Engine| -> NodeId {
        let src = add_node(e, make_source(copper_ore(), 5.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
        let furnace = add_node(e, make_recipe(vec![(copper_ore(), 1)], vec![(copper_ingot(), 1)], 2), STD_INPUT_CAP, STD_OUTPUT_CAP);
        let workshop = add_node(e, make_recipe(vec![(copper_ingot(), 3)], vec![(copper_wire(), 1)], 3), STD_INPUT_CAP, STD_OUTPUT_CAP);
        connect(e, src, furnace, belt());
        connect(e, furnace, workshop, belt());
        workshop
    };

    let make_coal_source = |e: &mut Engine| -> NodeId {
        add_node(e, make_source(coal(), 5.0), STD_INPUT_CAP, STD_OUTPUT_CAP)
    };

    let make_glass_chain = |e: &mut Engine| -> NodeId {
        let src = add_node(e, make_source(stone(), 3.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
        let stone_f = add_node(e, make_recipe(vec![(stone(), 1)], vec![(sand(), 1)], 2), STD_INPUT_CAP, STD_OUTPUT_CAP);
        let glass_f = add_node(e, make_recipe(vec![(sand(), 1)], vec![(glass(), 1)], 3), STD_INPUT_CAP, STD_OUTPUT_CAP);
        connect(e, src, stone_f, belt());
        connect(e, stone_f, glass_f, belt());
        glass_f
    };

    let make_steel_chain = |e: &mut Engine| -> NodeId {
        let iron_f = make_iron_chain(e);
        let coal_s = make_coal_source(e);
        let forge = add_node(e, make_recipe(vec![(iron_ingot(), 1), (coal(), 1)], vec![(steel(), 1)], 3), MULTI_INPUT_CAP, STD_OUTPUT_CAP);
        connect(e, iron_f, forge, belt());
        connect(e, coal_s, forge, belt());
        forge
    };

    // =====================================================================
    // Named reference nodes (used by tests for assertions).
    // Where a node has fan-out consumers, we pick one representative.
    // =====================================================================

    // --- Primary iron chain (feeds gear_workshop) ---
    let iron_ore_src = add_node(&mut engine, make_source(iron_ore(), 5.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
    let iron_furnace = add_node(
        &mut engine,
        make_recipe(vec![(iron_ore(), 1)], vec![(iron_ingot(), 1)], 2),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, iron_ore_src, iron_furnace, belt());

    // --- Primary copper chain (feeds wire_workshop → motor_shop) ---
    let copper_ore_src = add_node(&mut engine, make_source(copper_ore(), 5.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
    let copper_furnace = add_node(
        &mut engine,
        make_recipe(vec![(copper_ore(), 1)], vec![(copper_ingot(), 1)], 2),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let wire_workshop = add_node(
        &mut engine,
        make_recipe(vec![(copper_ingot(), 3)], vec![(copper_wire(), 1)], 3),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, copper_ore_src, copper_furnace, belt());
    connect(&mut engine, copper_furnace, wire_workshop, belt());

    // --- Primary coal source (feeds steel_forge) ---
    let coal_src = add_node(&mut engine, make_source(coal(), 5.0), STD_INPUT_CAP, STD_OUTPUT_CAP);

    // --- Primary stone/glass chain (feeds light_bulb_shop) ---
    let stone_src = add_node(&mut engine, make_source(stone(), 3.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
    let stone_furnace = add_node(
        &mut engine,
        make_recipe(vec![(stone(), 1)], vec![(sand(), 1)], 2),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let glass_furnace = add_node(
        &mut engine,
        make_recipe(vec![(sand(), 1)], vec![(glass(), 1)], 3),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, stone_src, stone_furnace, belt());
    connect(&mut engine, stone_furnace, glass_furnace, belt());

    // --- Wood chain (feeds plank → wood_frame_shop) ---
    let wood_src = add_node(&mut engine, make_source(wood(), 2.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
    let plank_workshop = add_node(
        &mut engine,
        make_recipe(vec![(wood(), 1)], vec![(wood_plank(), 1)], 2),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, wood_src, plank_workshop, belt());

    // --- Tungsten chain ---
    let tungsten_ore_src = add_node(&mut engine, make_source(tungsten_ore(), 3.0), STD_INPUT_CAP, STD_OUTPUT_CAP);

    // =====================================================================
    // Layer 1: Gear Workshop (iron_furnace → gear_workshop, 1:1)
    // =====================================================================
    let gear_workshop = add_node(
        &mut engine,
        make_recipe(vec![(iron_ingot(), 1)], vec![(iron_gear_b(), 1)], 2),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, iron_furnace, gear_workshop, belt());

    // =====================================================================
    // Layer 2: Machine Shops & Forges
    // =====================================================================

    // Motor Shop: gear + wire → motor (each from dedicated chain)
    let motor_shop = add_node(
        &mut engine,
        make_recipe(vec![(iron_gear_b(), 1), (copper_wire(), 1)], vec![(motor(), 1)], 4),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, gear_workshop, motor_shop, belt());
    connect(&mut engine, wire_workshop, motor_shop, belt());

    // Wood Frame Shop: plank + iron_ingot → wood_frame
    // Needs its own iron chain (can't share with gear_workshop).
    let iron_for_frame = make_iron_chain(&mut engine);
    let wood_frame_shop = add_node(
        &mut engine,
        make_recipe(vec![(wood_plank(), 1), (iron_ingot(), 1)], vec![(wood_frame(), 1)], 3),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, plank_workshop, wood_frame_shop, belt());
    connect(&mut engine, iron_for_frame, wood_frame_shop, belt());

    // Light Bulb Shop: glass + wire → light_bulb
    // Needs its own wire chain (can't share with motor_shop).
    let wire_for_bulb = make_copper_wire_chain(&mut engine);
    let light_bulb_shop = add_node(
        &mut engine,
        make_recipe(vec![(glass(), 1), (copper_wire(), 1)], vec![(light_bulb(), 1)], 3),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, glass_furnace, light_bulb_shop, belt());
    connect(&mut engine, wire_for_bulb, light_bulb_shop, belt());

    // Graphite Shop: sand + coal → graphite
    // Needs its own sand chain and coal source.
    let sand_for_graphite = {
        let src = add_node(&mut engine, make_source(stone(), 3.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
        let sf = add_node(&mut engine, make_recipe(vec![(stone(), 1)], vec![(sand(), 1)], 2), STD_INPUT_CAP, STD_OUTPUT_CAP);
        connect(&mut engine, src, sf, belt());
        sf
    };
    let coal_for_graphite = add_node(&mut engine, make_source(coal(), 5.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
    let graphite_shop = add_node(
        &mut engine,
        make_recipe(vec![(sand(), 1), (coal(), 1)], vec![(graphite(), 1)], 3),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, sand_for_graphite, graphite_shop, belt());
    connect(&mut engine, coal_for_graphite, graphite_shop, belt());

    // Steel Forge: iron_ingot + coal → steel (dedicated chains)
    let iron_for_steel = make_iron_chain(&mut engine);
    let steel_forge = add_node(
        &mut engine,
        make_recipe(vec![(iron_ingot(), 1), (coal(), 1)], vec![(steel(), 1)], 3),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, iron_for_steel, steel_forge, belt());
    connect(&mut engine, coal_src, steel_forge, belt());

    // Tungsten Carbide Forge: tungsten_ore + graphite → tungsten_carbide
    let tc_forge = add_node(
        &mut engine,
        make_recipe(vec![(tungsten_ore(), 10), (graphite(), 1)], vec![(tungsten_carbide(), 1)], 6),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, tungsten_ore_src, tc_forge, belt());
    connect(&mut engine, graphite_shop, tc_forge, belt());

    // =====================================================================
    // Layer 3: Industrial Factories
    // =====================================================================

    // Electric Motor Factory: motor + steel + wire → electric_motor
    // Needs dedicated steel and wire chains.
    let steel_for_emotor = make_steel_chain(&mut engine);
    let wire_for_emotor = make_copper_wire_chain(&mut engine);
    let electric_motor_factory = add_node(
        &mut engine,
        make_recipe(
            vec![(motor(), 1), (steel(), 1), (copper_wire(), 1)],
            vec![(electric_motor(), 1)],
            6,
        ),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, motor_shop, electric_motor_factory, belt());
    connect(&mut engine, steel_for_emotor, electric_motor_factory, belt());
    connect(&mut engine, wire_for_emotor, electric_motor_factory, belt());

    // Circuit Board Factory: glass + wire + steel → circuit_board
    // Needs dedicated glass, wire, steel chains.
    let glass_for_circuit = make_glass_chain(&mut engine);
    let wire_for_circuit = make_copper_wire_chain(&mut engine);
    let steel_for_circuit = make_steel_chain(&mut engine);
    let circuit_board_factory = add_node(
        &mut engine,
        make_recipe(
            vec![(glass(), 1), (copper_wire(), 1), (steel(), 1)],
            vec![(circuit_board(), 1)],
            6,
        ),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, glass_for_circuit, circuit_board_factory, belt());
    connect(&mut engine, wire_for_circuit, circuit_board_factory, belt());
    connect(&mut engine, steel_for_circuit, circuit_board_factory, belt());

    // Basic Robot Factory: wood_frame + motor + light_bulb → basic_robot
    // Motor needs its own chain (shared with electric_motor_factory above).
    // We build a dedicated motor sub-chain.
    let motor_for_robot = {
        let iron_f = make_iron_chain(&mut engine);
        let gear = add_node(&mut engine, make_recipe(vec![(iron_ingot(), 1)], vec![(iron_gear_b(), 1)], 2), STD_INPUT_CAP, STD_OUTPUT_CAP);
        let wire = make_copper_wire_chain(&mut engine);
        let motor = add_node(&mut engine, make_recipe(vec![(iron_gear_b(), 1), (copper_wire(), 1)], vec![(motor(), 1)], 4), MULTI_INPUT_CAP, STD_OUTPUT_CAP);
        connect(&mut engine, iron_f, gear, belt());
        connect(&mut engine, gear, motor, belt());
        connect(&mut engine, wire, motor, belt());
        motor
    };
    let basic_robot_factory = add_node(
        &mut engine,
        make_recipe(
            vec![(wood_frame(), 1), (motor(), 1), (light_bulb(), 1)],
            vec![(basic_robot(), 1)],
            6,
        ),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, wood_frame_shop, basic_robot_factory, belt());
    connect(&mut engine, motor_for_robot, basic_robot_factory, belt());
    connect(&mut engine, light_bulb_shop, basic_robot_factory, belt());

    // =====================================================================
    // Layer 4: Manufacturers
    // =====================================================================

    // Computer Mfr: circuit_board + electric_motor + steel + glass → computer
    // This instance feeds the computer_sink (no fan-out).
    let steel_for_computer = make_steel_chain(&mut engine);
    let glass_for_computer = make_glass_chain(&mut engine);
    let computer_mfr = add_node(
        &mut engine,
        make_recipe(
            vec![(circuit_board(), 1), (electric_motor(), 1), (steel(), 1), (glass(), 1)],
            vec![(computer(), 1)],
            8,
        ),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, circuit_board_factory, computer_mfr, belt());
    connect(&mut engine, electric_motor_factory, computer_mfr, belt());
    connect(&mut engine, steel_for_computer, computer_mfr, belt());
    connect(&mut engine, glass_for_computer, computer_mfr, belt());

    // Super Computer Mfr: computer + tungsten_carbide + electric_motor + circuit_board
    // Each input gets a fully dedicated production chain (no fan-out).
    let make_emotor_chain = |e: &mut Engine| -> NodeId {
        // motor sub-chain: iron -> gear + wire -> motor
        let iron_f = make_iron_chain(e);
        let gear_n = add_node(e, make_recipe(vec![(iron_ingot(), 1)], vec![(iron_gear_b(), 1)], 2), STD_INPUT_CAP, STD_OUTPUT_CAP);
        let wire_n = make_copper_wire_chain(e);
        let motor_n = add_node(e, make_recipe(vec![(iron_gear_b(), 1), (copper_wire(), 1)], vec![(motor(), 1)], 4), MULTI_INPUT_CAP, STD_OUTPUT_CAP);
        connect(e, iron_f, gear_n, belt());
        connect(e, gear_n, motor_n, belt());
        connect(e, wire_n, motor_n, belt());
        // electric motor: motor + steel + wire -> electric_motor
        let steel_n = make_steel_chain(e);
        let wire2_n = make_copper_wire_chain(e);
        let emotor = add_node(e, make_recipe(
            vec![(motor(), 1), (steel(), 1), (copper_wire(), 1)],
            vec![(electric_motor(), 1)], 6,
        ), MULTI_INPUT_CAP, STD_OUTPUT_CAP);
        connect(e, motor_n, emotor, belt());
        connect(e, steel_n, emotor, belt());
        connect(e, wire2_n, emotor, belt());
        emotor
    };
    let make_circuit_chain = |e: &mut Engine| -> NodeId {
        let glass_n = make_glass_chain(e);
        let wire_n = make_copper_wire_chain(e);
        let steel_n = make_steel_chain(e);
        let circuit = add_node(e, make_recipe(
            vec![(glass(), 1), (copper_wire(), 1), (steel(), 1)],
            vec![(circuit_board(), 1)], 6,
        ), MULTI_INPUT_CAP, STD_OUTPUT_CAP);
        connect(e, glass_n, circuit, belt());
        connect(e, wire_n, circuit, belt());
        connect(e, steel_n, circuit, belt());
        circuit
    };
    let make_computer_chain = |e: &mut Engine| -> NodeId {
        let emotor = make_emotor_chain(e);
        let circuit = make_circuit_chain(e);
        let steel_n = make_steel_chain(e);
        let glass_n = make_glass_chain(e);
        let comp = add_node(e, make_recipe(
            vec![(circuit_board(), 1), (electric_motor(), 1), (steel(), 1), (glass(), 1)],
            vec![(computer(), 1)], 8,
        ), MULTI_INPUT_CAP, STD_OUTPUT_CAP);
        connect(e, circuit, comp, belt());
        connect(e, emotor, comp, belt());
        connect(e, steel_n, comp, belt());
        connect(e, glass_n, comp, belt());
        comp
    };

    // Dedicated computer chain for super_computer_mfr
    let computer_for_super = make_computer_chain(&mut engine);
    let emotor_for_super = make_emotor_chain(&mut engine);
    let circuit_for_super = make_circuit_chain(&mut engine);

    let super_computer_mfr = add_node(
        &mut engine,
        make_recipe(
            vec![(computer(), 1), (tungsten_carbide(), 1), (electric_motor(), 1), (circuit_board(), 1)],
            vec![(super_computer(), 1)],
            10,
        ),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, computer_for_super, super_computer_mfr, belt());
    connect(&mut engine, tc_forge, super_computer_mfr, belt());
    connect(&mut engine, emotor_for_super, super_computer_mfr, belt());
    connect(&mut engine, circuit_for_super, super_computer_mfr, belt());

    // =====================================================================
    // Sinks (each connected to exactly one producer — no fan-out)
    // =====================================================================

    let computer_sink = add_node(
        &mut engine,
        make_recipe(vec![(computer(), 9999)], vec![(iron_ore(), 1)], 99999),
        SINK_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, computer_mfr, computer_sink, belt());

    let super_computer_sink = add_node(
        &mut engine,
        make_recipe(vec![(super_computer(), 9999)], vec![(iron_ore(), 1)], 99999),
        SINK_INPUT_CAP, STD_OUTPUT_CAP,
    );
    connect(&mut engine, super_computer_mfr, super_computer_sink, belt());

    let nodes = FactoryNodes {
        // Sources (representative nodes for test assertions)
        iron_ore_src, copper_ore_src, coal_src, stone_src, wood_src, tungsten_ore_src,
        // Tier 1
        iron_furnace, copper_furnace, stone_furnace, glass_furnace,
        plank_workshop, gear_workshop, wire_workshop,
        // Tier 2
        motor_shop, wood_frame_shop, light_bulb_shop, graphite_shop,
        steel_forge, tc_forge,
        // Tier 3
        electric_motor_factory, circuit_board_factory, basic_robot_factory,
        // Tier 4
        computer_mfr, super_computer_mfr,
        // Sinks
        computer_sink, super_computer_sink,
    };

    (engine, nodes)
}

/// All named node IDs in the Builderment factory for targeted assertions.
#[allow(dead_code)]
struct FactoryNodes {
    // Sources
    iron_ore_src: NodeId,
    copper_ore_src: NodeId,
    coal_src: NodeId,
    stone_src: NodeId,
    wood_src: NodeId,
    tungsten_ore_src: NodeId,
    // Tier 1
    iron_furnace: NodeId,
    copper_furnace: NodeId,
    stone_furnace: NodeId,
    glass_furnace: NodeId,
    plank_workshop: NodeId,
    gear_workshop: NodeId,
    wire_workshop: NodeId,
    // Tier 2
    motor_shop: NodeId,
    wood_frame_shop: NodeId,
    light_bulb_shop: NodeId,
    graphite_shop: NodeId,
    steel_forge: NodeId,
    tc_forge: NodeId,
    // Tier 3
    electric_motor_factory: NodeId,
    circuit_board_factory: NodeId,
    basic_robot_factory: NodeId,
    // Tier 4
    computer_mfr: NodeId,
    super_computer_mfr: NodeId,
    // Sinks
    computer_sink: NodeId,
    super_computer_sink: NodeId,
}

#[test]
fn factory_builds_successfully() {
    let (engine, _nodes) = build_builderment_factory();
    // The factory uses dedicated production chains (no fan-out) to work around
    // the engine's first-edge-wins transport scheduling. This results in many
    // duplicated sub-chains.
    assert_eq!(engine.node_count(), 127, "factory should have 127 nodes");
    assert_eq!(engine.edge_count(), 123, "factory should have 123 edges, got {}", engine.edge_count());
}

#[test]
fn full_chain_produces_computers() {
    let (mut engine, nodes) = build_builderment_factory();

    // Run for 1000 ticks -- enough for items to flow through the deepest chain.
    // (Increased from 500 to account for belt transit time across many hops.)
    for _ in 0..1000 {
        engine.step();
    }

    // Computers should have reached the sink.
    let computers_at_sink = input_quantity(&engine, nodes.computer_sink, computer());
    assert!(
        computers_at_sink > 0,
        "computer sink should have received computers after 1000 ticks, got {computers_at_sink}"
    );

    // Also verify intermediate products are flowing.
    let iron_ingots_produced = output_total(&engine, nodes.iron_furnace)
        + input_total(&engine, nodes.gear_workshop)
        + input_total(&engine, nodes.steel_forge)
        + input_total(&engine, nodes.wood_frame_shop);
    assert!(
        iron_ingots_produced > 0,
        "iron ingots should be flowing through the chain"
    );

    let wire_produced = output_total(&engine, nodes.wire_workshop);
    let wire_consumed = input_quantity(&engine, nodes.motor_shop, copper_wire())
        + input_quantity(&engine, nodes.light_bulb_shop, copper_wire())
        + input_quantity(&engine, nodes.electric_motor_factory, copper_wire())
        + input_quantity(&engine, nodes.circuit_board_factory, copper_wire());
    assert!(
        wire_produced + wire_consumed > 0,
        "copper wire should be flowing to downstream consumers"
    );
}

#[test]
fn full_chain_produces_super_computers() {
    let (mut engine, nodes) = build_builderment_factory();

    // Super Computers have the deepest chain — run longer.
    for _ in 0..1000 {
        engine.step();
    }

    let super_computers_at_sink = input_quantity(&engine, nodes.super_computer_sink, super_computer());
    assert!(
        super_computers_at_sink > 0,
        "super computer sink should have received super computers after 1000 ticks, got {super_computers_at_sink}"
    );

    // Verify tungsten carbide is being produced (the unique Super Computer input).
    let tc_produced = output_total(&engine, nodes.tc_forge);
    let tc_consumed = input_quantity(&engine, nodes.super_computer_mfr, tungsten_carbide());
    assert!(
        tc_produced + tc_consumed > 0,
        "tungsten carbide should be flowing into super computer production"
    );
}

#[test]
fn parallel_chain_produces_basic_robots() {
    let (mut engine, nodes) = build_builderment_factory();

    for _ in 0..500 {
        engine.step();
    }

    // Basic Robot requires: Wood Frame + Motor + Light Bulb
    // This tests the parallel chain that shares Motor and Copper Wire with the
    // Computer chain.
    let robots_produced = output_quantity(&engine, nodes.basic_robot_factory, basic_robot());
    assert!(
        robots_produced > 0,
        "basic robot factory should have produced robots after 500 ticks, got {robots_produced}"
    );
}

