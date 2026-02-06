# What is Factorial?

Factorial is a **headless factory-game engine** written in Rust.
It provides the simulation layer that every factory game needs --
[production graphs](glossary.md#production-graph),
[processors](glossary.md#processor),
[transport](glossary.md#transport),
power networks, fluid networks, tech trees, spatial placement,
[serialization](glossary.md#serialization), and
[determinism](glossary.md#determinism) --
so that game developers can focus on what makes their game unique:
UI, rendering, audio, and game-specific mechanics.

You integrate Factorial as a Rust crate dependency, or embed it via C FFI
or WASM. It runs the math; you draw the pixels and play the sounds.

For a detailed breakdown of what Factorial owns versus what you own,
see [What You Build vs. What We Handle](responsibilities.md).

---

## Architecture overview

Factorial is organized into a required core layer and a set of opt-in
framework modules. Your game code sits on top.

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

**Factorial Core** (`factorial-core`) contains the
[production graph](glossary.md#production-graph), the six-phase
simulation pipeline, [transport](glossary.md#transport) strategies,
the [event bus](glossary.md#event), query snapshots,
[serialization](glossary.md#serialization), and
[deterministic](glossary.md#determinism) fixed-point arithmetic.
Every Factorial game depends on this crate.

**Framework Modules** are optional crates that extend the core with
common factory-game subsystems:

| Crate                  | Provides                                    |
|------------------------|---------------------------------------------|
| `factorial-power`      | Power networks, generation, brownouts       |
| `factorial-fluid`      | Pipe-based fluid simulation                 |
| `factorial-tech-tree`  | Research prerequisites and unlocks          |
| `factorial-spatial`    | Grid placement, collision, blueprints       |
| `factorial-stats`      | Production/consumption rate tracking        |

You enable only the modules your game needs.

---

## Integration paths

| Path             | When to use                                         |
|------------------|-----------------------------------------------------|
| **Rust crate**   | Native Rust game or Bevy/macroquad project          |
| **C FFI**        | C, C++, C#, or any language with C interop          |
| **WASM**         | Browser-based game or sandboxed plugin environment  |

See the quick-start guides:
[Rust](../getting-started/rust.md) |
[C/FFI](../getting-started/ffi.md) |
[WASM](../getting-started/wasm.md)

---

## Quick taste

Three lines to create an [engine](glossary.md#engine), configure it,
and advance the simulation by one [tick](glossary.md#tick):

```rust
let mut engine = Engine::new(SimulationStrategy::Tick);
// ... configure nodes, processors, transport ...
engine.step(); // one simulation tick
```

The full version of this example -- an iron mine feeding an assembler
over a belt -- is in the [Rust Quick Start](../getting-started/rust.md).

---

## Research-informed design

Factorial's API and subsystem boundaries are drawn from a study of 20+
shipped factory games, including Factorio, Satisfactory, Dyson Sphere
Program, Shapez, Oxygen Not Included, Mindustry, Captain of Industry,
and others. Patterns that recur across these titles -- rate-based
production, belt/pipe/logistic transport, tiered research gating,
grid-snapped placement -- live in Factorial so that you do not have to
reinvent them.

---

## Next steps

- [What You Build vs. What We Handle](responsibilities.md) -- the
  responsibility split in detail.
- [Glossary](glossary.md) -- definitions of every term used in this
  documentation.
- [Rust Quick Start](../getting-started/rust.md) -- build a working
  factory in under five minutes.
