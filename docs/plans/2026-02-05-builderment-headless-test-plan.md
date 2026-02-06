# Builderment Headless Test Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a comprehensive headless integration test that models the full Builderment production chain (raw ore → Super Computer) with 28 nodes, 30 edges, and cross-crate integration testing power, tech tree, and stats modules.

**Architecture:** A new integration test crate (`tests/builderment_headless.rs`) in factorial-core that defines Builderment item types, recipes, and factory graph using the existing test_utils helpers. Cross-crate tests live in a separate workspace-level test crate since they need factorial-power, factorial-tech-tree, and factorial-stats as dependencies.

**Tech Stack:** Rust, factorial-core (Engine, Processor, Transport, Event), factorial-power (PowerModule), factorial-tech-tree (TechTree), factorial-stats (ProductionStats)

**Reference docs:**
- Design: `docs/plans/2026-02-05-builderment-headless-test-design.md`
- Story: `docs/stories/builderment.md`
- Existing integration tests: `crates/factorial-core/tests/integration.rs`
- Test utils: `crates/factorial-core/src/test_utils.rs`

---

## Task 1: Add Builderment Item Type Constants to test_utils

**Files:**
- Modify: `crates/factorial-core/src/test_utils.rs`

**Step 1: Add item type constructors after the existing ones (line ~43)**

Add after the existing `hydrogen()` function:

```rust
// =========================================================================
// Builderment item types
// =========================================================================

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
```

Note: Use `iron_gear_b()` to avoid colliding with the existing `gear()` helper (ItemTypeId(2)) used by other tests.

**Step 2: Run existing tests to verify no breakage**

Run: `cargo test -p factorial-core`
Expected: All existing tests PASS (we only added new functions)

**Step 3: Commit**

```bash
git add crates/factorial-core/src/test_utils.rs
git commit -m "feat: add Builderment item type constants to test_utils"
```

---

## Task 2: Create Builderment Factory Builder Function

**Files:**
- Create: `crates/factorial-core/tests/builderment_headless.rs`

**Step 1: Write the test file with factory builder and a basic smoke test**

```rust
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

/// Inventory capacity for standard buildings.
const STD_INPUT_CAP: u32 = 50;
const STD_OUTPUT_CAP: u32 = 50;
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

    let electric_motor_factory = add_node(
        &mut engine,
        make_recipe(
            vec![(motor(), 1), (steel(), 1), (copper_wire(), 1)],
            vec![(electric_motor(), 1)],
            6,
        ),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let circuit_board_factory = add_node(
        &mut engine,
        make_recipe(
            vec![(glass(), 1), (copper_wire(), 1), (steel(), 1)],
            vec![(circuit_board(), 1)],
            6,
        ),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let basic_robot_factory = add_node(
        &mut engine,
        make_recipe(
            vec![(wood_frame(), 1), (motor(), 1), (light_bulb(), 1)],
            vec![(basic_robot(), 1)],
            6,
        ),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );

    // =====================================================================
    // Layer 4: Manufacturers
    // =====================================================================

    let computer_mfr = add_node(
        &mut engine,
        make_recipe(
            vec![(circuit_board(), 1), (electric_motor(), 1), (steel(), 1), (glass(), 1)],
            vec![(computer(), 1)],
            8,
        ),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
    );
    let super_computer_mfr = add_node(
        &mut engine,
        make_recipe(
            vec![(computer(), 1), (tungsten_carbide(), 1), (electric_motor(), 1), (circuit_board(), 1)],
            vec![(super_computer(), 1)],
            10,
        ),
        STD_INPUT_CAP, STD_OUTPUT_CAP,
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
    assert_eq!(engine.node_count(), 28, "factory should have 28 nodes");
    assert!(engine.edge_count() >= 30, "factory should have at least 30 edges, got {}", engine.edge_count());
}
```

**Step 2: Run the smoke test**

Run: `cargo test -p factorial-core --test builderment_headless factory_builds_successfully -- --nocapture`
Expected: PASS — factory builds with 28 nodes and 30+ edges

**Step 3: Commit**

```bash
git add crates/factorial-core/tests/builderment_headless.rs crates/factorial-core/src/test_utils.rs
git commit -m "feat: add Builderment factory builder and smoke test"
```

---

## Task 3: End-to-End Flow Test — Computers

