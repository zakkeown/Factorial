//! Production statistics module for the Factorial engine.
//!
//! Tracks per-node, per-edge, and per-item-type throughput over configurable
//! time windows. Listens to core events (`ItemProduced`, `ItemConsumed`,
//! `BuildingStalled`, `BuildingResumed`, `ItemDelivered`, `TransportFull`)
//! and aggregates them into rolling metrics using [`Fixed64`] arithmetic.
//!
//! # Usage
//!
//! ```ignore
//! let mut stats = ProductionStats::new(StatsConfig::default());
//! // Feed events each tick:
//! stats.process_event(&event);
//! // Advance the tick counter:
//! stats.end_tick(current_tick);
//! // Query metrics:
//! let rate = stats.get_production_rate(node, item_type);
//! ```

use std::collections::HashMap;

use factorial_core::event::Event;
use factorial_core::fixed::{Fixed64, Ticks};
use factorial_core::id::{EdgeId, ItemTypeId, NodeId};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the statistics module.
#[derive(Debug, Clone)]
pub struct StatsConfig {
    /// Window size in ticks for rolling averages (e.g., 60 ticks).
    pub window_size: Ticks,
    /// Maximum number of historical snapshots to retain per metric.
    pub history_capacity: usize,
}

impl Default for StatsConfig {
    fn default() -> Self {
        Self {
            window_size: 60,
            history_capacity: 256,
        }
    }
}

// ---------------------------------------------------------------------------
// RingBuffer — generic ring buffer for historical data
// ---------------------------------------------------------------------------

/// A fixed-capacity ring buffer storing [`Fixed64`] values for trend analysis.
///
/// When full, the oldest entry is overwritten. Iterates oldest-to-newest.
#[derive(Debug, Clone)]
pub struct RingBuffer {
    data: Vec<Fixed64>,
    head: usize,
    len: usize,
}

impl RingBuffer {
    /// Create a new ring buffer with the given capacity.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "RingBuffer capacity must be > 0");
        Self {
            data: vec![Fixed64::ZERO; capacity],
            head: 0,
            len: 0,
        }
    }

    /// Push a value, overwriting the oldest entry if at capacity.
    pub fn push(&mut self, value: Fixed64) {
        self.data[self.head] = value;
        self.head = (self.head + 1) % self.capacity();
        if self.len < self.capacity() {
            self.len += 1;
        }
    }

    /// Number of values currently stored.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Total capacity.
    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    /// Get the most recently pushed value, if any.
    pub fn latest(&self) -> Option<Fixed64> {
        if self.len == 0 {
            return None;
        }
        let idx = if self.head == 0 {
            self.capacity() - 1
        } else {
            self.head - 1
        };
        Some(self.data[idx])
    }

    /// Iterate values from oldest to newest.
    pub fn iter(&self) -> RingBufferIter<'_> {
        let start = if self.len < self.capacity() {
            0
        } else {
            self.head
        };
        RingBufferIter {
            buffer: self,
            index: start,
            remaining: self.len,
        }
    }

    /// Collect all stored values into a Vec (oldest to newest).
    pub fn to_vec(&self) -> Vec<Fixed64> {
        self.iter().collect()
    }

    /// Clear all stored values without changing capacity.
    pub fn clear(&mut self) {
        for slot in &mut self.data {
            *slot = Fixed64::ZERO;
        }
        self.head = 0;
        self.len = 0;
    }
}

/// Iterator over [`RingBuffer`] values, oldest to newest.
pub struct RingBufferIter<'a> {
    buffer: &'a RingBuffer,
    index: usize,
    remaining: usize,
}

impl<'a> Iterator for RingBufferIter<'a> {
    type Item = Fixed64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let value = self.buffer.data[self.index];
        self.index = (self.index + 1) % self.buffer.capacity();
        self.remaining -= 1;
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for RingBufferIter<'_> {}

// ---------------------------------------------------------------------------
// Rolling window counter
// ---------------------------------------------------------------------------

/// A rolling window counter that tracks a count over the most recent N ticks.
///
/// Stores per-tick counts in a ring buffer. The `total` field is the sum of
/// all committed ticks in the window. The `current` field accumulates the
/// count for the in-progress tick (not yet committed to the ring buffer).
///
/// # Tick lifecycle
///
/// 1. Call [`add`](Self::add) zero or more times during the tick.
/// 2. Call [`commit`](Self::commit) exactly once at end-of-tick to write the
///    current tick into the ring buffer and prepare for the next tick.
///
/// [`rate`](Self::rate) and [`total`](Self::total) include the committed ticks
/// **plus** any in-progress current tick data, so queries are accurate at any
/// point during the tick.
#[derive(Debug, Clone)]
struct RollingWindow {
    /// Committed per-tick counts in a ring buffer.
    tick_counts: Vec<u64>,
    /// Write position for the next commit.
    write_pos: usize,
    /// Running total of committed tick counts in the window.
    committed_total: u64,
    /// Accumulator for the current (uncommitted) tick.
    current: u64,
    /// Window size (capacity of tick_counts).
    window_size: usize,
    /// Number of committed ticks stored (capped at window_size).
    committed_count: usize,
}

impl RollingWindow {
    fn new(window_size: usize) -> Self {
        assert!(window_size > 0, "RollingWindow size must be > 0");
        Self {
            tick_counts: vec![0; window_size],
            write_pos: 0,
            committed_total: 0,
            current: 0,
            window_size,
            committed_count: 0,
        }
    }

    /// Accumulate a count for the current (in-progress) tick.
    fn add(&mut self, count: u64) {
        self.current += count;
    }

    /// Commit the current tick into the ring buffer and prepare for the next.
    ///
    /// If the ring buffer is full, the oldest tick is evicted.
    fn commit(&mut self) {
        // Evict the oldest entry if at capacity.
        if self.committed_count == self.window_size {
            self.committed_total -= self.tick_counts[self.write_pos];
        }

        // Write current tick's count into the ring buffer.
        self.tick_counts[self.write_pos] = self.current;
        self.committed_total += self.current;
        self.current = 0;

        // Advance write position.
        self.write_pos = (self.write_pos + 1) % self.window_size;

        if self.committed_count < self.window_size {
            self.committed_count += 1;
        }
    }

