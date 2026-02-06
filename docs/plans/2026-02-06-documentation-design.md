# Factorial Documentation Design

**Date:** 2026-02-06
**Status:** Approved
**Audience:** Rust game developers and non-Rust developers (C FFI / WASM) as equal first-class citizens

## Goals

1. A developer reading for 15 minutes understands the architecture and knows what Factorial handles vs. what they build themselves.
2. Documentation never goes stale — all code in docs compiles in CI.
3. Documentation is self-sufficient — nobody should need to read `engine.rs` to understand usage.
4. Domain terminology is accessible via a glossary, not assumed.

## Delivery

- **Source of truth:** Markdown files in the repository under `docs/`.
- **Generated site:** mdBook, published to GitHub Pages.
- **API reference:** `cargo doc` / docs.rs for Rust; dedicated FFI Reference chapter in the book for C/WASM.

## Sitemap

```
Factorial Documentation
├── Introduction
│   ├── What is Factorial?
│   ├── What You Build vs. What We Handle
│   └── Glossary
│
├── Getting Started
│   ├── Rust Quick Start
│   ├── C/FFI Quick Start
│   └── WASM Quick Start
│
├── Core Concepts
│   ├── The Production Graph
│   ├── Processors
│   ├── Transport Strategies
│   ├── Events
│   ├── Queries
│   ├── Determinism & Fixed-Point
│   └── Serialization
│
├── Framework Modules
│   ├── Power Networks
│   ├── Fluid Networks
│   ├── Tech Trees
│   ├── Spatial Grid & Blueprints
│   └── Statistics
│
├── FFI Reference
│   ├── API Conventions & Safety
│   ├── Full Function Reference
│   └── Language-Specific Bindings
│
├── Cookbook
│   ├── Model a Smelting Chain
│   ├── Build a Multi-Step Production Line
│   ├── Choose the Right Transport Strategy
│   ├── React to Production Events
│   ├── Add Power with Brownout Handling
│   ├── Pipe Fluids Between Buildings
│   ├── Gate Buildings Behind Research
│   ├── Place Buildings on a Grid
│   ├── Save, Load, and Migrate State
│   └── Detect Multiplayer Desync
│
└── Architecture Deep Dive
    ├── Performance Model
    ├── Memory Layout (SoA, Arenas)
    └── Design Decisions & Trade-offs
```

## Example Crate Strategy

All code shown in documentation is excerpted from real, compiling example crates. Docs never contain standalone code blocks that aren't backed by a compilable source file.

### Directory Layout

```
examples/
├── minimal_factory/        # Mine → Smelter → Output
├── production_chain/       # Multi-step: ore → plate → gear → circuit
├── transport_showcase/     # One chain per transport strategy
├── events_and_queries/     # Subscribe to events, query state
├── power_network/          # Generators, consumers, brownout handling
├── fluid_network/          # Pipes, pressure, mixing
├── tech_tree/              # Research unlocks gating buildings
├── spatial_blueprints/     # Grid placement, blueprints, ghost buildings
├── save_load/              # Serialize, modify, deserialize, migrate
├── multiplayer_desync/     # Two engines in lockstep, state hash comparison
├── ffi_c_example/          # Standalone C project linking factorial-ffi
└── ffi_wasm_example/       # Browser-based minimal factory via wasm-bindgen
```

- Each Rust example is a workspace member with a `main.rs` of 100-200 lines.
- FFI examples (`ffi_c_example`, `ffi_wasm_example`) live outside the Rust workspace with their own build scripts.
- Guide pages include short inline snippets (10-20 lines) excerpted from the real example files, with a callout: *"Full source: `examples/power_network/main.rs`"*.

## Glossary

Defines domain terms for newcomers and establishes canonical terminology for consistency across all docs.

### Terms

| Term | Definition |
|------|-----------|
| Production Graph | The directed graph of buildings and connections that defines a factory's topology |
| Node | A vertex in the production graph, representing a building |
| Edge | A connection between two nodes carrying items or fluids |
| Junction | A node that routes items without processing (splitters, mergers, inserters) |
| Processor | The logic attached to a node that transforms inputs into outputs (Source, Fixed, Property, Demand) |
| Transport Strategy | How items move along an edge (Flow, Item, Batch, Vehicle) |
| Tick | One discrete simulation step; the engine evaluates all nodes in topological order |
| Fixed-Point (Fixed64/Fixed32) | Deterministic number representation (Q32.32 / Q16.16) used instead of floats |
| UPS | Updates Per Second; the simulation tick rate independent of rendering FPS |
| State Hash | A deterministic hash of the entire engine state, used for desync detection |
| Modifier | A multiplier applied to a node (speed, efficiency, productivity, quality) |
| Registry | The collection of all item types, recipes, and building definitions |

### Cross-linking Convention