**Files:**
- Modify: `crates/factorial-core/tests/builderment_headless.rs`

**Step 1: Write the failing test**

Add after the smoke test:

```rust
#[test]
fn full_chain_produces_computers() {
    let (mut engine, nodes) = build_builderment_factory();

    // Run for 500 ticks — enough for items to flow through the deepest chain:
    // Iron Ore → Iron Ingot → Iron Gear → Motor → Electric Motor → Computer
    // (6 production steps + belt transit time between each)
    for _ in 0..500 {
        engine.step();
    }

    // Computers should have reached the sink.
    let computers_at_sink = input_quantity(&engine, nodes.computer_sink, computer());
    assert!(
        computers_at_sink > 0,
        "computer sink should have received computers after 500 ticks, got {computers_at_sink}"
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
```

**Step 2: Run the test**

Run: `cargo test -p factorial-core --test builderment_headless full_chain_produces_computers -- --nocapture`
Expected: PASS — computers flow through the 6-step chain within 500 ticks

If FAIL: Increase tick count or debug which node is starved. The most likely issue is inventory capacity being too small for fan-out nodes.

**Step 3: Commit**

```bash
git add crates/factorial-core/tests/builderment_headless.rs
git commit -m "test: add end-to-end computer production test"
```

---

## Task 4: End-to-End Flow Test — Super Computers

**Files:**
- Modify: `crates/factorial-core/tests/builderment_headless.rs`

**Step 1: Write the test**

```rust
#[test]
fn full_chain_produces_super_computers() {
    let (mut engine, nodes) = build_builderment_factory();

    // Super Computers have the deepest chain:
    // Tungsten Ore → (needs Graphite from Sand+Coal) → Tungsten Carbide
    // + Computer (deep chain) + Electric Motor + Circuit Board
    // Run longer to account for the deep dependency tree.
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
```

**Step 2: Run the test**

Run: `cargo test -p factorial-core --test builderment_headless full_chain_produces_super_computers -- --nocapture`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/factorial-core/tests/builderment_headless.rs
git commit -m "test: add end-to-end super computer production test"
```

---

## Task 5: Basic Robot Parallel Chain Test

**Files:**
- Modify: `crates/factorial-core/tests/builderment_headless.rs`

**Step 1: Write the test**

```rust
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
```

**Step 2: Run**

Run: `cargo test -p factorial-core --test builderment_headless parallel_chain -- --nocapture`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/factorial-core/tests/builderment_headless.rs
git commit -m "test: add basic robot parallel chain test"
```

---

## Task 6: Serialization Round-Trip Test

**Files:**
- Modify: `crates/factorial-core/tests/builderment_headless.rs`

**Step 1: Write the test**

```rust
#[test]
fn serialize_round_trip_full_factory() {
    let (mut engine_straight, _) = build_builderment_factory();
    let (mut engine_split, _) = build_builderment_factory();

    // Run both to tick 250.
    for _ in 0..250 {
        engine_straight.step();
        engine_split.step();
    }

    // Serialize the split engine.
    let serialized = engine_split.serialize().expect("serialize should succeed");
    let mut engine_restored = Engine::deserialize(&serialized).expect("deserialize should succeed");

    // Verify restored engine matches split at tick 250.
    assert_eq!(
        engine_restored.state_hash(),
        engine_split.state_hash(),
        "restored engine should match original at tick 250"
    );

    // Run both remaining 250 ticks.
    for _ in 0..250 {
        engine_straight.step();
        engine_restored.step();
    }

    // Final state hashes must match.
    assert_eq!(
        engine_restored.state_hash(),
        engine_straight.state_hash(),
        "serialized round-trip (250+250) should match straight run (500)"
    );

    assert_eq!(
        engine_restored.sim_state.tick,
        engine_straight.sim_state.tick,
        "tick counts should match"
    );
}
```

**Step 2: Run**