    /// Running total over the window (committed ticks + current in-progress tick).
    fn total(&self) -> u64 {
        self.committed_total + self.current
    }

    /// Compute the rolling average as items per tick (Fixed64).
    ///
    /// Includes both committed ticks and the current in-progress tick.
    /// Divides by the number of contributing ticks.
    fn rate(&self) -> Fixed64 {
        let effective_count = if self.current > 0 {
            self.committed_count + 1
        } else {
            self.committed_count
        };
        if effective_count == 0 {
            return Fixed64::ZERO;
        }
        let total = self.committed_total + self.current;
        Fixed64::from_num(total) / Fixed64::from_num(effective_count)
    }

}

// ---------------------------------------------------------------------------
// Per-node statistics
// ---------------------------------------------------------------------------

/// Per-node statistics tracking production, consumption, and state ratios.
#[derive(Debug, Clone)]
struct NodeStats {
    /// Rolling production counts keyed by item type.
    production: HashMap<ItemTypeId, RollingWindow>,
    /// Rolling consumption counts keyed by item type.
    consumption: HashMap<ItemTypeId, RollingWindow>,
    /// Number of ticks this node has been idle (within the window).
    idle_ticks: RollingWindow,
    /// Number of ticks this node has been stalled (within the window).
    stall_ticks: RollingWindow,
    /// Number of ticks this node has been working (within the window).
    working_ticks: RollingWindow,
    /// Historical production rate snapshots.
    production_history: HashMap<ItemTypeId, RingBuffer>,
    /// Current state for this tick (set by events, reset each tick).
    current_state: NodeState,
    /// Window size for creating new rolling windows.
    window_size: usize,
    /// History capacity for creating new ring buffers.
    history_capacity: usize,
}

/// Tracks the node's state within a single tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum NodeState {
    #[default]
    Idle,
    Working,
    Stalled,
}

impl NodeStats {
    fn new(window_size: usize, history_capacity: usize) -> Self {
        Self {
            production: HashMap::new(),
            consumption: HashMap::new(),
            idle_ticks: RollingWindow::new(window_size),
            stall_ticks: RollingWindow::new(window_size),
            working_ticks: RollingWindow::new(window_size),
            production_history: HashMap::new(),
            current_state: NodeState::default(),
            window_size,
            history_capacity,
        }
    }

    fn get_or_create_production(&mut self, item_type: ItemTypeId) -> &mut RollingWindow {
        self.production
            .entry(item_type)
            .or_insert_with(|| RollingWindow::new(self.window_size))
    }

    fn get_or_create_consumption(&mut self, item_type: ItemTypeId) -> &mut RollingWindow {
        self.consumption
            .entry(item_type)
            .or_insert_with(|| RollingWindow::new(self.window_size))
    }

    fn get_or_create_history(&mut self, item_type: ItemTypeId) -> &mut RingBuffer {
        let cap = self.history_capacity;
        self.production_history
            .entry(item_type)
            .or_insert_with(|| RingBuffer::new(cap))
    }

    fn record_produced(&mut self, item_type: ItemTypeId, quantity: u32) {
        self.get_or_create_production(item_type).add(quantity as u64);
        self.current_state = NodeState::Working;
    }

    fn record_consumed(&mut self, item_type: ItemTypeId, quantity: u32) {
        self.get_or_create_consumption(item_type).add(quantity as u64);
        self.current_state = NodeState::Working;
    }

    fn record_stalled(&mut self) {
        self.current_state = NodeState::Stalled;
    }

    fn record_resumed(&mut self) {
        self.current_state = NodeState::Working;
    }

    /// End-of-tick accounting: record state tick, advance windows, snapshot history.
    fn end_tick(&mut self) {
        // Record the node state for this tick.
        match self.current_state {
            NodeState::Idle => self.idle_ticks.add(1),
            NodeState::Working => self.working_ticks.add(1),
            NodeState::Stalled => self.stall_ticks.add(1),
        }

        // Snapshot current production rates into history.
        let item_types: Vec<ItemTypeId> = self.production.keys().copied().collect();
        for item_type in item_types {
            let rate = self.production[&item_type].rate();
            self.get_or_create_history(item_type).push(rate);
        }

        // Advance all rolling windows.
        for window in self.production.values_mut() {
            window.commit();
        }
        for window in self.consumption.values_mut() {
            window.commit();
        }
        self.idle_ticks.commit();
        self.stall_ticks.commit();
        self.working_ticks.commit();

        // Reset per-tick state to idle (will be set by events next tick).
        self.current_state = NodeState::Idle;
    }

    fn production_rate(&self, item_type: ItemTypeId) -> Fixed64 {
        self.production
            .get(&item_type)
            .map(|w| w.rate())
            .unwrap_or(Fixed64::ZERO)
    }

    fn consumption_rate(&self, item_type: ItemTypeId) -> Fixed64 {
        self.consumption
            .get(&item_type)
            .map(|w| w.rate())
            .unwrap_or(Fixed64::ZERO)
    }

    /// Idle ratio: idle_ticks / total_ticks (0.0 to 1.0).
    fn idle_ratio(&self) -> Fixed64 {
        let total = self.total_tracked_ticks();
        if total == 0 {
            return Fixed64::ZERO;
        }
        Fixed64::from_num(self.idle_ticks.total()) / Fixed64::from_num(total)
    }

    /// Stall ratio: stall_ticks / total_ticks (0.0 to 1.0).
    fn stall_ratio(&self) -> Fixed64 {
        let total = self.total_tracked_ticks();
        if total == 0 {
            return Fixed64::ZERO;
        }
        Fixed64::from_num(self.stall_ticks.total()) / Fixed64::from_num(total)
    }

