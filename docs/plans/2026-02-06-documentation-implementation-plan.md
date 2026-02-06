# Factorial Beta Documentation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a complete mdBook documentation site with compile-tested examples covering the full Factorial engine for both Rust and FFI audiences.

**Architecture:** mdBook generates a GitHub Pages site from `docs/book/` source files. All code in docs is excerpted from real example crates that compile as workspace members. CI validates examples compile, doc-tests pass, and the book builds without broken links.

**Tech Stack:** mdBook, Rust (existing workspace), cbindgen (existing), GitHub Pages

**Worktree:** `/Users/zakkeown/Code/Factorial-worktrees/docs-beta-documentation`
**Branch:** `docs/beta-documentation`
**Design doc:** `docs/plans/2026-02-06-documentation-design.md`

---

## Task 1: mdBook Scaffolding

**Files:**
- Create: `docs/book/book.toml`
- Create: `docs/book/src/SUMMARY.md`
- Modify: `.gitignore` (add mdBook output directory)

**Step 1: Install mdBook locally (if not present)**

Run: `cargo install mdbook 2>/dev/null || echo "already installed"`

**Step 2: Create the mdBook config**

Create `docs/book/book.toml`:

```toml
[book]
authors = ["Factorial Contributors"]
language = "en"
multilingual = false
src = "src"
title = "Factorial Documentation"

[build]
build-dir = "../../target/book"

[output.html]
default-theme = "rust"
preferred-dark-theme = "ayu"
git-repository-url = "https://github.com/user/factorial"
edit-url-template = "https://github.com/user/factorial/edit/main/docs/book/src/{path}"

[output.html.search]
enable = true
```

**Step 3: Create the SUMMARY.md**

Create `docs/book/src/SUMMARY.md`:

```markdown
# Summary

# Introduction

- [What is Factorial?](introduction/what-is-factorial.md)
- [What You Build vs. What We Handle](introduction/responsibilities.md)
- [Glossary](introduction/glossary.md)

# Getting Started

- [Rust Quick Start](getting-started/rust.md)
- [C/FFI Quick Start](getting-started/ffi.md)
- [WASM Quick Start](getting-started/wasm.md)

# Core Concepts

- [The Production Graph](core-concepts/production-graph.md)
- [Processors](core-concepts/processors.md)
- [Transport Strategies](core-concepts/transport.md)
- [Events](core-concepts/events.md)
- [Queries](core-concepts/queries.md)
- [Determinism & Fixed-Point](core-concepts/determinism.md)
- [Serialization](core-concepts/serialization.md)

# Framework Modules

- [Power Networks](modules/power.md)
- [Fluid Networks](modules/fluid.md)
- [Tech Trees](modules/tech-tree.md)
- [Spatial Grid & Blueprints](modules/spatial.md)
- [Statistics](modules/stats.md)

# FFI Reference

- [API Conventions & Safety](ffi/conventions.md)
- [Function Reference](ffi/reference.md)
- [Language-Specific Bindings](ffi/bindings.md)

# Cookbook

- [Model a Smelting Chain](cookbook/smelting-chain.md)
- [Build a Multi-Step Production Line](cookbook/production-line.md)
- [Choose the Right Transport Strategy](cookbook/transport-strategies.md)
- [React to Production Events](cookbook/events.md)
- [Add Power with Brownout Handling](cookbook/power.md)
- [Pipe Fluids Between Buildings](cookbook/fluids.md)
- [Gate Buildings Behind Research](cookbook/tech-tree.md)
- [Place Buildings on a Grid](cookbook/spatial.md)
- [Save, Load, and Migrate State](cookbook/serialization.md)
- [Detect Multiplayer Desync](cookbook/multiplayer.md)

# Architecture Deep Dive

- [Performance Model](architecture/performance.md)
- [Memory Layout](architecture/memory.md)
- [Design Decisions & Trade-offs](architecture/decisions.md)
```

**Step 4: Create placeholder files for every page in SUMMARY.md**

Create each `.md` file listed in SUMMARY.md with a single `# Title` heading. mdBook will fail to build if any file referenced in SUMMARY.md is missing.

The directory structure under `docs/book/src/`:
```
src/
├── SUMMARY.md
├── introduction/
│   ├── what-is-factorial.md
│   ├── responsibilities.md
│   └── glossary.md
├── getting-started/
│   ├── rust.md
│   ├── ffi.md
│   └── wasm.md
├── core-concepts/
│   ├── production-graph.md
│   ├── processors.md
│   ├── transport.md
│   ├── events.md
│   ├── queries.md
│   ├── determinism.md
│   └── serialization.md
├── modules/
│   ├── power.md
│   ├── fluid.md
│   ├── tech-tree.md
│   ├── spatial.md
│   └── stats.md
├── ffi/
│   ├── conventions.md
│   ├── reference.md
│   └── bindings.md
├── cookbook/
│   ├── smelting-chain.md
│   ├── production-line.md
│   ├── transport-strategies.md
│   ├── events.md
│   ├── power.md
│   ├── fluids.md
│   ├── tech-tree.md
│   ├── spatial.md
│   ├── serialization.md
│   └── multiplayer.md
└── architecture/
    ├── performance.md
    ├── memory.md
    └── decisions.md
```