Run: `cargo test -p factorial-core --test builderment_headless serialize_round_trip -- --nocapture`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/factorial-core/tests/builderment_headless.rs
git commit -m "test: add serialization round-trip test for full factory"
```

---

## Task 7: Determinism Test

**Files:**
- Modify: `crates/factorial-core/tests/builderment_headless.rs`

**Step 1: Write the test**

```rust
#[test]
fn determinism_full_factory() {
    fn build_and_run() -> Vec<u64> {
        let (mut engine, _) = build_builderment_factory();
        let mut hashes = Vec::with_capacity(500);
        for _ in 0..500 {
            engine.step();
            hashes.push(engine.state_hash());
        }
        hashes
    }

    let run1 = build_and_run();
    let run2 = build_and_run();

    assert_eq!(run1.len(), run2.len());

    for (tick, (h1, h2)) in run1.iter().zip(run2.iter()).enumerate() {
        assert_eq!(
            h1, h2,
            "state hashes diverged at tick {}: run1={h1}, run2={h2}",
            tick + 1,
        );
    }

    // Verify the simulation actually evolved (not all same hash).
    let unique: std::collections::HashSet<u64> = run1.iter().copied().collect();
    assert!(
        unique.len() > 1,
        "state hashes should change between ticks"
    );
}
```

**Step 2: Run**

Run: `cargo test -p factorial-core --test builderment_headless determinism -- --nocapture`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/factorial-core/tests/builderment_headless.rs
git commit -m "test: add determinism test for full factory"
```

---

## Task 8: Create Cross-Crate Test Crate for Power/TechTree/Stats

The power, tech-tree, and stats crates are separate workspace members. To test their integration with the core engine, we need a workspace-level integration test crate.

**Files:**
- Create: `crates/factorial-integration-tests/Cargo.toml`
- Create: `crates/factorial-integration-tests/tests/builderment_cross_crate.rs`
- Modify: `Cargo.toml` (workspace root)

**Step 1: Create the integration test crate Cargo.toml**

```toml
[package]
name = "factorial-integration-tests"
version = "0.1.0"
edition = "2024"
publish = false

[dev-dependencies]
factorial-core = { path = "../factorial-core", features = ["test-utils"] }
factorial-power = { path = "../factorial-power" }
factorial-tech-tree = { path = "../factorial-tech-tree" }
factorial-stats = { path = "../factorial-stats" }
fixed = { workspace = true }
```

**Step 2: Add to workspace**

In root `Cargo.toml`, add `"crates/factorial-integration-tests"` to the members list:

```toml
[workspace]
resolver = "2"
members = [
    "crates/factorial-core",
    "crates/factorial-ffi",
    "crates/factorial-stats",
    "crates/factorial-tech-tree",
    "crates/factorial-power",
    "crates/factorial-integration-tests",
]
```

**Step 3: Create the cross-crate test file with a smoke test**

```rust
//! Cross-crate Builderment integration tests.
//!
//! Tests integration between factorial-core, factorial-power, factorial-tech-tree,
//! and factorial-stats using the full Builderment factory scenario.

use std::cell::RefCell;
use std::rc::Rc;

use factorial_core::engine::Engine;
use factorial_core::event::{Event, EventKind};
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;
use factorial_core::transport::*;

use factorial_power::{PowerModule, PowerNetworkId, PowerProducer, PowerConsumer};
use factorial_stats::{ProductionStats, StatsConfig};
use factorial_tech_tree::{TechTree, Technology, TechId, ResearchCost, Unlock};

// Re-use the same factory builder from the core tests.
// (Duplicated here since integration tests can't share code across crates easily.)

const STD_INPUT_CAP: u32 = 50;
const STD_OUTPUT_CAP: u32 = 50;
const SINK_INPUT_CAP: u32 = 5000;

fn belt() -> Transport {
    make_item_transport(8)
}

struct FactoryNodes {
    iron_ore_src: NodeId,
    copper_ore_src: NodeId,
    coal_src: NodeId,
    stone_src: NodeId,
    wood_src: NodeId,
    tungsten_ore_src: NodeId,
    iron_furnace: NodeId,
    copper_furnace: NodeId,
    stone_furnace: NodeId,
    glass_furnace: NodeId,
    plank_workshop: NodeId,
    gear_workshop: NodeId,
    wire_workshop: NodeId,
    motor_shop: NodeId,
    wood_frame_shop: NodeId,
    light_bulb_shop: NodeId,
    graphite_shop: NodeId,
    steel_forge: NodeId,
    tc_forge: NodeId,
    electric_motor_factory: NodeId,
    circuit_board_factory: NodeId,
    basic_robot_factory: NodeId,
    computer_mfr: NodeId,
    super_computer_mfr: NodeId,
    computer_sink: NodeId,
    super_computer_sink: NodeId,
    /// All production nodes (excludes sources and sinks).
    all_production: Vec<NodeId>,
}

fn build_builderment_factory() -> (Engine, FactoryNodes) {
    // ... identical to Task 2 factory builder, but also populates all_production ...
    // (Full code provided in implementation — copy from Task 2 and add all_production field)
    todo!("Copy from Task 2 factory builder")
}

#[test]
fn cross_crate_smoke_test() {
    let (engine, _nodes) = build_builderment_factory();
    assert_eq!(engine.node_count(), 28);
}
```