- First use of a glossary term on any page is **bold** and linked to `glossary.md#term`.
- Subsequent uses on the same page are plain text.
- No JavaScript tooltips — simple markdown links only.

## "What You Build vs. What We Handle" Page

### Responsibility Matrix

| Concern | Factorial Handles | You Build |
|---------|------------------|-----------|
| Production math | Recipe rates, throughput, modifiers | Recipe definitions (data) |
| Item transport | Belt/pipe/train/vehicle simulation | Rendering belts, animations |
| Power grid | Supply/demand/satisfaction calculation | UI for power overlay |
| Tech tree | Unlock logic, dependency resolution | Research UI, cost balancing |
| Save/load | Engine state serialization | Your game-specific state |
| Multiplayer sync | Deterministic simulation, state hashing | Netcode, lobby, input relay |
| Spatial placement | Collision, grid snaps, blueprints | Visual placement preview |
| Game loop | `engine.step()` / `engine.advance(dt)` | When/how often to call it |

### Coverage by Game Archetype

- **Pure factory** (Factorio-like): 70-80% coverage. You build: world gen, rendering, combat, UX.
- **Automation puzzler** (Shapez-like): 60-70% coverage. You build: shape logic, level progression, scoring.
- **Colony sim hybrid** (ONI-like): 25-35% coverage. You build: colonist AI, environment sim, moods. Factorial handles production/power/fluid subsystems.

## FFI Documentation Track

The FFI docs stand on their own — not an afterthought appendix.

### API Conventions Page

Covers patterns once, up front:
- All functions take an opaque `FactorialEngine*` pointer.
- Errors return status codes; engine sets a poisoned flag on panic.
- Pull-based event model — caller polls, no callbacks crossing FFI.
- Caller-allocated buffers with capacity parameters for bulk queries.
- All IDs are `uint32_t` handles, not pointers.

### Function Reference

Organized by the same conceptual grouping as the Rust docs (engine lifecycle, graph mutations, processors, transport, queries, events, serialization). Each entry: C signature, one-line description, link to corresponding Rust concept page.

### Language-Specific Bindings

- **C (exists today):** Header generation via `cbindgen`, linking instructions (static/dynamic), full `ffi_c_example` walkthrough.
- **Future bindings (C#/Unity, GDScript/Godot, Python):** Stub pages explaining the approach, marked as "community contribution welcome."
- **WASM:** Separate quick start page due to distinct toolchain (`wasm-pack`, `wasm-bindgen`).

## Cookbook

Each recipe follows a rigid template:

```
## Recipe Title (verb phrase)

**Goal:** One sentence.
**Prerequisites:** Links to concept pages.
**Example:** `examples/relevant_example/main.rs`

### Steps
1. Step with 5-10 line code snippet (excerpted from example)
2. ...

### What's Happening
Brief explanation of engine behavior.

### Variations
- Tweaks for different scenarios
- Links to related recipes
```

### Recipe List

| Recipe | Example Crate | Concepts Covered |
|--------|--------------|------------------|
| Model a Smelting Chain | `minimal_factory` | Nodes, Source/Fixed processors, Flow transport |
| Build a Multi-Step Production Line | `production_chain` | Chained processors, modifiers, throughput |
| Choose the Right Transport Strategy | `transport_showcase` | Flow vs Item vs Batch vs Vehicle |
| React to Production Events | `events_and_queries` | Event subscription, polling, query API |
| Add Power with Brownout Handling | `power_network` | Power module, satisfaction, priority |
| Pipe Fluids Between Buildings | `fluid_network` | Fluid module, pressure, mixing |
| Gate Buildings Behind Research | `tech_tree` | Tech tree module, unlock checks |
| Place Buildings on a Grid | `spatial_blueprints` | Spatial module, blueprints, ghosts |
| Save, Load, and Migrate State | `save_load` | Serialization, versioning, migration |
| Detect Multiplayer Desync | `multiplayer_desync` | Determinism, state hashing, replay |

## CI Pipeline

Three layers of staleness prevention:

### Layer 1: Example Crates Compile

`cargo build --workspace` includes all Rust example crates. API changes that break examples break the build. Primary defense — zero maintenance.

### Layer 2: Doc-Tests

`cargo test --doc` tests `///` code blocks on public types and methods. Catches signature drift on individual APIs.

### Layer 3: Link Checking

`mdbook build docs/` fails on broken internal links. Add `lychee` or `mdbook-linkcheck` for cross-reference and glossary link validation.

### CI Step

```yaml
- cargo build --workspace          # examples compile
- cargo test --doc                 # doc-tests pass
- mdbook build docs/               # book builds, links valid
```

### Deliberately Skipped

- No `skeptic` crate for markdown code block compilation (example crates serve this role).
- No screenshot testing (no UI to test).
- No automated prose freshness checks (caught in code review).