**Step 5: Add mdBook output to .gitignore**

Append to `.gitignore`:
```
# mdBook output
target/book/
```

**Step 6: Verify the book builds**

Run: `mdbook build docs/book`
Expected: `[INFO] ... Book build succeeded`

**Step 7: Commit**

```bash
git add docs/book/ .gitignore
git commit -m "docs: scaffold mdBook structure with SUMMARY and placeholders"
```

---

## Task 2: Introduction — What is Factorial?

**Files:**
- Modify: `docs/book/src/introduction/what-is-factorial.md`

**Step 1: Write the page**

This is the landing page. It must convey in under 2 minutes of reading:
- Factorial is a headless factory game engine written in Rust
- It handles the simulation layer (production math, transport, power, fluids, tech trees, spatial placement, serialization, determinism)
- Game developers focus on UI, rendering, audio, and game-specific mechanics
- It works via Rust dependency or C FFI / WASM embedding
- It's research-informed by studying 20+ factory games

Include a high-level ASCII architecture diagram:

```
┌─────────────────────────────────────────┐
│         Your Game Code                  │
│  (UI, rendering, audio, game logic)     │
├─────────────────────────────────────────┤
│      Framework Modules (opt-in)         │
│  Power · Fluid · Tech Tree · Spatial ·  │
│  Statistics                             │
├─────────────────────────────────────────┤
│           Factorial Core                │
│  Production Graph · Processors ·        │
│  Transport · Events · Queries ·         │
│  Serialization · Determinism            │
└─────────────────────────────────────────┘
```

Include a brief "hello world" teaser (3 lines of Rust) that links to the Rust Quick Start:

```rust
let mut engine = Engine::new(SimulationStrategy::Tick);
// ... configure nodes, processors, transport ...
engine.step(); // one simulation tick
```

Link to: Glossary (for any domain terms used), Responsibilities page (for the full breakdown).

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/introduction/what-is-factorial.md
git commit -m "docs: write What is Factorial introduction page"
```

---

## Task 3: Introduction — What You Build vs. What We Handle

**Files:**
- Modify: `docs/book/src/introduction/responsibilities.md`

**Step 1: Write the page**

Content is defined in the design doc. Include:

1. The responsibility matrix table (Concern / Factorial Handles / You Build)
2. Coverage by game archetype section with the three tiers:
   - Pure factory (70-80%)
   - Automation puzzler (60-70%)
   - Colony sim hybrid (25-35%)
3. A brief paragraph explaining that Factorial is opt-in — you can use just the core production graph, or layer on framework modules as needed

Link glossary terms on first use. Link each "Factorial Handles" cell to the relevant Core Concepts or Framework Modules page.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/introduction/responsibilities.md
git commit -m "docs: write responsibilities matrix page"
```

---

## Task 4: Introduction — Glossary

**Files:**
- Modify: `docs/book/src/introduction/glossary.md`

**Step 1: Write the glossary**

One definition per term, using HTML anchor IDs for cross-linking. Each entry format:

```markdown
## Production Graph {#production-graph}

The directed graph of buildings and connections that defines a factory's topology. Nodes represent buildings; edges represent transport connections between them. The engine evaluates the graph in topological order each tick.

**See:** [The Production Graph](../core-concepts/production-graph.md)
```

Terms to define (from design doc): Production Graph, Node, Edge, Junction, Processor, Transport Strategy, Tick, Fixed-Point (Fixed64/Fixed32), UPS, State Hash, Modifier, Registry.