    /// Uptime: working_ticks / total_ticks (0.0 to 1.0).
    fn uptime(&self) -> Fixed64 {
        let total = self.total_tracked_ticks();
        if total == 0 {
            return Fixed64::ZERO;
        }
        Fixed64::from_num(self.working_ticks.total()) / Fixed64::from_num(total)
    }

    fn total_tracked_ticks(&self) -> u64 {
        self.idle_ticks.total() + self.stall_ticks.total() + self.working_ticks.total()
    }
}

// ---------------------------------------------------------------------------
// Per-edge statistics
// ---------------------------------------------------------------------------

/// Per-edge statistics tracking throughput and utilization.
#[derive(Debug, Clone)]
struct EdgeStats {
    /// Rolling delivery count.
    throughput: RollingWindow,
    /// Number of ticks the edge was full (at capacity).
    full_ticks: RollingWindow,
    /// Total ticks tracked for this edge (within the window).
    total_ticks: RollingWindow,
    /// Historical throughput rate snapshots.
    throughput_history: RingBuffer,
    /// Whether this edge was full during the current tick.
    was_full_this_tick: bool,
}

impl EdgeStats {
    fn new(window_size: usize, history_capacity: usize) -> Self {
        Self {
            throughput: RollingWindow::new(window_size),
            full_ticks: RollingWindow::new(window_size),
            total_ticks: RollingWindow::new(window_size),
            throughput_history: RingBuffer::new(history_capacity),
            was_full_this_tick: false,
        }
    }

    fn record_delivered(&mut self, quantity: u32) {
        self.throughput.add(quantity as u64);
    }

    fn record_full(&mut self) {
        self.was_full_this_tick = true;
    }

    /// End-of-tick accounting.
    fn end_tick(&mut self) {
        // Record utilization for this tick.
        self.total_ticks.add(1);
        if self.was_full_this_tick {
            self.full_ticks.add(1);
        }

        // Snapshot throughput rate.
        self.throughput_history.push(self.throughput.rate());

        // Advance windows.
        self.throughput.commit();
        self.full_ticks.commit();
        self.total_ticks.commit();

        // Reset per-tick state.
        self.was_full_this_tick = false;
    }

    fn throughput_rate(&self) -> Fixed64 {
        self.throughput.rate()
    }

    /// Utilization: full_ticks / total_ticks (0.0 to 1.0).
    fn utilization(&self) -> Fixed64 {
        let total = self.total_ticks.total();
        if total == 0 {
            return Fixed64::ZERO;
        }
        Fixed64::from_num(self.full_ticks.total()) / Fixed64::from_num(total)
    }
}

// ---------------------------------------------------------------------------
// Global item statistics
// ---------------------------------------------------------------------------

/// Global per-item-type statistics (summed across all nodes).
#[derive(Debug, Clone)]
struct GlobalItemStats {
    production: RollingWindow,
    consumption: RollingWindow,
}

impl GlobalItemStats {
    fn new(window_size: usize) -> Self {
        Self {
            production: RollingWindow::new(window_size),
            consumption: RollingWindow::new(window_size),
        }
    }
}

// ---------------------------------------------------------------------------
// ProductionStats — main module struct
// ---------------------------------------------------------------------------

/// Main production statistics aggregator.
///
/// Accepts events via [`process_event`](ProductionStats::process_event), advances
/// time via [`end_tick`](ProductionStats::end_tick), and exposes per-node,
/// per-edge, and global metrics through getter methods.
///
/// All rate values use [`Fixed64`] arithmetic for determinism.
#[derive(Debug)]
pub struct ProductionStats {
    config: StatsConfig,
    nodes: HashMap<NodeId, NodeStats>,
    edges: HashMap<EdgeId, EdgeStats>,
    global: HashMap<ItemTypeId, GlobalItemStats>,
    /// Current tick (set by end_tick).
    current_tick: Ticks,
}

