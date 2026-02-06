//! Minimal factory example: two nodes connected by a transport belt.
//!
//! Creates an iron mine (Source) and an assembler (Fixed recipe),
//! connects them with a flow transport, and runs 10 ticks.
//! After each tick, queries and prints the state.
//!
//! Run with: `cargo run -p factorial-core --example minimal_factory`

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::item::Inventory;
use factorial_core::processor::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::transport::*;

fn main() {
    // Create the engine with tick-based simulation.
    let mut engine = Engine::new(SimulationStrategy::Tick);

    // --- Step 1: Add nodes to the graph ---

    // Queue node additions (applied in batch for determinism).
    let pending_mine = engine.graph.queue_add_node(BuildingTypeId(0));
    let pending_assembler = engine.graph.queue_add_node(BuildingTypeId(1));
    let result = engine.graph.apply_mutations();

    // Resolve pending operations to get actual NodeIds.
    let mine = result
        .resolve_node(pending_mine)
        .expect("mine node created");
    let assembler = result
        .resolve_node(pending_assembler)
        .expect("assembler node created");

    // --- Step 2: Connect nodes ---

    let pending_belt = engine.graph.queue_connect(mine, assembler);
    let result = engine.graph.apply_mutations();
    let belt = result
        .resolve_edge(pending_belt)
        .expect("belt edge created");

    // --- Step 3: Configure processors ---

    // Mine: produces 2 iron ore per tick, infinite supply.
    engine.set_processor(
        mine,
        Processor::Source(SourceProcessor {
            output_type: ItemTypeId(0), // iron ore
            base_rate: Fixed64::from_num(2),
            depletion: Depletion::Infinite,
            accumulated: Fixed64::from_num(0),
            initial_properties: None,
        }),
    );

    // Assembler: 2 iron ore -> 1 iron gear, takes 5 ticks.
    engine.set_processor(
        assembler,
        Processor::Fixed(FixedRecipe {
            inputs: vec![RecipeInput {
                item_type: ItemTypeId(0),
                quantity: 2,
            }],
            outputs: vec![RecipeOutput {
                item_type: ItemTypeId(1), // iron gear
                quantity: 1,
            }],
            duration: 5,
        }),
    );

    // --- Step 4: Set up inventories ---

    // Each node gets input and output inventories.
    // Inventory::new(input_slot_count, output_slot_count, capacity_per_slot)
    engine.set_input_inventory(mine, Inventory::new(1, 1, 100));
    engine.set_output_inventory(mine, Inventory::new(1, 1, 100));
    engine.set_input_inventory(assembler, Inventory::new(1, 1, 100));
    engine.set_output_inventory(assembler, Inventory::new(1, 1, 100));

    // --- Step 5: Configure transport ---

    engine.set_transport(
        belt,
        Transport::Flow(FlowTransport {
            rate: Fixed64::from_num(5), // 5 items/tick throughput
            buffer_capacity: Fixed64::from_num(100),
            latency: 0,
        }),
    );

    // --- Step 6: Run simulation ---

    println!("Running 10 ticks of minimal factory...\n");

    for tick in 0..10 {
        let result = engine.step();

        // Query all node snapshots.
        let snapshots = engine.snapshot_all_nodes();

        println!(
            "=== Tick {} (steps run: {}) ===",
            tick + 1,
            result.steps_run
        );
        for snap in &snapshots {
            println!(
                "  Node {:?} (building {:?}): state={:?}, progress={:.2}",
                snap.id, snap.building_type, snap.processor_state, snap.progress
            );
            if !snap.input_contents.is_empty() {
                println!("    Input:  {:?}", snap.input_contents);
            }
            if !snap.output_contents.is_empty() {
                println!("    Output: {:?}", snap.output_contents);
            }
        }
        println!();
    }

    println!("Final state hash: {}", engine.state_hash());
}