Also add these terms discovered during API exploration:
- **Inventory** — Input and output item storage attached to a node
- **BuildingTypeId** — A numeric identifier for a type of building in the game's registry
- **ItemTypeId** — A numeric identifier for a type of item
- **Stall** — When a processor cannot work (missing inputs, output full, no power, depleted)
- **Snapshot** — A read-only view of a node or transport's current state, used for rendering

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/introduction/glossary.md
git commit -m "docs: write glossary with cross-link anchors"
```

---

## Task 5: Example Crates — Restructure Existing Examples

The existing examples live at `crates/factorial-core/examples/`. The design calls for examples as separate workspace member crates under `examples/`. However, to avoid disrupting the existing workspace structure, we'll use a hybrid approach: keep examples as `[[example]]` targets within `factorial-core` but add new standalone example crates for cross-crate examples (power, fluid, tech-tree, spatial, etc.).

**Files:**
- Create: `examples/transport_showcase.rs` (in `crates/factorial-core/examples/`)
- Create: `examples/events_and_queries.rs` (in `crates/factorial-core/examples/`)
- Create: `examples/save_load.rs` (in `crates/factorial-core/examples/`)
- Create: `examples/multiplayer_desync.rs` (in `crates/factorial-core/examples/`)
- Create: `crates/factorial-examples/Cargo.toml`
- Create: `crates/factorial-examples/examples/power_network.rs`
- Create: `crates/factorial-examples/examples/fluid_network.rs`
- Create: `crates/factorial-examples/examples/tech_tree.rs`
- Create: `crates/factorial-examples/examples/spatial_blueprints.rs`
- Modify: `Cargo.toml` (add `factorial-examples` to workspace members)

**Step 1: Create the cross-crate examples crate**

Create `crates/factorial-examples/Cargo.toml`:

```toml
[package]
name = "factorial-examples"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
factorial-core = { path = "../factorial-core" }
factorial-power = { path = "../factorial-power" }
factorial-stats = { path = "../factorial-stats" }
factorial-tech-tree = { path = "../factorial-tech-tree" }
factorial-spatial = { path = "../factorial-spatial" }
factorial-fluid = { path = "../factorial-fluid" }
```

**Step 2: Add to workspace**

Add `"crates/factorial-examples"` to the `members` array in root `Cargo.toml`.

**Step 3: Write `transport_showcase.rs`**

Location: `crates/factorial-core/examples/transport_showcase.rs`

Demonstrates all four transport strategies on the same source node, with four parallel chains:
- Flow: belt (continuous rate)
- Item: discrete items on slots
- Batch: train batches
- Vehicle: truck round-trips

Each chain: Source → Transport → Demand consumer. Run 20 ticks, print throughput comparison.

Key API calls to demonstrate:
```rust
Transport::Flow(FlowTransport { rate, buffer_capacity, latency })
Transport::Item(ItemTransport { speed, slot_count, lanes })
Transport::Batch(BatchTransport { batch_size, cycle_time })
Transport::Vehicle(VehicleTransport { capacity, travel_time })
```

**Step 4: Write `events_and_queries.rs`**

Location: `crates/factorial-core/examples/events_and_queries.rs`

Demonstrates:
- `engine.on_passive(EventKind::ItemProduced, ...)` — passive listener
- `engine.on_reactive(EventKind::BuildingStalled, ...)` — reactive handler
- `engine.snapshot_all_nodes()` — bulk query
- `engine.get_processor_progress(node)` — single query
- `engine.get_edge_utilization(edge)` — transport query

Set up a small factory, run ticks, print events as they fire.

**Step 5: Write `save_load.rs`**

Location: `crates/factorial-core/examples/save_load.rs`

Demonstrates:
- Build a factory, run 10 ticks
- `engine.serialize()` — save state
- Modify the factory (add a node)
- `Engine::deserialize(bytes)` — restore saved state
- Verify state hash matches pre-save hash

**Step 6: Write `multiplayer_desync.rs`**

Location: `crates/factorial-core/examples/multiplayer_desync.rs`

Demonstrates:
- Create two identical engines
- Apply same operations to both
- `engine.state_hash()` on both — show they match
- Apply a different operation to one engine
- Show state hashes diverge — desync detected

**Step 7: Write `power_network.rs`**

Location: `crates/factorial-examples/examples/power_network.rs`

Demonstrates:
- `PowerModule::new()`, `create_network()`
- `add_producer(node, network, PowerProducer { capacity })`
- `add_consumer(node, network, PowerConsumer { demand })`
- `add_consumer_with_priority(node, network, spec, PowerPriority::High)`
- `power.tick(tick)`, `power.satisfaction(network)`
- Show brownout: demand exceeds supply, satisfaction < 1.0

**Step 8: Write `fluid_network.rs`**

Location: `crates/factorial-examples/examples/fluid_network.rs`

Demonstrates:
- `FluidModule::new()`, `create_network(fluid_type)`
- `add_producer`, `add_consumer`, `add_pipe`, `add_storage`
- `fluid.tick(tick)`, `fluid.pressure(network)`
- Show pressure dynamics and storage fill/drain

**Step 9: Write `tech_tree.rs`**

Location: `crates/factorial-examples/examples/tech_tree.rs`

Demonstrates:
- `TechTree::new()`, `register(Technology { ... })`
- Prerequisites and unlock chains
- `start_research(id)`, `contribute_points(id, points)`
- `is_completed(id)`, `all_unlocks()`
- Show research completion flow

**Step 10: Write `spatial_blueprints.rs`**

Location: `crates/factorial-examples/examples/spatial_blueprints.rs`

Demonstrates:
- `SpatialIndex::new()`, `place(node, position, footprint)`
- `can_place()`, `is_occupied()`, collision detection
- `nodes_in_radius()`, `neighbors_4()`
- Building rotation with `BuildingFootprint::rotated(Rotation::Cw90)`

**Step 11: Verify all examples compile**

Run: `cargo build --workspace --examples`
Expected: All examples compile successfully.

**Step 12: Verify all tests still pass**

Run: `cargo test --workspace`
Expected: All existing tests pass, no regressions.

**Step 13: Commit**

```bash
git add crates/factorial-core/examples/ crates/factorial-examples/ Cargo.toml
git commit -m "docs: add compile-tested example crates for all cookbook topics"
```

---

## Task 6: Getting Started — Rust Quick Start

**Files:**
- Modify: `docs/book/src/getting-started/rust.md`

**Step 1: Write the page**

Structure:
1. **Add the dependency** — `cargo add factorial-core` or Cargo.toml snippet
2. **Create an engine** — `Engine::new(SimulationStrategy::Tick)`
3. **Add nodes** — `queue_add_node`, `apply_mutations`, `resolve_node`
4. **Connect them** — `queue_connect`, `apply_mutations`, `resolve_edge`
5. **Configure processors** — `set_processor` with Source and Fixed examples
6. **Set up inventories** — `set_input_inventory`, `set_output_inventory`
7. **Configure transport** — `set_transport` with Flow
8. **Run the simulation** — `engine.step()` in a loop
9. **Query state** — `snapshot_all_nodes()`, `get_processor_progress()`

All code snippets are excerpted from `crates/factorial-core/examples/minimal_factory.rs`. Each snippet includes a comment with the line range reference.

End with: *"Full source: [`examples/minimal_factory.rs`](https://github.com/user/factorial/blob/main/crates/factorial-core/examples/minimal_factory.rs)"*

Link to: Glossary terms on first use, Core Concepts pages for deeper reading.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/getting-started/rust.md
git commit -m "docs: write Rust quick start guide"
```

