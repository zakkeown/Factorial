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
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;
use factorial_core::transport::*;

/// Inventory capacity for 1-2 input buildings (Furnaces, Workshops, Forges).
const STD_INPUT_CAP: u32 = 50;
const STD_OUTPUT_CAP: u32 = 50;
/// Larger capacity for 3-4 input buildings (Industrial Factories, Manufacturers).
/// Needed because simple_inventory uses a single slot shared across all item types.
/// See: Known Engine Limitations #2 in the implementation plan.
const MULTI_INPUT_CAP: u32 = 200;
/// Larger capacity for sinks that accumulate items.
const SINK_INPUT_CAP: u32 = 5000;

/// Belt configuration: 8 slots, speed 1.0, 1 lane (Builderment-style discrete belts).
fn belt() -> Transport {
    make_item_transport(8)
}

/// Build a complete Builderment factory from raw resources through Super Computer.
/// Returns the engine and a struct containing all node/edge IDs for assertions.
fn build_builderment_factory() -> (Engine, FactoryNodes) {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // =====================================================================
    // Layer 0: Resource Sources
    // =====================================================================
    let iron_ore_src = add_node(&mut engine, make_source(iron_ore(), 5.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
    let copper_ore_src = add_node(&mut engine, make_source(copper_ore(), 5.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
    let coal_src = add_node(&mut engine, make_source(coal(), 5.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
    let stone_src = add_node(&mut engine, make_source(stone(), 3.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
    let wood_src = add_node(&mut engine, make_source(wood(), 2.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
    let tungsten_ore_src = add_node(&mut engine, make_source(tungsten_ore(), 3.0), STD_INPUT_CAP, STD_OUTPUT_CAP);

    // =====================================================================
    // Layer 1: Furnaces & Workshops
    // =====================================================================

    // Furnaces (1 input → 1 output)
    let iron_furnace = add_node(
        &mut engine,
        make_recipe(vec![(iron_ore(), 1)], vec![(iron_ingot(), 1)], 2),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let copper_furnace = add_node(
        &mut engine,
        make_recipe(vec![(copper_ore(), 1)], vec![(copper_ingot(), 1)], 2),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
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

    // Workshops (1 input → 1 output)
    let plank_workshop = add_node(
        &mut engine,
        make_recipe(vec![(wood(), 1)], vec![(wood_plank(), 1)], 2),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let gear_workshop = add_node(
        &mut engine,
        make_recipe(vec![(iron_ingot(), 1)], vec![(iron_gear_b(), 1)], 2),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    // Copper Wire: 3 copper ingot → 1 copper wire (the bottleneck!)
    let wire_workshop = add_node(
        &mut engine,
        make_recipe(vec![(copper_ingot(), 3)], vec![(copper_wire(), 1)], 3),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );

    // =====================================================================
    // Layer 2: Machine Shops & Forges
    // =====================================================================

    // Machine Shops (2 inputs → 1 output)
    let motor_shop = add_node(
        &mut engine,
        make_recipe(vec![(iron_gear_b(), 1), (copper_wire(), 1)], vec![(motor(), 1)], 4),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let wood_frame_shop = add_node(
        &mut engine,
        make_recipe(vec![(wood_plank(), 1), (iron_ingot(), 1)], vec![(wood_frame(), 1)], 3),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let light_bulb_shop = add_node(
        &mut engine,
        make_recipe(vec![(glass(), 1), (copper_wire(), 1)], vec![(light_bulb(), 1)], 3),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let graphite_shop = add_node(
        &mut engine,
        make_recipe(vec![(sand(), 1), (coal(), 1)], vec![(graphite(), 1)], 3),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );

    // Forges (2 inputs → 1 output)
    let steel_forge = add_node(
        &mut engine,
        make_recipe(vec![(iron_ingot(), 1), (coal(), 1)], vec![(steel(), 1)], 3),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let tc_forge = add_node(
        &mut engine,
        make_recipe(vec![(tungsten_ore(), 10), (graphite(), 1)], vec![(tungsten_carbide(), 1)], 6),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );

    // =====================================================================
    // Layer 3: Industrial Factories
    // =====================================================================

    // Industrial Factories use MULTI_INPUT_CAP (3 different item types sharing one slot).
    let electric_motor_factory = add_node(
        &mut engine,
        make_recipe(
            vec![(motor(), 1), (steel(), 1), (copper_wire(), 1)],
            vec![(electric_motor(), 1)],
            6,
        ),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let circuit_board_factory = add_node(
        &mut engine,
        make_recipe(
            vec![(glass(), 1), (copper_wire(), 1), (steel(), 1)],
            vec![(circuit_board(), 1)],
            6,
        ),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let basic_robot_factory = add_node(
        &mut engine,
        make_recipe(
            vec![(wood_frame(), 1), (motor(), 1), (light_bulb(), 1)],
            vec![(basic_robot(), 1)],
            6,
        ),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );

    // =====================================================================
    // Layer 4: Manufacturers
    // =====================================================================

    // Manufacturers use MULTI_INPUT_CAP (4 different item types sharing one slot).
    let computer_mfr = add_node(
        &mut engine,
        make_recipe(
            vec![(circuit_board(), 1), (electric_motor(), 1), (steel(), 1), (glass(), 1)],
            vec![(computer(), 1)],
            8,
        ),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let super_computer_mfr = add_node(
        &mut engine,
        make_recipe(
            vec![(computer(), 1), (tungsten_carbide(), 1), (electric_motor(), 1), (circuit_board(), 1)],
            vec![(super_computer(), 1)],
            10,
        ),
        MULTI_INPUT_CAP, STD_OUTPUT_CAP,
    );

    // =====================================================================
    // Sinks
    // =====================================================================

    let computer_sink = add_node(
        &mut engine,
        make_recipe(vec![(computer(), 9999)], vec![(iron_ore(), 1)], 99999),
        SINK_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let super_computer_sink = add_node(
        &mut engine,
        make_recipe(vec![(super_computer(), 9999)], vec![(iron_ore(), 1)], 99999),
        SINK_INPUT_CAP, STD_OUTPUT_CAP,
    );

    // =====================================================================
    // Transport: Connect everything with belts
    // =====================================================================

    // Raw → Tier 1
    connect(&mut engine, iron_ore_src, iron_furnace, belt());
    connect(&mut engine, copper_ore_src, copper_furnace, belt());
    connect(&mut engine, stone_src, stone_furnace, belt());
    connect(&mut engine, wood_src, plank_workshop, belt());

    // Coal fan-out: Coal Source → Graphite Shop, Steel Forge
    connect(&mut engine, coal_src, graphite_shop, belt());
    connect(&mut engine, coal_src, steel_forge, belt());

    // Tungsten Ore → Tungsten Carbide Forge
    connect(&mut engine, tungsten_ore_src, tc_forge, belt());

    // Tier 1 chaining: Stone Furnace → Glass Furnace (Sand → Glass)
    connect(&mut engine, stone_furnace, glass_furnace, belt());

    // Iron Furnace fan-out → Gear Workshop, Steel Forge, Wood Frame Shop
    connect(&mut engine, iron_furnace, gear_workshop, belt());
    connect(&mut engine, iron_furnace, steel_forge, belt());
    connect(&mut engine, iron_furnace, wood_frame_shop, belt());

    // Copper Furnace → Wire Workshop
    connect(&mut engine, copper_furnace, wire_workshop, belt());

    // Sand fan-out: Stone Furnace → Graphite Shop
    connect(&mut engine, stone_furnace, graphite_shop, belt());

    // Glass fan-out: Glass Furnace → Light Bulb Shop, Circuit Board Factory, Computer Mfr
    connect(&mut engine, glass_furnace, light_bulb_shop, belt());
    connect(&mut engine, glass_furnace, circuit_board_factory, belt());
    connect(&mut engine, glass_furnace, computer_mfr, belt());

    // Plank Workshop → Wood Frame Shop
    connect(&mut engine, plank_workshop, wood_frame_shop, belt());

    // Gear Workshop → Motor Shop
    connect(&mut engine, gear_workshop, motor_shop, belt());

    // Wire Workshop fan-out (the bottleneck!) → Motor Shop, Light Bulb Shop,
    // Electric Motor Factory, Circuit Board Factory
    connect(&mut engine, wire_workshop, motor_shop, belt());
    connect(&mut engine, wire_workshop, light_bulb_shop, belt());
    connect(&mut engine, wire_workshop, electric_motor_factory, belt());
    connect(&mut engine, wire_workshop, circuit_board_factory, belt());

    // Motor Shop fan-out → Electric Motor Factory, Basic Robot Factory
    connect(&mut engine, motor_shop, electric_motor_factory, belt());
    connect(&mut engine, motor_shop, basic_robot_factory, belt());

    // Steel Forge fan-out → Electric Motor Factory, Circuit Board Factory, Computer Mfr
    connect(&mut engine, steel_forge, electric_motor_factory, belt());
    connect(&mut engine, steel_forge, circuit_board_factory, belt());
    connect(&mut engine, steel_forge, computer_mfr, belt());

    // Wood Frame Shop → Basic Robot Factory
    connect(&mut engine, wood_frame_shop, basic_robot_factory, belt());

    // Light Bulb Shop → Basic Robot Factory
    connect(&mut engine, light_bulb_shop, basic_robot_factory, belt());

    // Graphite Shop → Tungsten Carbide Forge
    connect(&mut engine, graphite_shop, tc_forge, belt());

    // Tier 3 → Tier 4
    connect(&mut engine, electric_motor_factory, computer_mfr, belt());
    connect(&mut engine, electric_motor_factory, super_computer_mfr, belt());
    connect(&mut engine, circuit_board_factory, computer_mfr, belt());
    connect(&mut engine, circuit_board_factory, super_computer_mfr, belt());

    // Computer Mfr → Super Computer Mfr and Computer Sink
    connect(&mut engine, computer_mfr, super_computer_mfr, belt());
    connect(&mut engine, computer_mfr, computer_sink, belt());

    // Tungsten Carbide Forge → Super Computer Mfr
    connect(&mut engine, tc_forge, super_computer_mfr, belt());

    // Super Computer Mfr → Super Computer Sink
    connect(&mut engine, super_computer_mfr, super_computer_sink, belt());

    let nodes = FactoryNodes {
        // Sources
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
    assert_eq!(engine.node_count(), 26, "factory should have 26 nodes");
    assert_eq!(engine.edge_count(), 38, "factory should have 38 edges, got {}", engine.edge_count());
}
