# Factorial Engine: Multi-Persona Stress Test Findings

**Date:** 2026-02-05
**Status:** Draft
**Method:** 8 independent persona reviews of engine design doc, public API surface, and implementation

---

## Table of Contents

1. [Methodology](#1-methodology)
2. [Cross-Persona Heat Map](#2-cross-persona-heat-map)
3. [Top 15 Engine Features by Demand](#3-top-15-engine-features-by-demand)
4. [Architecture and Documentation Gaps](#4-architecture-and-documentation-gaps)
5. [Game-Layer Exclusions](#5-game-layer-exclusions)
6. [Key Insights](#6-key-insights)
7. [Recommended Action](#7-recommended-action)

---

## 1. Methodology

Eight personas independently reviewed the Factorial engine's design doc, public API surface, and source code. Each produced a prioritized feature wishlist from their perspective, classifying every request as ENGINE, GAME, or HYBRID.

| # | Persona | Perspective | Primary Concerns |
|---|---------|-------------|------------------|
| P1 | Indie Solo Dev | First factory game, Godot, fast prototyping | Ease of use, docs, sensible defaults |
| P2 | Experienced Studio (8-12 engineers) | Factorio-scale commercial release | Scalability to 100k+ nodes, extensibility, long-term maintenance |
| P3 | Mobile Game Dev | iOS/Android factory/idle game | Memory footprint, battery, binary size, offline play |
| P4 | Modding Community Rep | Factorio/Satisfactory mod veteran | Runtime extensibility, data-driven config, no recompilation |
| P5 | Multiplayer Engineer | 2-8 player co-op/competitive | Determinism, state sync, conflict resolution, replays |
| P6 | Tycoon/Hybrid Dev | Big Pharma / Good Company style | Economic primitives, demand modeling, cost tracking |
| P7 | Educational Game Dev | Teaching supply chain / systems thinking | Observability, constraints, scenario authoring, replay |
| P8 | Engine Contributor | Open-source contributor with engine experience | Code quality, architecture, API consistency, testing |

Each persona was given full access to the design doc, all `lib.rs` files, `engine.rs`, and module-specific source files relevant to their perspective. They were asked to be honest about the engine-vs-game boundary.

---

## 2. Cross-Persona Heat Map

Features grouped by theme, ranked by persona count. Higher count = stronger cross-persona signal.

### Serialization & Persistence

| Feature | Personas | Count | Layer |
|---------|----------|-------|-------|
| Version Migration Framework | P1, P2, P3, P8 | **4** | ENGINE |
| Incremental / Dirty-State Serialization | P2, P3 | 2 | ENGINE |
| Snapshot Compression | P3 | 1 | ENGINE |
| Lazy Deserialization | P3 | 1 | ENGINE |

### Spatial & World Systems

| Feature | Personas | Count | Layer |
|---------|----------|-------|-------|
| Spatial Grid Module | P1, P2, P8 | **3** | ENGINE |
| Fluid Networks Module | P2, P8 | 2 | ENGINE |
| Logic/Circuit Networks Module | P2, P8 | 2 | ENGINE |
| Blueprint/Ghost Placement | P1, P8 | 2 | ENGINE |

### Observability & Debugging

| Feature | Personas | Count | Layer |
|---------|----------|-------|-------|
| Profiling / Diagnostic Instrumentation | P1, P2, P8 | **3** | ENGINE |
| Visual Debugging / State Inspector | P1, P8 | 2 | HYBRID |

### Extensibility & Modding

| Feature | Personas | Count | Layer |
|---------|----------|-------|-------|
| Module System (formal trait + lifecycle) | P2, P8 | 2 | ENGINE |
| Data-Driven Registry (JSON/RON/TOML) | P1, P4 | 2 | ENGINE |
| Runtime Processor Extension | P4 | 1 | ENGINE |
| Runtime Transport Extension | P4 | 1 | ENGINE |
| Scripting Language Integration | P4 | 1 | ENGINE |
| Mod Loading & Dependency System | P4 | 1 | ENGINE |

### Multiplayer & Determinism

| Feature | Personas | Count | Layer |
|---------|----------|-------|-------|
| Multiplayer Validation Tools | P2, P5 | 2 | ENGINE |
| Replay Recording & Playback | P5, P7 | 2 | ENGINE |
| Input Command Queue with Tick Assignment | P5 | 1 | ENGINE |
| Conflict Detection for Graph Mutations | P5 | 1 | ENGINE |

### Simulation & Processing

| Feature | Personas | Count | Layer |
|---------|----------|-------|-------|
| Modifier Stacking Rules | P2, P6 | 2 | ENGINE |
| Junctions (Inserters/Splitters/Mergers) | P2 | 1 | ENGINE |
| Belt Performance (transport lines) | P2 | 1 | ENGINE |
| Demand Sink Nodes (DemandProcessor) | P6 | 1 | ENGINE |
| Configurable Processor Stall Hooks | P6 | 1 | ENGINE |

### Events

| Feature | Personas | Count | Layer |
|---------|----------|-------|-------|
| Event Filtering & Priorities | P2, P7 | 2 | ENGINE |
| Event Buffer Configuration | P3, P8 | 2 | ENGINE |

### Resource & Lifecycle Management

| Feature | Personas | Count | Layer |
|---------|----------|-------|-------|
| Pause/Resume with Resource Cleanup | P3, P8 | 2 | ENGINE |
| Memory Budget API | P3 | 1 | ENGINE |
| Crate Feature Flags for Binary Size | P3 | 1 | ENGINE |
| Offline Simulation Progress | P3 | 1 | ENGINE |

### Documentation & DX

| Feature | Personas | Count | Layer |
|---------|----------|-------|-------|
| Quickstart / Examples | P1, P8 | 2 | DOC |
| Belt Rendering Guidance | P1 | 1 | DOC |
| Fixed64 Conversion Ergonomics | P1 | 1 | DOC |

---

## 3. Top 15 Engine Features by Demand

Ranked by persona count, then by highest priority level assigned. Filtered to engine-appropriate features only.

| # | Feature | Personas | Count | Highest Priority | Justification |
|---|---------|----------|-------|-----------------|---------------|
| 1 | **Serialization Version Migration Framework** | P1, P2, P3, P8 | 4 | CRITICAL | Version numbers exist but no migration helpers. Every persona touching save/load flagged this independently. Content patches, balance changes, and mod updates all break saves without migration paths. |
| 2 | **Spatial Grid Module** | P1, P2, P8 | 3 | CRITICAL | Designed in architecture doc as a planned module but not implemented. Most factory games require placement validation, adjacency queries, and area-of-effect mechanics. Blocks adoption for 80% of target games. |
| 3 | **Profiling / Diagnostic Instrumentation** | P1, P2, P8 | 3 | CRITICAL | Zero observability into phase timing, per-node hotspots, or memory usage. The 6-phase pipeline is a natural fit for per-phase timing. Essential for game devs optimizing tick budgets. |
| 4 | **Module System (formal Module trait)** | P2, P8 | 2 | CRITICAL | Framework modules exist as separate crates but lack a standardized trait with init/tick/serialize hooks. `phase_component()` is a no-op. Blocks clean third-party plugin development. |
| 5 | **Junctions (Inserters/Splitters/Mergers)** | P2 | 1 | CRITICAL | Designed as core graph elements in the architecture doc but not implemented. Any game with belt-based logistics is blocked. This is design doc debt, not a new feature request. |
| 6 | **Data-Driven Registry (JSON/RON)** | P1, P4 | 2 | CRITICAL | The design doc mentions data-driven loading but no loader exists. Both indie devs and modders need to define items/recipes without recompiling Rust. |
| 7 | **Multiplayer Validation Tools** | P2, P5 | 2 | CRITICAL | State diff utility, replay validator, and cross-platform determinism tests are needed to make determinism guarantees practically usable beyond unit tests. |
| 8 | **Incremental / Dirty-State Serialization** | P2, P3 | 2 | CRITICAL | Full-state serialization on every save is too expensive for mobile and large factories. Dirty tracking with delta encoding required for autosave and background persistence. |
| 9 | **Replay Recording & Deterministic Playback** | P5, P7 | 2 | CRITICAL | Snapshot + input log replay is described in design doc but not implemented. Educational, multiplayer, and debugging use cases all depend on it. |
| 10 | **Event Filtering & Priorities** | P2, P7 | 2 | HIGH | Subscribers receive all events of a type with no filtering. At 100k nodes producing 500k events/tick, allocating events nobody needs is wasteful. Need per-subscriber filtering and handler ordering. |
| 11 | **Modifier Stacking Rules** | P2, P6 | 2 | HIGH | Current system applies multiplicatively in registration order. Complex games need diminishing returns, additive groups, caps, and modifier interactions. |
| 12 | **Pause/Resume with Resource Management** | P3, P8 | 2 | CRITICAL | Mobile games need true pause with memory deallocation for backgrounded apps. No pause API or resource cleanup lifecycle exists. |
| 13 | **Fluid Networks Module** | P2, P8 | 2 | HIGH | Designed as a planned high-priority module. Pressure-based flow, mixing rules, and pipe network segmentation needed by any game with fluid logistics. |
| 14 | **Input Command Queue with Tick Assignment** | P5 | 1 | CRITICAL | Lockstep multiplayer requires a formal input queue where player actions are assigned to future ticks. Without this, determinism guarantees cannot be leveraged for networked play. |
| 15 | **Visual Debugging / State Inspector** | P1, P8 | 2 | CRITICAL | A debug mode exposing node states, processing progress, stall reasons, and event flow in structured form would reduce integration time for all adopters. |

---

## 4. Architecture and Documentation Gaps

Non-feature issues raised by persona reviews.

### 4.1 Code Architecture

| Issue | Raised By | Severity | Recommendation |
|-------|-----------|----------|----------------|
| **engine.rs is ~1800 lines** | P8 | HIGH | Decompose into phase modules (`phases/transport.rs`, `phases/process.rs`, etc.). Orchestration loop stays small (~200 lines); phase logic moves out. No performance impact (still monomorphized). |
| **Registry immutability not type-enforced** | P8 | MEDIUM | `RegistryBuilder::build()` produces `Registry` but nothing prevents post-build mutation at the type level. Wrap in `Arc<Registry>` or a frozen newtype. |
| **Panics instead of Results** | P8 | MEDIUM | Several code paths panic on invalid input. FFI callers get poisoned state instead of recoverable errors. Audit `panic!()` calls and convert to `EngineError` enum. |
| **FFI poisoned flag has no diagnostics** | P8 | MEDIUM | Once poisoned, all calls fail silently. Expose `factorial_get_last_error() -> *const c_char`. |
| **SoA description is misleading** | P8 | LOW | Implementation uses sparse SecondaryMaps, not packed contiguous arrays. Update docs to say "sparse SoA via SlotMap." |

### 4.2 Testing

| Issue | Raised By | Severity | Recommendation |
|-------|-----------|----------|----------------|
| **Only 9 integration tests** | P8 | HIGH | Add multi-crate integration tests exercising full tick pipelines, not just per-module unit tests. |
| **No fuzz testing** | P8 | MEDIUM | Graph mutation sequences and serialization round-trips are ideal fuzz targets. |
| **No stress tests at target scale** | P8 | MEDIUM | Benchmarks measure speed at 5,000 nodes but no tests verify correctness at that scale. |

### 4.3 Documentation

| Issue | Raised By | Severity | Recommendation |
|-------|-----------|----------|----------------|
| **No runnable examples** | P1, P8 | HIGH | Zero examples showing registry setup, graph construction, tick execution, and state querying. Add `examples/minimal_factory.rs` at minimum. |
| **Design doc oversells trait-based flexibility** | P8 | MEDIUM | Implementation uses enum dispatch. Update docs to match reality and document the `Custom` variant as the extension point. |
| **No belt rendering guidance** | P1 | MEDIUM | ItemTransport returns positions but no interpolation pattern is documented. |
| **No serialization migration examples** | P1 | MEDIUM | Version tolerance is described abstractly but no code shows how to handle a format change. |
| **Performance escape hatch not demonstrated** | P8 | LOW | `register_custom_system` is described but no example exists. |

---

## 5. Game-Layer Exclusions

Features requested by personas but that belong in the game layer, not the engine. Listed for completeness and to document the boundary decision.

| Feature | Requested By | Why Excluded |
|---------|-------------|--------------|
| Market Price Simulation | P6 | Economic models are game-design-specific; engine provides stats data for pricing inputs |
| Worker/Population AI | P6, P8 | Labor simulation varies per game; engine provides component slots, not AI |
| Tutorial / Progressive Complexity | P3, P7 | Tutorial UX is game-side; engine provides constraint primitives |
| Multiplayer Sync Protocol | P1, P5 | Networking (UDP, WebRTC) is game infrastructure; engine provides determinism + hashing |
| Late Join / Desync Recovery | P5 | Protocol is game-side; engine provides serialization and state hashing |
| Spectator Mode | P5 | Read-only view; query API already enables this |
| Pause Consensus / Jitter Buffer | P5 | Transport-layer timing is game networking |
| Supply Chain Contracts | P6 | Multi-factory business logic |
| Reputation/Quality Feedback Loop | P6 | Game-specific scoring system |
| Heatmaps & Visualizers | P7 | Rendering using stats module data |
| Hint System Hooks | P7 | Game-specific progressive disclosure UX |
| Scenario Authoring Format | P7 | Level design data format; engine provides constraint primitives |
| Player Identity & Permissions | P5 | Authentication is game infrastructure |
| Visual Scripting | P4 | IDE/tooling layer |
| Mod Sandboxing | P4 | Security model depends on distribution platform |

**Principle:** The engine provides primitives, infrastructure, and data. It does not implement game-specific systems. Many excluded features depend on engine APIs that are already provided or requested in the Top 15.

---

## 6. Key Insights

### 6.1 Serialization Migration is the #1 Signal

Four of eight personas independently flagged serialization version migration as a gap. This is unusual because migration frameworks are boring infrastructure, yet they outranked flashy features like fluid networks and logic circuits. Every game that ships updates will hit this immediately.

### 6.2 Mobile Readiness is a Blind Spot

The mobile persona's entire wishlist was resource management: pause/resume with memory deallocation, memory budgets, battery-aware ticking, incremental saves, and feature flags for binary size. None add simulation capabilities, but all are hard prerequisites for iOS/Android. The engine currently has zero mobile-readiness infrastructure. If mobile is a target platform, this is a dedicated workstream.

### 6.3 The Enum Dispatch Tradeoff is Understood but Undocumented

The modding persona was the only one to request runtime processor/transport extension. The `Custom(Box<dyn ...>)` escape hatch already exists in the design. The gap is not architecture but documentation: no examples show how to use the Custom variant, what the performance tradeoff is, or how to maintain determinism through it.

### 6.4 Demand-Side Modeling Unlocks an Entire Genre

Every persona except the tycoon dev thinks about supply (production, throughput). Adding a `DemandProcessor` variant (configurable consumption rate, satisfaction tracking) and per-node cost accumulation to the stats module would make the engine viable for tycoon games (~30% coverage to ~50%) with minimal architectural change.

### 6.5 "What-If" Branching Serves Four Personas with One Primitive

The educational persona requested forking engine state for speculative exploration. This same primitive serves blueprint validation (studio), multiplayer rollback (multiplayer), and A/B testing of designs (tycoon). It is architecturally simple (clone via serialization) but no API exposes it.

### 6.6 Junctions are Design Doc Debt

Requested by only one persona, but inserters/splitters/mergers are specified in the architecture doc as core graph elements alongside nodes and edges. They are not implemented. This is an unfulfilled design promise, not a new feature request.

### 6.7 Profiling Consensus was Unexpectedly Strong

Three personas independently rated diagnostic instrumentation as critical or high priority. The engine's benchmarks show excellent raw performance; the gap is visibility into that performance. Per-phase timing in the 6-phase pipeline would be trivial to add and immediately useful.

---

## 7. Recommended Action

This document captures findings only. Implementation decisions, prioritization, and scope should be determined separately. The data suggests three natural tiers:

**Tier 1 — Multi-Persona Consensus (4+ or 3 personas):**
- Serialization version migration framework
- Spatial grid module
- Profiling / diagnostic instrumentation

**Tier 2 — Strong Signal (2 personas, at least one CRITICAL):**
- Module system (formal trait)
- Data-driven registry loading
- Multiplayer validation tools
- Incremental serialization
- Replay recording and playback
- Pause/resume with resource management
- Event filtering and priorities

**Tier 3 — Single-Persona but Architecturally Significant:**
- Junctions (design doc debt)
- Input command queue (multiplayer enabler)
- DemandProcessor (genre enabler)
- Modifier stacking rules
- Fluid networks module

**Documentation quick wins** (high impact, low effort):
- Runnable examples (`examples/minimal_factory.rs`)
- Custom variant usage guide
- Belt rendering interpolation pattern
- Serialization migration walkthrough
- Update design doc to reflect enum dispatch reality