---

## Task 7: Getting Started — C/FFI Quick Start

**Files:**
- Modify: `docs/book/src/getting-started/ffi.md`

**Step 1: Write the page**

Structure:
1. **Build the library** — `cargo build -p factorial-ffi --release`
2. **Generate the header** — `cbindgen --crate factorial-ffi -o factorial.h`
3. **Link in your project** — compiler flags for static linking
4. **Create an engine** — `factorial_create()` returns `FactorialEngine*`
5. **Add nodes and connect** — `factorial_add_node`, `factorial_connect`, `factorial_apply_mutations`
6. **Configure** — `factorial_set_source`, `factorial_set_fixed_processor`
7. **Step the simulation** — `factorial_step(engine)`
8. **Poll events** — `factorial_poll_events(engine, &buffer)`
9. **Query state** — `factorial_get_processor_state`, `factorial_node_count`
10. **Cleanup** — `factorial_destroy(engine)`

Show C code snippets demonstrating each step. Note the error handling pattern (check `FactorialResult` return values).

Link to: FFI Conventions page, FFI Function Reference.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/getting-started/ffi.md
git commit -m "docs: write C/FFI quick start guide"
```

---

## Task 8: Getting Started — WASM Quick Start

**Files:**
- Modify: `docs/book/src/getting-started/wasm.md`

**Step 1: Write the page**

This is a stub with clear signposting, since WASM bindings are a future target. Structure:
1. **Status** — WASM support is planned but not yet available as a first-class target
2. **Architecture** — How it will work: compile `factorial-core` to `wasm32-unknown-unknown`, use `wasm-bindgen` for JS interop
3. **Why it will work** — Fixed-point arithmetic means no float non-determinism across platforms, no system dependencies
4. **Tracking** — Link to the relevant issue/milestone if one exists, otherwise note "coming soon"

Keep it short (under 100 lines). Don't promise timelines.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/getting-started/wasm.md
git commit -m "docs: write WASM quick start stub"
```

---

## Task 9: Core Concepts — The Production Graph

**Files:**
- Modify: `docs/book/src/core-concepts/production-graph.md`

**Step 1: Write the page**

Cover:
1. **What is a production graph** — Directed graph where nodes are buildings and edges are transport connections. The engine evaluates all nodes in topological order each tick.
2. **Creating nodes** — `queue_add_node(BuildingTypeId)`, batched mutations, `apply_mutations()`, `resolve_node()`. Explain why mutations are queued (determinism).
3. **Connecting nodes** — `queue_connect(from, to)`, edges are directional (items flow from → to).
4. **Removing nodes/edges** — `queue_remove_node`, `queue_disconnect`.
5. **Junctions** — Splitters, mergers, inserters via `set_junction()`. Nodes that route without processing.
6. **Topological ordering** — The engine automatically sorts the graph. No manual ordering needed. Cycles are detected and reported.
7. **ASCII diagram** of a simple graph:

