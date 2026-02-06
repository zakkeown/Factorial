//! Multiplayer desync detection: determinism verification.
//!
//! Creates two identical engines, applies identical operations to both,
//! and verifies their state hashes match. Then applies a divergent
//! operation to one engine and shows the hashes diverge.
//!
//! Run with: `cargo run -p factorial-core --example multiplayer_desync`

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::item::Inventory;
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::transport::*;

/// Apply identical factory setup to an engine. Returns (mine, smelter) node IDs.
fn setup_factory(engine: &mut Engine) -> (NodeId, NodeId) {
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
                quantity: 2,
            }],
            outputs: vec![RecipeOutput {
                item_type: ItemTypeId(1),
                quantity: 1,
            }],
            duration: 4,
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

    (mine, smelter)
}

fn main() {
    // --- Step 1: Create two identical engines ---

    let mut engine_a = Engine::new(SimulationStrategy::Tick);
    let mut engine_b = Engine::new(SimulationStrategy::Tick);

    let (_mine_a, _smelter_a) = setup_factory(&mut engine_a);
    let (_mine_b, smelter_b) = setup_factory(&mut engine_b);

    // --- Step 2: Run both for 10 ticks ---

    println!("Running 10 identical ticks on both engines...\n");

    for tick in 0..10 {
        engine_a.step();
        engine_b.step();

        if (tick + 1) % 5 == 0 {
            println!(
                "  Tick {}: A hash={}, B hash={}",
                tick + 1,
                engine_a.state_hash(),
                engine_b.state_hash()
            );
        }
    }

    let hash_a = engine_a.state_hash();
    let hash_b = engine_b.state_hash();

    println!("\nAfter 10 identical ticks:");
    println!("  Engine A hash: {}", hash_a);
    println!("  Engine B hash: {}", hash_b);
    assert_eq!(hash_a, hash_b, "identical inputs produce identical state");
    println!("  MATCH: hashes are identical.\n");

    // --- Step 3: Apply a divergent operation to engine B only ---

    // Add a speed modifier to the smelter in engine B.
    engine_b.set_modifiers(
        smelter_b,
        vec![Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(Fixed64::from_num(2)),
            stacking: StackingRule::default(),
        }],
    );

    println!("Applied 2x speed modifier to engine B's smelter.\n");

    // --- Step 4: Run both for 5 more ticks ---

    println!("Running 5 more ticks...\n");

    for tick in 10..15 {
        engine_a.step();
        engine_b.step();

        println!(
            "  Tick {}: A hash={}, B hash={}",
            tick + 1,
            engine_a.state_hash(),
            engine_b.state_hash()
        );
    }

    let hash_a = engine_a.state_hash();
    let hash_b = engine_b.state_hash();

    println!("\nAfter divergent operation:");
    println!("  Engine A hash: {}", hash_a);
    println!("  Engine B hash: {}", hash_b);
    assert_ne!(hash_a, hash_b, "divergent inputs produce different state");
    println!("  DIVERGED: hashes differ, desync detected!");
}
