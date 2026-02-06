//! Transport strategies for moving items along edges in the production graph.
//!
//! Each edge in the production graph has an assigned transport strategy that
//! determines how items move from source to destination. The system uses
//! **enum dispatch** (not trait objects) for performance: sized inline storage,
//! predictable branching, and compiler-optimizable hot loops.
//!
//! During the transport phase, edges are grouped by transport variant and
//! processed in homogeneous batches for cache locality.
//!
//! # Transport Types
//!
//! - [`FlowTransport`] — continuous rate-based flow (pipes, conveyors)
//! - [`ItemTransport`] — discrete belt with slots (Factorio-style belts)
//! - [`BatchTransport`] — discrete chunks per cycle (train loads, pallets)
//! - [`VehicleTransport`] — vehicle with capacity and travel time (trucks, drones)

use crate::fixed::Fixed64;
use crate::id::ItemTypeId;
use crate::item::ItemStack;

// ---------------------------------------------------------------------------
// Transport configuration (per-edge, immutable after creation)
// ---------------------------------------------------------------------------

/// Transport strategy assigned to an edge. Determines how items move.
///
/// Uses enum dispatch for sized inline storage and branch-predictor-friendly
/// processing when edges are grouped by variant.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Transport {
    /// Continuous rate-based flow (pipes in Builderment/Satisfactory).
    Flow(FlowTransport),
    /// Discrete belt with slots (Factorio conveyor belts).
    Item(ItemTransport),
    /// Discrete chunks per cycle (train loads, pallets).
    Batch(BatchTransport),
    /// Vehicle with capacity and travel time (trucks, drones).
    Vehicle(VehicleTransport),
}

/// Continuous rate-based flow transport.
///
/// Items flow at a fixed rate per tick, with an optional latency delay
/// before items appear at the destination. A buffer accumulates fractional
/// items between ticks.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowTransport {
    /// Items per tick (fractional via fixed-point).
    pub rate: Fixed64,
    /// Maximum buffered amount before back-pressure kicks in.
    pub buffer_capacity: Fixed64,
    /// Ticks delay before items appear at destination.
    pub latency: u32,
}

/// Discrete belt transport with individually tracked slots.
///
/// Models conveyor belts where each slot can hold one item. Items advance
/// through slots each tick at the configured speed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ItemTransport {
    /// Slots advanced per tick (fractional via fixed-point).
    pub speed: Fixed64,
    /// Total number of slots on the belt.
    pub slot_count: u32,
    /// Number of parallel lanes (typically 1-2).
    pub lanes: u8,
}

/// Discrete batch transport delivering chunks per cycle.
///
/// Delivers `batch_size` items every `cycle_time` ticks. Simple model for
/// train loads, courier pallets, etc.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchTransport {
    /// Items per batch delivery.
    pub batch_size: u32,
    /// Ticks per batch cycle.
    pub cycle_time: u32,
}

/// Vehicle transport with capacity and travel time.
///
/// A vehicle travels from source to destination, loads up to `capacity` items,
/// delivers them, then returns. The round trip takes `2 * travel_time` ticks.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VehicleTransport {
    /// Maximum items the vehicle can carry.
    pub capacity: u32,
    /// Ticks for one trip (source to destination).
    pub travel_time: u32,
}

// ---------------------------------------------------------------------------
// Transport state (per-edge, mutable each tick)
// ---------------------------------------------------------------------------

/// Mutable transport state, stored externally in typed arenas for SoA locality.
/// Variants match the [`Transport`] enum one-to-one.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TransportState {
    Flow(FlowState),
    Item(BeltState),
    Batch(BatchState),
    Vehicle(VehicleState),
}

/// State for [`FlowTransport`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowState {
    /// Amount currently buffered (fractional items in transit).
    pub buffered: Fixed64,
    /// Remaining latency ticks before buffered items start delivering.
    pub latency_remaining: u32,
}