```
[Mine] --belt--> [Smelter] --belt--> [Assembler]
                     |
                     +---belt--> [Storage]
```

Code snippets excerpted from `minimal_factory.rs` and `production_chain.rs`.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/core-concepts/production-graph.md
git commit -m "docs: write production graph concepts page"
```

---

## Task 10: Core Concepts — Processors

**Files:**
- Modify: `docs/book/src/core-concepts/processors.md`

**Step 1: Write the page**

Cover each processor type with its use case, construction, and behavior:

1. **Source** — Generates items from nothing (mines, pumps). Fields: `output_type`, `base_rate`, `depletion` (Infinite/Finite/Decaying).
2. **Fixed** — Transforms inputs to outputs over a duration (smelters, assemblers). Fields: `inputs`, `outputs`, `duration`.
3. **Property** — Transforms item properties without changing item type (quality upgrades, sorting). Fields: `input_type`, `output_type`, `transform`.
4. **Demand** — Consumes items at a rate (research labs, sinks). Fields: `input_type`, `base_rate`, `accepted_types`.
5. **Passthrough** — No processing, just passes items through (used with junctions).

Include ProcessorState (Idle/Working/Stalled) and StallReason (MissingInputs/OutputFull/NoPower/Depleted).

Include Modifiers section: ModifierKind (Speed/Productivity/Efficiency), StackingRule (Multiplicative/Additive/Diminishing/Capped).

Code from `production_chain.rs` (shows Source + Fixed + Modifiers).

Link to: existing `docs/guides/custom-processors.md` for the deep dive.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/core-concepts/processors.md
git commit -m "docs: write processors concepts page"
```

---

## Task 11: Core Concepts — Transport Strategies

**Files:**
- Modify: `docs/book/src/core-concepts/transport.md`

**Step 1: Write the page**

Cover each strategy:
1. **Flow** — Continuous rate-based (belts, pipes). Best for high-throughput, simple connections. Fields: `rate`, `buffer_capacity`, `latency`.
2. **Item** — Discrete items on slots (conveyor belts with visible items). Fields: `speed`, `slot_count`, `lanes`.
3. **Batch** — Periodic bulk transfers (trains, logistics drones). Fields: `batch_size`, `cycle_time`.
4. **Vehicle** — Round-trip transport with travel time (trucks, rockets). Fields: `capacity`, `travel_time`.

Include a comparison table:

| Strategy | Use Case | Throughput Model | Latency |
|----------|----------|-----------------|---------|
| Flow | Belts, pipes | Continuous rate | Configurable |
| Item | Visual belts | Slot-based | Depends on speed/slots |
| Batch | Trains | Periodic bursts | cycle_time |
| Vehicle | Trucks | Round-trip | 2 × travel_time |

Code from `transport_showcase.rs`.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/core-concepts/transport.md
git commit -m "docs: write transport strategies concepts page"
```

---

## Task 12: Core Concepts — Events

**Files:**
- Modify: `docs/book/src/core-concepts/events.md`

**Step 1: Write the page**

Cover:
1. **Event types** — All 12 events: ItemProduced, ItemConsumed, RecipeStarted, RecipeCompleted, BuildingStalled, BuildingResumed, ItemDelivered, TransportFull, NodeAdded, NodeRemoved, EdgeAdded, EdgeRemoved.
2. **Passive listeners** — `engine.on_passive(EventKind, callback)`. Read-only observation. For stats, logging, UI updates.
3. **Reactive handlers** — `engine.on_reactive(EventKind, handler)`. Can return EventMutation (AddNode, RemoveNode, Connect, Disconnect). For automated responses.
4. **Event suppression** — `engine.suppress_event(EventKind)`. For performance when you don't need certain events.
5. **Pull-based polling (FFI)** — `factorial_poll_events()`. Ring buffer with bounded memory. Explain why callbacks don't cross FFI.

Code from `events_and_queries.rs`.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/core-concepts/events.md
git commit -m "docs: write events concepts page"
```

---

## Task 13: Core Concepts — Queries

**Files:**
- Modify: `docs/book/src/core-concepts/queries.md`

**Step 1: Write the page**

Cover the read-only query API:
1. **Node snapshots** — `snapshot_node(id)`, `snapshot_all_nodes()`. Returns `NodeSnapshot` with id, building_type, processor_state, progress, input_contents, output_contents.
2. **Transport snapshots** — `snapshot_transport(edge)`. Returns `TransportSnapshot`.
3. **Progress queries** — `get_processor_progress(node)` returns Fixed64 (0.0-1.0). Use for animation.
4. **Utilization** — `get_edge_utilization(edge)` returns Fixed64.
5. **Counts** — `node_count()`, `edge_count()`.
6. **Topology** — `get_inputs(node)`, `get_outputs(node)`.
7. **Diagnostics** — `diagnose_node(node)` returns `DiagnosticInfo`.

