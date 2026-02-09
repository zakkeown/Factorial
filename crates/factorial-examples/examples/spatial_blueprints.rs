//! Spatial index example: building placement, collisions, and queries.
//!
//! Creates a spatial index, places buildings with various footprints,
//! checks for collisions, queries by radius and adjacency, and
//! demonstrates rotation of building footprints.
//!
//! Run with: `cargo run -p factorial-examples --example spatial_blueprints`

use factorial_core::engine::Engine;
use factorial_core::id::BuildingTypeId;
use factorial_core::sim::SimulationStrategy;
use factorial_spatial::*;

fn main() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let mut spatial = SpatialIndex::new();

    // --- Create some nodes to place on the grid ---

    let p0 = engine.graph.queue_add_node(BuildingTypeId(0)); // furnace (2x2)
    let p1 = engine.graph.queue_add_node(BuildingTypeId(1)); // assembler (3x3)
    let p2 = engine.graph.queue_add_node(BuildingTypeId(2)); // belt (1x1)
    let p3 = engine.graph.queue_add_node(BuildingTypeId(3)); // lab (2x3)
    let p4 = engine.graph.queue_add_node(BuildingTypeId(4)); // small storage (1x1)
    let r = engine.graph.apply_mutations();

    let furnace = r.resolve_node(p0).unwrap();
    let assembler = r.resolve_node(p1).unwrap();
    let belt = r.resolve_node(p2).unwrap();
    let lab = r.resolve_node(p3).unwrap();
    let small_storage = r.resolve_node(p4).unwrap();

    // --- Place buildings ---

    println!("=== Placing buildings ===\n");

    // Furnace: 2x2 at origin (0, 0).
    let furnace_fp = BuildingFootprint {
        width: 2,
        height: 2,
    };
    spatial
        .place(furnace, GridPosition::new(0, 0), furnace_fp)
        .expect("place furnace");
    println!("Furnace (2x2) placed at (0, 0)");

    // Belt: 1x1 right next to the furnace.
    spatial
        .place(belt, GridPosition::new(2, 0), BuildingFootprint::single())
        .expect("place belt");
    println!("Belt (1x1) placed at (2, 0)");

    // Assembler: 3x3 further right.
    let asm_fp = BuildingFootprint {
        width: 3,
        height: 3,
    };
    spatial
        .place(assembler, GridPosition::new(3, 0), asm_fp)
        .expect("place assembler");
    println!("Assembler (3x3) placed at (3, 0)");

    // Small storage: 1x1 below the furnace.
    spatial
        .place(
            small_storage,
            GridPosition::new(0, 2),
            BuildingFootprint::single(),
        )
        .expect("place small storage");
    println!("Small Storage (1x1) placed at (0, 2)");

    // --- Collision detection ---

    println!("\n=== Collision detection ===\n");

    // Try to place the lab on top of the furnace -- should fail.
    let lab_fp = BuildingFootprint {
        width: 2,
        height: 3,
    };
    let collision = spatial.place(lab, GridPosition::new(0, 0), lab_fp);
    println!(
        "Place lab (2x3) at (0,0): {}",
        if collision.is_err() {
            "BLOCKED (collision)"
        } else {
            "OK"
        }
    );

    // Check if a position is occupied.
    println!(
        "Is (0,0) occupied? {}",
        spatial.is_occupied(GridPosition::new(0, 0))
    );
    println!(
        "Is (10,10) occupied? {}",
        spatial.is_occupied(GridPosition::new(10, 10))
    );

    // Can we place the lab at (0, 3)?
    let can_place = spatial.can_place(GridPosition::new(0, 3), lab_fp);
    println!("Can place lab (2x3) at (0,3)? {}", can_place);

    // Place the lab there.
    spatial
        .place(lab, GridPosition::new(0, 3), lab_fp)
        .expect("place lab at (0,3)");
    println!("Lab (2x3) placed at (0, 3)");

    // --- Point queries ---

    println!("\n=== Point queries ===\n");

    let node_at_origin = spatial.node_at(GridPosition::new(0, 0));
    println!(
        "Node at (0,0): {:?} (furnace: {:?})",
        node_at_origin, furnace
    );

    let furnace_pos = spatial.get_position(furnace);
    println!("Furnace origin: {:?}", furnace_pos);

    let furnace_footprint = spatial.get_footprint(furnace);
    println!("Furnace footprint: {:?}", furnace_footprint);

    // --- Area queries: radius search ---

    println!("\n=== Radius query (Manhattan distance 3 from (1,1)) ===\n");

    let nearby = spatial.nodes_in_radius(GridPosition::new(1, 1), 3);
    println!("Found {} unique buildings within radius 3:", nearby.len());
    for node in &nearby {
        let pos = spatial.get_position(*node).unwrap();
        let fp = spatial.get_footprint(*node).unwrap();
        println!("  Node {:?} at {:?}, footprint {:?}", node, pos, fp);
    }

    // --- Adjacency: 4-directional neighbors ---

    println!("\n=== Adjacency (4-direction neighbors of belt at (2,0)) ===\n");

    let neighbors = spatial.neighbors_4(belt);
    println!("Belt has {} neighbors:", neighbors.len());
    for (dir, neighbor) in &neighbors {
        let pos = spatial.get_position(*neighbor).unwrap();
        println!("  {:?}: node {:?} at {:?}", dir, neighbor, pos);
    }

    // --- Rotation ---

    println!("\n=== Rotation ===\n");

    let original = BuildingFootprint {
        width: 2,
        height: 3,
    };
    println!("Original footprint: {:?}", original);

    for rotation in Rotation::all() {
        let rotated = original.rotated(rotation);
        println!("  {:?}: {:?}", rotation, rotated);
    }

    // Demonstrate rotation chaining.
    let mut rot = Rotation::None;
    print!("\nRotation chain (CW): None");
    for _ in 0..4 {
        rot = rot.rotate_cw();
        print!(" -> {:?}", rot);
    }
    println!();

    println!("\nSpatial index demo complete.");
}
