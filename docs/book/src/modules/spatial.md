# Spatial Grid & Blueprints

The `factorial-spatial` crate provides a 2D grid-based spatial index that maps
grid positions to [production graph](../core-concepts/production-graph.md)
nodes. It supports multi-tile buildings, collision detection, adjacency queries,
area searches, and a blueprint/ghost placement system for staging builds before
committing them.

## Key concepts

- Every placed building is identified by a
  [`NodeId`](../introduction/glossary.md#nodeid) from the production graph.
- The index maintains a bidirectional mapping: position-to-node and
  node-to-position/footprint.
- Multi-tile buildings occupy multiple grid cells but resolve to a single
  `NodeId`.
- The **blueprint** system lets you preview a layout as ghost tiles, validate
  placement, and commit atomically.

## Core types

### GridPosition

```rust
use factorial_spatial::GridPosition;

let pos = GridPosition::new(5, 10);
let dist = pos.manhattan_distance(&GridPosition::new(8, 14)); // 7
let cheb = pos.chebyshev_distance(&GridPosition::new(8, 14)); // 4
```

`GridPosition` is a signed 2D integer coordinate. It provides Manhattan and
Chebyshev distance helpers.

### BuildingFootprint

```rust
use factorial_spatial::BuildingFootprint;

let fp = BuildingFootprint { width: 3, height: 2 };
let single = BuildingFootprint::single(); // 1x1
```

A footprint defines the width and height of a building in grid cells. The origin
(top-left corner) is the position passed to `place()`.

`BuildingFootprint::tiles(origin)` returns an iterator over every
`GridPosition` the footprint occupies at the given origin.

### Rotation

```rust
use factorial_spatial::Rotation;

let rotated_fp = fp.rotated(Rotation::Cw90); // swaps width and height
```

`Rotation` has four variants: `None`, `Cw90`, `Cw180`, `Cw270`. For 90-degree
and 270-degree rotations, `rotated()` swaps width and height. Rotations can be
chained:

```rust
let mut rot = Rotation::None;
rot = rot.rotate_cw();  // Cw90
rot = rot.rotate_cw();  // Cw180
rot = rot.rotate_ccw(); // back to Cw90
```

## Creating the spatial index

```rust
use factorial_spatial::SpatialIndex;

let mut spatial = SpatialIndex::new();
```

## Placing and removing buildings

### place

```rust
spatial.place(furnace_node, GridPosition::new(0, 0), BuildingFootprint { width: 2, height: 2 })?;
```

`place()` checks that:

1. The node is not already placed (`AlreadyPlaced` error).
2. Every tile in the footprint is free (`Occupied` error).

On success, all tiles are mapped to the node.

### remove

```rust
let origin: GridPosition = spatial.remove(furnace_node)?;
```

Returns the origin position and frees all occupied tiles. Returns
`NotPlaced` if the node is not on the grid.

### can_place

```rust
let ok: bool = spatial.can_place(GridPosition::new(0, 0), footprint);
```

Non-mutating check. Returns `true` if every tile in the footprint is
unoccupied.

### is_occupied

```rust
let occupied: bool = spatial.is_occupied(GridPosition::new(0, 0));
```

## Point queries

```rust
let node: Option<NodeId> = spatial.node_at(GridPosition::new(0, 0));
let pos:  Option<GridPosition> = spatial.get_position(furnace_node);
let fp:   Option<BuildingFootprint> = spatial.get_footprint(furnace_node);
```

## Area queries

### nodes_in_radius

```rust
let nearby: Vec<NodeId> = spatial.nodes_in_radius(GridPosition::new(5, 5), 3);
```

Returns all unique nodes within the given Manhattan distance from the center.
Multi-tile buildings are deduplicated.

### nodes_in_rect

```rust
let nodes: Vec<NodeId> = spatial.nodes_in_rect(
    GridPosition::new(0, 0),   // min corner
    GridPosition::new(10, 10), // max corner (inclusive)
);
```

Returns all unique nodes with at least one tile inside the axis-aligned
rectangle.

## Adjacency queries

### neighbors_4

```rust
let neighbors: Vec<(Direction, NodeId)> = spatial.neighbors_4(belt_node);
```

Returns 4-directional (cardinal) neighbors. For multi-tile buildings, all edge
tiles are checked. Results include the direction from which each neighbor was
found. `Direction` has variants `North`, `East`, `South`, `West`.

### neighbors_8

```rust
let neighbors: Vec<NodeId> = spatial.neighbors_8(belt_node);
```

Returns 8-directional neighbors (including diagonals). Unlike `neighbors_4`,
no direction information is returned.

### neighbor_in_direction

```rust
let east: Option<NodeId> = spatial.neighbor_in_direction(belt_node, Direction::East);
```

Returns the first neighbor found in the specified direction, or `None`.

## Blueprint system

The `Blueprint` struct stages a set of buildings and connections as **ghost
tiles** that can be previewed before being committed to the engine and spatial
index atomically.

### Creating and populating a blueprint

```rust
use factorial_spatial::{Blueprint, BlueprintEntry, BlueprintNodeRef};
use factorial_core::processor::Processor;
use factorial_core::transport::Transport;
use factorial_core::id::BuildingTypeId;

let mut bp = Blueprint::new();

let entry_id = bp.add(BlueprintEntry {
    building_type: BuildingTypeId(0),
    position: GridPosition::new(10, 10),
    footprint: BuildingFootprint { width: 2, height: 2 },
    rotation: Rotation::None,
    processor: Processor::default(),
    input_capacity: 10,
    output_capacity: 10,
}, &spatial)?;
```

`add()` validates against both the real spatial index and existing ghost tiles,
returning `OverlapsExisting` or `OverlapsPlanned` on collision.

### Querying ghosts

```rust
bp.is_ghost_at(GridPosition::new(10, 10));   // true
bp.ghost_at(GridPosition::new(10, 10));      // Some(BlueprintEntryId)
bp.get(entry_id);                            // Option<&BlueprintEntry>
```

### Moving and removing entries

```rust
bp.move_entry(entry_id, GridPosition::new(20, 20), &spatial)?;
bp.remove(entry_id)?;
```

`move_entry` atomically relocates an entry, rolling back to the original
position if the new location is blocked. `remove` clears ghost tiles and any
connections referencing the entry.

### Connections

```rust
bp.connect(
    BlueprintNodeRef::Planned(entry_a),
    BlueprintNodeRef::Planned(entry_b),
    Transport::default(),
    None, // no item filter
);
```

Connections can reference both `Planned` entries (inside the blueprint) and
`Existing` nodes (already committed to the graph).

### Committing a blueprint

Committing materializes every entry and connection into the engine and spatial
index. On success it returns a `BlueprintCommitResult` containing a map from
`BlueprintEntryId` to the newly created `NodeId`, plus edge IDs for all
connections. The commit result provides an `undo_record()` for rollback.

## Full example

The following excerpt from
`crates/factorial-examples/examples/spatial_blueprints.rs` places buildings,
detects collisions, and queries adjacency.

```rust
use factorial_core::engine::Engine;
use factorial_core::id::BuildingTypeId;
use factorial_core::sim::SimulationStrategy;
use factorial_spatial::*;

let mut engine = Engine::new(SimulationStrategy::Tick);
let mut spatial = SpatialIndex::new();

let p0 = engine.graph.queue_add_node(BuildingTypeId(0));
let p1 = engine.graph.queue_add_node(BuildingTypeId(1));
let p2 = engine.graph.queue_add_node(BuildingTypeId(2));
let r = engine.graph.apply_mutations();

let furnace   = r.resolve_node(p0).unwrap();
let assembler = r.resolve_node(p1).unwrap();
let belt      = r.resolve_node(p2).unwrap();

// Place a 2x2 furnace at the origin
spatial.place(furnace, GridPosition::new(0, 0), BuildingFootprint { width: 2, height: 2 }).unwrap();

// Place a 1x1 belt adjacent to the furnace
spatial.place(belt, GridPosition::new(2, 0), BuildingFootprint::single()).unwrap();

// Place a 3x3 assembler further right
spatial.place(assembler, GridPosition::new(3, 0), BuildingFootprint { width: 3, height: 3 }).unwrap();

// Collision detection
assert!(spatial.is_occupied(GridPosition::new(0, 0)));
assert!(!spatial.can_place(GridPosition::new(0, 0), BuildingFootprint { width: 2, height: 2 }));
assert!(spatial.can_place(GridPosition::new(0, 3), BuildingFootprint { width: 2, height: 3 }));

// Radius query
let nearby = spatial.nodes_in_radius(GridPosition::new(1, 1), 3);
println!("Buildings within radius 3: {}", nearby.len());

// 4-directional adjacency
let neighbors = spatial.neighbors_4(belt);
for (dir, neighbor) in &neighbors {
    let pos = spatial.get_position(*neighbor).unwrap();
    println!("{dir:?}: node at {pos:?}");
}

// Rotation
let fp = BuildingFootprint { width: 2, height: 3 };
for rot in Rotation::all() {
    let rotated = fp.rotated(rot);
    println!("{rot:?}: {}x{}", rotated.width, rotated.height);
}
```

## Statistics

```rust
spatial.node_count()  // number of unique placed nodes
spatial.tile_count()  // total number of occupied tiles
```