Explain the pattern: queries are cheap, read-only, and safe to call every frame for rendering.

Code from `events_and_queries.rs`.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/core-concepts/queries.md
git commit -m "docs: write queries concepts page"
```

---

## Task 14: Core Concepts — Determinism & Fixed-Point

**Files:**
- Modify: `docs/book/src/core-concepts/determinism.md`

**Step 1: Write the page**

Cover:
1. **Why determinism matters** — Multiplayer lockstep, replay, save/load consistency, reproducible bugs.
2. **Fixed-point arithmetic** — `Fixed64` (Q32.32) and `Fixed32` (Q16.16). No IEEE 754 floats anywhere in simulation. Same result on x86, ARM, WASM.
3. **Topological evaluation order** — Nodes evaluated in deterministic order every tick.
4. **Queued mutations** — Graph changes are batched and applied atomically, not interleaved with simulation.
5. **State hashing** — `engine.state_hash()` produces a u64 hash of the entire engine state. Two engines with the same inputs produce the same hash.
6. **How to use for multiplayer** — Each client runs its own engine. Periodically compare state hashes. If hashes diverge, desync detected.

Code from `multiplayer_desync.rs`.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/core-concepts/determinism.md
git commit -m "docs: write determinism and fixed-point concepts page"
```

---

## Task 15: Core Concepts — Serialization

**Files:**
- Modify: `docs/book/src/core-concepts/serialization.md`

**Step 1: Write the page**

Cover:
1. **Save/load** — `engine.serialize()` returns bytes, `Engine::deserialize(bytes)` restores state.
2. **Binary format** — Uses `bitcode` for compact binary encoding. Not JSON — optimized for size and speed.
3. **Versioning** — Snapshots include a version field. Future versions can migrate old saves.
4. **Module hooks** — Framework modules can register serialization hooks for their own state.
5. **Dirty tracking** — `engine.is_dirty()` tells you if state has changed since last save. Use to implement auto-save or "unsaved changes" indicators.

Code from `save_load.rs`.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/core-concepts/serialization.md
git commit -m "docs: write serialization concepts page"
```

---

## Task 16: Framework Modules — All Five Pages

**Files:**
- Modify: `docs/book/src/modules/power.md`
- Modify: `docs/book/src/modules/fluid.md`
- Modify: `docs/book/src/modules/tech-tree.md`
- Modify: `docs/book/src/modules/spatial.md`
- Modify: `docs/book/src/modules/stats.md`

**Step 1: Write power.md**

Cover: PowerModule lifecycle, networks, producers/consumers/storage, priority levels, satisfaction ratio, tick integration. Code from `power_network.rs`.

**Step 2: Write fluid.md**

Cover: FluidModule lifecycle, fluid types, producers/consumers/storage/pipes, pressure model, tick integration. Code from `fluid_network.rs`.

**Step 3: Write tech-tree.md**

Cover: TechTree registration, Technology struct, prerequisites, ResearchCost variants (Items/Points/Delivery/Rate/ItemRate/Custom), CostScaling, research flow, unlock checking, events. Code from `tech_tree.rs`.

**Step 4: Write spatial.md**

Cover: SpatialIndex, GridPosition, BuildingFootprint, Rotation, placement/removal, collision detection, spatial queries (radius, rect, neighbors), Blueprint system overview. Code from `spatial_blueprints.rs`.

**Step 5: Write stats.md**

Cover: ProductionStats, StatsConfig (window size, history capacity), process_event/end_tick integration, rate queries (production, consumption, throughput), uptime/idle/stall ratios, historical data via RingBuffer.

**Step 6: Verify book builds**

Run: `mdbook build docs/book`

**Step 7: Commit**

```bash
git add docs/book/src/modules/
git commit -m "docs: write all five framework module pages"
```

---

## Task 17: FFI Reference — Conventions & Safety

**Files:**
- Modify: `docs/book/src/ffi/conventions.md`

**Step 1: Write the page**

Cover:
1. **Opaque pointer pattern** — `FactorialEngine*` is opaque. Never dereference. Only pass to `factorial_*` functions.
2. **Result codes** — `FactorialResult` enum: `Ok`, `NullPointer`, `InvalidId`, `InvalidState`, `SerializationError`, `Poisoned`. Always check return value.
3. **Poisoned flag** — If Rust panics, engine is poisoned. All subsequent calls return `Poisoned`. You must destroy and recreate.
4. **Pull-based events** — No callbacks. Call `factorial_poll_events()` each frame. Ring buffer is bounded — events are dropped if not polled.
5. **Buffer ownership** — Caller allocates buffers for bulk queries. Serialization returns library-allocated buffers freed with `factorial_free_buffer()`.
6. **ID types** — `FfiNodeId` and `FfiEdgeId` are `u64`. Not pointers. Safe to store, compare, serialize.
7. **Thread safety** — Engine is NOT thread-safe. Call all functions from the same thread (or synchronize externally).

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/ffi/conventions.md
git commit -m "docs: write FFI conventions and safety page"
```