/// State for [`ItemTransport`].
///
/// Slots are stored as a flat array. Each slot is `None` (empty) or
/// `Some(ItemTypeId)`. The array is pre-allocated at creation to the belt's
/// declared length times lane count. No runtime reallocation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BeltState {
    /// Flat array of slots: `lanes * slot_count` entries.
    /// Layout: lane 0 slots [0..slot_count), lane 1 slots [slot_count..2*slot_count), etc.
    pub slots: Vec<Option<ItemTypeId>>,
}

/// State for [`BatchTransport`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchState {
    /// Current progress through the cycle (0..cycle_time).
    pub progress: u32,
    /// Items pending delivery in the current batch.
    pub pending: u32,
}

/// State for [`VehicleTransport`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VehicleState {
    /// Position along the route: 0 = at source, travel_time = at destination.
    pub position: u32,
    /// Items currently being carried.
    pub cargo: Vec<ItemStack>,
    /// Whether the vehicle is on the return trip.
    pub returning: bool,
}

// ---------------------------------------------------------------------------
// Transport result
// ---------------------------------------------------------------------------

/// The outcome of a single `advance` call on a transport edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportResult {
    /// Items accepted from the source this tick (consumed from `available`).
    pub items_moved: u32,
    /// Items that arrived at the destination this tick (ready for pickup).
    pub items_delivered: u32,
}

// ---------------------------------------------------------------------------
// State factory
// ---------------------------------------------------------------------------