**Step 4: Verify it compiles**

Run: `cargo test -p factorial-integration-tests cross_crate_smoke_test`
Expected: PASS (after filling in the factory builder)

**Step 5: Commit**

```bash
git add crates/factorial-integration-tests/ Cargo.toml
git commit -m "feat: add cross-crate integration test crate"
```

---

## Task 9: Power Brownout and Recovery Test

**Files:**
- Modify: `crates/factorial-integration-tests/tests/builderment_cross_crate.rs`

**Step 1: Write the test**

```rust
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
                !matches!(event, factorial_power::PowerEvent::PowerGridBrownout { .. }),
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
            if matches!(event, factorial_power::PowerEvent::PowerGridBrownout { .. }) {
                saw_brownout = true;
            }
        }
    }
    assert!(saw_brownout, "should have seen a brownout event after removing producer");

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
            if matches!(event, factorial_power::PowerEvent::PowerGridRestored { .. }) {
                saw_restored = true;
            }
        }
    }
    assert!(saw_restored, "should have seen a restored event after re-adding producer");

    let satisfaction = power.satisfaction(network_id).unwrap();
    assert_eq!(
        satisfaction,
        Fixed64::from_num(1),
        "power should be fully satisfied after recovery"
    );
}
```

**Step 2: Run**

