# Place Buildings on a Grid

**Goal:** Place buildings with footprints on a 2D grid, detect collisions, query neighbors, and rotate footprints.
**Prerequisites:** [Spatial Grid & Blueprints](../modules/spatial.md), [The Production Graph](../core-concepts/production-graph.md)
**Example:** `crates/factorial-examples/examples/spatial_blueprints.rs`

## Steps

### 1. Create nodes and a spatial index

```rust
let mut engine = Engine::new(SimulationStrategy::Tick);
let mut spatial = SpatialIndex::new();

let p0 = engine.graph.queue_add_node(BuildingTypeId(0)); // furnace (2x2)
let p1 = engine.graph.queue_add_node(BuildingTypeId(1)); // assembler (3x3)
let p2 = engine.graph.queue_add_node(BuildingTypeId(2)); // belt (1x1)
let r = engine.graph.apply_mutations();
let furnace = r.resolve_node(p0).unwrap();
let assembler = r.resolve_node(p1).unwrap();
let belt = r.resolve_node(p2).unwrap();
```

The `SpatialIndex` is a side-car module, separate from the [production graph](../introduction/glossary.md#production-graph). [Nodes](../introduction/glossary.md#node) are created in the graph first, then placed on the grid.

### 2. Place buildings with footprints

```rust
let furnace_fp = BuildingFootprint { width: 2, height: 2 };
spatial.place(furnace, GridPosition::new(0, 0), furnace_fp).expect("place furnace");

spatial.place(belt, GridPosition::new(2, 0), BuildingFootprint::single()).expect("place belt");

let asm_fp = BuildingFootprint { width: 3, height: 3 };
spatial.place(assembler, GridPosition::new(3, 0), asm_fp).expect("place assembler");
```

Each building occupies grid cells based on its footprint. `BuildingFootprint::single()` is a shorthand for a 1x1 building. The position is the top-left corner of the footprint.

### 3. Detect collisions

```rust
let lab_fp = BuildingFootprint { width: 2, height: 3 };
let collision = spatial.place(lab, GridPosition::new(0, 0), lab_fp);
assert!(collision.is_err()); // blocked by the furnace

let is_free = spatial.can_place(GridPosition::new(0, 3), lab_fp);
assert!(is_free); // this position is clear

spatial.place(lab, GridPosition::new(0, 3), lab_fp).expect("place lab");
```

`place()` returns an error if any cell in the footprint is already occupied. Use `can_place()` for a non-destructive check before committing.

### 4. Query the grid

```rust
let node_at_origin = spatial.node_at(GridPosition::new(0, 0)); // returns Some(furnace)
let furnace_pos = spatial.get_position(furnace);                // returns Some(GridPosition(0, 0))
let furnace_fp = spatial.get_footprint(furnace);                // returns Some(BuildingFootprint { 2, 2 })

let is_occupied = spatial.is_occupied(GridPosition::new(0, 0)); // true
let is_empty = spatial.is_occupied(GridPosition::new(10, 10));  // false
```

### 5. Search by radius and adjacency

```rust
let nearby = spatial.nodes_in_radius(GridPosition::new(1, 1), 3);
// Returns all nodes whose footprint is within Manhattan distance 3

let neighbors = spatial.neighbors_4(belt);
// Returns 4-directional neighbors: (Direction, NodeId) pairs
```

### 6. Rotate footprints

```rust
let original = BuildingFootprint { width: 2, height: 3 };
let rotated = original.rotated(Rotation::Cw90);
// rotated = BuildingFootprint { width: 3, height: 2 }

for rotation in Rotation::all() {
    let r = original.rotated(rotation);
    // None: 2x3, Cw90: 3x2, Cw180: 2x3, Cw270: 3x2
}
```

## What's Happening

The spatial index maintains a cell-level occupancy map. When a building is placed, every cell in its footprint is marked with the building's `NodeId`. Collision detection is an O(w*h) scan of the footprint cells. Point queries (`node_at`, `is_occupied`) are O(1) lookups. Radius queries scan a bounding box and filter by Manhattan distance. Adjacency checks the four cardinal cells adjacent to the building's footprint boundary.

## Variations

- **Blueprint stamps:** Pre-define layouts of multiple buildings with relative positions, then place them as a group with a single collision check.
- **Removal:** Call `spatial.remove(node_id)` to free the cells, then `engine.graph.queue_remove_node()` to remove from the production graph.
- **Terrain layers:** Use multiple `SpatialIndex` instances for different layers (ground buildings, underground belts, elevated rails).
- **Auto-connect:** After placing, use `neighbors_4()` to find adjacent buildings and automatically create [edges](../introduction/glossary.md#edge) in the production graph.
