//! Save/load example: serialization round-trip.
//!
//! Builds a factory, runs 10 ticks, serializes the engine state to bytes,
//! deserializes it into a new engine, and verifies the state hash matches.
//!
//! Run with: `cargo run -p factorial-core --example save_load`

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::item::Inventory;
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::transport::*;

/// Build a small factory: mine -> smelter.
fn build_factory() -> Engine {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    let p_mine = engine.graph.queue_add_node(BuildingTypeId(0));
    let p_smelter = engine.graph.queue_add_node(BuildingTypeId(1));
    let r = engine.graph.apply_mutations();
    let mine = r.resolve_node(p_mine).unwrap();
    let smelter = r.resolve_node(p_smelter).unwrap();

    let p_belt = engine.graph.queue_connect(mine, smelter);
    let r = engine.graph.apply_mutations();
    let belt = r.resolve_edge(p_belt).unwrap();

    engine.set_processor(
        mine,
        Processor::Source(SourceProcessor {
            output_type: ItemTypeId(0),
            base_rate: Fixed64::from_num(3),
            depletion: Depletion::Infinite,
            accumulated: Fixed64::from_num(0),
            initial_properties: None,
        }),
    );

    engine.set_processor(
        smelter,
        Processor::Fixed(FixedRecipe {
            inputs: vec![RecipeInput {
                item_type: ItemTypeId(0),
                quantity: 1,
            }],
            outputs: vec![RecipeOutput {
                item_type: ItemTypeId(1),
                quantity: 1,
            }],
            duration: 3,
        }),
    );

    for node in [mine, smelter] {
        engine.set_input_inventory(node, Inventory::new(1, 1, 100));
        engine.set_output_inventory(node, Inventory::new(1, 1, 100));
    }

    engine.set_transport(
        belt,
        Transport::Flow(FlowTransport {
            rate: Fixed64::from_num(5),
            buffer_capacity: Fixed64::from_num(100),
            latency: 0,
        }),
    );

    engine
}

fn main() {
    // --- Step 1: Build and run ---

    let mut engine = build_factory();

    println!("Running 10 ticks...\n");
    for _ in 0..10 {
        engine.step();
    }

    let hash_before = engine.state_hash();
    println!("State hash before save: {}", hash_before);
    println!("Tick count: {}", engine.sim_state.tick);

    // --- Step 2: Serialize ---

    let bytes = engine.serialize().expect("serialization should succeed");
    println!("Serialized to {} bytes", bytes.len());

    // --- Step 3: Deserialize ---

    let mut restored = Engine::deserialize(&bytes).expect("deserialization should succeed");
    println!("Deserialized successfully");

    // --- Step 4: Verify state hash matches ---

    // The restored engine has the same state but needs a step to recompute
    // the hash (hash is computed during bookkeeping phase). Instead, compare
    // tick counts and snapshots directly.
    println!("Restored tick count: {}", restored.sim_state.tick);
    assert_eq!(
        engine.sim_state.tick, restored.sim_state.tick,
        "tick counts should match"
    );

    // Run one more step on both and compare hashes.
    engine.step();
    restored.step();

    let hash_original = engine.state_hash();
    let hash_restored = restored.state_hash();

    println!("\nAfter one more tick:");
    println!("  Original hash:  {}", hash_original);
    println!("  Restored hash:  {}", hash_restored);

    assert_eq!(
        hash_original, hash_restored,
        "hashes should match after save/load round trip"
    );

    println!("\nSave/load round trip verified successfully.");
}
