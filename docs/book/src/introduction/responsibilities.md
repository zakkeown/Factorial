# What You Build vs. What We Handle

Factorial is a simulation engine, not a game engine. It solves the hard
numerical and logical problems behind factory games --
[production graphs](glossary.md#production-graph), throughput math, resource
balancing -- and leaves everything else (rendering, input, audio, UI) to you
and your engine of choice.

This page maps out exactly where Factorial's responsibilities end and yours
begin.

## Responsibility Matrix

| Concern | Factorial Handles | You Build |
|---------|-------------------|-----------|
| Production math | [Recipe](glossary.md#recipe) rates, [throughput](glossary.md#throughput), [modifiers](glossary.md#modifier) ([Processors](../core-concepts/processors.md)) | Recipe definitions (data) |
| Item transport | [Belt](glossary.md#belt)/pipe/train/vehicle simulation ([Transport Strategies](../core-concepts/transport.md)) | Rendering belts, animations |
| Power grid | Supply/demand/satisfaction calculation ([Power Networks](../modules/power.md)) | UI for power overlay |
| Tech tree | Unlock logic, dependency resolution ([Tech Trees](../modules/tech-tree.md)) | Research UI, cost balancing |
| Save/load | [Engine](glossary.md#engine) state [serialization](glossary.md#serialization) ([Serialization](../core-concepts/serialization.md)) | Your game-specific state |
| Multiplayer sync | [Deterministic](glossary.md#determinism) simulation, state hashing ([Determinism & Fixed-Point](../core-concepts/determinism.md)) | Netcode, lobby, input relay |
| Spatial placement | Collision, grid snaps, [blueprints](glossary.md#blueprint) ([Spatial Grid & Blueprints](../modules/spatial.md)) | Visual placement preview |
| Game loop | `engine.step()` / `engine.advance(dt)` ([Production Graph](../core-concepts/production-graph.md)) | When/how often to call it |

## Coverage by Game Archetype

Different factory-game subgenres overlap with Factorial's feature set to
different degrees. The percentages below are rough estimates of how much
back-end logic Factorial can replace.

### Pure Factory (Factorio-like) -- 70-80% coverage

Factorial covers production, logistics, power, research, and spatial placement.
You build: world generation, rendering, combat/enemies, and UX (inventory
screens, map view, alerts).

### Automation Puzzler (Shapez-like) -- 60-70% coverage

The [production graph](glossary.md#production-graph) and
[transport](glossary.md#transport) layers map well to shape-processing
pipelines. You build: shape logic and combination rules, level progression,
scoring, and the puzzle editor.

### Colony Sim Hybrid (ONI-like) -- 25-35% coverage

Factorial handles the production, [power](glossary.md#power-network), and
[fluid](glossary.md#fluid-network) subsystems. You build: colonist AI,
pathfinding, environment simulation (gas diffusion, temperature), mood and
needs systems, and job scheduling.

## Opt-In by Design

Every Factorial subsystem is optional. At a minimum, you can depend on just the
core [production graph](../core-concepts/production-graph.md) to evaluate
recipes and compute throughput. From there, layer on
[framework modules](../modules/power.md) -- power, fluids, tech trees, spatial
grids -- only when your game needs them. There is no all-or-nothing framework
cost: import what you use, ignore what you do not.
