# Factorial Engine Design

Factorial is a factory game engine that abstracts the mechanics, math, and optimizations required for factory games, letting game developers focus on UI, rendering, and what else makes their game unique.

This design was informed by deep research into 20 factory games spanning the full spectrum of the genre: Factorio, Satisfactory, Dyson Sphere Program, Shapez, Shapez 2, Oxygen Not Included, Captain of Industry, Mindustry, Builderment, Infinifactory, Big Pharma, Good Company, Rise of Industry, Factory Town, Automation Empire, Production Line, Assembly Line, Voxel Tycoon, Little Big Workshop, and Techtonica. The engine must be configurable and flexible enough that it could conceivably power any of them.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Core Data Model](#2-core-data-model)
3. [Simulation Loop and Strategy](#3-simulation-loop-and-strategy)
4. [Transport Strategies](#4-transport-strategies)
5. [Processor System](#5-processor-system)
6. [Event System](#6-event-system)
7. [Query API](#7-query-api)
8. [Serialization and Snapshots](#8-serialization-and-snapshots)
9. [Performance Architecture](#9-performance-architecture)
10. [Framework Modules](#10-framework-modules)
11. [Integration Architecture](#11-integration-architecture)
12. [Scope Boundaries](#12-scope-boundaries)
13. [Validation Matrix](#13-validation-matrix)

---

## 1. Architecture Overview

Factorial is a three-layer system:

```
+---------------------------------------------+
|           Game Developer's Code              |
|    (UI, rendering, audio, game-specific)     |
+---------------------------------------------+
|          Framework Modules (opt-in)          |
|  Tech Trees | Power Networks | Spatial Grid  |
|  Fluid Networks | Logic Networks | Workers   |
|  Statistics | Player Experience | ...         |
+---------------------------------------------+
|              Factorial Core                  |
|                                              |
|  Production Graph ---- Simulation Strategy   |
|       |                 (tick/delta/event)    |
|       +-- Nodes (buildings + components)     |
|       +-- Edges (transport links)            |
|       +-- Junctions (splitters/mergers)      |
|       +-- Transport Strategy                 |
|       |    (flow/item/batch/vehicle)          |
|       +-- Processors                         |
|       |    (Source/Fixed/Property/Custom)     |
|       +-- Items (typed + optional props)     |
|                                              |
|  Registry --- Serialization --- Events       |
|  (types,       (save/load +     (subscribe/  |
|   recipes,      snapshots)       emit)       |
|   buildings)                                 |
|                                              |
|  Written in Rust | C FFI | WASM target       |
+---------------------------------------------+
```

**Core** is a headless simulation library. It has no opinions about rendering, input, or audio. It owns the production graph, simulates it forward in time, and emits events about what happened. Game devs interact with it through the Rust API directly or through C/language bindings.

**Framework modules** are independent crates that depend on the core but not on each other. A game using Factorial picks which modules it needs. Each module registers its own components, event handlers, and serialization logic with the core.

**Game developer code** handles everything Factorial doesn't: rendering, UI, input, audio, narrative, world generation, and any game-specific mechanics that fall outside factory simulation.

### Language and Runtime

Rust, with a C-style FFI wrapper for universal embedding.

- **Rust-native API** for game devs using Rust (Bevy, etc.)
- **C API wrapper** (`extern "C"`) for universal embedding
- **Generated bindings** from the C API: C# (Unity), GDScript/C++ (Godot), Python (tooling/modding)
- **WASM target** for browser-based factory games and web-based planning tools

No garbage collector. No floating-point in simulation-critical paths. Predictable frame-to-frame performance for tick-based simulation.

### Numeric Representation

All simulation-critical arithmetic uses **fixed-point integers**. This is a foundational decision that affects the entire codebase:

```rust
/// Q32.32 fixed-point: 32 integer bits, 32 fractional bits
/// Range: ±2,147,483,647 with precision to ~0.00000000023
type Fixed64 = i64;

/// Q16.16 fixed-point for compact storage (item properties, etc.)
/// Range: ±32,767 with precision to ~0.000015
type Fixed32 = i32;
```

Why fixed-point over IEEE 754 floats:

- **Cross-platform determinism.** `Fixed64` arithmetic produces identical results on x86, ARM, and WASM. No FMA instruction differences, no extended precision, no compiler reordering.
- **Multiplayer.** Lockstep simulation requires bit-identical results across all clients. Floats cannot guarantee this without per-operation compliance enforcement that is fragile and compiler-dependent.
- **Snapshot/replay integrity.** A replay that diverges due to float rounding is silently wrong and undebuggable.

The `Fixed64` type wraps `i64` and provides `Add`, `Sub`, `Mul`, `Div` with overflow-checked arithmetic. The Rust API exposes ergonomic conversion: `Fixed64::from_f64(1.5)` for initialization, `.to_f64()` for display. The C API uses raw `i64` with documented scaling.

**Where floats are permitted:** Game developer code outside the simulation loop (UI, rendering, interpolation between ticks) can use floats freely. The boundary is the `ProcessContext` and event system -- everything inside the simulation phases uses `Fixed64`.

---

## 2. Core Data Model

The production graph is the heart of Factorial. Everything the core does is defined in terms of nodes, edges, junctions, items, and processors.

### Registry

The registry uses a three-phase lifecycle: **registration**, **mutation**, **finalization**.

```rust
let mut builder = RegistryBuilder::new();

// Phase 1: Registration — add types, recipes, buildings.
// Order does not matter. Modules and mods all register here.
builder.register_item("iron_ore", &[]);
builder.register_item("iron_plate", &[]);
builder.register_building("smelter", BuildingTemplate { ... });

// Phase 2: Mutation — modify anything registered so far.
// Mods can alter recipes, add ingredients to existing buildings,
// change costs. This enables Factorio-style mod interop where
// mod A adds an item and mod B uses it in a recipe.
builder.mutate_recipe("steel_plate", |recipe| {
    recipe.add_input("chromium", 1);
});
builder.mutate_building("smelter", |template| {
    template.add_component(PollutionEmitter { rate: 2 });
});

// Phase 3: Finalization — freeze the registry. Immutable from here.
// The core pre-computes fast paths, validates cross-references,
// and detects errors (missing item types in recipes, etc.).
let registry = builder.build()?;
```

After `build()`, the registry is **immutable**. Item types, building templates, and recipes do not change at runtime. This enables the core to pre-compute fast paths and guarantees determinism. Games that want to "unlock" buildings don't modify the registry; they use the tech tree module to gate what the player can place.

**Data-driven loading.** The `RegistryBuilder` accepts data from any source: Rust code, JSON, TOML, RON, or any format the game developer deserializes into builder calls. The builder API is the universal entry point; the data source is the game developer's choice. This enables recipe iteration without recompiling Rust.

**Runtime product creation.** Games like Good Company where players design products at runtime do NOT create new registry item types. Instead, a "product" is an item type registered at startup (e.g., `"player_product"`) with properties that encode the design configuration. The product's features, quality, and market value are computed from those properties by a `CustomProcessor` at the assembly building.

### Items

Items are defined in the registry with a type ID and optional property declarations:

```rust
// No properties = fungible. Storage is a counter, flow transport is a number.
builder.register_item("iron_plate", &[]);

// Properties declared at the type level, not ad-hoc.
// Property values use Fixed32 or Fixed64.
builder.register_item("water", &[
    Property::new("temperature", Fixed32, default: Fixed32::from(20)),
    Property::new("mass", Fixed32, default: Fixed32::from(1)),
    Property::new("germs", U32, default: 0),
]);

builder.register_item("ingredient", &[
    Property::new("concentration", U8, default: 0),
]);

builder.register_item("iron_plate_quality", &[
    Property::new("quality", Quality, default: Quality::Normal),
]);
```

Fungible items (no properties) get a fast path everywhere: storage is a counter, flow-rate transport is a single number, recipe matching is type-only. This is the common case and should be as cheap as possible.

Stateful items (with properties) are individually tracked. Each instance carries its property values in a **fixed-size struct determined at registration time** (not a runtime property map). All instances of `"water"` have exactly `[Fixed32, Fixed32, U32]` packed contiguously. See [Performance Architecture](#9-performance-architecture) for storage details.

### Buildings

Buildings are registered as templates with a processor and optional components:

```rust
builder.register_building("smelter", BuildingTemplate {
    processor: FixedRecipe {
        inputs: &[("iron_ore", 1)],
        outputs: &[("iron_plate", 1)],
        duration: Duration::from_ticks(60),
    },
    components: &[
        Inventory { input_slots: 1, output_slots: 1, capacity: 50 },
        PowerConsumer { demand: Fixed64::from(90_000) },
    ],
});
```

Buildings use **composition, not inheritance**. A building in the core is a node in the production graph identified by `NodeId`. Capabilities are added through components.

The core ships with `Inventory` and `Processor` as built-in component types. Framework modules register additional component types: `PowerConsumer`, `HeatEmitter`, `WorkerRequirement`, `PollutionSource`, etc. Third-party mods and game code can also register custom component types using the same mechanism. See [Performance Architecture](#9-performance-architecture) for the component storage model.

Example compositions at different complexity levels:

```rust
// Builderment: minimal
building("furnace")
  .processor(FixedRecipe("iron_ore -> iron_ingot", 2s))
  .inventory(inputs: 1, outputs: 1)

// Factorio: moderate
building("assembler_mk3")
  .processor(FixedRecipe(...))
  .inventory(inputs: 4, outputs: 1)
  .power_consumer(375kW)
  .module_slots(4)
  .pollution_emitter(2/min)

// ONI: full complexity
building("electrolyzer")
  .processor(FixedRecipe("water -> oxygen + hydrogen"))
  .power_consumer(120W)
  .heat_emitter(1.25 kDTU/s)
  .overheat_threshold(75C)
  .overpressure_limit(1800g)
  .pipe_connections(input: water, output: [gas, gas])
  .worker_requirement(operating)
  .decor(-10, radius: 2)
```

### Edges

Edges connect an output slot on one node to an input slot on another, with a transport strategy assigned per-edge:

```rust
graph.connect(
    smelter_output(0),
    assembler_input(2),
    Transport::Flow(FlowTransport { capacity: 120_per_min }),
);
```

### Junctions

Junctions model transport elements that mediate between multiple inputs and/or multiple outputs without processing items. They are distinct from nodes (which have processors) and edges (which are point-to-point).

```rust
enum Junction {
    /// Evenly distributes from N inputs to M outputs.
    /// Supports priority output (Factorio priority splitter).
    Splitter {
        inputs: Vec<EdgeId>,
        outputs: Vec<EdgeId>,
        filter: Option<ItemFilter>,
        priority: Option<OutputPriority>,
    },

    /// Merges N inputs into 1 output.
    Merger {
        inputs: Vec<EdgeId>,
        output: EdgeId,
    },

    /// Mediates transfer between an edge and a node with its own
    /// throughput, timing, and filter constraints.
    /// Models Factorio inserters: swing time, stack size, filter.
    Inserter {
        source: EdgeOrNode,
        destination: EdgeOrNode,
        cycle_time: Ticks,
        stack_size: u32,
        filter: Option<ItemFilter>,
    },
}
```

Junctions have their own transport phase processing: the splitter distributes items according to its rules, the merger combines flows, the inserter mediates with timing constraints. The junction is a first-class graph element alongside nodes and edges.

Inserters solve the Factorio problem where items don't teleport from belts into buildings. An inserter grabs from a belt (edge) and places into a building (node) with its own throughput limit and swing time.

### Production Graph

The core models the factory as a graph of nodes, edges, and junctions:

- **Nodes** (buildings) have input rates, output rates, recipes, inventories
- **Edges** (transport links) have capacity and transfer rules
- **Junctions** (splitters, mergers, inserters) mediate multi-way connections
- The core computes: can this node produce? Does the edge have capacity? Update inventories.

The graph interface is purely topological by default. Spatial awareness (grids, positions, adjacency) is provided by an optional framework module that augments the graph with position data.

```rust
// Core defines the graph interface.
// Graph and SimulationStrategy are generic type parameters,
// monomorphized at compile time. See Performance Architecture.
trait ProductionGraph {
    add_node(building) -> NodeId
    remove_node(node) -> Result<()>
    connect(from, to, transport) -> EdgeId
    add_junction(junction) -> JunctionId
    get_inputs(node) -> &[EdgeId]
    get_outputs(node) -> &[EdgeId]
    topological_order() -> &[NodeId]  // cached, invalidated on mutation
}

// Default: pure topology
TopologicalGraph implements ProductionGraph

// Spatial plugin: same interface, adds position awareness
SpatialGraph implements ProductionGraph {
    // all topological methods, plus:
    place_at(node, position) -> Result<()>
    query_radius(position, radius) -> &[NodeId]
    get_neighbors(node) -> &[NodeId]
    check_placement(node, position) -> PlacementResult
    // PlacementResult includes rejection reason if invalid
}
```

Games like Big Pharma and Good Company use `TopologicalGraph` and never think about grids. Games like Factorio and ONI swap in `SpatialGraph` and get placement rules, adjacency bonuses, and area-of-effect systems.

---

## 3. Simulation Loop and Strategy

### Phase Order

Each simulation step follows a fixed phase order. This ordering is critical for determinism: every node processes in the same sequence every time.

```
+------------------------------------------+
|            Simulation Step               |
|                                          |
|  1. Pre-tick phase                       |
|     (apply queued mutations from         |
|      previous post-tick handlers,        |
|      inject player actions)              |
|                                          |
|  2. Transport phase                      |
|     (move items along edges,             |
|      process junctions)                  |
|                                          |
|  3. Process phase                        |
|     (buildings consume inputs,           |
|      advance recipes, produce outputs)   |
|                                          |
|  4. Component phase                      |
|     (power networks balance,             |
|      module-registered systems run)      |
|                                          |
|  5. Post-tick phase                      |
|     (deliver buffered events,            |
|      reactive handlers enqueue           |
|      mutations for next pre-tick)        |
|                                          |
|  6. Bookkeeping                          |
|     (update production statistics,       |
|      throughput counters, utilization)    |
+------------------------------------------+
```

**Node processing order** within each phase defaults to **topological order** (upstream before downstream). The core caches the topological sort and invalidates it when the graph changes. This ensures that a producer's output is available to its downstream consumer within the same tick, eliminating the insertion-order latency bug where build order affects throughput.

Games that need a different ordering (e.g., round-robin for fairness) can override the order via `engine.set_processing_order(OrderingStrategy)`.

### Simulation Strategies

The engine is generic over its simulation strategy: `Engine<G: ProductionGraph, S: SimulationStrategy>`. The strategy is chosen at compile time via the type parameter, not at runtime via dynamic dispatch. All three strategies execute the same phases in the same order:

**Tick mode** — the game calls `engine.step()` at a fixed rate. Each call executes exactly one pass through the phases. Deterministic by construction. This is what Factorio-style games use.

**Delta mode** — the game calls `engine.advance(dt)` with real elapsed time. Internally, the engine accumulates time and runs as many fixed steps as fit, carrying the remainder forward. Same deterministic phases under the hood, just decoupled from the caller's frame rate. **Note:** The remainder is local state. For multiplayer, all clients must use tick mode or synchronize their remainder.

**Event mode** — the game calls `engine.advance_to(target_time)`. The engine looks ahead through the graph, identifies the next time anything actually changes (a recipe completes, a transport delivers), and jumps directly to that moment. Empty time is free. This is how mobile-style games run efficiently on battery, and how any game implements fast-forward or offline progress. **Caveat:** Event mode is incompatible with `ItemTransport`. Belt simulation requires per-tick advancement; it cannot be skipped. Games using `ItemTransport` on any edge must use tick or delta mode, or accept that those edges simulate tick-by-tick even during event-mode jumps.

Because all three strategies execute the same phases in the same order:
- Game devs can switch strategies without behavior changes (subject to the ItemTransport caveat)
- Tests run in tick mode for reproducibility regardless of what the game ships with
- Fast-forward is just "run N ticks" or "advance to time T"

### Determinism Guarantees

The core guarantees deterministic simulation. Same initial state + same inputs = same result, always. This is enforced by:

- **Fixed processing order** for all nodes (topological by default)
- **Fixed-point arithmetic** (`Fixed64`/`Fixed32`) for all simulation-critical values. No IEEE 754 floats in the simulation loop. Cross-platform identical results guaranteed.
- **Canonical evaluation order** for modifier stacking: modifiers are sorted by `ModifierId` (registration order) and applied sequentially. `a * b * c` always evaluates left-to-right in the same order.
- **No hash-map iteration dependencies.** Simulation-critical paths use `Vec` and `SlotMap` with deterministic iteration. `HashMap` is banned in simulation state (enforced by code review and `#[deny]` lints on simulation modules).
- **No GC pauses or JIT reordering** (Rust)
- **RNG state** is serialized and deterministic. `CustomProcessor` callbacks that need randomness MUST use the engine-provided RNG (`context.rng()`), never system random.
- **Module registration order** is part of the determinism contract. Modules must be registered in the same order on all clients. The engine stores a hash of the registration sequence and validates it on deserialization.

### State Hashing

The core computes an incremental state hash for desync detection:

```rust
engine.state_hash() -> u64
```

The hash covers all simulation state: node state, edge state, item instances, RNG state, module state. Each subsystem contributes to the hash independently, enabling per-subsystem comparison when a mismatch is detected:

```rust
engine.subsystem_hashes() -> SubsystemHashes {
    graph: u64,
    transport: u64,
    processors: u64,
    modules: HashMap<&str, u64>,
}
```

Hashing is **incremental**: only dirty entities contribute to a rehash. A dirty flag is set whenever an entity's state changes. The hash is computed during the bookkeeping phase.

For multiplayer, clients exchange `state_hash()` every N ticks (configurable). On mismatch, `subsystem_hashes()` narrows the divergence, and a full state dump enables diffing.

### Graph Mutation Rules

Graph topology (adding/removing nodes, edges, junctions) can be requested at any time but **mutations are queued and applied during the pre-tick phase** of the next simulation step. This ensures:

- No topology change mid-tick disrupts transport or processing
- Processing order is stable for the full tick
- Events emitted during post-tick reference a consistent graph

```rust
// These queue the mutation; they do not take effect immediately.
graph.queue_add_node(building) -> PendingNodeId
graph.queue_remove_node(node)
graph.queue_connect(from, to, transport) -> PendingEdgeId

// Mutations apply at the start of the next engine.step().
```

Determinism enables:
- Multiplayer via input synchronization (Factorio's model)
- Replays
- Reproducible bug reports
- Reliable testing
- Snapshot + input replay for undo

---

## 4. Transport Strategies

Transport moves items along edges between nodes. Each edge has an individually assigned transport strategy. A single game can mix strategies: belts use one, trains use another, drone networks use a third.

### Dispatch Model

Transport strategies use **enum dispatch**, not trait objects. This gives sized inline storage (no heap allocation per edge), predictable branching, and allows the compiler to optimize each variant's hot loop:

```rust
enum Transport {
    Flow(FlowTransport),
    Item(ItemTransport),
    Batch(BatchTransport),
    Vehicle(VehicleTransport),
    /// Escape hatch for game-specific transport.
    /// Only this variant uses dynamic dispatch.
    Custom(Box<dyn TransportStrategy>),
}
```

During the transport phase, edges are **grouped by transport variant** and processed in homogeneous batches. All `Flow` edges run in one tight loop, then all `Item` edges, then all `Batch`, etc. This preserves instruction cache locality and enables the branch predictor to settle into a pattern.

### FlowTransport

The fast path for simple games and fungible items. No individual items tracked.

```rust
FlowTransport {
    capacity: Fixed64,       // items per minute
    latency: Ticks,          // time for items to "arrive"
    merge_fn: Option<MergeFnId>,  // for stateful items: how properties combine
}
```

The edge tracks a rate and a buffer. When a producer outputs 2/tick and the edge capacity allows it, the consumer's input buffer increases by 2/tick after the latency period.

For stateful items (ONI water with temperature), the optional `merge_fn` handles property aggregation when streams combine (e.g., temperature averaging weighted by mass). `MergeFnId` references a function registered in the registry, not a raw function pointer, ensuring determinism.

Covers: Builderment, DSP, Big Pharma, Good Company, Rise of Industry, drone/robot logistics.

### ItemTransport

Full belt simulation with individual item tracking.

```rust
ItemTransport {
    length: u16,             // tiles/slots
    speed: u8,               // slots per tick
    lanes: u8,               // Factorio-style dual lanes
}
```

Belt state is stored externally in a contiguous arena, not inline in the edge struct. See [Performance Architecture](#9-performance-architecture) for the belt storage model.

Every item has a position. Items advance per tick, compress, and back up when downstream is full. Junctions (splitters, mergers, inserters) mediate multi-way interactions.

Expensive, but only allocated for edges that need it. **Not compatible with event-mode simulation** — belt advancement requires per-tick processing.

Covers: Factorio belts, Shapez belts, Mindustry conveyors.

### BatchTransport

Discrete chunks per cycle.

```rust
BatchTransport {
    batch_size: u32,         // items per batch
    cycle_time: Ticks,       // one batch delivered per cycle
}
```

A fixed number of items move as a unit on a fixed schedule. Simple, predictable, good for puzzle games.

Covers: Infinifactory, Good Company courier pallets, production-line style games.

### VehicleTransport

Agents with capacity traveling between nodes.

```rust
VehicleTransport {
    vehicle_capacity: u32,   // items per vehicle
    travel_time: Ticks,      // one way
    loading_time: Ticks,
    schedule: ScheduleId,    // which stops, wait conditions
}
```

Vehicles are entities with state: position, cargo, current route segment. Vehicle state is stored in a dedicated arena. The schedule system maps naturally to Factorio train schedules, DSP logistics station supply/demand, and Captain of Industry truck assignments.

Covers: trains, trucks, drones, logistics vessels, cargo ships.

### Mixing Transport

A single game can use multiple transport strategies simultaneously. All feed the same production graph. A building doesn't know or care what transport strategy its input edges use: it just sees items arriving in its inventory.

Example for a Factorio-like game:
- `Transport::Item` for belts
- `Transport::Vehicle` for trains
- `Transport::Flow` for logistics robot networks

---

## 5. Processor System

Processors define what a building does to transform inputs into outputs. The core ships with prepackaged types for common patterns plus a constrained callback for anything custom.

### Dispatch Model

Like transport, processors use **enum dispatch**:

```rust
enum Processor {
    Source(SourceProcessor),
    Fixed(FixedRecipe),
    Property(PropertyProcessor),
    Custom(CustomProcessor),
}
```

### SourceProcessor

Models resource extraction — how items enter the system. Every researched game has some form of this: mining, drilling, pumping, harvesting.

```rust
SourceProcessor {
    output: ItemType,
    base_rate: Fixed64,           // items per tick
    depletion: Option<Depletion>,  // if the source runs out
}

enum Depletion {
    /// Infinite source (Satisfactory resource nodes, Builderment)
    Infinite,
    /// Finite pool that depletes (Factorio ore patches)
    Finite { remaining: Fixed64 },
    /// Rate decays over time (some mining games)
    Decaying { half_life: Ticks },
}
```

The core manages the production lifecycle: apply rate modifiers (mining productivity research), check output inventory space, produce items, decrement remaining if finite, emit `ItemProduced`. If output is full, the source idles.

The `base_rate` is affected by modifiers, which is how infinite research like Factorio's mining productivity works: a `ProductivityModifier` on the source building increases effective output.

Covers: Factorio miners, Satisfactory resource extractors, ONI geysers, Captain of Industry mining towers, DSP miners.

### FixedRecipe

The workhorse. Covers approximately 80% of all buildings across the 20 researched games.

```rust
FixedRecipe {
    inputs: &[(ItemType, u32)],
    outputs: &[(ItemType, u32)],
    duration: Ticks,
}
```

Supports multi-output for oil cracking, ONI electrolyzers, and similar:

```rust
FixedRecipe {
    inputs: &[("water", 1000)],
    outputs: &[("oxygen", 888), ("hydrogen", 112)],
    duration: Ticks(60),
}
```

The core handles the full lifecycle: check inputs available, reserve inputs, advance progress timer, on completion consume inputs and place outputs in output inventory, emit `ItemProduced` event.

#### Output Blocking Semantics

**All outputs must have room for the recipe to complete.** If any single output slot is full, the building stalls and emits `BuildingStalled { reason: OutputFull(slot_index) }`. This is the standard behavior across Factorio, ONI, DSP, and Captain of Industry. A multi-output recipe where oxygen backs up stalls the entire electrolyzer, even if hydrogen has room.

This is a deliberate design choice: it creates the gameplay pressure that makes byproduct management interesting. Players must handle all outputs or the production chain halts.

Buildings can support multiple recipes with a selection rule:

```rust
RecipeSet {
    recipes: &[recipe_a, recipe_b, recipe_c],
    selection: RecipeSelection::PlayerChosen,  // or ::AutoDetect
}
```

`PlayerChosen` means the game dev sets which recipe is active (Factorio assembler style). `AutoDetect` means the processor picks the first recipe whose inputs are satisfied (Captain of Industry assembly buildings with priority ordering).

### PropertyProcessor

Modifies properties on items passing through.

```rust
PropertyProcessor {
    input: ItemType,
    output: ItemType,            // same type, modified
    duration: Ticks,
    transforms: &[PropertyTransform],
}

enum PropertyTransform {
    Add(PropertyId, Fixed32),
    Multiply(PropertyId, Fixed32),
    Set(PropertyId, Fixed32),
    DivideBy(PropertyId, Fixed32),
    /// Registered transform function, referenced by ID.
    /// The function is registered in the registry at startup,
    /// NOT a raw fn pointer. It receives only the item's
    /// properties and the engine's deterministic RNG.
    Registered(TransformFnId),
}
```

The item enters, the processor modifies its declared properties, and the same item exits with new values.

Covers: Big Pharma concentration system, ONI temperature-modifying buildings.

### CustomProcessor

Constrained escape hatch for anything the prepackaged types cannot express.

```rust
trait CustomProcessorLogic: Send + Sync {
    /// Pure function of inputs + context. MUST be deterministic.
    /// The context provides a constrained, read-only view — NOT
    /// full engine state.
    fn process(
        &self,
        inputs: &[ItemStack],
        context: &ProcessContext,
    ) -> ProcessResult;
}

struct CustomProcessor {
    logic: Box<dyn CustomProcessorLogic>,
    duration: Ticks,   // or Dynamic to query from logic
}
```

`ProcessContext` exposes a **constrained interface**:

```rust
struct ProcessContext<'a> {
    /// The current tick number.
    tick: u64,

    /// Deterministic RNG seeded per-node-per-tick.
    /// The ONLY source of randomness a processor may use.
    rng: &'a mut DeterministicRng,

    /// Read the current node's own component values.
    /// Cannot read other nodes.
    self_components: &'a ComponentView,

    /// Query item type metadata from the registry.
    registry: &'a Registry,
}
```

`ProcessContext` intentionally does NOT provide:
- Access to other nodes' state (prevents ordering-dependent reads)
- Wall clock or system time
- Global mutable state
- The full engine state

The core still manages the lifecycle (input checking, timing, output placement), but the transformation itself is opaque within the determinism constraints.

Covers: Shapez shape algebra (cut, rotate, stack, paint), Infinifactory welding and evisceration, Good Company blueprint assembly.

### Modifiers

Modifiers wrap any processor to alter its behavior without the processor needing to know they exist.

```rust
SpeedModifier { multiplier: Fixed64 }       // 50% faster
ProductivityModifier { bonus: Fixed64 }      // 10% free outputs
EfficiencyModifier { multiplier: Fixed64 }   // 30% less power
```

Modifiers stack in a **canonical order**: sorted by `ModifierId` (registration order), applied left-to-right. `a * b * c` always evaluates in the same sequence on all clients. All arithmetic uses `Fixed64`.

Modifiers affect duration, output count, and power consumption. The core computes effective values.

Covers: Factorio modules (speed/productivity/efficiency), DSP proliferators, Satisfactory overclocking and Somersloops, Mindustry overdrive projectors.

---

## 6. Event System

Events are how the core communicates outward without coupling to game-specific code. Every meaningful state change emits an event.

### Design Principles

Events are typed, immutable, and emitted in the post-tick phase. Subscribers receive them in **registration order**, which is part of the determinism contract (module registration order must match across clients).

```rust
engine.on::<ItemProduced>(|event| {
    // event.node_id, event.item_type, event.quantity, event.tick
});

engine.on::<BuildingStalled>(|event| {
    // event.node_id, event.reason (OutputFull, InputEmpty, NoPower)
});
```

### Core Events

```
Production:
  ItemProduced { node, item_type, quantity, tick }
  ItemConsumed { node, item_type, quantity, tick }
  RecipeStarted { node, recipe_id, tick }
  RecipeCompleted { node, recipe_id, tick }

Building State:
  BuildingStalled { node, reason, tick }
  BuildingResumed { node, tick }

Transport:
  ItemDelivered { edge, item_type, quantity, tick }
  TransportFull { edge, tick }
  VehicleDeparted { edge, vehicle_id, cargo, tick }
  VehicleArrived { edge, vehicle_id, cargo, tick }

Inventory:
  InventoryFull { node, slot, item_type, tick }
  InventoryEmpty { node, slot, item_type, tick }

Graph:
  NodeAdded { node, building_type, tick }
  NodeRemoved { node, tick }
  EdgeAdded { edge, from, to, tick }
  EdgeRemoved { edge, tick }
```

### Framework Module Events

Each module emits its own events using the same system. Modules may also define custom event types that game code and other modules can subscribe to:

```
Power (module):
  PowerGridBrownout { grid_id, deficit, tick }
  PowerGridRestored { grid_id, tick }

Tech Tree (module):
  ResearchCompleted { tech_id, tick }
  ResearchStarted { tech_id, tick }
```

### Subscriber Types

**Passive listeners** are read-only. Used for UI updates, audio cues, analytics, and achievement tracking. Cannot modify engine state. This is what game devs use most. Passive listeners may safely call query API methods (read-only) from within their handler.

**Reactive handlers** respond to events by **enqueuing state mutations that apply at the start of the next tick** (during pre-tick phase). They CANNOT modify state immediately. This eliminates:
- Re-entrancy issues (no cascading events within a single tick)
- Ordering-dependent state mutations
- The contradiction between "modify state" and "changes apply next tick"

For example, the tech tree module listens for `ItemConsumed` at research buildings. When research completes, it enqueues an `UnlockBuilding` mutation that applies at the next pre-tick. Any `ResearchCompleted` event is emitted in the current post-tick, but the unlock doesn't take effect until next tick. This one-tick delay is acceptable and eliminates an entire class of event-cascade bugs.

### Event Buffering and Allocation

Events are collected during phases 2-4 of the simulation step and delivered in batch during phase 5.

Events use a **pre-allocated ring buffer** per event type, not heap allocation per event. Each event type has a configurable buffer capacity. When the buffer is full, the oldest undelivered events are dropped (with a `EventsDropped` warning event). This bounds memory usage regardless of factory size.

Games that don't need certain event types can **suppress** them at registration time. Suppressed events are never allocated, not just unsubscribed:

```rust
engine.suppress_event::<ItemDelivered>();  // zero cost if not needed
```

---

## 7. Query API

The query API is how game code reads current simulation state for rendering, UI, and diagnostics. This is the primary integration surface for game engines.

### Node Queries

```rust
engine.get_processor_progress(node) -> Fixed64  // 0.0 to 1.0, for animations
engine.get_processor_state(node) -> ProcessorState  // Idle, Working, Stalled(reason)
engine.get_inventory(node, slot) -> &InventorySlot  // item types and quantities
engine.get_components(node) -> &ComponentView  // read all component values
engine.get_building_type(node) -> BuildingTypeId
```

### Edge Queries

```rust
engine.get_transport_state(edge) -> TransportSnapshot
engine.get_edge_utilization(edge) -> Fixed64  // 0.0 to 1.0
```

For `ItemTransport` edges, `TransportSnapshot` includes item positions for rendering:

```rust
struct TransportSnapshot {
    items: &[(ItemType, Fixed32)],  // (type, position along belt)
}
```

### Bulk Queries

To avoid per-entity FFI overhead, the query API provides bulk reads:

```rust
// All nodes with their current state, for a full UI refresh.
engine.snapshot_all_nodes() -> &[NodeSnapshot]

// All belt items in a spatial region (for camera-culled rendering).
// Requires SpatialGraph.
engine.query_belt_items_in_rect(rect) -> &[(EdgeId, ItemType, Position)]

// All nodes matching a filter.
engine.query_nodes(filter) -> &[NodeId]
```

### Graph Introspection

```rust
engine.node_count() -> usize
engine.edge_count() -> usize
engine.get_inputs(node) -> &[EdgeId]
engine.get_outputs(node) -> &[EdgeId]
engine.get_edge_endpoints(edge) -> (NodeId, NodeId)
```

---

## 8. Serialization and Snapshots

Save/load is designed into the core from the start because every system stores state. Retrofitting it breaks APIs and misses edge cases.

### Core Serialization

The core owns the serialization format for its own state:

```rust
let snapshot: Vec<u8> = engine.serialize();
let engine = Engine::deserialize(&snapshot, &registry)?;
```

What gets serialized in the core:

```
Graph State:
  - All nodes (building type, component state)
  - All edges (transport type, transport state)
  - All junctions (type, configuration, state)
  - Item instances (type + properties if any)
  - Inventory contents per node
  - NodeId assignments (preserved exactly for processing order stability)

Simulation State:
  - Current tick number
  - Processor progress per node (how far through current recipe)
  - Transport state (items in transit, vehicle positions)
  - RNG state (for deterministic replay)
  - State hash (for desync detection on load)
```

The registry is NOT serialized. Item types, building templates, and recipes are code-defined. Deserialization requires the same registry. Version mismatch produces an explicit error, not silent corruption.

**`NodeId` stability**: Node IDs are preserved across serialize/deserialize round-trips. Deserialization reconstructs nodes with their original IDs in the original order. This guarantees processing order stability.

### Module Serialization

Framework modules register their own serializers with the core:

```rust
engine.register_serializer("tech_tree", TechTreeModule {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(&mut self, data: &[u8]);
    fn state_hash(&self) -> u64;  // contributes to engine state hash
});
```

Game dev custom state uses the same hook:

```rust
engine.register_serializer("my_game", MyGameState { ... });
```

### Binary Format

The serialized format is binary, versioned, with a magic number header. Each section is independently versioned so a module can evolve its format without breaking other sections.

```
[FACTORIAL magic][core version][tick count]
[core state blob]
[module: "tech_tree"][version][blob]
[module: "power"][version][blob]
[module: "my_game"][version][blob]
```

When a module's version changes, its deserializer receives the old version number and can migrate. The core provides no automatic migration: each module handles its own.

### Snapshots and Replay

The core supports periodic snapshots for undo, replay, and debugging:

```
Snapshot System:
  - Full state snapshot every N ticks (configurable)
  - Incremental snapshots between full snapshots (only dirty entities)
  - Input log records all external actions between snapshots
  - Replay: load snapshot + replay inputs = identical state
  - Undo: load previous snapshot + replay to (target_tick - 1)
  - Memory budget: ring buffer of last M snapshots, oldest evicted
  - Snapshots are optional and can be fully disabled (zero overhead)
```

Determinism makes this reliable. Same snapshot + same inputs = same result, always. This enables:
- Undo in puzzle games (Infinifactory, Shapez)
- Replays for competitive/speedrun contexts (Factorio)
- Time-rewind mechanics
- Debugging tools
- **Multiplayer desync recovery**: authoritative client sends snapshot, desynced client loads it

---

## 9. Performance Architecture

This section specifies the data layout, allocation strategy, and dispatch model that make the abstractions in Sections 2-6 performant at scale.

### Target Scale

| Game Category | Buildings | Edges | Items in Transit | Target |
|---------------|-----------|-------|-----------------|--------|
| Mobile/Casual (Builderment) | 200-2,000 | 500-5,000 | 5,000 | 60 UPS, <2ms/tick |
| Mid-complexity (ONI, Big Pharma) | 500-5,000 | 2,000-10,000 | 10,000-50,000 | 60 UPS, <5ms/tick |
| Factorio-like (mid-game) | 5,000-20,000 | 15,000-50,000 | 50,000-200,000 | 60 UPS, <8ms/tick |
| Factorio megabase | 50,000+ | 200,000+ | 1,000,000+ | Requires custom systems (see escape hatch) |

The abstract strategy interfaces are the **default implementation** that works well up to the mid-complexity tier. Factorio megabase scale requires the same kind of specialized, non-abstract, data-oriented systems that took the Factorio team years to develop. The engine provides an escape hatch for this (see below).

### Component Storage

Components are stored in **struct-of-arrays (SoA) layout**, not per-entity bags.

```
Storage layout (conceptual):

inventories:     [Inv0, Inv1, Inv2, Inv3, ...]    // contiguous
processors:      [Proc0, Proc1, Proc2, Proc3, ...]  // contiguous
power_consumers: [Pwr0, _, Pwr2, _, ...]             // sparse, indexed by NodeId
heat_emitters:   [_, Heat1, _, Heat3, ...]            // sparse, indexed by NodeId
```

Each component type gets its own contiguous storage array. When the process phase iterates all processors, it streams through a packed `&[Processor]` slice. When the power module iterates all power consumers, it streams through a packed `&[PowerConsumer]` slice. No pointer chasing, no per-entity vtable lookups.

Implementation: component storage uses `SlotMap<NodeId, T>` per component type, where `SlotMap` is a generational-index arena with O(1) access and contiguous backing storage. Framework modules register new component storage arrays at startup. Third-party code can do the same.

### Item Instance Storage

Stateful item instances use **typed arenas per item type**:

```
water_instances:  [WaterProps0, WaterProps1, WaterProps2, ...]
// where WaterProps = { temperature: Fixed32, mass: Fixed32, germs: u32 }
// Fixed-size, known at registration, packed contiguously.

ingredient_instances: [IngProps0, IngProps1, ...]
// where IngProps = { concentration: u8 }
```

Each item type's property layout is determined at registration time and compiled into a fixed-size struct. All instances of the same type live in the same arena. Item instances are referenced by `InstanceId` (a generational index into the arena). Creation and destruction are O(1) with no heap allocation per instance.

Fungible items remain counters with zero allocation overhead.

### Transport State Storage

Transport state is stored **externally**, grouped by transport type:

```
flow_edges:    [FlowState0, FlowState1, ...]     // rate + buffer, tiny
item_edges:    [BeltState0, BeltState1, ...]      // slot arrays, large
batch_edges:   [BatchState0, BatchState1, ...]    // simple counter
vehicle_edges: [VehicleState0, VehicleState1, ...] // vehicle arrays
```

`BeltState` for `ItemTransport` uses a **flat array of slot data** per belt, not `VecDeque`. Slot arrays are pre-allocated at edge creation to the belt's declared length. No runtime reallocation during transport updates.

For belts, the hot loop advances items through a contiguous array of slots. The common case (slot is empty, or slot advances forward) is branchless.

### Allocation Strategy

- **Arena allocators** for entity storage (nodes, edges, items, junctions). O(1) create/destroy with generational indices.
- **Pre-allocated ring buffers** for events (per event type, configurable capacity).
- **Pre-allocated flat arrays** for belt slot data (sized at edge creation, never resized).
- **Pool allocator** for vehicle state (fixed-size vehicle structs).
- **No `Vec` resizing during simulation ticks.** All growable collections are pre-allocated with capacity hints at graph construction time. If capacity is exceeded, the engine logs a warning and reallocates between ticks (during pre-tick phase), never mid-phase.

### Performance Escape Hatch

For games that outgrow the abstract strategy interfaces, the engine provides a **custom system registration** mechanism:

```rust
engine.register_custom_system("optimized_belts", |phase, state| {
    // Replace the generic per-edge ItemTransport dispatch with
    // a game-specific batch processor that uses transport lines,
    // SIMD, or any other optimization.
    // Runs during the specified phase.
    // Has mutable access to transport state arrays.
});
```

A custom system replaces the engine's default processing for a specific phase and transport/processor type. The engine skips its generic dispatch for entities claimed by the custom system.

This is how a Factorio-scale game would implement Factorio-style transport lines: register a custom system for the transport phase that groups contiguous belts and processes them with gap-distance arrays instead of per-slot iteration. The engine provides the data layout utilities (arenas, SoA storage); the custom system provides the algorithm.

---

## 10. Framework Modules

Framework modules are opt-in crates that sit above the core. Each is independent: they depend on the core but never on each other.

### Tech Tree Module (Essential)

Research systems vary widely across factory games but share a common skeleton: nodes with prerequisites and costs, unlocking capabilities when completed.

```rust
let tech_tree = TechTree::new();

tech_tree.register(Technology {
    id: "steel_smelting",
    prerequisites: &["iron_smelting"],
    cost: ResearchCost::Items(&[("red_science", 50), ("green_science", 50)]),
    unlocks: &[
        Unlock::Building("steel_furnace"),
        Unlock::Recipe("steel_plate"),
    ],
});
```

The module supports multiple cost models:

```rust
enum ResearchCost {
    // Factorio/DSP: consume items at research buildings
    Items(&[(ItemType, u32)]),

    // ONI: spend points generated by research stations
    Points(u32),

    // Satisfactory: one-time delivery of specific items
    Delivery(&[(ItemType, u32)]),

    // Captain of Industry: points over time (Fixed64, not f64)
    Rate { points_per_tick: Fixed64, total: u32 },

    // Shapez: deliver items at a target rate
    ItemRate { item: ItemType, rate: Fixed64, duration: Ticks },

    // Anything else. Receives constrained context, not &EngineState.
    Custom(ResearchCostFnId),
}
```

Unlocks are pluggable. `Unlock::Building` and `Unlock::Recipe` are built-in. The module emits `ResearchCompleted` events. Game code listens and applies whatever "unlocking" means in context.

Infinite research (Factorio mining productivity, DSP white science) is modeled as technologies with `repeatable: true` and scaling costs:

```rust
Technology {
    id: "mining_productivity",
    repeatable: true,
    cost_scaling: CostScaling::Linear { base: 1000, increment: 500 },
    effect_per_level: Modifier::ProductionBonus("miners", Fixed64::from_f64(0.10)),
}
```

### Power Networks Module (Essential)

Power appears in 16 of 20 researched games. The module models the common abstraction: a network of producers and consumers that must balance.

```rust
PowerProducer { output: Fixed64 }    // watts
PowerConsumer { demand: Fixed64 }    // watts
PowerStorage { capacity: Fixed64, charge: Fixed64 }  // joules
```

The module groups connected buildings into networks. Producer and consumer node IDs are stored in **contiguous arrays per network** for cache-friendly reduction:

```rust
PowerNetwork {
    producers: Vec<NodeId>,
    consumers: Vec<NodeId>,
    storage: Vec<NodeId>,
    satisfaction: Fixed64,  // 0.0 to 1.0
}
```

Each tick, the module:

1. Sums total production and demand per network
2. If production >= demand: all consumers satisfied, excess charges storage
3. If production < demand: drain storage, then reduce `satisfaction` ratio
4. Applies satisfaction to building performance (configurable curve)
5. Emits `PowerGridBrownout` or `PowerGridRestored` events

Network topology depends on the spatial model. With `TopologicalGraph`, the game dev explicitly assigns buildings to power networks. With `SpatialGraph`, the module can auto-detect networks via connected power poles.

Fuel-based power (Factorio boilers, Satisfactory coal generators) is just a building with a `FixedRecipe` that consumes fuel and a `PowerProducer` component. The power module doesn't care how power is generated.

Variable output (ONI solar panels, DSP wind turbines) uses a registered function:

```rust
PowerProducer {
    output: DynamicOutput(PowerFnId),
    // PowerFnId references a function registered in the registry.
    // Receives constrained context (tick, self-components), not &EngineState.
}
```

### Production Statistics Module (Essential)

Tracks per-node, per-edge, and per-item-type throughput over configurable time windows.

```rust
// Per-node statistics
stats.get_production_rate(node, item_type) -> Fixed64  // items/min, rolling average
stats.get_consumption_rate(node, item_type) -> Fixed64
stats.get_idle_ratio(node) -> Fixed64                  // 0.0 to 1.0
stats.get_stall_ratio(node) -> Fixed64                 // 0.0 to 1.0
stats.get_uptime(node) -> Fixed64                      // 0.0 to 1.0

// Per-edge statistics
stats.get_throughput(edge) -> Fixed64                  // items/min
stats.get_utilization(edge) -> Fixed64                 // 0.0 to 1.0 (actual vs capacity)

// Global statistics
stats.get_total_production(item_type) -> Fixed64       // items/min across all nodes
stats.get_total_consumption(item_type) -> Fixed64

// Historical data (ring buffer, configurable depth)
stats.get_history(node, item_type, window: Ticks) -> &[Fixed64]
```

The module listens to `ItemProduced`, `ItemConsumed`, `BuildingStalled`, and `BuildingResumed` events. It maintains per-node and per-edge counters in the bookkeeping phase. All counters use `Fixed64`.

This is the data that feeds every factory game's production statistics screen, bottleneck visualizer, and efficiency overlay.

### Planned Future Modules

These are not blocking initial release but are designed to be addable without core changes:

| Module | Covers | Priority |
|--------|--------|----------|
| Spatial Grid | Placement, adjacency, area effects, placement validation with rejection reasons | High |
| Fluid Networks | See [Fluid Network Boundary](#fluid-network-boundary) below | High |
| Logic/Circuit Networks | Signal wiring, combinators, sensors. Cross-cuts transport (enable/disable edges) and processing (recipe selection, inserter filters). Must define interaction points with core phases. | Medium |
| Vehicle Logistics | Pathing, scheduling, fleet management | Medium |
| Population/Workers | Labor requirements, skills, morale | Medium |
| Market/Economy | Supply/demand, pricing, competition | Lower |
| Environmental Grid | See [Future: Environmental Grid](#future-environmental-grid) below | Future |
| Player Experience | See [Future: Player Experience](#future-player-experience) below | Future |

#### Fluid Network Boundary

Fluid simulation interacts deeply with the core's transport and processing systems. This boundary must be defined now even though the module is built later.

**What the core provides for fluids:**
- `FlowTransport` with `merge_fn` handles simple pipe networks (rate-limited, latency-delayed flow with property aggregation). This covers Satisfactory-style pipes and DSP-style fluid belts.
- `PropertyProcessor` handles buildings that modify fluid properties (temperature changers, filters).

**What the fluid module adds:**
- **Pressure-based flow.** Replaces `FlowTransport` rate calculation on fluid-tagged edges with pressure-differential flow: flow rate = f(pressure_delta, pipe_capacity, viscosity). The module registers a custom transport system (via the performance escape hatch) for fluid edges.
- **Fluid mixing rules.** Pipes carry one fluid type and block on mismatch (Factorio) OR allow mixing with property blending (configurable per-game).
- **Pipe network segmentation.** Groups connected fluid edges into networks (like power networks). Pressure balances within a network.
- **Fluid-specific events:** `FluidMixed`, `PipePressureWarning`, `FluidStateChanged`.

**What the fluid module does NOT do:**
- Open-tile gas/liquid simulation (ONI atmospheric physics). That is a cellular automaton operating on a spatial grid, fundamentally outside the production graph. See [Future: Environmental Grid](#future-environmental-grid).

#### Future: Environmental Grid

*Mega-high-level sketch for future consideration. Not blocking any current work.*

Some colony sim games (ONI, Rimworld) need dense per-tile environmental state: temperature, pressure, gas composition, germ counts, material phase. This is a cellular automaton, not a production graph.

**Concept:** An `EnvironmentalGrid` module that owns a dense 2D (or 3D) grid of tile state, running its own per-tick simulation during the component phase. Integration with the core:

- Buildings with `HeatEmitter` components inject heat into their tile via the grid module during the component phase.
- Buildings with `GasOutput` components (like ONI electrolyzers) emit gas into adjacent tiles rather than into an output edge.
- The grid module reads spatial building positions from `SpatialGraph`.
- Grid state is serialized via the module serialization hook.
- Grid events (`TemperatureThreshold`, `OverpressureEvent`) use the standard event system.

The core's component phase already has a defined slot for "module-registered systems run." The environmental grid would register as one such system. The key design decision (deferred): how tightly does the grid couple to the production graph? Does a building's operation depend on its tile's temperature (grid reads affecting processor state), or is it purely one-directional (buildings affect grid, grid doesn't affect buildings)?

#### Future: Player Experience

*Mega-high-level sketch for future consideration. Not blocking any current work.*

Raw simulation events (`ItemProduced`, `BuildingStalled`) are bookkeeping-grain. Game designers need player-experience-grain events: "the player just produced steel for the first time," "throughput crossed 100/min," "a new production chain came online."

**Concept:** A `PlayerExperience` module that subscribes to raw events and synthesizes derived events:

- `FirstProduction { item_type }` — first time an item type is produced in this save.
- `ThroughputMilestone { item_type, rate }` — production rate crossed a configured threshold.
- `ChainActivated { source_node, sink_node }` — items flowed from source to sink for the first time.
- `CascadingStall { root_node, affected_count }` — a stall propagated downstream, with causal attribution.
- `EfficiencyAchieved { node, threshold }` — a building hit a target utilization.

The module would also provide:
- **Milestone tracking:** Composite conditions ("researched X AND built 5 assemblers AND achieved 30 iron/min") with a simple condition combinator API.
- **Blueprint/ghost support:** Validate a proposed build against the graph without committing it. Returns expected throughput, connection validity, and resource requirements.
- **Interpolation guidance:** Documented patterns for interpolating simulation state between ticks for smooth rendering at frame rates above the tick rate.

---

## 11. Integration Architecture

This section addresses how game engines (Godot, Unity, Bevy, custom) integrate with Factorial across the FFI boundary.

### C API Design

The C API is a **first-class API surface**, not an afterthought wrapper. It is designed alongside the Rust API with its own constraints in mind:

```c
// Lifecycle
FactorialEngine* factorial_create(const FactorialRegistry* registry);
void factorial_destroy(FactorialEngine* engine);
void factorial_step(FactorialEngine* engine);

// Graph mutation (queued, applies at next step)
PendingNodeId factorial_add_node(FactorialEngine* engine, BuildingTypeId type);
void factorial_remove_node(FactorialEngine* engine, NodeId node);
PendingEdgeId factorial_connect(FactorialEngine* engine,
                                 NodeId from, uint32_t from_slot,
                                 NodeId to, uint32_t to_slot,
                                 TransportConfig transport);

// Queries (read-only, safe to call anytime between steps)
ProcessorState factorial_get_processor_state(const FactorialEngine* engine, NodeId node);
int64_t factorial_get_processor_progress(const FactorialEngine* engine, NodeId node);
uint32_t factorial_get_inventory_count(const FactorialEngine* engine,
                                        NodeId node, uint32_t slot);

// Bulk queries (returns pointer + length into engine-owned memory,
// valid until next factorial_step call)
uint32_t factorial_snapshot_nodes(const FactorialEngine* engine,
                                   const NodeSnapshot** out_data);
uint32_t factorial_query_belt_items(const FactorialEngine* engine,
                                     Rect region,
                                     const BeltItem** out_data);

// Serialization (caller must call factorial_free_buffer on result)
FactorialBuffer factorial_serialize(const FactorialEngine* engine);
FactorialEngine* factorial_deserialize(const uint8_t* data, size_t len,
                                        const FactorialRegistry* registry);
void factorial_free_buffer(FactorialBuffer buffer);
```

**Error handling:** Every C API function that can fail returns a `FactorialResult` status code. Rust panics are caught at the FFI boundary with `catch_unwind`. The C API never triggers undefined behavior.

**Lifetime rule:** Pointers returned by bulk queries point into engine-owned memory and are valid until the next `factorial_step()` call. This avoids per-frame allocation for the common case of reading state for rendering.

### Event Delivery Across FFI

Events use a **pull-based model** across FFI. After each `factorial_step()`, the game drains the event buffer:

```c
// Poll events by type. Returns count, fills caller's buffer.
uint32_t factorial_poll_events_item_produced(
    const FactorialEngine* engine,
    ItemProducedEvent* buffer,
    uint32_t buffer_capacity);

// Or drain all events as a tagged union stream:
uint32_t factorial_poll_all_events(
    const FactorialEngine* engine,
    FactorialEvent* buffer,
    uint32_t buffer_capacity);
```

Pull-based (not callback-based) because:
- No reentrancy risk. The game reads events after `step()` completes, never during.
- No Rust-to-C callback during simulation (which would block the hot loop).
- Compatible with every language binding (C#, GDScript, Python) without function pointer gymnastics.
- Events are plain data structs — trivially serializable and debuggable.

### GDExtension Plan

Official Godot integration is a **first-party GDExtension** maintained by the Factorial team:

- Built with `godot-rust` (gdext), targeting Godot 4.x stable releases.
- Ships pre-built binaries for Windows, macOS, Linux, and web (WASM).
- Exposes Factorial as Godot nodes: `FactorialEngine`, `FactorialGraph`, `FactorialBuilding` (thin wrappers that bridge to the C API).
- Events are bridged to Godot signals: `engine.item_produced.connect(my_handler)`.
- Registry population from Godot resources (`.tres` files) via an `EditorPlugin` that calls the builder API at game startup.
- **Synchronization protocol**: Factorial is authoritative for simulation state. The game mirrors it in the scene tree. Recommended pattern: call Factorial API to mutate → call `engine.step()` → poll events → update scene tree.

### WASM Considerations

- The core is **single-threaded**. All parallelism (future custom system parallelism via rayon) is behind feature flags disabled for WASM.
- WASM bindings use `wasm-bindgen`, not the C FFI layer. This is a separate binding surface.
- Memory: WASM linear memory grows but does not shrink. The engine's arena/pool allocation strategy mitigates this by reusing freed slots rather than growing.
- No `std::thread`, no `SharedArrayBuffer` requirement.

---

## 12. Scope Boundaries

### Factorial provides

- Headless factory simulation (production graph, items, recipes, transport, junctions)
- Pluggable simulation timing (tick, delta, event)
- Pluggable transport (flow, item, batch, vehicle, mixable per-edge)
- Flexible recipe/processor system (source, fixed, property, custom)
- Items with optional properties (fungible fast path, stateful when needed)
- Composable building components (minimal core, opt-in complexity)
- Topological production graph (spatial via plugin)
- Fixed-point arithmetic and deterministic simulation guarantees
- State hashing for desync detection
- Event system for game-engine integration
- Query API for reading simulation state
- Production statistics module
- Serialization with module hooks, snapshot/replay support, and NodeId stability
- Tech tree and power network framework modules
- Registry with registration/mutation/finalize lifecycle
- C API with pull-based events and bulk queries
- First-party GDExtension for Godot 4
- Rust core with C FFI and WASM targets
- Performance escape hatch for custom batch-processing systems

### Factorial does NOT provide

- Rendering, UI, or audio
- Input handling
- Networking or multiplayer protocol (determinism + state hashing enables it; sync layer is game-side)
- Combat, health, damage, or unit AI (factories don't fight)
- Terrain generation or world generation
- Physics (collision, gravity, ragdoll)
- Open-tile environmental simulation (atmosphere, fluid dynamics — see future Environmental Grid module)
- A specific game (Factorial is always a library, never a standalone product)

### Partial Coverage Acknowledgment

Factorial's core strength is **production chain simulation**: items flowing through buildings via transport, transformed by recipes. Games where this is the central mechanic (Factorio, Satisfactory, Shapez, DSP, Builderment) get 70-80% coverage from the core + modules.

Games where production is one system among many equally important systems get partial coverage:

- **Tycoon games** (Big Pharma, Good Company, Rise of Industry): Factorial handles production and logistics (~30-35% of the game). Market simulation, financial modeling, employee systems, and competition AI are game-side. The event system and serialization hooks provide clean integration points.
- **Colony sims** (ONI): Factorial handles production recipes, piped transport, power, and tech tree (~25-30%). Atmospheric physics, agent AI, and environmental simulation are game-side.
- **Puzzle factory games** (Infinifactory): Factorial handles recipe processing and batch transport. Spatial puzzle constraints and level design are game-side.

This is a valid value proposition: Factorial handles the hardest 25-80% of the simulation layer (depending on game type), with clean integration points for everything else.

---

## 13. Validation Matrix

Mapping the 20 researched games to confirm Factorial can express them:

| Game | Core Features | Framework Modules | Coverage |
|------|---------------|-------------------|----------|
| **Factorio** | FixedRecipe, SourceProcessor, ItemTransport, junctions, modifiers | Power, spatial, fluids, logic, stats | High |
| **Satisfactory** | FixedRecipe, SourceProcessor, FlowTransport, VehicleTransport | Power, spatial, fluids, stats | High |
| **Dyson Sphere Program** | FixedRecipe, SourceProcessor, FlowTransport, VehicleTransport | Power, tech tree, stats | High |
| **Shapez** | CustomProcessor, ItemTransport, junctions | Tech tree, stats | High |
| **Shapez 2** | CustomProcessor, ItemTransport, junctions | Tech tree, stats | High |
| **Oxygen Not Included** | PropertyProcessor, FlowTransport, SourceProcessor, stateful items | Power, spatial, fluids, workers, stats | Partial — atmospheric sim and agent AI are game-side |
| **Captain of Industry** | FixedRecipe, SourceProcessor, VehicleTransport, stateful items | Power, spatial, fluids, workers, market, stats | Partial — terrain modification and population are game-side |
| **Mindustry** | FixedRecipe, SourceProcessor, FlowTransport | Power, spatial, fluids, stats | High — combat/turrets are game-side |
| **Builderment** | FixedRecipe, SourceProcessor, FlowTransport, event sim | Tech tree, power, stats | High |
| **Infinifactory** | CustomProcessor, BatchTransport | Spatial, stats | Partial — spatial puzzle design is game-side |
| **Big Pharma** | PropertyProcessor, FlowTransport | Tech tree, market, stats | Partial — market and business sim are game-side |
| **Good Company** | FixedRecipe, BatchTransport | Tech tree, market, workers, stats | Partial — product design and business sim are game-side |
| **Rise of Industry** | FixedRecipe, SourceProcessor, VehicleTransport | Tech tree, market, stats | Partial — business sim is game-side |
| **Factory Town** | FixedRecipe, SourceProcessor, VehicleTransport, FlowTransport | Power, spatial, workers, stats | High |
| **Automation Empire** | FixedRecipe, FlowTransport, VehicleTransport | Power, spatial, stats | High |
| **Production Line** | FixedRecipe, FlowTransport | Market, stats | Partial — business sim is game-side |
| **Assembly Line** | FixedRecipe, FlowTransport | Tech tree, stats | High |
| **Voxel Tycoon** | FixedRecipe, SourceProcessor, VehicleTransport | Power, spatial, stats | High |
| **Little Big Workshop** | FixedRecipe, BatchTransport | Workers, stats | High |
| **Techtonica** | FixedRecipe, SourceProcessor, ItemTransport | Power, spatial, fluids, stats | High |

Every game maps to a combination of core features plus framework modules. The Coverage column honestly indicates where Factorial handles the factory layer but significant game systems live outside the engine.