impl ProductionStats {
    /// Create a new production stats tracker with the given configuration.
    pub fn new(config: StatsConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            edges: HashMap::new(),
            global: HashMap::new(),
            current_tick: 0,
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &StatsConfig {
        &self.config
    }

    /// Get the current tick.
    pub fn current_tick(&self) -> Ticks {
        self.current_tick
    }

    // -- Event processing ---------------------------------------------------

    /// Process a single event, updating internal counters.
    ///
    /// Call this for each event in a tick, then call [`end_tick`](Self::end_tick)
    /// to finalize the tick and advance rolling windows.
    pub fn process_event(&mut self, event: &Event) {
        match event {
            Event::ItemProduced {
                node,
                item_type,
                quantity,
                ..
            } => {
                self.get_or_create_node(*node).record_produced(*item_type, *quantity);
                self.get_or_create_global(*item_type)
                    .production
                    .add(*quantity as u64);
            }

            Event::ItemConsumed {
                node,
                item_type,
                quantity,
                ..
            } => {
                self.get_or_create_node(*node).record_consumed(*item_type, *quantity);
                self.get_or_create_global(*item_type)
                    .consumption
                    .add(*quantity as u64);
            }

            Event::BuildingStalled { node, .. } => {
                self.get_or_create_node(*node).record_stalled();
            }

            Event::BuildingResumed { node, .. } => {
                self.get_or_create_node(*node).record_resumed();
            }

            Event::ItemDelivered {
                edge, quantity, ..
            } => {
                self.get_or_create_edge(*edge).record_delivered(*quantity);
            }

            Event::TransportFull { edge, .. } => {
                self.get_or_create_edge(*edge).record_full();
            }

            // Other events are not tracked by the stats module.
            _ => {}
        }
    }

    /// Finalize the current tick and advance all rolling windows.
    ///
    /// Must be called once per tick after all events have been processed.
    pub fn end_tick(&mut self, tick: Ticks) {
        self.current_tick = tick;

        for node in self.nodes.values_mut() {
            node.end_tick();
        }
        for edge in self.edges.values_mut() {
            edge.end_tick();
        }
        for global in self.global.values_mut() {
            global.production.commit();
            global.consumption.commit();
        }
    }

    // -- Per-node queries ---------------------------------------------------

    /// Get the production rate (items/tick) for a node and item type.
    ///
    /// Returns the rolling average over the configured window.
    pub fn get_production_rate(&self, node: NodeId, item_type: ItemTypeId) -> Fixed64 {
        self.nodes
            .get(&node)
            .map(|n| n.production_rate(item_type))
            .unwrap_or(Fixed64::ZERO)
    }

    /// Get the consumption rate (items/tick) for a node and item type.
    pub fn get_consumption_rate(&self, node: NodeId, item_type: ItemTypeId) -> Fixed64 {
        self.nodes
            .get(&node)
            .map(|n| n.consumption_rate(item_type))
            .unwrap_or(Fixed64::ZERO)
    }

    /// Get the idle ratio (0.0 to 1.0) for a node.
    pub fn get_idle_ratio(&self, node: NodeId) -> Fixed64 {
        self.nodes
            .get(&node)
            .map(|n| n.idle_ratio())
            .unwrap_or(Fixed64::ZERO)
    }

    /// Get the stall ratio (0.0 to 1.0) for a node.
    pub fn get_stall_ratio(&self, node: NodeId) -> Fixed64 {
        self.nodes
            .get(&node)
            .map(|n| n.stall_ratio())
            .unwrap_or(Fixed64::ZERO)
    }

    /// Get the uptime ratio (0.0 to 1.0) for a node.
    pub fn get_uptime(&self, node: NodeId) -> Fixed64 {
        self.nodes
            .get(&node)
            .map(|n| n.uptime())
            .unwrap_or(Fixed64::ZERO)
    }

    // -- Per-edge queries ---------------------------------------------------

    /// Get the throughput (items/tick) for an edge.
    pub fn get_throughput(&self, edge: EdgeId) -> Fixed64 {
        self.edges
            .get(&edge)
            .map(|e| e.throughput_rate())
            .unwrap_or(Fixed64::ZERO)
    }

    /// Get the utilization ratio (0.0 to 1.0) for an edge.
    ///
    /// Utilization represents the fraction of ticks the edge was at full
    /// capacity (received a `TransportFull` event).
    pub fn get_utilization(&self, edge: EdgeId) -> Fixed64 {
        self.edges
            .get(&edge)
            .map(|e| e.utilization())
            .unwrap_or(Fixed64::ZERO)
    }

    // -- Global queries -----------------------------------------------------

    /// Get the total production rate (items/tick) for an item type across all nodes.
    pub fn get_total_production(&self, item_type: ItemTypeId) -> Fixed64 {
        self.global
            .get(&item_type)
            .map(|g| g.production.rate())
            .unwrap_or(Fixed64::ZERO)
    }

    /// Get the total consumption rate (items/tick) for an item type across all nodes.
    pub fn get_total_consumption(&self, item_type: ItemTypeId) -> Fixed64 {
        self.global
            .get(&item_type)
            .map(|g| g.consumption.rate())
            .unwrap_or(Fixed64::ZERO)
    }

    // -- Historical data ----------------------------------------------------

    /// Get the historical production rate data for a node and item type.
    ///
    /// Returns a Vec of [`Fixed64`] values from oldest to newest, representing
    /// the production rate at each past tick.
    pub fn get_history(&self, node: NodeId, item_type: ItemTypeId) -> Vec<Fixed64> {
        self.nodes
            .get(&node)
            .and_then(|n| n.production_history.get(&item_type))
            .map(|h| h.to_vec())
            .unwrap_or_default()
    }

    /// Get the edge throughput history.
    pub fn get_edge_history(&self, edge: EdgeId) -> Vec<Fixed64> {
        self.edges
            .get(&edge)
            .map(|e| e.throughput_history.to_vec())
            .unwrap_or_default()
    }

    // -- Utility ------------------------------------------------------------

    /// Remove all statistics for a node (e.g., when the node is destroyed).
    pub fn remove_node(&mut self, node: NodeId) {
        self.nodes.remove(&node);
    }

    /// Remove all statistics for an edge.
    pub fn remove_edge(&mut self, edge: EdgeId) {
        self.edges.remove(&edge);
    }

    /// Clear all statistics, resetting to a fresh state.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.global.clear();
        self.current_tick = 0;
    }

    /// Number of tracked nodes.
    pub fn tracked_node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of tracked edges.
    pub fn tracked_edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Number of tracked item types (global).
    pub fn tracked_item_type_count(&self) -> usize {
        self.global.len()
    }

    // -- Internal helpers ---------------------------------------------------

    fn get_or_create_node(&mut self, node: NodeId) -> &mut NodeStats {
        let ws = self.config.window_size as usize;
        let hc = self.config.history_capacity;
        self.nodes
            .entry(node)
            .or_insert_with(|| NodeStats::new(ws, hc))
    }

    fn get_or_create_edge(&mut self, edge: EdgeId) -> &mut EdgeStats {
        let ws = self.config.window_size as usize;
        let hc = self.config.history_capacity;
        self.edges
            .entry(edge)
            .or_insert_with(|| EdgeStats::new(ws, hc))
    }

