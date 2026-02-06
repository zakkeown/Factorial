//! Production chain example: iron ore -> iron plate -> iron gear.
//!
//! Demonstrates a 3-node chain with two processing steps,
//! flow transports between each step, and modifier usage.
//!
//! Run with: `cargo run -p factorial-core --example production_chain`

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::item::Inventory;
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::transport::*;

fn main() {
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // --- Create 3 nodes: Mine -> Smelter -> Assembler ---

    let p_mine = engine.graph.queue_add_node(BuildingTypeId(0));
    let p_smelter = engine.graph.queue_add_node(BuildingTypeId(1));
    let p_assembler = engine.graph.queue_add_node(BuildingTypeId(2));
    let r = engine.graph.apply_mutations();
    let mine = r.resolve_node(p_mine).unwrap();
    let smelter = r.resolve_node(p_smelter).unwrap();
    let assembler = r.resolve_node(p_assembler).unwrap();

    // --- Connect: mine->smelter, smelter->assembler ---

    let p_belt1 = engine.graph.queue_connect(mine, smelter);
    let p_belt2 = engine.graph.queue_connect(smelter, assembler);
    let r = engine.graph.apply_mutations();
    let belt1 = r.resolve_edge(p_belt1).unwrap();
    let belt2 = r.resolve_edge(p_belt2).unwrap();

    // --- Configure processors ---

    // Mine: produces 3 iron ore per tick.
    engine.set_processor(
        mine,
        Processor::Source(SourceProcessor {
            output_type: ItemTypeId(0), // iron ore
            base_rate: Fixed64::from_num(3),
            depletion: Depletion::Infinite,
            accumulated: Fixed64::from_num(0),
            initial_properties: None,
        }),
    );

    // Smelter: 1 iron ore -> 1 iron plate, 3 ticks.
    engine.set_processor(
        smelter,
        Processor::Fixed(FixedRecipe {
            inputs: vec![RecipeInput {
                item_type: ItemTypeId(0), // iron ore
                quantity: 1,
            }],
            outputs: vec![RecipeOutput {
                item_type: ItemTypeId(1), // iron plate
                quantity: 1,
            }],
            duration: 3,
        }),
    );

    // Assembler: 2 iron plates -> 1 iron gear, 5 ticks.
    engine.set_processor(
        assembler,
        Processor::Fixed(FixedRecipe {
            inputs: vec![RecipeInput {
                item_type: ItemTypeId(1), // iron plate
                quantity: 2,
            }],
            outputs: vec![RecipeOutput {
                item_type: ItemTypeId(2), // iron gear
                quantity: 1,
            }],
            duration: 5,
        }),
    );

    // Apply a 1.5x speed modifier to the assembler.
    engine.set_modifiers(
        assembler,
        vec![Modifier {
            id: ModifierId(0),
            kind: ModifierKind::Speed(Fixed64::from_num(1.5)),
            stacking: StackingRule::default(),
        }],
    );

    // --- Set up inventories ---

    for node in [mine, smelter, assembler] {
        engine.set_input_inventory(node, Inventory::new(1, 1, 100));
        engine.set_output_inventory(node, Inventory::new(1, 1, 100));
    }

    // --- Set up transports ---

    let flow = |rate: f64| {
        Transport::Flow(FlowTransport {
            rate: Fixed64::from_num(rate),
            buffer_capacity: Fixed64::from_num(100),
            latency: 0,
        })
    };

    engine.set_transport(belt1, flow(5.0));
    engine.set_transport(belt2, flow(5.0));

    // --- Run 30 ticks ---

    println!("Running 30-tick production chain: ore -> plate -> gear\n");

    for tick in 0..30 {
        engine.step();

        if (tick + 1) % 5 == 0 {
            println!("=== After tick {} ===", tick + 1);
            for snap in engine.snapshot_all_nodes() {
                let name = match snap.building_type.0 {
                    0 => "Mine",
                    1 => "Smelter",
                    2 => "Assembler",
                    _ => "Unknown",
                };
                println!(
                    "  {}: state={:?}, inputs={:?}, outputs={:?}",
                    name, snap.processor_state, snap.input_contents, snap.output_contents
                );
            }
            println!();
        }
    }

    println!("Final state hash: {}", engine.state_hash());
}