---

## Task 18: FFI Reference — Function Reference

**Files:**
- Modify: `docs/book/src/ffi/reference.md`

**Step 1: Write the page**

Organized by category, each entry has: C signature, brief description, link to Rust concept page. Categories:

1. **Lifecycle** — `factorial_create`, `factorial_create_delta`, `factorial_destroy`
2. **Simulation** — `factorial_step`, `factorial_advance`
3. **Graph Mutation** — `factorial_add_node`, `factorial_remove_node`, `factorial_connect`, `factorial_disconnect`, `factorial_apply_mutations`
4. **Processor Configuration** — `factorial_set_source`, `factorial_set_fixed_processor`
5. **Queries** — `factorial_node_count`, `factorial_edge_count`, `factorial_get_tick`, `factorial_get_state_hash`, `factorial_get_processor_state`
6. **Inventory** — `factorial_get_input_inventory_count`, `factorial_get_output_inventory_count`
7. **Events** — `factorial_poll_events`
8. **Serialization** — `factorial_serialize`, `factorial_deserialize`, `factorial_free_buffer`

Source: `crates/factorial-ffi/src/lib.rs` — extract exact signatures from the `extern "C"` functions.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/ffi/reference.md
git commit -m "docs: write FFI function reference"
```

---

## Task 19: FFI Reference — Language-Specific Bindings

**Files:**
- Modify: `docs/book/src/ffi/bindings.md`

**Step 1: Write the page**

**C section (complete):**
1. Generate header: `cbindgen --crate factorial-ffi -o factorial.h`
2. Static linking: `gcc -o mygame mygame.c -L target/release -lfactorial_ffi`
3. Dynamic linking: platform-specific instructions (Linux .so, macOS .dylib, Windows .dll)
4. Full walkthrough referencing the C/FFI Quick Start

**Future bindings (stubs):**
- C# / Unity — Approach: P/Invoke wrapper around C API. Status: community contribution welcome.
- GDScript / Godot — Approach: GDExtension wrapper. Status: community contribution welcome.
- Python — Approach: PyO3 or ctypes wrapper. Status: community contribution welcome.

Each stub explains the general approach (wrap the C API in idiomatic bindings) and links to the FFI Conventions page.

**Step 2: Verify book builds**

Run: `mdbook build docs/book`

**Step 3: Commit**

```bash
git add docs/book/src/ffi/bindings.md
git commit -m "docs: write language-specific bindings page"
```

---

## Task 20: Cookbook — All Ten Recipes

**Files:**
- Modify: all 10 files in `docs/book/src/cookbook/`

Each recipe follows the template from the design doc:

```markdown
## Recipe Title

**Goal:** One sentence.
**Prerequisites:** Links to concept pages.
**Example:** `path/to/example.rs`

### Steps
(5-10 line snippets excerpted from the real example)

### What's Happening
(Engine behavior explanation)