    fn get_or_create_global(&mut self, item_type: ItemTypeId) -> &mut GlobalItemStats {
        let ws = self.config.window_size as usize;
        self.global
            .entry(item_type)
            .or_insert_with(|| GlobalItemStats::new(ws))
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use factorial_core::fixed::f64_to_fixed64;
    use factorial_core::processor::StallReason;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_node_id() -> NodeId {
        use slotmap::SlotMap;
        let mut sm = SlotMap::<NodeId, ()>::with_key();
        sm.insert(())
    }

    fn make_edge_id() -> EdgeId {
        use slotmap::SlotMap;
        let mut sm = SlotMap::<EdgeId, ()>::with_key();
        sm.insert(())
    }

    fn iron() -> ItemTypeId {
        ItemTypeId(0)
    }

    fn copper() -> ItemTypeId {
        ItemTypeId(1)
    }

    fn small_config() -> StatsConfig {
        StatsConfig {
            window_size: 10,
            history_capacity: 16,
        }
    }

    /// Helper to assert that two Fixed64 values are approximately equal.
    fn assert_fixed_approx(actual: Fixed64, expected: f64, tolerance: f64) {
        let actual_f64: f64 = actual.to_num();
        assert!(
            (actual_f64 - expected).abs() < tolerance,
            "expected ~{expected}, got {actual_f64}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 1: RingBuffer basic push and iterate
    // -----------------------------------------------------------------------
    #[test]
    fn ring_buffer_push_and_iterate() {
        let mut buf = RingBuffer::new(4);
        buf.push(f64_to_fixed64(1.0));
        buf.push(f64_to_fixed64(2.0));
        buf.push(f64_to_fixed64(3.0));

        assert_eq!(buf.len(), 3);
        assert!(!buf.is_empty());

        let values: Vec<Fixed64> = buf.iter().collect();
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], f64_to_fixed64(1.0));
        assert_eq!(values[1], f64_to_fixed64(2.0));
        assert_eq!(values[2], f64_to_fixed64(3.0));
    }

    // -----------------------------------------------------------------------
    // Test 2: RingBuffer wraps correctly
    // -----------------------------------------------------------------------
    #[test]
    fn ring_buffer_wraps_correctly() {
        let mut buf = RingBuffer::new(3);
        // Push 5 values into capacity-3 buffer.
        for i in 1..=5 {
            buf.push(f64_to_fixed64(i as f64));
        }

        assert_eq!(buf.len(), 3);
        assert_eq!(buf.capacity(), 3);

        // Should contain 3, 4, 5 (oldest to newest).
        let values: Vec<Fixed64> = buf.iter().collect();
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], f64_to_fixed64(3.0));
        assert_eq!(values[1], f64_to_fixed64(4.0));
        assert_eq!(values[2], f64_to_fixed64(5.0));
    }

    // -----------------------------------------------------------------------
    // Test 3: RingBuffer latest
    // -----------------------------------------------------------------------
    #[test]
    fn ring_buffer_latest() {
        let mut buf = RingBuffer::new(4);
        assert!(buf.latest().is_none());

        buf.push(f64_to_fixed64(10.0));
        assert_eq!(buf.latest(), Some(f64_to_fixed64(10.0)));

        buf.push(f64_to_fixed64(20.0));
        assert_eq!(buf.latest(), Some(f64_to_fixed64(20.0)));
    }

    // -----------------------------------------------------------------------
    // Test 4: RingBuffer clear
    // -----------------------------------------------------------------------
    #[test]
    fn ring_buffer_clear() {
        let mut buf = RingBuffer::new(4);
        buf.push(f64_to_fixed64(1.0));
        buf.push(f64_to_fixed64(2.0));
        assert_eq!(buf.len(), 2);

        buf.clear();
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
        assert!(buf.latest().is_none());
    }

    // -----------------------------------------------------------------------
    // Test 5: RingBuffer capacity of 1
    // -----------------------------------------------------------------------
    #[test]
    fn ring_buffer_capacity_one() {
        let mut buf = RingBuffer::new(1);
        buf.push(f64_to_fixed64(1.0));
        buf.push(f64_to_fixed64(2.0));

        assert_eq!(buf.len(), 1);
        assert_eq!(buf.latest(), Some(f64_to_fixed64(2.0)));
        let values = buf.to_vec();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], f64_to_fixed64(2.0));
    }

    // -----------------------------------------------------------------------
    // Test 6: RingBuffer ExactSizeIterator
    // -----------------------------------------------------------------------
    #[test]
    fn ring_buffer_exact_size_iterator() {
        let mut buf = RingBuffer::new(8);
        for i in 0..5 {
            buf.push(f64_to_fixed64(i as f64));
        }
        let iter = buf.iter();
        assert_eq!(iter.len(), 5);
    }

    // -----------------------------------------------------------------------
    // Test 7: Production rate computed from events
    // -----------------------------------------------------------------------
    #[test]
    fn production_rate_from_events() {
        let mut stats = ProductionStats::new(small_config());
        let node = make_node_id();

        // Simulate 10 ticks producing 5 iron per tick.
        for tick in 1..=10 {
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: 5,
                tick,
            });
            stats.end_tick(tick);
        }

        // Rate should be 5.0 items/tick.
        let rate = stats.get_production_rate(node, iron());
        assert_fixed_approx(rate, 5.0, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 8: Consumption rate computed from events
    // -----------------------------------------------------------------------
    #[test]
    fn consumption_rate_from_events() {
        let mut stats = ProductionStats::new(small_config());
        let node = make_node_id();

        for tick in 1..=10 {
            stats.process_event(&Event::ItemConsumed {
                node,
                item_type: copper(),
                quantity: 3,
                tick,
            });
            stats.end_tick(tick);
        }

        let rate = stats.get_consumption_rate(node, copper());
        assert_fixed_approx(rate, 3.0, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 9: Rolling average over configurable window
    // -----------------------------------------------------------------------
    #[test]
    fn rolling_average_window() {
        let config = StatsConfig {
            window_size: 5,
            history_capacity: 16,
        };
        let mut stats = ProductionStats::new(config);
        let node = make_node_id();

        // Produce for 5 ticks: quantities 10, 20, 30, 40, 50.
        for (i, tick) in (1..=5).enumerate() {
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: (i as u32 + 1) * 10,
                tick,
            });
            stats.end_tick(tick);
        }

        // Average should be (10+20+30+40+50)/5 = 30.0 items/tick.
        let rate = stats.get_production_rate(node, iron());
        assert_fixed_approx(rate, 30.0, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 10: Rolling window drops old data
    // -----------------------------------------------------------------------
    #[test]
    fn rolling_window_drops_old_data() {
        let config = StatsConfig {
            window_size: 3,
            history_capacity: 16,
        };
        let mut stats = ProductionStats::new(config);
        let node = make_node_id();

        // Tick 1: produce 10
        stats.process_event(&Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 10,
            tick: 1,
        });
        stats.end_tick(1);

        // Tick 2: produce 20
        stats.process_event(&Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 20,
            tick: 2,
        });
        stats.end_tick(2);

        // Tick 3: produce 30
        stats.process_event(&Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 30,
            tick: 3,
        });
        stats.end_tick(3);

        // At this point, window contains [10, 20, 30], avg = 20.0
        assert_fixed_approx(stats.get_production_rate(node, iron()), 20.0, 0.01);

        // Tick 4: produce 60 — oldest (10) falls off. Window: [20, 30, 60], avg = ~36.67
        stats.process_event(&Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 60,
            tick: 4,
        });
        stats.end_tick(4);

        assert_fixed_approx(stats.get_production_rate(node, iron()), 110.0 / 3.0, 0.1);
    }

    // -----------------------------------------------------------------------
    // Test 11: Idle ratio tracks correctly
    // -----------------------------------------------------------------------
    #[test]
    fn idle_ratio_tracks_correctly() {
        let config = StatsConfig {
            window_size: 10,
            history_capacity: 16,
        };
        let mut stats = ProductionStats::new(config);
        let node = make_node_id();

        // 7 idle ticks, 3 working ticks.
        for tick in 1..=7 {
            // No events = idle for this node.
            stats.get_or_create_node(node); // Ensure node is tracked.
            stats.end_tick(tick);
        }
        for tick in 8..=10 {
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: 1,
                tick,
            });
            stats.end_tick(tick);
        }

        // Idle ratio should be 7/10 = 0.7.
        assert_fixed_approx(stats.get_idle_ratio(node), 0.7, 0.01);
        // Uptime should be 3/10 = 0.3.
        assert_fixed_approx(stats.get_uptime(node), 0.3, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 12: Stall ratio tracks correctly
    // -----------------------------------------------------------------------
    #[test]
    fn stall_ratio_tracks_correctly() {
        let config = StatsConfig {
            window_size: 10,
            history_capacity: 16,
        };
        let mut stats = ProductionStats::new(config);
        let node = make_node_id();

        // 4 stalled ticks, 6 working ticks.
        for tick in 1..=4 {
            stats.process_event(&Event::BuildingStalled {
                node,
                reason: StallReason::MissingInputs,
                tick,
            });
            stats.end_tick(tick);
        }
        for tick in 5..=10 {
            stats.process_event(&Event::BuildingResumed { node, tick });
            stats.end_tick(tick);
        }

        // Stall ratio should be 4/10 = 0.4.
        assert_fixed_approx(stats.get_stall_ratio(node), 0.4, 0.01);
        // Uptime should be 6/10 = 0.6.
        assert_fixed_approx(stats.get_uptime(node), 0.6, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 13: Edge throughput rate
    // -----------------------------------------------------------------------
    #[test]
    fn edge_throughput_rate() {
        let mut stats = ProductionStats::new(small_config());
        let edge = make_edge_id();

        // Deliver 8 items per tick for 10 ticks.
        for tick in 1..=10 {
            stats.process_event(&Event::ItemDelivered {
                edge,
                quantity: 8,
                tick,
            });
            stats.end_tick(tick);
        }

        assert_fixed_approx(stats.get_throughput(edge), 8.0, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 14: Edge utilization ratio
    // -----------------------------------------------------------------------
    #[test]
    fn edge_utilization_ratio() {
        let config = StatsConfig {
            window_size: 10,
            history_capacity: 16,
        };
        let mut stats = ProductionStats::new(config);
        let edge = make_edge_id();

        // Full for 3 out of 10 ticks.
        for tick in 1..=3 {
            stats.process_event(&Event::TransportFull { edge, tick });
            stats.end_tick(tick);
        }
        for tick in 4..=10 {
            // Deliver something so the edge is tracked, but not full.
            stats.process_event(&Event::ItemDelivered {
                edge,
                quantity: 1,
                tick,
            });
            stats.end_tick(tick);
        }

        // Utilization = 3/10 = 0.3.
        assert_fixed_approx(stats.get_utilization(edge), 0.3, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 15: Global production/consumption totals
    // -----------------------------------------------------------------------
    #[test]
    fn global_production_consumption() {
        let mut stats = ProductionStats::new(small_config());
        let node_a = make_node_id();
        let node_b = make_node_id();

        // Both nodes produce iron: A produces 5/tick, B produces 3/tick.
        for tick in 1..=10 {
            stats.process_event(&Event::ItemProduced {
                node: node_a,
                item_type: iron(),
                quantity: 5,
                tick,
            });
            stats.process_event(&Event::ItemProduced {
                node: node_b,
                item_type: iron(),
                quantity: 3,
                tick,
            });
            stats.process_event(&Event::ItemConsumed {
                node: node_a,
                item_type: iron(),
                quantity: 2,
                tick,
            });
            stats.end_tick(tick);
        }

        // Total production: 5+3 = 8/tick.
        assert_fixed_approx(stats.get_total_production(iron()), 8.0, 0.01);
        // Total consumption: 2/tick.
        assert_fixed_approx(stats.get_total_consumption(iron()), 2.0, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 16: Historical data in ring buffer
    // -----------------------------------------------------------------------
    #[test]
    fn historical_data_ring_buffer() {
        let config = StatsConfig {
            window_size: 5,
            history_capacity: 4,
        };
        let mut stats = ProductionStats::new(config);
        let node = make_node_id();

        // Produce for 6 ticks — history capacity is 4, so oldest 2 fall off.
        for tick in 1..=6 {
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: tick as u32 * 10,
                tick,
            });
            stats.end_tick(tick);
        }

        let history = stats.get_history(node, iron());
        // Should have 4 entries (capacity).
        assert_eq!(history.len(), 4);

        // The history stores the rolling rate at each tick, not the raw count.
        // After 6 ticks with window_size=5:
        // Each entry is the rate snapshot at that tick.
        // All entries should be positive Fixed64 values.
        for value in &history {
            assert!(*value > Fixed64::ZERO);
        }
    }

    // -----------------------------------------------------------------------
    // Test 17: Edge throughput history
    // -----------------------------------------------------------------------
    #[test]
    fn edge_throughput_history() {
        let config = StatsConfig {
            window_size: 5,
            history_capacity: 4,
        };
        let mut stats = ProductionStats::new(config);
        let edge = make_edge_id();

        for tick in 1..=6 {
            stats.process_event(&Event::ItemDelivered {
                edge,
                quantity: 10,
                tick,
            });
            stats.end_tick(tick);
        }

        let history = stats.get_edge_history(edge);
        assert_eq!(history.len(), 4);
        for value in &history {
            assert!(*value > Fixed64::ZERO);
        }
    }

    // -----------------------------------------------------------------------
    // Test 18: Multiple item types tracked independently
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_item_types_independent() {
        let mut stats = ProductionStats::new(small_config());
        let node = make_node_id();

        for tick in 1..=10 {
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: 5,
                tick,
            });
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: copper(),
                quantity: 2,
                tick,
            });
            stats.end_tick(tick);
        }

        assert_fixed_approx(stats.get_production_rate(node, iron()), 5.0, 0.01);
        assert_fixed_approx(stats.get_production_rate(node, copper()), 2.0, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 19: No events returns zero rates
    // -----------------------------------------------------------------------
    #[test]
    fn no_events_returns_zero() {
        let stats = ProductionStats::new(small_config());
        let node = make_node_id();
        let edge = make_edge_id();

        assert_eq!(stats.get_production_rate(node, iron()), Fixed64::ZERO);
        assert_eq!(stats.get_consumption_rate(node, iron()), Fixed64::ZERO);
        assert_eq!(stats.get_idle_ratio(node), Fixed64::ZERO);
        assert_eq!(stats.get_stall_ratio(node), Fixed64::ZERO);
        assert_eq!(stats.get_uptime(node), Fixed64::ZERO);
        assert_eq!(stats.get_throughput(edge), Fixed64::ZERO);
        assert_eq!(stats.get_utilization(edge), Fixed64::ZERO);
        assert_eq!(stats.get_total_production(iron()), Fixed64::ZERO);
        assert_eq!(stats.get_total_consumption(iron()), Fixed64::ZERO);
        assert!(stats.get_history(node, iron()).is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 20: Remove node clears its stats
    // -----------------------------------------------------------------------
    #[test]
    fn remove_node_clears_stats() {
        let mut stats = ProductionStats::new(small_config());
        let node = make_node_id();

        stats.process_event(&Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 10,
            tick: 1,
        });
        stats.end_tick(1);
        assert_eq!(stats.tracked_node_count(), 1);

        stats.remove_node(node);
        assert_eq!(stats.tracked_node_count(), 0);
        assert_eq!(stats.get_production_rate(node, iron()), Fixed64::ZERO);
    }

    // -----------------------------------------------------------------------
    // Test 21: Remove edge clears its stats
    // -----------------------------------------------------------------------
    #[test]
    fn remove_edge_clears_stats() {
        let mut stats = ProductionStats::new(small_config());
        let edge = make_edge_id();

        stats.process_event(&Event::ItemDelivered {
            edge,
            quantity: 5,
            tick: 1,
        });
        stats.end_tick(1);
        assert_eq!(stats.tracked_edge_count(), 1);

        stats.remove_edge(edge);
        assert_eq!(stats.tracked_edge_count(), 0);
        assert_eq!(stats.get_throughput(edge), Fixed64::ZERO);
    }

    // -----------------------------------------------------------------------
    // Test 22: Clear resets everything
    // -----------------------------------------------------------------------
    #[test]
    fn clear_resets_everything() {
        let mut stats = ProductionStats::new(small_config());
        let node = make_node_id();
        let edge = make_edge_id();

        stats.process_event(&Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 10,
            tick: 1,
        });
        stats.process_event(&Event::ItemDelivered {
            edge,
            quantity: 5,
            tick: 1,
        });
        stats.end_tick(1);

        stats.clear();
        assert_eq!(stats.tracked_node_count(), 0);
        assert_eq!(stats.tracked_edge_count(), 0);
        assert_eq!(stats.tracked_item_type_count(), 0);
        assert_eq!(stats.current_tick(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 23: Uptime + idle + stall = 1.0
    // -----------------------------------------------------------------------
    #[test]
    fn ratios_sum_to_one() {
        let config = StatsConfig {
            window_size: 12,
            history_capacity: 16,
        };
        let mut stats = ProductionStats::new(config);
        let node = make_node_id();

        // 4 idle, 5 working, 3 stalled = 12 ticks.
        for tick in 1..=4 {
            stats.get_or_create_node(node);
            stats.end_tick(tick);
        }
        for tick in 5..=9 {
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: 1,
                tick,
            });
            stats.end_tick(tick);
        }
        for tick in 10..=12 {
            stats.process_event(&Event::BuildingStalled {
                node,
                reason: StallReason::OutputFull,
                tick,
            });
            stats.end_tick(tick);
        }

        let idle = stats.get_idle_ratio(node);
        let stall = stats.get_stall_ratio(node);
        let uptime = stats.get_uptime(node);
        let sum = idle + stall + uptime;

        // Sum should be approximately 1.0.
        assert_fixed_approx(sum, 1.0, 0.01);
        assert_fixed_approx(idle, 4.0 / 12.0, 0.01);
        assert_fixed_approx(stall, 3.0 / 12.0, 0.01);
        assert_fixed_approx(uptime, 5.0 / 12.0, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 24: Stall then resume transitions correctly
    // -----------------------------------------------------------------------
    #[test]
    fn stall_then_resume_transitions() {
        let config = StatsConfig {
            window_size: 4,
            history_capacity: 16,
        };
        let mut stats = ProductionStats::new(config);
        let node = make_node_id();

        // Tick 1: stalled.
        stats.process_event(&Event::BuildingStalled {
            node,
            reason: StallReason::NoPower,
            tick: 1,
        });
        stats.end_tick(1);

        // Tick 2: still stalled (no resume event).
        stats.process_event(&Event::BuildingStalled {
            node,
            reason: StallReason::NoPower,
            tick: 2,
        });
        stats.end_tick(2);

        // Tick 3: resumed.
        stats.process_event(&Event::BuildingResumed { node, tick: 3 });
        stats.end_tick(3);

        // Tick 4: producing.
        stats.process_event(&Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 4,
        });
        stats.end_tick(4);

        // 2 stalled, 2 working = stall_ratio 0.5, uptime 0.5.
        assert_fixed_approx(stats.get_stall_ratio(node), 0.5, 0.01);
        assert_fixed_approx(stats.get_uptime(node), 0.5, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 25: Large quantity events
    // -----------------------------------------------------------------------
    #[test]
    fn large_quantity_events() {
        let mut stats = ProductionStats::new(small_config());
        let node = make_node_id();

        for tick in 1..=10 {
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: 1000,
                tick,
            });
            stats.end_tick(tick);
        }

        assert_fixed_approx(stats.get_production_rate(node, iron()), 1000.0, 0.1);
        assert_fixed_approx(stats.get_total_production(iron()), 1000.0, 0.1);
    }

    // -----------------------------------------------------------------------
    // Test 26: RollingWindow rate with partial fill
    // -----------------------------------------------------------------------
    #[test]
    fn rolling_window_partial_fill() {
        let config = StatsConfig {
            window_size: 100,
            history_capacity: 16,
        };
        let mut stats = ProductionStats::new(config);
        let node = make_node_id();

        // Only 5 ticks into a 100-tick window.
        for tick in 1..=5 {
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: 10,
                tick,
            });
            stats.end_tick(tick);
        }

        // Rate should be 10/tick (averaged over 5 filled ticks, not 100).
        assert_fixed_approx(stats.get_production_rate(node, iron()), 10.0, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 27: Multiple events per tick accumulate
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_events_per_tick_accumulate() {
        let mut stats = ProductionStats::new(small_config());
        let node = make_node_id();

        // Two production events in the same tick.
        for tick in 1..=10 {
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: 3,
                tick,
            });
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: 7,
                tick,
            });
            stats.end_tick(tick);
        }

        // Rate should be 10/tick (3+7).
        assert_fixed_approx(stats.get_production_rate(node, iron()), 10.0, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 28: Default config values
    // -----------------------------------------------------------------------
    #[test]
    fn default_config() {
        let config = StatsConfig::default();
        assert_eq!(config.window_size, 60);
        assert_eq!(config.history_capacity, 256);
    }

    // -----------------------------------------------------------------------
    // Test 29: Ignored event types don't cause tracking
    // -----------------------------------------------------------------------
    #[test]
    fn ignored_events_dont_track() {
        use factorial_core::id::BuildingTypeId;

        let mut stats = ProductionStats::new(small_config());
        let node = make_node_id();

        // NodeAdded is not tracked by stats.
        stats.process_event(&Event::NodeAdded {
            node,
            building_type: BuildingTypeId(0),
            tick: 1,
        });
        stats.end_tick(1);

        assert_eq!(stats.tracked_node_count(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 30: Concurrent production and consumption on same node
    // -----------------------------------------------------------------------
    #[test]
    fn concurrent_production_and_consumption() {
        let mut stats = ProductionStats::new(small_config());
        let node = make_node_id();

        for tick in 1..=10 {
            stats.process_event(&Event::ItemConsumed {
                node,
                item_type: iron(),
                quantity: 2,
                tick,
            });
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: copper(),
                quantity: 1,
                tick,
            });
            stats.end_tick(tick);
        }

        assert_fixed_approx(stats.get_consumption_rate(node, iron()), 2.0, 0.01);
        assert_fixed_approx(stats.get_production_rate(node, copper()), 1.0, 0.01);
        // Iron production should be zero — only consumption.
        assert_eq!(stats.get_production_rate(node, iron()), Fixed64::ZERO);
    }

    // -----------------------------------------------------------------------
    // Test 31: Transport full and delivery in same tick
    // -----------------------------------------------------------------------
    #[test]
    fn transport_full_and_delivery_same_tick() {
        let config = StatsConfig {
            window_size: 10,
            history_capacity: 16,
        };
        let mut stats = ProductionStats::new(config);
        let edge = make_edge_id();

        // All 10 ticks: deliver items AND report full.
        for tick in 1..=10 {
            stats.process_event(&Event::ItemDelivered {
                edge,
                quantity: 5,
                tick,
            });
            stats.process_event(&Event::TransportFull { edge, tick });
            stats.end_tick(tick);
        }

        assert_fixed_approx(stats.get_throughput(edge), 5.0, 0.01);
        // All ticks were full => utilization = 1.0.
        assert_fixed_approx(stats.get_utilization(edge), 1.0, 0.01);
    }

    // -----------------------------------------------------------------------
    // Test 32: Window boundary — rate drops to zero after idle period
    // -----------------------------------------------------------------------
    #[test]
    fn rate_drops_after_idle_period() {
        let config = StatsConfig {
            window_size: 5,
            history_capacity: 16,
        };
        let mut stats = ProductionStats::new(config);
        let node = make_node_id();

        // Produce for 5 ticks.
        for tick in 1..=5 {
            stats.process_event(&Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: 10,
                tick,
            });
            stats.end_tick(tick);
        }
        assert_fixed_approx(stats.get_production_rate(node, iron()), 10.0, 0.01);

        // Idle for 5 more ticks (no production events).
        for tick in 6..=10 {
            stats.end_tick(tick);
        }

        // All production has rolled off the window.
        assert_fixed_approx(stats.get_production_rate(node, iron()), 0.0, 0.01);
    }
}
