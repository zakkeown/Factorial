# Glossary

## Production Graph {#production-graph}

The directed graph of buildings and connections that defines a factory's topology. Nodes represent buildings; edges represent transport connections between them. The engine evaluates the graph in topological order each tick.

**See:** [The Production Graph](../core-concepts/production-graph.md)

## Node {#node}

A vertex in the production graph, representing a building (miner, smelter, assembler, splitter, etc.). Each node has a processor, inventories, and optionally modifiers.

**See:** [The Production Graph](../core-concepts/production-graph.md)

## Edge {#edge}

A connection between two nodes carrying items or fluids. Each edge has a transport strategy that determines how items move.

**See:** [The Production Graph](../core-concepts/production-graph.md)

## Junction {#junction}

A node that routes items without processing them. Splitters divide input across multiple outputs; mergers combine multiple inputs.

**See:** [The Production Graph](../core-concepts/production-graph.md)

## Processor {#processor}

The logic attached to a node that transforms inputs into outputs. Types: Source (generates items), Fixed (recipe-based crafting), Property (transforms item properties), Demand (consumes items), Passthrough (no transformation).

**See:** [Processors](../core-concepts/processors.md)

## Transport Strategy {#transport-strategy}

How items move along an edge. Flow (continuous rate), Item (discrete slots), Batch (periodic bulk), Vehicle (round-trip).

**See:** [Transport Strategies](../core-concepts/transport.md)

## Tick {#tick}

One discrete simulation step. The engine evaluates all nodes in topological order per tick.

**See:** [Determinism & Fixed-Point](../core-concepts/determinism.md)

## Fixed-Point {#fixed-point}

Deterministic number representation (Fixed64 = Q32.32, Fixed32 = Q16.16) used instead of IEEE 754 floats. Guarantees identical results across x86, ARM, and WASM.

**See:** [Determinism & Fixed-Point](../core-concepts/determinism.md)

## UPS {#ups}

Updates Per Second. The simulation tick rate, independent of rendering FPS. Factorial targets 60 UPS at under 8ms per tick for 5,000+ buildings.

**See:** [Performance Model](../architecture/performance.md)

## State Hash {#state-hash}

A deterministic u64 hash of the entire engine state. Two engines with identical inputs produce identical hashes. Used for multiplayer desync detection.

**See:** [Determinism & Fixed-Point](../core-concepts/determinism.md)

## Modifier {#modifier}

A multiplier applied to a node that adjusts its behavior. Types: Speed, Productivity, Efficiency. Stacking rules: Multiplicative, Additive, Diminishing, Capped.

**See:** [Processors](../core-concepts/processors.md)

## Registry {#registry}

The collection of all item types, recipes, and building definitions that define a game's content. Game-specific data, not engine logic.

**See:** [The Production Graph](../core-concepts/production-graph.md)

## Inventory {#inventory}

Input and output item storage attached to a node. Each inventory has slots with configurable capacity. Items flow from output inventories through transport to input inventories.

**See:** [The Production Graph](../core-concepts/production-graph.md)

## BuildingTypeId {#building-type-id}

A numeric identifier (u32) for a type of building in the game's registry. Used when adding nodes to the graph.

**See:** [The Production Graph](../core-concepts/production-graph.md)

## ItemTypeId {#item-type-id}

A numeric identifier (u32) for a type of item. Used in processor recipes, inventories, and transport.

**See:** [Processors](../core-concepts/processors.md)

## Stall {#stall}

When a processor cannot work. Reasons: MissingInputs (input inventory empty), OutputFull (output inventory at capacity), NoPower (power module reports insufficient supply), Depleted (source resource exhausted).

**See:** [Processors](../core-concepts/processors.md)

## Snapshot {#snapshot}

A read-only view of a node or transport's current state. Contains processor state, progress, inventory contents. Used for rendering and UI. Cheap to create, safe to call every frame.

**See:** [Queries](../core-concepts/queries.md)