Run: `cargo test -p factorial-integration-tests power_brownout -- --nocapture`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/factorial-integration-tests/tests/builderment_cross_crate.rs
git commit -m "test: add power brownout and recovery test"
```

---

## Task 10: Tech Tree Progression Test

**Files:**
- Modify: `crates/factorial-integration-tests/tests/builderment_cross_crate.rs`

**Step 1: Write the test**

```rust
#[test]
fn tech_tree_progression() {
    let (mut engine, nodes) = build_builderment_factory();

    // Set up tech tree with 5 Builderment-style technologies.
    let mut tech_tree = TechTree::new();

    let basic_smelting_id = TechId(0);
    let workshops_id = TechId(1);
    let machine_shops_id = TechId(2);
    let industrial_id = TechId(3);
    let manufacturing_id = TechId(4);

    tech_tree.register(Technology {
        id: basic_smelting_id,
        name: "Basic Smelting".to_string(),
        prerequisites: vec![],
        cost: ResearchCost::Items(vec![(iron_ingot(), 10)]),
        unlocks: vec![Unlock::Building(BuildingTypeId(1))],
        repeatable: false,
        cost_scaling: None,
    }).unwrap();

    tech_tree.register(Technology {
        id: workshops_id,
        name: "Workshops".to_string(),
        prerequisites: vec![basic_smelting_id],
        cost: ResearchCost::Items(vec![(iron_gear_b(), 20), (copper_wire(), 10)]),
        unlocks: vec![Unlock::Building(BuildingTypeId(2))],
        repeatable: false,
        cost_scaling: None,
    }).unwrap();

    tech_tree.register(Technology {
        id: machine_shops_id,
        name: "Machine Shops".to_string(),
        prerequisites: vec![workshops_id],
        cost: ResearchCost::Items(vec![(motor(), 15)]),
        unlocks: vec![Unlock::Building(BuildingTypeId(3))],
        repeatable: false,
        cost_scaling: None,
    }).unwrap();

    tech_tree.register(Technology {
        id: industrial_id,
        name: "Industrial".to_string(),
        prerequisites: vec![machine_shops_id],
        cost: ResearchCost::Items(vec![(steel(), 10), (circuit_board(), 10)]),
        unlocks: vec![Unlock::Building(BuildingTypeId(4))],
        repeatable: false,
        cost_scaling: None,
    }).unwrap();

    tech_tree.register(Technology {
        id: manufacturing_id,
        name: "Manufacturing".to_string(),
        prerequisites: vec![industrial_id],
        cost: ResearchCost::Items(vec![(computer(), 5)]),
        unlocks: vec![Unlock::Building(BuildingTypeId(5))],
        repeatable: false,
        cost_scaling: None,
    }).unwrap();

    // Verify prerequisites work: can't start workshops before basic_smelting.
    assert!(
        tech_tree.start_research(workshops_id, 0).is_err(),
        "should not be able to start workshops before basic_smelting"
    );

    // Start basic_smelting.
    tech_tree.start_research(basic_smelting_id, 0).unwrap();

    // Run the factory, periodically contribute items to research.
    // We'll run 1000 ticks and contribute every 50 ticks from the appropriate
    // output inventories.
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
        if tick % 10 == 0 {
            if let Some(tech_id) = current_research {
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
        }

        // Check for completed research.
        let events = tech_tree.drain_events();
        for event in events {
            if let factorial_tech_tree::TechEvent::ResearchCompleted { tech_id, .. } = event {
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
            .filter(|(_, &t)| t == window[0] || t == window[1])
            .map(|(i, _)| i)
            .collect();
        if expected_order.len() == 2 {
            assert!(
                expected_order[0] < expected_order[1],
                "tech {:?} should complete before {:?}",
                window[0], window[1]
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
```

**Step 2: Run**

Run: `cargo test -p factorial-integration-tests tech_tree_progression -- --nocapture`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/factorial-integration-tests/tests/builderment_cross_crate.rs
git commit -m "test: add tech tree progression test"
```

---

## Task 11: Stats Module Integration Test

**Files:**
- Modify: `crates/factorial-integration-tests/tests/builderment_cross_crate.rs`

**Step 1: Write the test**

```rust
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
        engine.on_passive(kind, Box::new(move |event| {
            buf.borrow_mut().push(event.clone());
        }));
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
```

**Step 2: Run**

Run: `cargo test -p factorial-integration-tests stats_tracking -- --nocapture`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/factorial-integration-tests/tests/builderment_cross_crate.rs
git commit -m "test: add stats tracking and bottleneck detection test"
```

---

## Task 12: Final Review and Cleanup

**Step 1: Run all tests across the workspace**

Run: `cargo test --workspace`
Expected: All tests PASS

**Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets`
Expected: No warnings

**Step 3: Fix any issues from clippy**

**Step 4: Final commit**

```bash
git add -A
git commit -m "chore: fix clippy warnings in Builderment headless tests"
```

**Step 5: Summary verification**

Run: `cargo test -p factorial-core --test builderment_headless -- --list`
Expected output should list:
- `factory_builds_successfully`
- `full_chain_produces_computers`
- `full_chain_produces_super_computers`
- `parallel_chain_produces_basic_robots`
- `serialize_round_trip_full_factory`
- `determinism_full_factory`

Run: `cargo test -p factorial-integration-tests -- --list`
Expected output should list:
- `cross_crate_smoke_test`
- `power_brownout_and_recovery`
- `tech_tree_progression`
- `stats_tracking_and_bottleneck_detection`

---

## Summary

| Task | Description | Tests Added |
|------|-------------|-------------|
| 1 | Add Builderment item type constants | (infrastructure) |
| 2 | Create factory builder + smoke test | `factory_builds_successfully` |
| 3 | End-to-end computers | `full_chain_produces_computers` |
| 4 | End-to-end super computers | `full_chain_produces_super_computers` |
| 5 | Basic robot parallel chain | `parallel_chain_produces_basic_robots` |
| 6 | Serialization round-trip | `serialize_round_trip_full_factory` |
| 7 | Determinism | `determinism_full_factory` |
| 8 | Cross-crate test crate | `cross_crate_smoke_test` |
| 9 | Power brownout/recovery | `power_brownout_and_recovery` |
| 10 | Tech tree progression | `tech_tree_progression` |
| 11 | Stats + bottleneck detection | `stats_tracking_and_bottleneck_detection` |
| 12 | Final review + cleanup | (maintenance) |

**Total: 10 new tests across 2 test files, verifying 28-node factory with cross-crate integration.**