impl TransportState {
    /// Create a fresh state matching the given transport configuration.
    pub fn new_for(transport: &Transport) -> Self {
        match transport {
            Transport::Flow(flow) => TransportState::Flow(FlowState {
                buffered: Fixed64::ZERO,
                latency_remaining: flow.latency,
            }),
            Transport::Item(item) => {
                let total_slots = item.slot_count as usize * item.lanes as usize;
                TransportState::Item(BeltState {
                    slots: vec![None; total_slots],
                })
            }
            Transport::Batch(_) => TransportState::Batch(BatchState {
                progress: 0,
                pending: 0,
            }),
            Transport::Vehicle(_) => TransportState::Vehicle(VehicleState {
                position: 0,
                cargo: Vec::new(),
                returning: false,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Advance logic
// ---------------------------------------------------------------------------

impl Transport {
    /// Advance this transport by one tick.
    ///
    /// - `state`: mutable transport state (must match `self` variant).
    /// - `available`: number of items available at the source for pickup.
    ///
    /// Returns a [`TransportResult`] describing items moved and delivered.
    ///
    /// # Panics
    ///
    /// Panics if `state` variant does not match `self` variant.
    pub fn advance(&self, state: &mut TransportState, available: u32) -> TransportResult {
        match (self, state) {
            (Transport::Flow(flow), TransportState::Flow(fs)) => {
                advance_flow(flow, fs, available)
            }
            (Transport::Item(item), TransportState::Item(bs)) => {
                advance_item(item, bs, available)
            }
            (Transport::Batch(batch), TransportState::Batch(bs)) => {
                advance_batch(batch, bs, available)
            }
            (Transport::Vehicle(vehicle), TransportState::Vehicle(vs)) => {
                advance_vehicle(vehicle, vs, available)
            }
            _ => {
                debug_assert!(false, "Transport variant does not match TransportState variant");
                TransportResult { items_moved: 0, items_delivered: 0 }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Flow transport advance
// ---------------------------------------------------------------------------

/// Advance a flow transport by one tick.
///
/// Behavior:
/// 1. Accept up to `rate` items from source, limited by buffer capacity.
/// 2. If latency has expired, deliver buffered items (up to rate) to destination.
/// 3. If latency has not expired, decrement latency counter (no delivery).
fn advance_flow(
    flow: &FlowTransport,
    state: &mut FlowState,
    available: u32,
) -> TransportResult {
    let available_fixed = Fixed64::from_num(available);
    let rate = flow.rate;

    // How much can we accept? Limited by rate and remaining buffer capacity.
    let space_in_buffer = flow.buffer_capacity - state.buffered;
    let can_accept = rate.min(available_fixed).min(space_in_buffer);
    // Clamp to zero in case of negative from rounding.
    let accepted = if can_accept > Fixed64::ZERO { can_accept } else { Fixed64::ZERO };

    state.buffered += accepted;

    let items_moved: u32 = accepted.to_num();

    // Delivery: only after latency has expired.
    let items_delivered = if state.latency_remaining > 0 {
        state.latency_remaining -= 1;
        0
    } else {
        // Deliver up to rate from the buffer.
        let can_deliver = rate.min(state.buffered);
        let delivered = if can_deliver > Fixed64::ZERO { can_deliver } else { Fixed64::ZERO };
        state.buffered -= delivered;
        delivered.to_num()
    };

    TransportResult {
        items_moved,
        items_delivered,
    }
}

// ---------------------------------------------------------------------------
// Item (belt) transport advance
// ---------------------------------------------------------------------------

/// Advance a belt transport by one tick.
///
/// Items advance through slots from high index toward low index (slot 0 is
/// the output end, slot N-1 is the input end). Each tick:
/// 1. Move items forward through slots (respecting occupancy — no passing).
/// 2. Try to insert new items at the input end if slots are free.
///
/// Returns items that fell off the output end (delivered) and items inserted
/// at the input end (moved).
///
/// For simplicity in this initial implementation, `speed` is treated as
/// integer slots per tick (the integer part of the fixed-point value).
fn advance_item(
    item: &ItemTransport,
    state: &mut BeltState,
    available: u32,
) -> TransportResult {
    let slot_count = item.slot_count as usize;
    let lanes = item.lanes as usize;
    let steps: usize = item.speed.to_num::<u32>() as usize;
    let steps = steps.max(1); // At least 1 step per tick.

    let mut items_delivered = 0u32;
    let mut items_moved = 0u32;

    // Process each lane independently.
    for lane in 0..lanes {
        let base = lane * slot_count;

        for _step in 0..steps {
            // Phase 1: Advance items toward slot 0.
            // Walk from slot 1 to slot N-1; if current slot has item and
            // previous slot is empty, move it forward.
            for i in 1..slot_count {
                if state.slots[base + i].is_some() && state.slots[base + i - 1].is_none() {
                    state.slots[base + i - 1] = state.slots[base + i].take();
                }
            }

            // Phase 2: Check if output slot (index 0) has an item to deliver.
            if state.slots[base].is_some() {
                state.slots[base] = None;
                items_delivered += 1;
            }

            // Phase 3: Insert new item at input end if available and slot is free.
            let input_slot = base + slot_count - 1;
            if available > items_moved && state.slots[input_slot].is_none() {
                // Use a placeholder ItemTypeId since we track count, not type,
                // in this simplified model.
                state.slots[input_slot] = Some(ItemTypeId(0));
                items_moved += 1;
            }
        }
    }

    TransportResult {
        items_moved,
        items_delivered,
    }
}

// ---------------------------------------------------------------------------
// Batch transport advance
// ---------------------------------------------------------------------------

/// Advance a batch transport by one tick.
///
/// Behavior:
/// 1. Increment progress counter.
/// 2. Accept items into the pending buffer (up to batch_size).
/// 3. When progress reaches cycle_time, deliver pending items and reset.
fn advance_batch(
    batch: &BatchTransport,
    state: &mut BatchState,
    available: u32,
) -> TransportResult {
    // Accept items into pending (up to batch_size).
    let space = batch.batch_size.saturating_sub(state.pending);
    let accepted = available.min(space);
    state.pending += accepted;

    // Advance progress.
    state.progress += 1;

    // Check if cycle is complete.
    let items_delivered;
    if state.progress >= batch.cycle_time {
        items_delivered = state.pending;
        state.pending = 0;
        state.progress = 0;
    } else {
        items_delivered = 0;
    }

    TransportResult {
        items_moved: accepted,
        items_delivered,
    }
}

// ---------------------------------------------------------------------------
// Vehicle transport advance
// ---------------------------------------------------------------------------

/// Advance a vehicle transport by one tick.
///
/// Vehicle lifecycle:
/// 1. At source (position=0, not returning): load items up to capacity.
/// 2. Travel toward destination (position increments each tick).
/// 3. At destination (position >= travel_time): deliver cargo, begin return.
/// 4. Return trip (returning=true): position decrements each tick.
/// 5. Back at source (position=0, returning): ready for next load.
fn advance_vehicle(
    vehicle: &VehicleTransport,
    state: &mut VehicleState,
    available: u32,
) -> TransportResult {
    let mut items_moved = 0u32;
    let mut items_delivered = 0u32;

    if state.returning {
        // Returning to source.
        if state.position > 0 {
            state.position -= 1;
        }
        if state.position == 0 {
            // Arrived back at source, ready to load again.
            state.returning = false;
        }
    } else {
        // Going toward destination.
        if state.position == 0 && state.cargo.is_empty() {
            // At source — load up.
            let to_load = available.min(vehicle.capacity);
            if to_load > 0 {
                state.cargo.push(ItemStack::new(ItemTypeId(0), to_load));
                items_moved = to_load;
            }
            // Depart (even with partial load, to keep things moving).
            if !state.cargo.is_empty() {
                state.position += 1;
            }
        } else {
            // In transit toward destination.
            state.position += 1;

            if state.position >= vehicle.travel_time {
                // Arrived at destination — deliver cargo.
                items_delivered = state.cargo.iter().map(|s| s.quantity).sum();
                state.cargo.clear();
                state.returning = true;
                // Position stays at travel_time; will decrement on return.
            }
        }
    }

    TransportResult {
        items_moved,
        items_delivered,
    }
}

// ---------------------------------------------------------------------------
// Helper: count occupied slots on a belt
// ---------------------------------------------------------------------------

impl BeltState {
    /// Count total occupied slots across all lanes.
    pub fn occupied_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_flow(rate: f64, buffer_capacity: f64, latency: u32) -> (Transport, TransportState) {
        let t = Transport::Flow(FlowTransport {
            rate: Fixed64::from_num(rate),
            buffer_capacity: Fixed64::from_num(buffer_capacity),
            latency,
        });
        let s = TransportState::new_for(&t);
        (t, s)
    }

    fn make_belt(speed: f64, slot_count: u32, lanes: u8) -> (Transport, TransportState) {
        let t = Transport::Item(ItemTransport {
            speed: Fixed64::from_num(speed),
            slot_count,
            lanes,
        });
        let s = TransportState::new_for(&t);
        (t, s)
    }

    fn make_batch(batch_size: u32, cycle_time: u32) -> (Transport, TransportState) {
        let t = Transport::Batch(BatchTransport {
            batch_size,
            cycle_time,
        });
        let s = TransportState::new_for(&t);
        (t, s)
    }

    fn make_vehicle(capacity: u32, travel_time: u32) -> (Transport, TransportState) {
        let t = Transport::Vehicle(VehicleTransport {
            capacity,
            travel_time,
        });
        let s = TransportState::new_for(&t);
        (t, s)
    }

    // -----------------------------------------------------------------------
    // Test 1: FlowTransport — items flow at declared rate
    // -----------------------------------------------------------------------
    #[test]
    fn flow_items_at_declared_rate() {
        let (t, mut s) = make_flow(2.0, 100.0, 0);

        // With 10 available and rate=2, should move 2 per tick.
        let r = t.advance(&mut s, 10);
        assert_eq!(r.items_moved, 2);
        // With zero latency, items deliver immediately.
        assert_eq!(r.items_delivered, 2);

        // Second tick: same behavior.
        let r = t.advance(&mut s, 10);
        assert_eq!(r.items_moved, 2);
        assert_eq!(r.items_delivered, 2);
    }

    // -----------------------------------------------------------------------
    // Test 2: FlowTransport — respects buffer capacity
    // -----------------------------------------------------------------------
    #[test]
    fn flow_respects_buffer_capacity() {
        // Rate is 5, but buffer only holds 3 total. With latency=999 (high),
        // nothing delivers so buffer fills up.
        let (t, mut s) = make_flow(5.0, 3.0, 999);

        // Tick 1: accept 3 (limited by buffer capacity, not rate).
        let r = t.advance(&mut s, 100);
        assert_eq!(r.items_moved, 3);
        assert_eq!(r.items_delivered, 0); // latency not expired

        // Tick 2: buffer is full, can't accept more.
        let r = t.advance(&mut s, 100);
        assert_eq!(r.items_moved, 0);
        assert_eq!(r.items_delivered, 0);
    }

    // -----------------------------------------------------------------------
    // Test 3: FlowTransport — latency delay
    // -----------------------------------------------------------------------
    #[test]
    fn flow_latency_delay() {
        let (t, mut s) = make_flow(5.0, 100.0, 3);

        // Ticks 1-3: items enter buffer but nothing delivers (latency=3).
        for _ in 0..3 {
            let r = t.advance(&mut s, 100);
            assert_eq!(r.items_moved, 5);
            assert_eq!(r.items_delivered, 0, "should not deliver during latency");
        }

        // Tick 4: latency has expired, items start delivering.
        let r = t.advance(&mut s, 100);
        assert_eq!(r.items_moved, 5);
        assert_eq!(r.items_delivered, 5);
    }

    // -----------------------------------------------------------------------
    // Test 4: ItemTransport — items advance through belt
    // -----------------------------------------------------------------------
    #[test]
    fn belt_items_advance_through() {
        // 5-slot belt, speed=1, 1 lane.
        let (t, mut s) = make_belt(1.0, 5, 1);

        // Tick 1: insert at input end (slot 4).
        let r = t.advance(&mut s, 1);
        assert_eq!(r.items_moved, 1);
        assert_eq!(r.items_delivered, 0);

        // Ticks 2-4: item advances toward output.
        for _ in 0..3 {
            let r = t.advance(&mut s, 0);
            assert_eq!(r.items_delivered, 0);
        }

        // Tick 5: item reaches slot 0 and is delivered.
        let r = t.advance(&mut s, 0);
        assert_eq!(r.items_delivered, 1);
    }

    // -----------------------------------------------------------------------
    // Test 5: ItemTransport — back-pressure (belt full)
    // -----------------------------------------------------------------------
    #[test]
    fn belt_back_pressure() {
        // 3-slot belt, speed=1, 1 lane.
        let (t, mut s) = make_belt(1.0, 3, 1);

        // Manually fill all slots to simulate a full belt.
        if let TransportState::Item(ref mut bs) = s {
            bs.slots[0] = Some(ItemTypeId(0));
            bs.slots[1] = Some(ItemTypeId(0));
            bs.slots[2] = Some(ItemTypeId(0));
        }

        // Belt is completely full. Advance tries to shift items forward,
        // but all slots are occupied so nothing shifts. Then slot 0 is
        // delivered, freeing it. However, items can't cascade-shift within
        // the same tick step after delivery, so the input slot (slot 2)
        // is still occupied and no new item can be inserted.
        let r = t.advance(&mut s, 10);
        assert_eq!(r.items_delivered, 1);
        assert_eq!(r.items_moved, 0, "cannot insert when input slot is occupied");

        // After the tick, belt has 2 items: slots 1 and 2 occupied, slot 0 free.
        if let TransportState::Item(ref bs) = s {
            assert_eq!(bs.occupied_count(), 2);
        }

        // Next tick: items shift forward (slot 1->0, slot 2->1), slot 0
        // delivers, and now input slot (2) is free for a new item.
        let r = t.advance(&mut s, 10);
        assert_eq!(r.items_delivered, 1);
        assert_eq!(r.items_moved, 1, "input slot is now free after shift");

        // Belt still has 2 items (one shifted in, one at slot 1).
        if let TransportState::Item(ref bs) = s {
            assert_eq!(bs.occupied_count(), 2);
        }
    }

    // -----------------------------------------------------------------------
    // Test 6: BatchTransport — discrete chunks per cycle
    // -----------------------------------------------------------------------
    #[test]
    fn batch_discrete_chunks() {
        // batch_size=10, cycle_time=5
        let (t, mut s) = make_batch(10, 5);

        // Ticks 1-4: items accumulate, nothing delivered.
        for tick in 1..=4 {
            let r = t.advance(&mut s, 100);
            assert!(r.items_moved > 0 || tick > 1, "should accept items");
            assert_eq!(r.items_delivered, 0, "should not deliver mid-cycle");
        }

        // Tick 5: cycle completes, batch delivered.
        let r = t.advance(&mut s, 100);
        assert_eq!(r.items_delivered, 10);
    }

    // -----------------------------------------------------------------------
    // Test 7: VehicleTransport — travel time
    // -----------------------------------------------------------------------
    #[test]
    fn vehicle_travel_time() {
        // capacity=50, travel_time=5
        let (t, mut s) = make_vehicle(50, 5);

        // Tick 1: vehicle loads and departs (position goes from 0 to 1).
        let r = t.advance(&mut s, 50);
        assert_eq!(r.items_moved, 50);
        assert_eq!(r.items_delivered, 0);

        // Ticks 2-4: vehicle in transit, no delivery.
        for _ in 0..3 {
            let r = t.advance(&mut s, 0);
            assert_eq!(r.items_delivered, 0);
        }

        // Tick 5: vehicle arrives, delivers cargo.
        let r = t.advance(&mut s, 0);
        assert_eq!(r.items_delivered, 50);

        // Verify vehicle is now returning.
        if let TransportState::Vehicle(ref vs) = s {
            assert!(vs.returning);
            assert!(vs.cargo.is_empty());
        }
    }

    // -----------------------------------------------------------------------
    // Test 8: VehicleTransport — capacity limit
    // -----------------------------------------------------------------------
    #[test]
    fn vehicle_capacity_limit() {
        // capacity=20, travel_time=1
        let (t, mut s) = make_vehicle(20, 1);

        // 100 available but capacity is only 20.
        let r = t.advance(&mut s, 100);
        assert_eq!(r.items_moved, 20, "should only load up to capacity");
    }

    // -----------------------------------------------------------------------
    // Test 9: TransportState::new_for creates correct variants
    // -----------------------------------------------------------------------
    #[test]
    fn state_new_for_creates_correct_variants() {
        let flow = Transport::Flow(FlowTransport {
            rate: Fixed64::from_num(1),
            buffer_capacity: Fixed64::from_num(10),
            latency: 5,
        });
        let state = TransportState::new_for(&flow);
        assert!(matches!(state, TransportState::Flow(FlowState {
            latency_remaining: 5, ..
        })));

        let item = Transport::Item(ItemTransport {
            speed: Fixed64::from_num(1),
            slot_count: 10,
            lanes: 2,
        });
        let state = TransportState::new_for(&item);
        if let TransportState::Item(bs) = &state {
            assert_eq!(bs.slots.len(), 20); // 10 slots * 2 lanes
        } else {
            panic!("expected BeltState");
        }

        let batch = Transport::Batch(BatchTransport {
            batch_size: 5,
            cycle_time: 3,
        });
        let state = TransportState::new_for(&batch);
        assert!(matches!(state, TransportState::Batch(BatchState {
            progress: 0,
            pending: 0,
        })));

        let vehicle = Transport::Vehicle(VehicleTransport {
            capacity: 100,
            travel_time: 10,
        });
        let state = TransportState::new_for(&vehicle);
        if let TransportState::Vehicle(vs) = &state {
            assert_eq!(vs.position, 0);
            assert!(vs.cargo.is_empty());
            assert!(!vs.returning);
        } else {
            panic!("expected VehicleState");
        }
    }

    // -----------------------------------------------------------------------
    // Test 10: VehicleTransport — full round trip
    // -----------------------------------------------------------------------
    #[test]
    fn vehicle_full_round_trip() {
        // capacity=10, travel_time=3
        let (t, mut s) = make_vehicle(10, 3);

        // Tick 1: load and depart.
        let r = t.advance(&mut s, 10);
        assert_eq!(r.items_moved, 10);
        assert_eq!(r.items_delivered, 0);

        // Tick 2: in transit.
        let r = t.advance(&mut s, 0);
        assert_eq!(r.items_delivered, 0);

        // Tick 3: arrives, delivers.
        let r = t.advance(&mut s, 0);
        assert_eq!(r.items_delivered, 10);

        // Ticks 4-6: returning (position 3 -> 2 -> 1 -> 0).
        for _ in 0..3 {
            let r = t.advance(&mut s, 0);
            assert_eq!(r.items_moved, 0);
            assert_eq!(r.items_delivered, 0);
        }

        // Vehicle should be back at source and ready.
        if let TransportState::Vehicle(ref vs) = s {
            assert_eq!(vs.position, 0);
            assert!(!vs.returning);
        }

        // Tick 7: can load again.
        let r = t.advance(&mut s, 10);
        assert_eq!(r.items_moved, 10);
    }

    // -----------------------------------------------------------------------
    // Test 11: FlowTransport — zero available
    // -----------------------------------------------------------------------
    #[test]
    fn flow_zero_available() {
        let (t, mut s) = make_flow(5.0, 100.0, 0);

        let r = t.advance(&mut s, 0);
        assert_eq!(r.items_moved, 0);
        assert_eq!(r.items_delivered, 0);
    }

    // -----------------------------------------------------------------------
    // Test 12: BatchTransport — partial batch
    // -----------------------------------------------------------------------
    #[test]
    fn batch_partial_batch() {
        // batch_size=10, cycle_time=3. Only 4 available total.
        let (t, mut s) = make_batch(10, 3);

        // Tick 1: accept 4 (all available).
        let r = t.advance(&mut s, 4);
        assert_eq!(r.items_moved, 4);

        // Tick 2: no more available.
        let r = t.advance(&mut s, 0);
        assert_eq!(r.items_moved, 0);

        // Tick 3: cycle completes, delivers only 4 (partial batch).
        let r = t.advance(&mut s, 0);
        assert_eq!(r.items_delivered, 4);
    }

    // -----------------------------------------------------------------------
    // Test 13: ItemTransport — multi-lane belt
    // -----------------------------------------------------------------------
    #[test]
    fn belt_multi_lane() {
        // 3-slot belt, speed=1, 2 lanes.
        let (t, mut s) = make_belt(1.0, 3, 2);

        // Verify state has 6 slots (3 * 2).
        if let TransportState::Item(ref bs) = s {
            assert_eq!(bs.slots.len(), 6);
        }

        // Tick 1: insert items on both lanes.
        let r = t.advance(&mut s, 10);
        assert_eq!(r.items_moved, 2); // One per lane.
        assert_eq!(r.items_delivered, 0);
    }

    // -----------------------------------------------------------------------
    // Test 14: Mismatched variant panics in debug, returns no-op in release
    // -----------------------------------------------------------------------
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Transport variant does not match TransportState variant")]
    fn mismatched_variant_panics() {
        let t = Transport::Flow(FlowTransport {
            rate: Fixed64::from_num(1),
            buffer_capacity: Fixed64::from_num(10),
            latency: 0,
        });
        let mut s = TransportState::Batch(BatchState {
            progress: 0,
            pending: 0,
        });
        t.advance(&mut s, 10);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn mismatched_variant_returns_noop_in_release() {
        let t = Transport::Flow(FlowTransport {
            rate: Fixed64::from_num(1),
            buffer_capacity: Fixed64::from_num(10),
            latency: 0,
        });
        let mut s = TransportState::Batch(BatchState {
            progress: 0,
            pending: 0,
        });
        let result = t.advance(&mut s, 10);
        assert_eq!(result, TransportResult { items_moved: 0, items_delivered: 0 });
    }
}
