# Design Decisions & Trade-offs

Every non-trivial design choice in Factorial is a trade-off. This page documents the key decisions, explains what was gained and what was sacrificed, and provides enough context for contributors to understand why the engine is structured the way it is.

---

## Fixed-point over floats

**Trade-off: determinism over convenience.**

Factorial uses [fixed-point](../introduction/glossary.md#fixed-point) arithmetic (`Fixed64` = Q32.32, `Fixed32` = Q16.16) for all simulation math. IEEE 754 floating-point is not used in any simulation path.

**Why.** IEEE 754 floats produce different results across x86, ARM, and WASM due to differences in extended precision registers (x87 80-bit vs. SSE 64-bit), fused multiply-add availability, and compiler optimization choices. A factory that runs identically on one platform can diverge after a few thousand [ticks](../introduction/glossary.md#tick) on another. For lockstep multiplayer and deterministic replay, this is unacceptable.

Fixed-point operations are integer arithmetic with a fixed binary point. The results are identical on every platform, every compiler, and every optimization level. Two engines given the same inputs produce the same [state hash](../introduction/glossary.md#state-hash) bit-for-bit.

**What was sacrificed.** Fixed-point arithmetic is less convenient than `f64`. There is no hardware division instruction for Q32.32 on most platforms, so division is emulated in software. The `fixed` crate handles this transparently, but division-heavy code paths are measurably slower than their float equivalents. Additionally, the range and precision of Q32.32 are narrower than `f64` (roughly +/- 2 billion with 32 bits of fractional precision, versus +/- 10^308 for `f64`), which is more than sufficient for factory simulation but requires awareness at the API boundary.

Conversion functions (`f64_to_fixed64`, `fixed64_to_f64`) exist for initialization and display. They are explicitly documented as forbidden on the simulation hot path.

---

## Enum dispatch over trait objects

**Trade-off: performance over extensibility.**

[Processors](../introduction/glossary.md#processor) and [transport strategies](../introduction/glossary.md#transport-strategy) are represented as Rust enums, not trait objects:

```rust
pub enum Processor {
    Source(SourceProcessor),
    Fixed(FixedRecipe),
    Property(PropertyProcessor),
    Demand(DemandProcessor),
    Passthrough,
}

pub enum Transport {
    Flow(FlowTransport),
    Item(ItemTransport),
    Batch(BatchTransport),
    Vehicle(VehicleTransport),
}
```

**Why.** The set of processor and transport types is known at compile time. Enum dispatch gives the compiler full visibility into all variants, enabling:

- **Inlining**: the compiler can inline variant-specific logic directly into the match arm. Trait objects require virtual dispatch through a vtable pointer, which prevents inlining.
- **Sized storage**: enum variants are stored inline (no heap allocation for the trait object itself). This is critical for the SoA layout where `SecondaryMap<NodeId, Processor>` stores processors contiguously.
- **Branch prediction**: in factories with homogeneous regions (common in practice), the branch predictor can learn the dominant variant and predict correctly at near-100% rates.
- **No `dyn` overhead**: trait objects carry two pointers (data + vtable). Enum dispatch carries only the data plus a discriminant byte.

**What was sacrificed.** Adding a new processor or transport type requires modifying the enum definition and every `match` site. This is a closed-world assumption. Third-party crates cannot add new processor types without forking the core. In practice, the five processor types and four transport types cover the design space discovered across 20+ factory games, so the closed set is a deliberate constraint, not an oversight.

---

## Queued mutations over immediate

**Trade-off: determinism over responsiveness.**

Graph changes (adding/removing [nodes](../introduction/glossary.md#node) and [edges](../introduction/glossary.md#edge)) are never applied immediately. They are queued into a `VecDeque` of pending mutations and applied atomically during the pre-tick phase of the next `step()`:

```rust
// Queue mutations (no immediate effect).
let pending_node = engine.graph.queue_add_node(building_type);
let pending_edge = engine.graph.queue_connect(from, to);

// Mutations take effect here, at the start of the next tick.
engine.step();
```

Reactive event handlers follow the same pattern. Mutations returned by reactive handlers during post-tick are collected and applied during the next tick's pre-tick phase.

**Why.** Immediate mutations during a tick would create order-dependent behavior. If node A is evaluated before node B, and A's handler removes node B mid-tick, the result depends on evaluation order. With queued mutations, the graph is immutable during phases 2-5. Every [processor](../introduction/glossary.md#processor) and transport evaluates against the same snapshot of the graph. The order in which handlers enqueue mutations does not affect the final result -- only the set of mutations matters.

This also simplifies the engine's internal iteration. Phases 2 (Transport) and 3 (Process) can iterate over `SecondaryMap` values without worrying about concurrent modification or invalidated indices.

**What was sacrificed.** There is a one-tick delay between requesting a mutation and seeing it take effect. In a game running at 60 [UPS](../introduction/glossary.md#ups), this is ~16.7 ms of latency -- imperceptible to players in practice, but something integrators need to account for when writing game logic that depends on graph structure changes.

---

## Pull events over push for FFI

**Trade-off: safety over simplicity.**

The C FFI layer uses a pull-based (poll) model for events rather than push-based callbacks:

```c
// After stepping, poll for events.
FfiEventBuffer buf;
factorial_poll_events(engine, &buf);
for (size_t i = 0; i < buf.count; i++) {
    handle_event(&buf.events[i]);
}
```

Internally, the FFI layer registers passive listeners on all event kinds. These listeners capture events into a thread-local cache during `step()`. The caller then reads from this cache via `factorial_poll_events`.

**Why.** Callbacks crossing the FFI boundary are unsafe and difficult to reason about. A push-based model would require the C caller to register a function pointer that Rust invokes during simulation. This creates several problems:

- **Lifetime hazards**: the callback may reference C-side data that has been freed.
- **Reentrancy**: the callback may call back into the engine (e.g., queuing a mutation), which is a potential source of undefined behavior if the engine holds mutable borrows during event delivery.
- **Thread safety**: if the callback touches shared state without synchronization, data races are possible.
- **Unwind safety**: if the callback panics (possible in C++ or via `longjmp`), it would unwind through Rust frames, which is undefined behavior.

The pull model avoids all of these issues. All Rust code runs on the Rust side. The only data crossing the FFI boundary is a pointer to a flat array of `FfiEvent` structs owned by the engine. The caller reads from it; the engine overwrites it on the next step.

**What was sacrificed.** The caller must remember to poll after each step. Events are only available until the next `step()` call, at which point the buffer is overwritten. This is a less ergonomic API than push-based callbacks, but the safety guarantees are worth the trade-off for a cross-language boundary.

---

## Single-threaded by default

**Trade-off: determinism over parallelism.**

The default simulation is single-threaded. All six phases of the pipeline execute sequentially on the calling thread. There is no internal thread pool, no work stealing, and no lock contention.

**Why.** Single-threaded execution guarantees deterministic evaluation order. When the engine processes nodes in topological order, every node sees the exact same state of its upstream neighbors on every run, on every platform. This is required for:

- **Lockstep multiplayer**: two players running the same inputs must produce identical state hashes. Non-deterministic thread scheduling would break this invariant.
- **Deterministic replay**: recorded inputs must reproduce identical simulation states when replayed. Thread-sensitive ordering would make replays unreliable.
- **Debugging**: simulation bugs are reproducible when evaluation order is deterministic. Race conditions in parallel evaluation are notoriously difficult to diagnose.

**What was sacrificed.** A single thread cannot saturate multi-core hardware. For very large factories (tens of thousands of nodes), parallel evaluation could reduce tick time by exploiting data parallelism in the transport and process phases. The engine's SoA layout and immutable graph (between mutations) are designed to be parallelism-friendly, so adding opt-in parallelism behind a feature flag for non-lockstep scenarios is a planned extension. The important point is that determinism is the default, and parallelism is opt-in for use cases where exact reproducibility is not required.

---

## Summary

| Decision | Gained | Sacrificed |
|----------|--------|------------|
| Fixed-point | Cross-platform determinism | Float convenience, division speed |
| Enum dispatch | Inlining, sized storage, branch prediction | Open-world extensibility |
| Queued mutations | Deterministic evaluation, safe iteration | One-tick mutation delay |
| Pull events (FFI) | Memory safety, no UB | Polling ergonomics |
| Single-threaded default | Deterministic ordering, reproducibility | Multi-core utilization |

Each decision prioritizes correctness and determinism. Performance is achieved through data layout and allocation avoidance (see [Memory Layout](memory.md) and [Performance Model](performance.md)), not through sacrificing the invariants that factory games depend on.

---

## Next steps

- [Performance Model](performance.md) -- how the engine meets its tick budget.
- [Memory Layout](memory.md) -- SoA, arenas, and ring buffers in detail.
