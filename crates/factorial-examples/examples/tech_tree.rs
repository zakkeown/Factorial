//! Tech tree example: registration, research, and unlocks.
//!
//! Creates a tech tree with three technologies, each with prerequisites
//! and unlock rewards. Demonstrates research progression using the Points
//! cost model, completing technologies, and querying unlocks.
//!
//! Run with: `cargo run -p factorial-examples --example tech_tree`

use factorial_core::id::{BuildingTypeId, RecipeId};
use factorial_tech_tree::*;

fn main() {
    let mut tree = TechTree::new();

    // --- Register technologies ---

    // Tier 1: Basic Smelting (no prerequisites, 100 science points).
    let basic_smelting_id = TechId(0);
    tree.register(Technology {
        id: basic_smelting_id,
        name: "Basic Smelting".to_string(),
        prerequisites: vec![],
        cost: ResearchCost::Points(100),
        unlocks: vec![
            Unlock::Building(BuildingTypeId(1)), // stone furnace
            Unlock::Recipe(RecipeId(0)),          // iron plate recipe
        ],
        repeatable: false,
        cost_scaling: None,
    })
    .expect("register Basic Smelting");

    // Tier 2: Advanced Smelting (requires Basic Smelting, 200 points).
    let advanced_smelting_id = TechId(1);
    tree.register(Technology {
        id: advanced_smelting_id,
        name: "Advanced Smelting".to_string(),
        prerequisites: vec![basic_smelting_id],
        cost: ResearchCost::Points(200),
        unlocks: vec![
            Unlock::Building(BuildingTypeId(2)), // electric furnace
            Unlock::Recipe(RecipeId(1)),          // steel plate recipe
        ],
        repeatable: false,
        cost_scaling: None,
    })
    .expect("register Advanced Smelting");

    // Tier 3: Mining Productivity (repeatable, scales linearly).
    let mining_prod_id = TechId(2);
    tree.register(Technology {
        id: mining_prod_id,
        name: "Mining Productivity".to_string(),
        prerequisites: vec![basic_smelting_id],
        cost: ResearchCost::Points(500),
        unlocks: vec![Unlock::Custom("mining_productivity_bonus".to_string())],
        repeatable: true,
        cost_scaling: Some(CostScaling::Linear {
            base: 500,
            increment: 250,
        }),
    })
    .expect("register Mining Productivity");

    println!("Registered {} technologies.\n", tree.technology_count());

    // --- Research Basic Smelting ---

    println!("=== Researching: Basic Smelting (100 points) ===\n");

    tree.start_research(basic_smelting_id, 0)
        .expect("start Basic Smelting");
    assert!(tree.is_in_progress(basic_smelting_id));
    println!("Research started.");

    // Contribute points in batches.
    let consumed = tree
        .contribute_points(basic_smelting_id, 60, 1)
        .expect("contribute 60 points");
    println!("Contributed {} points (60 offered).", consumed);

    let consumed = tree
        .contribute_points(basic_smelting_id, 60, 2)
        .expect("contribute remaining points");
    println!("Contributed {} points (60 offered).", consumed);

    assert!(tree.is_completed(basic_smelting_id));
    println!("Basic Smelting completed!\n");

    // Drain events.
    let events = tree.drain_events();
    for event in &events {
        println!("  Event: {:?}", event);
    }
    println!();

    // --- Check prerequisites ---

    let can_start_advanced = tree
        .prerequisites_met(advanced_smelting_id)
        .expect("check prerequisites");
    println!(
        "Can start Advanced Smelting? {} (Basic Smelting completed: {})",
        can_start_advanced,
        tree.is_completed(basic_smelting_id)
    );

    // --- Research Advanced Smelting ---

    println!("\n=== Researching: Advanced Smelting (200 points) ===\n");

    tree.start_research(advanced_smelting_id, 3)
        .expect("start Advanced Smelting");

    let consumed = tree
        .contribute_points(advanced_smelting_id, 200, 4)
        .expect("contribute all points");
    println!("Contributed {} points. Completed!", consumed);

    let events = tree.drain_events();
    for event in &events {
        println!("  Event: {:?}", event);
    }
    println!();

    // --- Research Mining Productivity (repeatable) ---

    println!("=== Researching: Mining Productivity (repeatable, 500 base) ===\n");

    // Level 0: costs 500 (base).
    let cost = tree
        .effective_cost(mining_prod_id)
        .expect("get effective cost");
    println!("Level 0 cost: {:?}", cost);

    tree.start_research(mining_prod_id, 5)
        .expect("start Mining Productivity");
    tree.contribute_points(mining_prod_id, 500, 6)
        .expect("complete level 0");
    println!(
        "Level 0 completed! Count: {}",
        tree.completion_count(mining_prod_id)
    );

    // Level 1: costs 500 + 250 = 750.
    let cost = tree
        .effective_cost(mining_prod_id)
        .expect("get effective cost");
    println!("Level 1 cost: {:?}", cost);

    tree.start_research(mining_prod_id, 7)
        .expect("start level 1");
    tree.contribute_points(mining_prod_id, 750, 8)
        .expect("complete level 1");
    println!(
        "Level 1 completed! Count: {}",
        tree.completion_count(mining_prod_id)
    );

    let events = tree.drain_events();
    for event in &events {
        println!("  Event: {:?}", event);
    }

    // --- List all unlocks ---

    println!("\n=== All Unlocks ===\n");
    for unlock in tree.all_unlocks() {
        println!("  {:?}", unlock);
    }

    println!("\nTech tree demo complete.");
}
