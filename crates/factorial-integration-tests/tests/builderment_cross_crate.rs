//! Cross-crate Builderment integration tests.
//!
//! Provides the shared factory builder used by power, tech-tree, and stats
//! integration tests. The factory builder is copied from
//! `factorial-core/tests/builderment_headless.rs` to avoid cross-test-crate
//! dependencies.
//!
//! Reference: docs/plans/2026-02-05-builderment-headless-test-design.md

use std::cell::RefCell;
use std::rc::Rc;

use factorial_core::engine::Engine;
use factorial_core::event::{Event, EventKind};
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;
use factorial_core::transport::*;
use factorial_power::{PowerConsumer, PowerEvent, PowerModule, PowerProducer};
use factorial_stats::{ProductionStats, StatsConfig};
use factorial_tech_tree::{ResearchCost, TechEvent, TechId, TechTree, Technology, Unlock};

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

/// All named node IDs in the Builderment factory for targeted assertions.
#[allow(dead_code)]
pub struct FactoryNodes {
    // Sources
    pub iron_ore_src: NodeId,
    pub copper_ore_src: NodeId,
    pub coal_src: NodeId,
    pub stone_src: NodeId,
    pub wood_src: NodeId,
    pub tungsten_ore_src: NodeId,
    // Tier 1
    pub iron_furnace: NodeId,
    pub copper_furnace: NodeId,
    pub stone_furnace: NodeId,
    pub glass_furnace: NodeId,
    pub plank_workshop: NodeId,
    pub gear_workshop: NodeId,
    pub wire_workshop: NodeId,
    // Tier 2
    pub motor_shop: NodeId,
    pub wood_frame_shop: NodeId,
    pub light_bulb_shop: NodeId,
    pub graphite_shop: NodeId,
    pub steel_forge: NodeId,
    pub tc_forge: NodeId,
    // Tier 3
    pub electric_motor_factory: NodeId,
    pub circuit_board_factory: NodeId,
    pub basic_robot_factory: NodeId,
    // Tier 4
    pub computer_mfr: NodeId,
    pub super_computer_mfr: NodeId,
    // Sinks
    pub computer_sink: NodeId,
    pub super_computer_sink: NodeId,
    // All production nodes (everything except sources and sinks)
    pub all_production: Vec<NodeId>,
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
pub fn build_builderment_factory() -> (Engine, FactoryNodes) {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // Helper: build a complete sub-chain from source through furnace.
    // Returns the furnace NodeId.
    let make_iron_chain = |e: &mut Engine| -> NodeId {
        let src = add_node(
            e,
            make_source(iron_ore(), 5.0),
            STD_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        let furnace = add_node(
            e,
            make_recipe(vec![(iron_ore(), 1)], vec![(iron_ingot(), 1)], 2),
            STD_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        connect(e, src, furnace, belt());
        furnace
    };

    let make_copper_wire_chain = |e: &mut Engine| -> NodeId {
        let src = add_node(
            e,
            make_source(copper_ore(), 5.0),
            STD_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        let furnace = add_node(
            e,
            make_recipe(vec![(copper_ore(), 1)], vec![(copper_ingot(), 1)], 2),
            STD_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        let workshop = add_node(
            e,
            make_recipe(vec![(copper_ingot(), 3)], vec![(copper_wire(), 1)], 3),
            STD_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        connect(e, src, furnace, belt());
        connect(e, furnace, workshop, belt());
        workshop
    };

    let make_coal_source = |e: &mut Engine| -> NodeId {
        add_node(e, make_source(coal(), 5.0), STD_INPUT_CAP, STD_OUTPUT_CAP)
    };

    let make_glass_chain = |e: &mut Engine| -> NodeId {
        let src = add_node(e, make_source(stone(), 3.0), STD_INPUT_CAP, STD_OUTPUT_CAP);
        let stone_f = add_node(
            e,
            make_recipe(vec![(stone(), 1)], vec![(sand(), 1)], 2),
            STD_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        let glass_f = add_node(
            e,
            make_recipe(vec![(sand(), 1)], vec![(glass(), 1)], 3),
            STD_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        connect(e, src, stone_f, belt());
        connect(e, stone_f, glass_f, belt());
        glass_f
    };

    let make_steel_chain = |e: &mut Engine| -> NodeId {
        let iron_f = make_iron_chain(e);
        let coal_s = make_coal_source(e);
        let forge = add_node(
            e,
            make_recipe(vec![(iron_ingot(), 1), (coal(), 1)], vec![(steel(), 1)], 3),
            MULTI_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        connect(e, iron_f, forge, belt());
        connect(e, coal_s, forge, belt());
        forge
    };

    // =====================================================================
    // Named reference nodes (used by tests for assertions).
    // Where a node has fan-out consumers, we pick one representative.
    // =====================================================================

    // --- Primary iron chain (feeds gear_workshop) ---
    let iron_ore_src = add_node(
        &mut engine,
        make_source(iron_ore(), 5.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let iron_furnace = add_node(
        &mut engine,
        make_recipe(vec![(iron_ore(), 1)], vec![(iron_ingot(), 1)], 2),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, iron_ore_src, iron_furnace, belt());

    // --- Primary copper chain (feeds wire_workshop -> motor_shop) ---
    let copper_ore_src = add_node(
        &mut engine,
        make_source(copper_ore(), 5.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let copper_furnace = add_node(
        &mut engine,
        make_recipe(vec![(copper_ore(), 1)], vec![(copper_ingot(), 1)], 2),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let wire_workshop = add_node(
        &mut engine,
        make_recipe(vec![(copper_ingot(), 3)], vec![(copper_wire(), 1)], 3),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, copper_ore_src, copper_furnace, belt());
    connect(&mut engine, copper_furnace, wire_workshop, belt());

    // --- Primary coal source (feeds steel_forge) ---
    let coal_src = add_node(
        &mut engine,
        make_source(coal(), 5.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // --- Primary stone/glass chain (feeds light_bulb_shop) ---
    let stone_src = add_node(
        &mut engine,
        make_source(stone(), 3.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let stone_furnace = add_node(
        &mut engine,
        make_recipe(vec![(stone(), 1)], vec![(sand(), 1)], 2),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let glass_furnace = add_node(
        &mut engine,
        make_recipe(vec![(sand(), 1)], vec![(glass(), 1)], 3),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, stone_src, stone_furnace, belt());
    connect(&mut engine, stone_furnace, glass_furnace, belt());

    // --- Wood chain (feeds plank -> wood_frame_shop) ---
    let wood_src = add_node(
        &mut engine,
        make_source(wood(), 2.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let plank_workshop = add_node(
        &mut engine,
        make_recipe(vec![(wood(), 1)], vec![(wood_plank(), 1)], 2),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, wood_src, plank_workshop, belt());

    // --- Tungsten chain ---
    let tungsten_ore_src = add_node(
        &mut engine,
        make_source(tungsten_ore(), 3.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );

    // =====================================================================
    // Layer 1: Gear Workshop (iron_furnace -> gear_workshop, 1:1)
    // =====================================================================
    let gear_workshop = add_node(
        &mut engine,
        make_recipe(vec![(iron_ingot(), 1)], vec![(iron_gear_b(), 1)], 2),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, iron_furnace, gear_workshop, belt());

    // =====================================================================
    // Layer 2: Machine Shops & Forges
    // =====================================================================

    // Motor Shop: gear + wire -> motor (each from dedicated chain)
    let motor_shop = add_node(
        &mut engine,
        make_recipe(
            vec![(iron_gear_b(), 1), (copper_wire(), 1)],
            vec![(motor(), 1)],
            4,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, gear_workshop, motor_shop, belt());
    connect(&mut engine, wire_workshop, motor_shop, belt());

    // Wood Frame Shop: plank + iron_ingot -> wood_frame
    // Needs its own iron chain (can't share with gear_workshop).
    let iron_for_frame = make_iron_chain(&mut engine);
    let wood_frame_shop = add_node(
        &mut engine,
        make_recipe(
            vec![(wood_plank(), 1), (iron_ingot(), 1)],
            vec![(wood_frame(), 1)],
            3,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, plank_workshop, wood_frame_shop, belt());
    connect(&mut engine, iron_for_frame, wood_frame_shop, belt());

    // Light Bulb Shop: glass + wire -> light_bulb
    // Needs its own wire chain (can't share with motor_shop).
    let wire_for_bulb = make_copper_wire_chain(&mut engine);
    let light_bulb_shop = add_node(
        &mut engine,
        make_recipe(
            vec![(glass(), 1), (copper_wire(), 1)],
            vec![(light_bulb(), 1)],
            3,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, glass_furnace, light_bulb_shop, belt());
    connect(&mut engine, wire_for_bulb, light_bulb_shop, belt());

    // Graphite Shop: sand + coal -> graphite
    // Needs its own sand chain and coal source.
    let sand_for_graphite = {
        let src = add_node(
            &mut engine,
            make_source(stone(), 3.0),
            STD_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        let sf = add_node(
            &mut engine,
            make_recipe(vec![(stone(), 1)], vec![(sand(), 1)], 2),
            STD_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        connect(&mut engine, src, sf, belt());
        sf
    };
    let coal_for_graphite = add_node(
        &mut engine,
        make_source(coal(), 5.0),
        STD_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    let graphite_shop = add_node(
        &mut engine,
        make_recipe(vec![(sand(), 1), (coal(), 1)], vec![(graphite(), 1)], 3),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, sand_for_graphite, graphite_shop, belt());
    connect(&mut engine, coal_for_graphite, graphite_shop, belt());

    // Steel Forge: iron_ingot + coal -> steel (dedicated chains)
    let iron_for_steel = make_iron_chain(&mut engine);
    let steel_forge = add_node(
        &mut engine,
        make_recipe(vec![(iron_ingot(), 1), (coal(), 1)], vec![(steel(), 1)], 3),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, iron_for_steel, steel_forge, belt());
    connect(&mut engine, coal_src, steel_forge, belt());

    // Tungsten Carbide Forge: tungsten_ore + graphite -> tungsten_carbide
    let tc_forge = add_node(
        &mut engine,
        make_recipe(
            vec![(tungsten_ore(), 10), (graphite(), 1)],
            vec![(tungsten_carbide(), 1)],
            6,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, tungsten_ore_src, tc_forge, belt());
    connect(&mut engine, graphite_shop, tc_forge, belt());

    // =====================================================================
    // Layer 3: Industrial Factories
    // =====================================================================

    // Electric Motor Factory: motor + steel + wire -> electric_motor
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
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, motor_shop, electric_motor_factory, belt());
    connect(
        &mut engine,
        steel_for_emotor,
        electric_motor_factory,
        belt(),
    );
    connect(&mut engine, wire_for_emotor, electric_motor_factory, belt());

    // Circuit Board Factory: glass + wire + steel -> circuit_board
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
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(
        &mut engine,
        glass_for_circuit,
        circuit_board_factory,
        belt(),
    );
    connect(&mut engine, wire_for_circuit, circuit_board_factory, belt());
    connect(
        &mut engine,
        steel_for_circuit,
        circuit_board_factory,
        belt(),
    );

    // Basic Robot Factory: wood_frame + motor + light_bulb -> basic_robot
    // Motor needs its own chain (shared with electric_motor_factory above).
    // We build a dedicated motor sub-chain.
    let motor_for_robot = {
        let iron_f = make_iron_chain(&mut engine);
        let gear = add_node(
            &mut engine,
            make_recipe(vec![(iron_ingot(), 1)], vec![(iron_gear_b(), 1)], 2),
            STD_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        let wire = make_copper_wire_chain(&mut engine);
        let motor = add_node(
            &mut engine,
            make_recipe(
                vec![(iron_gear_b(), 1), (copper_wire(), 1)],
                vec![(motor(), 1)],
                4,
            ),
            MULTI_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
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
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, wood_frame_shop, basic_robot_factory, belt());
    connect(&mut engine, motor_for_robot, basic_robot_factory, belt());
    connect(&mut engine, light_bulb_shop, basic_robot_factory, belt());

    // =====================================================================
    // Layer 4: Manufacturers
    // =====================================================================

    // Computer Mfr: circuit_board + electric_motor + steel + glass -> computer
    // This instance feeds the computer_sink (no fan-out).
    let steel_for_computer = make_steel_chain(&mut engine);
    let glass_for_computer = make_glass_chain(&mut engine);
    let computer_mfr = add_node(
        &mut engine,
        make_recipe(
            vec![
                (circuit_board(), 1),
                (electric_motor(), 1),
                (steel(), 1),
                (glass(), 1),
            ],
            vec![(computer(), 1)],
            8,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
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
        let gear_n = add_node(
            e,
            make_recipe(vec![(iron_ingot(), 1)], vec![(iron_gear_b(), 1)], 2),
            STD_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        let wire_n = make_copper_wire_chain(e);
        let motor_n = add_node(
            e,
            make_recipe(
                vec![(iron_gear_b(), 1), (copper_wire(), 1)],
                vec![(motor(), 1)],
                4,
            ),
            MULTI_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        connect(e, iron_f, gear_n, belt());
        connect(e, gear_n, motor_n, belt());
        connect(e, wire_n, motor_n, belt());
        // electric motor: motor + steel + wire -> electric_motor
        let steel_n = make_steel_chain(e);
        let wire2_n = make_copper_wire_chain(e);
        let emotor = add_node(
            e,
            make_recipe(
                vec![(motor(), 1), (steel(), 1), (copper_wire(), 1)],
                vec![(electric_motor(), 1)],
                6,
            ),
            MULTI_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
        connect(e, motor_n, emotor, belt());
        connect(e, steel_n, emotor, belt());
        connect(e, wire2_n, emotor, belt());
        emotor
    };
    let make_circuit_chain = |e: &mut Engine| -> NodeId {
        let glass_n = make_glass_chain(e);
        let wire_n = make_copper_wire_chain(e);
        let steel_n = make_steel_chain(e);
        let circuit = add_node(
            e,
            make_recipe(
                vec![(glass(), 1), (copper_wire(), 1), (steel(), 1)],
                vec![(circuit_board(), 1)],
                6,
            ),
            MULTI_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
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
        let comp = add_node(
            e,
            make_recipe(
                vec![
                    (circuit_board(), 1),
                    (electric_motor(), 1),
                    (steel(), 1),
                    (glass(), 1),
                ],
                vec![(computer(), 1)],
                8,
            ),
            MULTI_INPUT_CAP,
            STD_OUTPUT_CAP,
        );
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
            vec![
                (computer(), 1),
                (tungsten_carbide(), 1),
                (electric_motor(), 1),
                (circuit_board(), 1),
            ],
            vec![(super_computer(), 1)],
            10,
        ),
        MULTI_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, computer_for_super, super_computer_mfr, belt());
    connect(&mut engine, tc_forge, super_computer_mfr, belt());
    connect(&mut engine, emotor_for_super, super_computer_mfr, belt());
    connect(&mut engine, circuit_for_super, super_computer_mfr, belt());

    // =====================================================================
    // Sinks (each connected to exactly one producer -- no fan-out)
    // =====================================================================

    let computer_sink = add_node(
        &mut engine,
        make_recipe(vec![(computer(), 9999)], vec![(iron_ore(), 1)], 99999),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, computer_mfr, computer_sink, belt());

    let super_computer_sink = add_node(
        &mut engine,
        make_recipe(vec![(super_computer(), 9999)], vec![(iron_ore(), 1)], 99999),
        SINK_INPUT_CAP,
        STD_OUTPUT_CAP,
    );
    connect(&mut engine, super_computer_mfr, super_computer_sink, belt());

    let nodes = FactoryNodes {
        // Sources (representative nodes for test assertions)
        iron_ore_src,
        copper_ore_src,
        coal_src,
        stone_src,
        wood_src,
        tungsten_ore_src,
        // Tier 1
        iron_furnace,
        copper_furnace,
        stone_furnace,
        glass_furnace,
        plank_workshop,
        gear_workshop,
        wire_workshop,
        // Tier 2
        motor_shop,
        wood_frame_shop,
        light_bulb_shop,
        graphite_shop,
        steel_forge,
        tc_forge,
        // Tier 3
        electric_motor_factory,
        circuit_board_factory,
        basic_robot_factory,
        // Tier 4
        computer_mfr,
        super_computer_mfr,
        // Sinks
        computer_sink,
        super_computer_sink,
        // All production nodes (everything except sources and sinks)
        all_production: vec![
            iron_furnace,
            copper_furnace,
            stone_furnace,
            glass_furnace,
            plank_workshop,
            gear_workshop,
            wire_workshop,
            motor_shop,
            wood_frame_shop,
            light_bulb_shop,
            graphite_shop,
            steel_forge,
            tc_forge,
            electric_motor_factory,
            circuit_board_factory,
            basic_robot_factory,
            computer_mfr,
            super_computer_mfr,
        ],
    };

    (engine, nodes)
}

#[test]
fn cross_crate_smoke_test() {
    let (engine, _nodes) = build_builderment_factory();
    assert_eq!(engine.node_count(), 127);
    assert_eq!(engine.edge_count(), 123);
}

#[test]
fn power_brownout_and_recovery() {
    let (mut engine, nodes) = build_builderment_factory();

    // Set up power module.
    let mut power = PowerModule::new();
    let network_id = power.create_network();

    // Add a power producer (Coal Power Plant).
    let producer_node = nodes.coal_src; // Reuse coal source as the power node.
    power.add_producer(
        network_id,
        producer_node,
        PowerProducer {
            capacity: Fixed64::from_num(100),
        },
    );

    // Register all production buildings as consumers.
    for &node in &nodes.all_production {
        power.add_consumer(
            network_id,
            node,
            PowerConsumer {
                demand: Fixed64::from_num(5),
            },
        );
    }

    // Phase 1: Normal operation (200 ticks).
    for tick in 1..=200u64 {
        engine.step();
        let events = power.tick(tick);
        // Should have no brownout events during normal operation.
        for event in &events {
            assert!(
                !matches!(event, PowerEvent::PowerGridBrownout { .. }),
                "unexpected brownout at tick {tick}"
            );
        }
    }

    // Verify power satisfaction is 1.0.
    let satisfaction = power.satisfaction(network_id).unwrap();
    assert_eq!(
        satisfaction,
        Fixed64::from_num(1),
        "power should be fully satisfied before brownout"
    );

    // Phase 2: Remove the producer — trigger brownout.
    power.remove_node(producer_node);

    let mut saw_brownout = false;
    for tick in 201..=250u64 {
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
        "should have seen a brownout event after removing producer"
    );

    // Verify satisfaction dropped to 0.
    let satisfaction = power.satisfaction(network_id).unwrap();
    assert_eq!(
        satisfaction,
        Fixed64::from_num(0),
        "power satisfaction should be 0 during brownout"
    );

    // Phase 3: Restore the producer.
    power.add_producer(
        network_id,
        producer_node,
        PowerProducer {
            capacity: Fixed64::from_num(100),
        },
    );

    let mut saw_restored = false;
    for tick in 251..=300u64 {
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
        "should have seen a restored event after re-adding producer"
    );

    let satisfaction = power.satisfaction(network_id).unwrap();
    assert_eq!(
        satisfaction,
        Fixed64::from_num(1),
        "power should be fully satisfied after recovery"
    );
}

#[test]
fn tech_tree_progression() {
    let (mut engine, _nodes) = build_builderment_factory();

    // Set up tech tree with 5 Builderment-style technologies.
    let mut tech_tree = TechTree::new();

    let basic_smelting_id = TechId(0);
    let workshops_id = TechId(1);
    let machine_shops_id = TechId(2);
    let industrial_id = TechId(3);
    let manufacturing_id = TechId(4);

    tech_tree
        .register(Technology {
            id: basic_smelting_id,
            name: "Basic Smelting".to_string(),
            prerequisites: vec![],
            cost: ResearchCost::Items(vec![(iron_ingot(), 10)]),
            unlocks: vec![Unlock::Building(BuildingTypeId(1))],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

    tech_tree
        .register(Technology {
            id: workshops_id,
            name: "Workshops".to_string(),
            prerequisites: vec![basic_smelting_id],
            cost: ResearchCost::Items(vec![(iron_gear_b(), 20), (copper_wire(), 10)]),
            unlocks: vec![Unlock::Building(BuildingTypeId(2))],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

    tech_tree
        .register(Technology {
            id: machine_shops_id,
            name: "Machine Shops".to_string(),
            prerequisites: vec![workshops_id],
            cost: ResearchCost::Items(vec![(motor(), 15)]),
            unlocks: vec![Unlock::Building(BuildingTypeId(3))],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

    tech_tree
        .register(Technology {
            id: industrial_id,
            name: "Industrial".to_string(),
            prerequisites: vec![machine_shops_id],
            cost: ResearchCost::Items(vec![(steel(), 10), (circuit_board(), 10)]),
            unlocks: vec![Unlock::Building(BuildingTypeId(4))],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

    tech_tree
        .register(Technology {
            id: manufacturing_id,
            name: "Manufacturing".to_string(),
            prerequisites: vec![industrial_id],
            cost: ResearchCost::Items(vec![(computer(), 5)]),
            unlocks: vec![Unlock::Building(BuildingTypeId(5))],
            repeatable: false,
            cost_scaling: None,
        })
        .unwrap();

    // Verify prerequisites work: can't start workshops before basic_smelting.
    assert!(
        tech_tree.start_research(workshops_id, 0).is_err(),
        "should not be able to start workshops before basic_smelting"
    );

    // Start basic_smelting.
    tech_tree.start_research(basic_smelting_id, 0).unwrap();

    // Run the factory, periodically contribute items to research.
    let mut completed_techs: Vec<TechId> = Vec::new();
    let mut current_research: Option<TechId> = Some(basic_smelting_id);
    let research_order = [
        basic_smelting_id,
        workshops_id,
        machine_shops_id,
        industrial_id,
        manufacturing_id,
    ];
    let mut research_idx = 0;

    for tick in 1..=2000u64 {
        engine.step();

        // Every 10 ticks, contribute items to current research.
        if tick % 10 == 0
            && let Some(tech_id) = current_research
        {
            // Contribute a generous amount — the factory should be producing enough.
            let contributions: Vec<(ItemTypeId, u32)> = match tech_id {
                t if t == basic_smelting_id => vec![(iron_ingot(), 5)],
                t if t == workshops_id => vec![(iron_gear_b(), 5), (copper_wire(), 5)],
                t if t == machine_shops_id => vec![(motor(), 5)],
                t if t == industrial_id => vec![(steel(), 5), (circuit_board(), 5)],
                t if t == manufacturing_id => vec![(computer(), 5)],
                _ => vec![],
            };

            // Ignore errors (might not have enough yet).
            let _ = tech_tree.contribute_items(tech_id, &contributions, tick);
        }

        // Check for completed research.
        let events = tech_tree.drain_events();
        for event in events {
            if let TechEvent::ResearchCompleted { tech_id, .. } = event {
                completed_techs.push(tech_id);
                research_idx += 1;

                // Start next research if available.
                if research_idx < research_order.len() {
                    let next = research_order[research_idx];
                    let _ = tech_tree.start_research(next, tick);
                    current_research = Some(next);
                } else {
                    current_research = None;
                }
            }
        }
    }

    // Verify at least basic_smelting completed (10 iron ingots is trivial).
    assert!(
        completed_techs.contains(&basic_smelting_id),
        "basic_smelting should have completed"
    );

    // Verify techs completed in order (each should appear before the next).
    for window in completed_techs.windows(2) {
        let expected_order: Vec<usize> = research_order
            .iter()
            .enumerate()
            .filter(|&(_, &t)| t == window[0] || t == window[1])
            .map(|(i, _)| i)
            .collect();
        if expected_order.len() == 2 {
            assert!(
                expected_order[0] < expected_order[1],
                "tech {:?} should complete before {:?}",
                window[0],
                window[1]
            );
        }
    }

    // Verify all unlocks accumulated.
    let all_unlocks = tech_tree.all_unlocks();
    assert!(
        all_unlocks.len() >= completed_techs.len(),
        "should have at least one unlock per completed tech"
    );
}

#[test]
fn stats_tracking_and_bottleneck_detection() {
    let (mut engine, nodes) = build_builderment_factory();

    // Set up stats module.
    let mut stats = ProductionStats::new(StatsConfig {
        window_size: 50,
        history_capacity: 10,
    });

    // Collect events via passive listener into a shared buffer.
    let event_buffer: Rc<RefCell<Vec<Event>>> = Rc::new(RefCell::new(Vec::new()));

    // Register passive listeners for all event types we care about.
    let kinds = [
        EventKind::ItemProduced,
        EventKind::ItemConsumed,
        EventKind::BuildingStalled,
        EventKind::BuildingResumed,
        EventKind::ItemDelivered,
        EventKind::TransportFull,
    ];
    for kind in kinds {
        let buf = Rc::clone(&event_buffer);
        engine.on_passive(
            kind,
            Box::new(move |event| {
                buf.borrow_mut().push(event.clone());
            }),
        );
    }

    // Run 500 ticks, feeding events to stats after each step.
    for tick in 1..=500u64 {
        engine.step();

        // Drain collected events and feed to stats.
        let events = event_buffer.borrow().clone();
        event_buffer.borrow_mut().clear();
        for event in &events {
            stats.process_event(event);
        }
        stats.end_tick(tick);
    }

    // Assertion 1: Iron Furnace should show non-zero production rate.
    let iron_ingot_rate = stats.get_production_rate(nodes.iron_furnace, iron_ingot());
    assert!(
        iron_ingot_rate > Fixed64::from_num(0),
        "iron furnace should have non-zero production rate, got {iron_ingot_rate}"
    );

    // Assertion 2: Total iron ingot production should exceed total copper wire production
    // (due to the 3:1 bottleneck at Wire Workshop).
    let total_iron_ingot = stats.get_total_production(iron_ingot());
    let total_copper_wire = stats.get_total_production(copper_wire());
    assert!(
        total_iron_ingot > total_copper_wire,
        "iron ingot production ({total_iron_ingot}) should exceed copper wire ({total_copper_wire}) due to 3:1 bottleneck"
    );

    // Assertion 3: Stats should be tracking our nodes.
    assert!(
        stats.tracked_node_count() > 0,
        "stats should track production nodes"
    );
}
