# Statistics

The `factorial-stats` crate tracks per-node, per-edge, and per-item-type
throughput over configurable time windows. It listens to core
[events](../core-concepts/events.md) (`ItemProduced`, `ItemConsumed`,
`BuildingStalled`, `BuildingResumed`, `ItemDelivered`, `TransportFull`) and
aggregates them into rolling metrics using
[`Fixed64`](../introduction/glossary.md#fixed64) arithmetic for determinism.

## Key concepts

- **Rolling windows** compute averages over a configurable number of
  [ticks](../introduction/glossary.md#tick), not all-time totals.
- Three node states -- idle, working, stalled -- are tracked each tick to
  produce uptime and efficiency ratios.
- A **RingBuffer** stores historical rate snapshots for trend analysis and
  graphing.
- The module does not generate its own events; it passively consumes core
  events.

## Creating the stats tracker

```rust
use factorial_stats::{ProductionStats, StatsConfig};

let stats = ProductionStats::new(StatsConfig {
    window_size: 60,         // rolling average over 60 ticks
    history_capacity: 256,   // retain up to 256 historical snapshots
});
```

`StatsConfig::default()` uses `window_size: 60` and `history_capacity: 256`.

## Integration: process_event / end_tick

The stats module follows a two-phase per-tick lifecycle:

1. **During the tick** -- feed every relevant event via `process_event`:

   ```rust
   for event in &tick_events {
       stats.process_event(event);
   }
   ```

2. **At end of tick** -- finalize counters and advance rolling windows:

   ```rust
   stats.end_tick(current_tick);
   ```

`end_tick` records node states (idle / working / stalled), snapshots production
rates into history ring buffers, commits all rolling windows, and resets
per-tick accumulators.

### Tracked events

| Core event | What it records |
|-----------|----------------|
| `ItemProduced { node, item_type, quantity, .. }` | Per-node and global production count |
| `ItemConsumed { node, item_type, quantity, .. }` | Per-node and global consumption count |
| `BuildingStalled { node, .. }` | Marks the node as stalled for the current tick |
| `BuildingResumed { node, .. }` | Marks the node as working for the current tick |
| `ItemDelivered { edge, quantity, .. }` | Per-edge throughput count |
| `TransportFull { edge, .. }` | Marks the edge as full for the current tick |

All other event types are silently ignored.

## Per-node queries

### Production and consumption rates

```rust
let rate: Fixed64 = stats.get_production_rate(node, item_type);
let rate: Fixed64 = stats.get_consumption_rate(node, item_type);
```

Returns the rolling average (items per tick) over the configured window. If the
window is partially filled (e.g. only 5 ticks into a 60-tick window), the
average is computed over the filled portion, not the full window size.

### Idle, stall, and uptime ratios

```rust
let idle:   Fixed64 = stats.get_idle_ratio(node);   // 0.0 to 1.0
let stall:  Fixed64 = stats.get_stall_ratio(node);  // 0.0 to 1.0
let uptime: Fixed64 = stats.get_uptime(node);       // 0.0 to 1.0
```

These three ratios always sum to 1.0 over the tracked window:

| Ratio | Meaning |
|-------|---------|
| `idle_ratio` | Fraction of ticks the node had no activity (no production, no stall event) |
| `stall_ratio` | Fraction of ticks the node was stalled (missing inputs, output full, no power) |
| `uptime` | Fraction of ticks the node was actively working (produced or consumed items, or received a `BuildingResumed` event) |

A smelter that produces iron every tick has an uptime near 1.0. A smelter
waiting for ore has a high stall ratio. A smelter that was never given a recipe
has a high idle ratio.

## Per-edge queries

### Throughput

```rust
let throughput: Fixed64 = stats.get_throughput(edge);
```

Rolling average of items delivered per tick over the window.

### Utilization

```rust
let util: Fixed64 = stats.get_utilization(edge);
```

Fraction of ticks the edge was at full capacity (received a `TransportFull`
event). A utilization of 1.0 means the edge was saturated every tick in the
window -- a signal that throughput may need to be increased.

## Global queries

```rust
let total_prod: Fixed64 = stats.get_total_production(item_type);
let total_cons: Fixed64 = stats.get_total_consumption(item_type);
```

Summed across all nodes for a given item type. Useful for factory-wide
dashboards.

## RingBuffer history

Historical rate snapshots are stored in a `RingBuffer` with capacity set by
`StatsConfig::history_capacity`. Each tick, the current production rate for
every tracked (node, item_type) pair is pushed into the buffer. When the buffer
is full, the oldest entry is overwritten.

```rust
let history: Vec<Fixed64> = stats.get_history(node, item_type);
// oldest to newest; length <= history_capacity
```

For edges:

```rust
let edge_history: Vec<Fixed64> = stats.get_edge_history(edge);
```

### RingBuffer API

`RingBuffer` is also available as a public type for custom use:

```rust
use factorial_stats::RingBuffer;

let mut buf = RingBuffer::new(64);
buf.push(Fixed64::from_num(42));

buf.latest();       // Some(42)
buf.len();          // 1
buf.capacity();     // 64
buf.is_empty();     // false

// Iterate oldest to newest
for value in buf.iter() {
    println!("{value}");
}

buf.to_vec();       // Vec<Fixed64>, oldest to newest
buf.clear();        // reset without changing capacity
```

`RingBufferIter` implements `ExactSizeIterator`.

## Cleanup and lifecycle

```rust
stats.remove_node(node);    // drop all stats for a destroyed node
stats.remove_edge(edge);    // drop all stats for a destroyed edge
stats.clear();              // reset everything to a fresh state

stats.tracked_node_count();      // number of nodes being tracked
stats.tracked_edge_count();      // number of edges being tracked
stats.tracked_item_type_count(); // number of global item types
stats.current_tick();            // tick set by last end_tick call
```

## Practical usage pattern

A typical game loop integrates statistics as follows:

```rust
use factorial_stats::{ProductionStats, StatsConfig};

let mut stats = ProductionStats::new(StatsConfig::default());

// Each tick:
for tick in 1..=600 {
    // 1. Run simulation, collect events
    let events = engine.tick();

    // 2. Feed events to stats
    for event in &events {
        stats.process_event(event);
    }

    // 3. Finalize the tick
    stats.end_tick(tick);

    // 4. Query as needed (e.g. every 60 ticks for UI refresh)
    if tick % 60 == 0 {
        let iron_rate = stats.get_production_rate(smelter_node, iron_type);
        let smelter_uptime = stats.get_uptime(smelter_node);
        let belt_throughput = stats.get_throughput(belt_edge);
        // update UI ...
    }
}
```

Rates are accurate at any point during the tick because the rolling window
includes both committed (past ticks) and in-progress (current tick) data.