### Variations
(Tweaks and related recipe links)
```

**Step 1: Write `smelting-chain.md`** — Excerpts from `minimal_factory.rs`. Goal: mine ore, smelt into plates.

**Step 2: Write `production-line.md`** — Excerpts from `production_chain.rs`. Goal: multi-step chain with modifiers.

**Step 3: Write `transport-strategies.md`** — Excerpts from `transport_showcase.rs`. Goal: compare all four transport types.

**Step 4: Write `events.md`** — Excerpts from `events_and_queries.rs`. Goal: react to production events and query state.

**Step 5: Write `power.md`** — Excerpts from `power_network.rs`. Goal: add power grid with brownout handling.

**Step 6: Write `fluids.md`** — Excerpts from `fluid_network.rs`. Goal: pipe fluids between buildings.

**Step 7: Write `tech-tree.md`** — Excerpts from `tech_tree.rs`. Goal: gate buildings behind research.

**Step 8: Write `spatial.md`** — Excerpts from `spatial_blueprints.rs`. Goal: place buildings on a grid.

**Step 9: Write `serialization.md`** — Excerpts from `save_load.rs`. Goal: save, load, and verify state.

**Step 10: Write `multiplayer.md`** — Excerpts from `multiplayer_desync.rs`. Goal: detect desync between two engines.

**Step 11: Verify book builds**

Run: `mdbook build docs/book`

**Step 12: Commit**

```bash
git add docs/book/src/cookbook/
git commit -m "docs: write all ten cookbook recipes"
```

---

## Task 21: Architecture Deep Dive — All Three Pages

**Files:**
- Modify: `docs/book/src/architecture/performance.md`
- Modify: `docs/book/src/architecture/memory.md`
- Modify: `docs/book/src/architecture/decisions.md`

**Step 1: Write `performance.md`**

Cover: Target budget (60 UPS at <8ms/tick for 5,000+ buildings), tick breakdown (topology sort, processor eval, transport tick, event dispatch), profiling feature flag (`features = ["profiling"]`), `last_tick_profile()` API, benchmark results from `sim_bench.rs`.

**Step 2: Write `memory.md`**

Cover: Struct-of-arrays layout (components stored in separate SlotMaps), arena allocation with generational indices (SlotMap), pre-allocated ring buffers for events and belt state, no unbounded allocations during simulation.

**Step 3: Write `decisions.md`**

Cover: Key trade-offs made and why:
- Fixed-point over floats (determinism > convenience)
- Enum dispatch over trait objects (performance > extensibility)
- Queued mutations over immediate (determinism > responsiveness)
- Pull events over push for FFI (safety > simplicity)
- Single-threaded by default (determinism > parallelism)

Source: `docs/plans/2026-02-04-factorial-engine-design.md` contains extensive rationale.

**Step 4: Verify book builds**

Run: `mdbook build docs/book`

**Step 5: Commit**

```bash
git add docs/book/src/architecture/
git commit -m "docs: write architecture deep dive pages"
```

---

## Task 22: README Overhaul

**Files:**
- Modify: `README.md` (root of repository)

**Step 1: Write the new README**

The README becomes the landing page that links into the book. Structure:

1. **Title + one-line description** — "Factorial — A headless factory game engine"
2. **Badges** — Build status, docs link, crates.io (when published)
3. **What is Factorial?** — 3-sentence version of the Introduction page
4. **Quick example** — The minimal 15-line Rust snippet from the Quick Start
5. **Documentation** — Link to the mdBook site on GitHub Pages
6. **Features** — Bullet list of key capabilities (production graph, 4 transport types, 5 framework modules, FFI, determinism, serialization)
7. **Getting Started** — Links to Rust Quick Start, FFI Quick Start, WASM Quick Start
8. **Architecture** — The ASCII layer diagram from the Introduction
9. **License** — Whatever the project uses

Keep it under 150 lines. The book has the details; the README is a funnel.

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: overhaul README as landing page for documentation"
```

---

## Task 23: CI — mdBook Build Verification

**Files:**
- Create: `.github/workflows/docs.yml`

**Step 1: Write the GitHub Actions workflow**

```yaml
name: Documentation

on:
  push:
    branches: [main]
    paths:
      - 'docs/book/**'
      - 'crates/*/examples/**'
      - 'crates/factorial-examples/**'
  pull_request:
    paths:
      - 'docs/book/**'
      - 'crates/*/examples/**'
      - 'crates/factorial-examples/**'

jobs:
  build-docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build examples
        run: cargo build --workspace --examples

      - name: Run doc tests
        run: cargo test --doc --workspace

      - name: Install mdBook
        run: cargo install mdbook

      - name: Build book
        run: mdbook build docs/book

      - name: Deploy to GitHub Pages
        if: github.ref == 'refs/heads/main'
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./target/book
```

**Step 2: Commit**

```bash
git add .github/workflows/docs.yml
git commit -m "ci: add documentation build and deploy workflow"
```

---

## Task 24: Final Verification

**Step 1: Build everything**

Run: `cargo build --workspace --examples`
Expected: All crates and examples compile.

**Step 2: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass, no regressions.

**Step 3: Build the book**

Run: `mdbook build docs/book`
Expected: Book builds successfully, no broken links.

**Step 4: Spot-check the rendered output**

Run: `mdbook serve docs/book --open`
Manually verify: navigation works, code blocks render, glossary links resolve, all pages have content.

**Step 5: Final commit if any fixes needed**

Fix any issues found during verification, commit.

---

## Execution Order & Dependencies

Tasks can be parallelized in groups:

| Group | Tasks | Dependencies |
|-------|-------|-------------|
| **Foundation** | 1 (scaffolding) | None |
| **Introduction** | 2, 3, 4 | Task 1 |
| **Examples** | 5 | Task 1 |
| **Getting Started** | 6, 7, 8 | Tasks 1, 5 |
| **Core Concepts** | 9, 10, 11, 12, 13, 14, 15 | Tasks 1, 4, 5 |
| **Framework Modules** | 16 | Tasks 1, 4, 5 |
| **FFI Reference** | 17, 18, 19 | Tasks 1, 4 |
| **Cookbook** | 20 | Tasks 1, 4, 5 |
| **Architecture** | 21 | Task 1 |
| **README** | 22 | Tasks 2, 6, 7 |
| **CI** | 23 | Tasks 1, 5 |
| **Verification** | 24 | All previous |

**Maximum parallelism:** After Task 1, tasks 2/3/4/5/21 can run in parallel. After Task 5, most remaining content tasks can run in parallel.
