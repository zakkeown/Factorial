# Performance Model

Factorial targets **60 [UPS](../introduction/glossary.md#ups)** with a per-[tick](../introduction/glossary.md#tick) budget of **less than 8 ms** for factories containing 5,000 or more [nodes](../introduction/glossary.md#node). This page explains how the engine spends that budget, how to measure it, and the techniques used to keep allocations out of the hot path.

---

## Tick budget breakdown

Each call to `Engine::step()` executes the six-phase simulation pipeline. The phases run sequentially, and the total wall time must stay under the 8 ms ceiling:

| Phase | Name | What happens |
|-------|------|--------------|
| 1 | **Pre-tick** | Apply queued graph mutations (adds, removes, connects, disconnects) and reactive handler mutations from the previous tick. Includes topological re-sort when the graph changes. |
| 2 | **Transport** | Move items along all [edges](../introduction/glossary.md#edge). Belts advance slots, flow transports transfer fractional amounts, batches and vehicles tick their state machines. |
| 3 | **Process** | Each [processor](../introduction/glossary.md#processor) consumes inputs, runs its recipe/source/demand logic, and writes outputs. Evaluated in topological order. |
| 4 | **Component** | Module-registered systems run (power balance, fluid flow, statistics accumulation, etc.). |
| 5 | **Post-tick** | Deliver buffered events to subscribers. Reactive handlers return mutations to enqueue for the next tick. |
| 6 | **Bookkeeping** | Increment the tick counter and compute the deterministic [state hash](../introduction/glossary.md#state-hash). |

In a typical 5,000-node factory, phases 2 (Transport) and 3 (Process) dominate the tick. Phase 1 is near-zero when no mutations are pending. Phase 6 is a linear scan but touches only the hash accumulator.

---

## Profiling feature flag

Factorial includes built-in per-phase timing instrumentation behind a Cargo feature flag. Enable it in your `Cargo.toml`:

```toml
[dependencies]
factorial-core = { version = "0.1", features = ["profiling"] }
```

When the `profiling` feature is active, the engine records wall-clock `Duration` values for each of the six phases on every tick. Access the data after stepping:

```rust
engine.step();

if let Some(profile) = engine.last_tick_profile() {
    println!("Tick {} total: {:?}", profile.tick, profile.total);
    println!("  pre_tick:     {:?}", profile.pre_tick);
    println!("  transport:    {:?}", profile.transport);
    println!("  process:      {:?}", profile.process);
    println!("  component:    {:?}", profile.component);
    println!("  post_tick:    {:?}", profile.post_tick);
    println!("  bookkeeping:  {:?}", profile.bookkeeping);

    let (name, dur) = profile.bottleneck_phase();
    println!("  bottleneck:   {} ({:?})", name, dur);
}
```

`last_tick_profile()` returns `Option<&TickProfile>`. It is `None` before the first `step()` call and `Some` afterward. The profile is overwritten on every step, so read it before calling `step()` again if you need to retain it.

The `TickProfile` struct exposes a `bottleneck_phase()` method that returns the name and duration of the slowest phase -- useful for quickly identifying where time is spent.

> **Note:** The `profiling` feature adds `std::time::Instant::now()` calls around each phase. This has negligible overhead on x86/ARM but may affect WASM targets where `Instant` resolution varies. Do not ship with `profiling` enabled in production builds.

---

## Benchmark suites

The project includes two Criterion benchmark suites for regression testing and performance characterization.

### `sim_bench.rs` (`factorial-core`)

Located at `crates/factorial-core/benches/sim_bench.rs`. Four benchmark groups:

| Group | Configuration | Target |
|-------|--------------|--------|
| `small_factory` | 200 nodes, ~500 edges, all FlowTransport | < 2 ms/tick |
| `medium_factory` | 5,000 nodes, ~10,000 edges, mixed transport | < 5 ms/tick |
| `belt_heavy` | 1,000 ItemTransport belts, 50 slots each | Measure belt throughput |
| `serialization` | 5,000 nodes, full/partitioned/incremental serialize + deserialize | Track serialization cost |

Run the simulation benchmarks:

```bash
cargo bench -p factorial-core --bench sim_bench
```

### `blueprint_bench.rs` (`factorial-spatial`)

Located at `crates/factorial-spatial/benches/blueprint_bench.rs`. Two benchmark groups:

| Group | Configuration |
|-------|--------------|
| `add_100_entries` | Insert 100 blueprint entries in a 10x10 grid |
| `commit_50_nodes_49_connections` | Commit a 50-node blueprint with 49 connections to the engine |

Run the blueprint benchmarks:

```bash
cargo bench -p factorial-spatial --bench blueprint_bench
```

---

## Avoiding allocations during simulation

Allocations during the hot loop are the primary enemy of consistent tick times. Factorial uses several techniques to eliminate them.

### Pre-allocated ring buffers for events

The [EventBus](../core-concepts/events.md) stores events in fixed-capacity ring buffers (`EventBuffer`), one per event kind. Buffers are allocated once (lazily on first emit or at construction) with a configurable capacity (default: 1,024 events per kind). When a buffer is full, the oldest events are silently overwritten -- no reallocation occurs. Suppressed event kinds (`EventBus::suppress`) have zero allocation cost; their buffers are never created.

### Struct-of-Arrays with SlotMap

Per-node and per-edge state is stored in `SecondaryMap` collections (from the `slotmap` crate) keyed by `NodeId` or `EdgeId`. These maps use flat, contiguous backing storage with generational indices. Inserting a new node or edge writes into an existing slot or appends to the end of the backing array -- no pointer chasing, no allocator pressure during steady-state simulation. See the [Memory Layout](memory.md) page for details.

### Belt slots as flat arrays

`BeltState` for `ItemTransport` edges stores slots as a `Vec<Option<ItemTypeId>>` pre-allocated at creation time to `lanes * slot_count` entries. During the transport phase, belt advancement is a sequential scan and shift over this flat array. No per-tick allocation.

### No heap allocation in transport or processor logic

The `Transport::advance()` and `Processor` evaluation paths operate entirely on mutable references to pre-existing state. They return small, stack-allocated result structs (`TransportResult`, `ProcessorResult`). No `Vec`, `Box`, or `String` is created on the hot path.

### Queued mutations avoid mid-tick reallocation

Graph mutations (add node, remove node, connect, disconnect) are queued into a `VecDeque` and applied atomically during the pre-tick phase. This means the topological sort and adjacency lists are rebuilt at most once per tick, not once per mutation. Between ticks, the graph structure is immutable, so transport and processor phases can iterate without worrying about invalidated indices.

---

## Performance guidelines for integrators

1. **Suppress events you do not need.** Call `engine.event_bus.suppress(EventKind::ItemProduced)` for any event kind your game does not consume. Suppressed events skip buffering entirely.

2. **Use `SimulationStrategy::Tick`** for lockstep multiplayer. It is simpler and avoids accumulator drift.

3. **Batch graph mutations.** Queue multiple `add_node`/`connect` calls before calling `step()`. They are applied together in one pre-tick phase, incurring only one topological re-sort.

4. **Profile before optimizing.** Enable the `profiling` feature, identify the bottleneck phase, then focus effort there.

5. **Run benchmarks in CI.** Use the Criterion suites to catch performance regressions early.

---

## Next steps

- [Memory Layout](memory.md) -- how data is arranged for cache-friendly access.
- [Design Decisions & Trade-offs](decisions.md) -- why the engine is structured this way.
