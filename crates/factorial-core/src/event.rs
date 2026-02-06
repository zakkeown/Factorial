//! Typed event system with pre-allocated ring buffers.
//!
//! Events are emitted during simulation phases 2-4 (transport, process,
//! component) and delivered in batch during phase 5 (post-tick). Each event
//! type has its own [`EventBuffer`] ring buffer with a configurable capacity.
//!
//! # Subscriber Types
//!
//! - **Passive listeners**: read-only, used for UI updates, audio, analytics.
//! - **Reactive handlers**: return mutations to enqueue for the next tick.
//!
//! # Suppression
//!
//! Event types can be suppressed via [`EventBus::suppress`], which prevents
//! any allocation or recording for that type. Suppressed events have zero cost.

use crate::fixed::Ticks;
use crate::id::*;
use crate::processor::StallReason;

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

/// A simulation event. All events carry the tick at which they occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    // -- Production --
    ItemProduced {
        node: NodeId,
        item_type: ItemTypeId,
        quantity: u32,
        tick: Ticks,
    },
    ItemConsumed {
        node: NodeId,
        item_type: ItemTypeId,
        quantity: u32,
        tick: Ticks,
    },
    RecipeStarted {
        node: NodeId,
        tick: Ticks,
    },
    RecipeCompleted {
        node: NodeId,
        tick: Ticks,
    },

    // -- Building state --
    BuildingStalled {
        node: NodeId,
        reason: StallReason,
        tick: Ticks,
    },
    BuildingResumed {
        node: NodeId,
        tick: Ticks,
    },

    // -- Transport --
    ItemDelivered {
        edge: EdgeId,
        quantity: u32,
        tick: Ticks,
    },
    TransportFull {
        edge: EdgeId,
        tick: Ticks,
    },

    // -- Graph --
    NodeAdded {
        node: NodeId,
        building_type: BuildingTypeId,
        tick: Ticks,
    },
    NodeRemoved {
        node: NodeId,
        tick: Ticks,
    },
    EdgeAdded {
        edge: EdgeId,
        from: NodeId,
        to: NodeId,
        tick: Ticks,
    },
    EdgeRemoved {
        edge: EdgeId,
        tick: Ticks,
    },
}

/// Discriminant tag for event types, used for suppression and filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    ItemProduced,
    ItemConsumed,
    RecipeStarted,
    RecipeCompleted,
    BuildingStalled,
    BuildingResumed,
    ItemDelivered,
    TransportFull,
    NodeAdded,
    NodeRemoved,
    EdgeAdded,
    EdgeRemoved,
}

/// Total number of event kinds.
const EVENT_KIND_COUNT: usize = 12;

impl Event {
    /// Get the discriminant kind for this event.
    pub fn kind(&self) -> EventKind {
        match self {
            Event::ItemProduced { .. } => EventKind::ItemProduced,
            Event::ItemConsumed { .. } => EventKind::ItemConsumed,
            Event::RecipeStarted { .. } => EventKind::RecipeStarted,
            Event::RecipeCompleted { .. } => EventKind::RecipeCompleted,
            Event::BuildingStalled { .. } => EventKind::BuildingStalled,
            Event::BuildingResumed { .. } => EventKind::BuildingResumed,
            Event::ItemDelivered { .. } => EventKind::ItemDelivered,
            Event::TransportFull { .. } => EventKind::TransportFull,
            Event::NodeAdded { .. } => EventKind::NodeAdded,
            Event::NodeRemoved { .. } => EventKind::NodeRemoved,
            Event::EdgeAdded { .. } => EventKind::EdgeAdded,
            Event::EdgeRemoved { .. } => EventKind::EdgeRemoved,
        }
    }
}

impl EventKind {
    /// Convert to usize index for array lookups.
    fn index(self) -> usize {
        self as usize
    }
}

// ---------------------------------------------------------------------------
// Mutations (returned by reactive handlers)
// ---------------------------------------------------------------------------

/// A mutation that a reactive handler wants to enqueue for the next tick.
/// These are collected during post-tick and applied during the next pre-tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventMutation {
    /// Queue a node to be added.
    AddNode { building_type: BuildingTypeId },
    /// Queue a node for removal.
    RemoveNode { node: NodeId },
    /// Queue an edge connecting two nodes.
    Connect { from: NodeId, to: NodeId },
    /// Queue an edge for removal.
    Disconnect { edge: EdgeId },
}

// ---------------------------------------------------------------------------
// EventBuffer — pre-allocated ring buffer
// ---------------------------------------------------------------------------

/// A pre-allocated ring buffer for events. Fixed capacity; when full, the
/// oldest events are dropped.
#[derive(Debug)]
pub struct EventBuffer {
    /// Pre-allocated storage.
    events: Vec<Option<Event>>,
    /// Write position (wraps around).
    head: usize,
    /// Number of events currently stored (may be less than capacity).
    len: usize,
    /// Total events ever written (including dropped).
    total_written: u64,
}

impl EventBuffer {
    /// Create a new ring buffer with the given capacity.
    /// A capacity of 0 is clamped to 1.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            events: (0..capacity).map(|_| None).collect(),
            head: 0,
            len: 0,
            total_written: 0,
        }
    }

    /// Push an event into the ring buffer. If full, the oldest event is dropped.
    pub fn push(&mut self, event: Event) {
        self.events[self.head] = Some(event);
        self.head = (self.head + 1) % self.capacity();
        if self.len < self.capacity() {
            self.len += 1;
        }
        self.total_written += 1;
    }

    /// The total capacity of the buffer.
    pub fn capacity(&self) -> usize {
        self.events.len()
    }

    /// Number of events currently stored.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Total events written since creation (including dropped).
    pub fn total_written(&self) -> u64 {
        self.total_written
    }

    /// Number of events that were dropped because the buffer was full.
    pub fn dropped_count(&self) -> u64 {
        self.total_written.saturating_sub(self.capacity() as u64)
    }

    /// Iterate over events in order from oldest to newest.
    pub fn iter(&self) -> EventBufferIter<'_> {
        let start = if self.len < self.capacity() {
            0
        } else {
            // head points to the next write position, which is the oldest entry
            self.head
        };
        EventBufferIter {
            buffer: self,
            index: start,
            remaining: self.len,
        }
    }

    /// Clear all events from the buffer.
    pub fn clear(&mut self) {
        for slot in &mut self.events {
            *slot = None;
        }
        self.head = 0;
        self.len = 0;
    }
}

/// Iterator over events in an [`EventBuffer`], from oldest to newest.
pub struct EventBufferIter<'a> {
    buffer: &'a EventBuffer,
    index: usize,
    remaining: usize,
}

impl<'a> Iterator for EventBufferIter<'a> {
    type Item = &'a Event;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let event = self.buffer.events[self.index].as_ref();
        self.index = (self.index + 1) % self.buffer.capacity();
        self.remaining -= 1;
        event
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for EventBufferIter<'_> {}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// A passive listener receives events read-only.
pub type PassiveListener = Box<dyn FnMut(&Event)>;

/// A reactive handler receives an event and returns zero or more mutations
/// to enqueue for the next tick.
pub type ReactiveHandler = Box<dyn FnMut(&Event) -> Vec<EventMutation>>;

/// Subscriber that can be either passive or reactive.
enum Subscriber {
    Passive(PassiveListener),
    Reactive(ReactiveHandler),
}

impl std::fmt::Debug for Subscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Subscriber::Passive(_) => write!(f, "Passive(<fn>)"),
            Subscriber::Reactive(_) => write!(f, "Reactive(<fn>)"),
        }
    }
}

// ---------------------------------------------------------------------------
// Subscriber priorities & filters
// ---------------------------------------------------------------------------

/// Priority level for event subscribers. Lower priorities run first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SubscriberPriority {
    Pre = 0,
    Normal = 1,
    Post = 2,
}

/// Optional predicate that filters events for a subscriber.
pub type EventFilter = Box<dyn Fn(&Event) -> bool>;

/// Wraps a [`Subscriber`] with priority, optional filter, and insertion order.
struct SubscriberEntry {
    subscriber: Subscriber,
    priority: SubscriberPriority,
    filter: Option<EventFilter>,
    insertion_order: u64,
}

impl std::fmt::Debug for SubscriberEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubscriberEntry")
            .field("subscriber", &self.subscriber)
            .field("priority", &self.priority)
            .field(
                "filter",
                &if self.filter.is_some() {
                    "Some(<fn>)"
                } else {
                    "None"
                },
            )
            .field("insertion_order", &self.insertion_order)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// EventBus
// ---------------------------------------------------------------------------

/// The central event bus. Holds one ring buffer per event kind, subscriber
/// lists, and suppression flags.
pub struct EventBus {
    /// One ring buffer per event kind.
    buffers: [Option<EventBuffer>; EVENT_KIND_COUNT],

    /// Suppressed event kinds. Suppressed events are never buffered.
    suppressed: [bool; EVENT_KIND_COUNT],

    /// Subscribers indexed by event kind.
    subscribers: [Vec<SubscriberEntry>; EVENT_KIND_COUNT],

    /// Mutations collected from reactive handlers during delivery.
    /// Drained by the engine after post-tick to apply during next pre-tick.
    pending_mutations: Vec<EventMutation>,

    /// Default buffer capacity for new event buffers.
    default_capacity: usize,

    /// Monotonically increasing counter for stable sort ordering.
    next_insertion_order: u64,
}

impl std::fmt::Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus")
            .field("buffers", &self.buffers)
            .field("suppressed", &self.suppressed)
            .field("pending_mutations", &self.pending_mutations)
            .field("default_capacity", &self.default_capacity)
            .finish_non_exhaustive()
    }
}

const fn empty_subscriber_array() -> [Vec<SubscriberEntry>; EVENT_KIND_COUNT] {
    // Cannot use Default in const context, so we build it manually.
    [
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ]
}

impl EventBus {
    /// Create a new event bus with the given default buffer capacity per type.
    pub fn new(default_capacity: usize) -> Self {
        Self {
            buffers: Default::default(),
            suppressed: [false; EVENT_KIND_COUNT],
            subscribers: empty_subscriber_array(),
            pending_mutations: Vec::new(),
            default_capacity,
            next_insertion_order: 0,
        }
    }

    /// Suppress an event kind. Suppressed events are never allocated or buffered.
    pub fn suppress(&mut self, kind: EventKind) {
        self.suppressed[kind.index()] = true;
        // Drop the buffer if it exists -- zero allocation for suppressed events.
        self.buffers[kind.index()] = None;
    }

    /// Check if an event kind is suppressed.
    pub fn is_suppressed(&self, kind: EventKind) -> bool {
        self.suppressed[kind.index()]
    }

    /// Emit an event. Stores it in the appropriate ring buffer. No-ops if
    /// the event kind is suppressed.
    pub fn emit(&mut self, event: Event) {
        let kind = event.kind();
        let idx = kind.index();

        if self.suppressed[idx] {
            return;
        }

        // Lazily allocate buffer on first emit.
        if self.buffers[idx].is_none() {
            self.buffers[idx] = Some(EventBuffer::new(self.default_capacity));
        }

        self.buffers[idx].as_mut().unwrap().push(event);
    }

    /// Register a passive listener for an event kind. Listeners are called
    /// in registration order during delivery with Normal priority and no filter.
    pub fn on_passive(&mut self, kind: EventKind, listener: PassiveListener) {
        self.on_passive_filtered(kind, SubscriberPriority::Normal, None, listener);
    }

    /// Register a reactive handler for an event kind. Handlers are called
    /// in registration order during delivery with Normal priority and no filter.
    pub fn on_reactive(&mut self, kind: EventKind, handler: ReactiveHandler) {
        self.on_reactive_filtered(kind, SubscriberPriority::Normal, None, handler);
    }

    /// Register a passive listener with explicit priority and optional filter.
    pub fn on_passive_filtered(
        &mut self,
        kind: EventKind,
        priority: SubscriberPriority,
        filter: Option<EventFilter>,
        listener: PassiveListener,
    ) {
        let order = self.next_insertion_order;
        self.next_insertion_order += 1;
        self.subscribers[kind.index()].push(SubscriberEntry {
            subscriber: Subscriber::Passive(listener),
            priority,
            filter,
            insertion_order: order,
        });
    }

    /// Register a reactive handler with explicit priority and optional filter.
    pub fn on_reactive_filtered(
        &mut self,
        kind: EventKind,
        priority: SubscriberPriority,
        filter: Option<EventFilter>,
        handler: ReactiveHandler,
    ) {
        let order = self.next_insertion_order;
        self.next_insertion_order += 1;
        self.subscribers[kind.index()].push(SubscriberEntry {
            subscriber: Subscriber::Reactive(handler),
            priority,
            filter,
            insertion_order: order,
        });
    }

    /// Deliver all buffered events to subscribers. Called during post-tick.
    ///
    /// For each event kind that has buffered events:
    /// 1. Sort subscribers by `(priority, insertion_order)`.
    /// 2. Iterate events oldest-to-newest.
    /// 3. For each subscriber, check the optional filter; skip if it returns false.
    /// 4. Call passive listeners / reactive handlers; collect mutations.
    /// 5. Clear the buffer after delivery.
    ///
    /// Reactive handler mutations accumulate in `pending_mutations`.
    pub fn deliver(&mut self) {
        for idx in 0..EVENT_KIND_COUNT {
            if self.suppressed[idx] {
                continue;
            }

            let Some(buffer) = self.buffers[idx].as_ref() else {
                continue;
            };

            if buffer.is_empty() {
                continue;
            }

            // Collect events into a temporary Vec to avoid borrow conflicts
            // between the buffer and subscribers.
            let events: Vec<Event> = buffer.iter().cloned().collect();

            // Sort subscribers by (priority, insertion_order) for stable ordering.
            self.subscribers[idx]
                .sort_by_key(|entry| (entry.priority as u8, entry.insertion_order));

            // Deliver to each subscriber in priority order.
            for entry in &mut self.subscribers[idx] {
                for event in &events {
                    // Check optional filter — skip if it returns false.
                    if let Some(ref filter) = entry.filter
                        && !filter(event)
                    {
                        continue;
                    }

                    match &mut entry.subscriber {
                        Subscriber::Passive(listener) => {
                            listener(event);
                        }
                        Subscriber::Reactive(handler) => {
                            let mutations = handler(event);
                            self.pending_mutations.extend(mutations);
                        }
                    }
                }
            }

            // Clear the buffer after delivery.
            if let Some(buffer) = self.buffers[idx].as_mut() {
                buffer.clear();
            }
        }
    }

    /// Drain pending mutations (collected from reactive handlers).
    /// Returns all mutations and clears the internal list.
    pub fn drain_mutations(&mut self) -> Vec<EventMutation> {
        std::mem::take(&mut self.pending_mutations)
    }

    /// Get the event buffer for a specific event kind (read-only).
    pub fn buffer(&self, kind: EventKind) -> Option<&EventBuffer> {
        self.buffers[kind.index()].as_ref()
    }

    /// Get the count of events currently buffered for a kind.
    pub fn buffered_count(&self, kind: EventKind) -> usize {
        self.buffers[kind.index()]
            .as_ref()
            .map(|b| b.len())
            .unwrap_or(0)
    }

    /// Get the total events ever emitted for a kind (including dropped).
    pub fn total_emitted(&self, kind: EventKind) -> u64 {
        self.buffers[kind.index()]
            .as_ref()
            .map(|b| b.total_written())
            .unwrap_or(0)
    }

    /// Clear all buffers. Does not remove subscribers or suppression settings.
    pub fn clear_all(&mut self) {
        for buffer in &mut self.buffers {
            if let Some(b) = buffer.as_mut() {
                b.clear();
            }
        }
        self.pending_mutations.clear();
    }

    /// Get a count of pending mutations.
    pub fn pending_mutation_count(&self) -> usize {
        self.pending_mutations.len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

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

    // -----------------------------------------------------------------------
    // Test 1: EventBuffer basic push and iterate
    // -----------------------------------------------------------------------
    #[test]
    fn event_buffer_push_and_iterate() {
        let mut buf = EventBuffer::new(8);
        let node = make_node_id();

        buf.push(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 5,
            tick: 1,
        });
        buf.push(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 3,
            tick: 2,
        });

        assert_eq!(buf.len(), 2);
        assert_eq!(buf.total_written(), 2);
        assert_eq!(buf.dropped_count(), 0);

        let events: Vec<&Event> = buf.iter().collect();
        assert_eq!(events.len(), 2);

        // Oldest first.
        assert_eq!(
            events[0],
            &Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: 5,
                tick: 1,
            }
        );
        assert_eq!(
            events[1],
            &Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: 3,
                tick: 2,
            }
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Ring buffer wraps correctly and drops oldest
    // -----------------------------------------------------------------------
    #[test]
    fn event_buffer_ring_wraps_and_drops_oldest() {
        let mut buf = EventBuffer::new(3);
        let node = make_node_id();

        // Push 5 events into a buffer of capacity 3.
        for i in 0..5u64 {
            buf.push(Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: i as u32,
                tick: i,
            });
        }

        assert_eq!(buf.len(), 3);
        assert_eq!(buf.total_written(), 5);
        assert_eq!(buf.dropped_count(), 2);

        // Should contain events 2, 3, 4 (oldest-to-newest).
        let events: Vec<&Event> = buf.iter().collect();
        assert_eq!(events.len(), 3);

        for (i, event) in events.iter().enumerate() {
            match event {
                Event::ItemProduced { quantity, tick, .. } => {
                    assert_eq!(*quantity, (i + 2) as u32);
                    assert_eq!(*tick, (i + 2) as u64);
                }
                _ => panic!("expected ItemProduced"),
            }
        }
    }

    // -----------------------------------------------------------------------
    // Test 3: EventBuffer clear
    // -----------------------------------------------------------------------
    #[test]
    fn event_buffer_clear() {
        let mut buf = EventBuffer::new(4);
        let node = make_node_id();

        buf.push(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 0,
        });
        assert_eq!(buf.len(), 1);

        buf.clear();
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
        // total_written is NOT reset by clear (it's a lifetime counter).
        assert_eq!(buf.total_written(), 1);
    }

    // -----------------------------------------------------------------------
    // Test 4: EventBus emit and buffered_count
    // -----------------------------------------------------------------------
    #[test]
    fn event_bus_emit_and_count() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 5,
            tick: 1,
        });
        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 3,
            tick: 2,
        });
        bus.emit(Event::BuildingStalled {
            node,
            reason: StallReason::OutputFull,
            tick: 1,
        });

        assert_eq!(bus.buffered_count(EventKind::ItemProduced), 2);
        assert_eq!(bus.buffered_count(EventKind::BuildingStalled), 1);
        assert_eq!(bus.buffered_count(EventKind::RecipeStarted), 0);
    }

    // -----------------------------------------------------------------------
    // Test 5: Suppressed events have zero allocation cost
    // -----------------------------------------------------------------------
    #[test]
    fn suppressed_events_zero_allocation() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();

        bus.suppress(EventKind::ItemProduced);

        // Emit some ItemProduced events -- they should be silently dropped.
        for i in 0..10 {
            bus.emit(Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: i,
                tick: i as u64,
            });
        }

        assert!(bus.is_suppressed(EventKind::ItemProduced));
        assert_eq!(bus.buffered_count(EventKind::ItemProduced), 0);
        assert_eq!(bus.total_emitted(EventKind::ItemProduced), 0);

        // Buffer should not exist at all.
        assert!(bus.buffer(EventKind::ItemProduced).is_none());
    }

    // -----------------------------------------------------------------------
    // Test 6: Passive listeners receive events in registration order
    // -----------------------------------------------------------------------
    #[test]
    fn passive_listeners_registration_order() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();

        let order = Rc::new(RefCell::new(Vec::new()));

        // Register listener A.
        let order_a = order.clone();
        bus.on_passive(
            EventKind::ItemProduced,
            Box::new(move |_event| {
                order_a.borrow_mut().push('A');
            }),
        );

        // Register listener B.
        let order_b = order.clone();
        bus.on_passive(
            EventKind::ItemProduced,
            Box::new(move |_event| {
                order_b.borrow_mut().push('B');
            }),
        );

        // Register listener C.
        let order_c = order.clone();
        bus.on_passive(
            EventKind::ItemProduced,
            Box::new(move |_event| {
                order_c.borrow_mut().push('C');
            }),
        );

        // Emit one event.
        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 1,
        });

        bus.deliver();

        // All three listeners should have been called in order: A, B, C.
        assert_eq!(*order.borrow(), vec!['A', 'B', 'C']);
    }

    // -----------------------------------------------------------------------
    // Test 7: Reactive handlers enqueue mutations
    // -----------------------------------------------------------------------
    #[test]
    fn reactive_handlers_enqueue_mutations() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();

        bus.on_reactive(
            EventKind::RecipeCompleted,
            Box::new(|event| {
                if let Event::RecipeCompleted { node, .. } = event {
                    vec![EventMutation::RemoveNode { node: *node }]
                } else {
                    vec![]
                }
            }),
        );

        bus.emit(Event::RecipeCompleted { node, tick: 5 });
        bus.deliver();

        let mutations = bus.drain_mutations();
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0], EventMutation::RemoveNode { node });
    }

    // -----------------------------------------------------------------------
    // Test 8: Delivery clears buffers
    // -----------------------------------------------------------------------
    #[test]
    fn delivery_clears_buffers() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 1,
        });
        assert_eq!(bus.buffered_count(EventKind::ItemProduced), 1);

        bus.deliver();
        assert_eq!(bus.buffered_count(EventKind::ItemProduced), 0);
    }

    // -----------------------------------------------------------------------
    // Test 9: EventKind discriminant covers all variants
    // -----------------------------------------------------------------------
    #[test]
    fn event_kind_discriminant() {
        let node = make_node_id();
        let edge = make_edge_id();

        let events = vec![
            Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: 1,
                tick: 0,
            },
            Event::ItemConsumed {
                node,
                item_type: iron(),
                quantity: 1,
                tick: 0,
            },
            Event::RecipeStarted { node, tick: 0 },
            Event::RecipeCompleted { node, tick: 0 },
            Event::BuildingStalled {
                node,
                reason: StallReason::MissingInputs,
                tick: 0,
            },
            Event::BuildingResumed { node, tick: 0 },
            Event::ItemDelivered {
                edge,
                quantity: 1,
                tick: 0,
            },
            Event::TransportFull { edge, tick: 0 },
            Event::NodeAdded {
                node,
                building_type: BuildingTypeId(0),
                tick: 0,
            },
            Event::NodeRemoved { node, tick: 0 },
            Event::EdgeAdded {
                edge,
                from: node,
                to: node,
                tick: 0,
            },
            Event::EdgeRemoved { edge, tick: 0 },
        ];

        let kinds: Vec<EventKind> = events.iter().map(|e| e.kind()).collect();
        assert_eq!(
            kinds,
            vec![
                EventKind::ItemProduced,
                EventKind::ItemConsumed,
                EventKind::RecipeStarted,
                EventKind::RecipeCompleted,
                EventKind::BuildingStalled,
                EventKind::BuildingResumed,
                EventKind::ItemDelivered,
                EventKind::TransportFull,
                EventKind::NodeAdded,
                EventKind::NodeRemoved,
                EventKind::EdgeAdded,
                EventKind::EdgeRemoved,
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Test 10: Multiple event types don't interfere
    // -----------------------------------------------------------------------
    #[test]
    fn multiple_event_types_independent() {
        let mut bus = EventBus::new(4);
        let node = make_node_id();

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 1,
        });
        bus.emit(Event::RecipeStarted { node, tick: 1 });
        bus.emit(Event::RecipeStarted { node, tick: 2 });

        assert_eq!(bus.buffered_count(EventKind::ItemProduced), 1);
        assert_eq!(bus.buffered_count(EventKind::RecipeStarted), 2);
    }

    // -----------------------------------------------------------------------
    // Test 11: Passive listener receives correct event data
    // -----------------------------------------------------------------------
    #[test]
    fn passive_listener_receives_correct_data() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();

        let received = Rc::new(RefCell::new(Vec::new()));
        let received_clone = received.clone();

        bus.on_passive(
            EventKind::ItemProduced,
            Box::new(move |event| {
                if let Event::ItemProduced { quantity, tick, .. } = event {
                    received_clone.borrow_mut().push((*quantity, *tick));
                }
            }),
        );

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 5,
            tick: 10,
        });
        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 3,
            tick: 11,
        });

        bus.deliver();

        let data = received.borrow();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0], (5, 10));
        assert_eq!(data[1], (3, 11));
    }

    // -----------------------------------------------------------------------
    // Test 12: Reactive handler with multiple events produces multiple mutations
    // -----------------------------------------------------------------------
    #[test]
    fn reactive_handler_multiple_events_multiple_mutations() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();

        bus.on_reactive(
            EventKind::BuildingStalled,
            Box::new(|event| {
                if let Event::BuildingStalled { node, .. } = event {
                    vec![EventMutation::RemoveNode { node: *node }]
                } else {
                    vec![]
                }
            }),
        );

        // Emit 3 stall events.
        for tick in 0..3 {
            bus.emit(Event::BuildingStalled {
                node,
                reason: StallReason::NoPower,
                tick,
            });
        }

        bus.deliver();

        let mutations = bus.drain_mutations();
        assert_eq!(mutations.len(), 3);
        for m in &mutations {
            assert_eq!(*m, EventMutation::RemoveNode { node });
        }
    }

    // -----------------------------------------------------------------------
    // Test 13: Mixed passive and reactive subscribers
    // -----------------------------------------------------------------------
    #[test]
    fn mixed_passive_and_reactive_subscribers() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();

        let passive_count = Rc::new(RefCell::new(0u32));
        let pc = passive_count.clone();

        bus.on_passive(
            EventKind::ItemConsumed,
            Box::new(move |_| {
                *pc.borrow_mut() += 1;
            }),
        );

        bus.on_reactive(
            EventKind::ItemConsumed,
            Box::new(|_event| {
                vec![EventMutation::AddNode {
                    building_type: BuildingTypeId(99),
                }]
            }),
        );

        bus.emit(Event::ItemConsumed {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 1,
        });

        bus.deliver();

        assert_eq!(*passive_count.borrow(), 1);
        let mutations = bus.drain_mutations();
        assert_eq!(mutations.len(), 1);
        assert_eq!(
            mutations[0],
            EventMutation::AddNode {
                building_type: BuildingTypeId(99),
            }
        );
    }

    // -----------------------------------------------------------------------
    // Test 14: drain_mutations clears the list
    // -----------------------------------------------------------------------
    #[test]
    fn drain_mutations_clears() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();

        bus.on_reactive(
            EventKind::RecipeCompleted,
            Box::new(|_| {
                vec![EventMutation::AddNode {
                    building_type: BuildingTypeId(1),
                }]
            }),
        );

        bus.emit(Event::RecipeCompleted { node, tick: 1 });
        bus.deliver();

        let first = bus.drain_mutations();
        assert_eq!(first.len(), 1);

        let second = bus.drain_mutations();
        assert!(second.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 15: ExactSizeIterator for EventBuffer
    // -----------------------------------------------------------------------
    #[test]
    fn event_buffer_exact_size_iterator() {
        let mut buf = EventBuffer::new(8);
        let node = make_node_id();

        for i in 0..5 {
            buf.push(Event::ItemProduced {
                node,
                item_type: iron(),
                quantity: i,
                tick: i as u64,
            });
        }

        let iter = buf.iter();
        assert_eq!(iter.len(), 5);
    }

    // -----------------------------------------------------------------------
    // Test 16: Suppression after events already buffered
    // -----------------------------------------------------------------------
    #[test]
    fn suppress_after_buffering_drops_buffer() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 1,
        });
        assert_eq!(bus.buffered_count(EventKind::ItemProduced), 1);

        bus.suppress(EventKind::ItemProduced);

        // Buffer should be dropped.
        assert!(bus.buffer(EventKind::ItemProduced).is_none());
        assert_eq!(bus.buffered_count(EventKind::ItemProduced), 0);
    }

    // -----------------------------------------------------------------------
    // Test 17: Ring buffer capacity of 1
    // -----------------------------------------------------------------------
    #[test]
    fn event_buffer_capacity_one() {
        let mut buf = EventBuffer::new(1);
        let node = make_node_id();

        buf.push(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 1,
        });
        buf.push(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 2,
            tick: 2,
        });

        assert_eq!(buf.len(), 1);
        assert_eq!(buf.total_written(), 2);
        assert_eq!(buf.dropped_count(), 1);

        let events: Vec<&Event> = buf.iter().collect();
        assert_eq!(events.len(), 1);
        match events[0] {
            Event::ItemProduced { quantity, .. } => assert_eq!(*quantity, 2),
            _ => panic!("expected ItemProduced"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 18: clear_all on EventBus
    // -----------------------------------------------------------------------
    #[test]
    fn event_bus_clear_all() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 1,
        });
        bus.emit(Event::RecipeStarted { node, tick: 1 });

        assert_eq!(bus.buffered_count(EventKind::ItemProduced), 1);
        assert_eq!(bus.buffered_count(EventKind::RecipeStarted), 1);

        bus.clear_all();

        assert_eq!(bus.buffered_count(EventKind::ItemProduced), 0);
        assert_eq!(bus.buffered_count(EventKind::RecipeStarted), 0);
    }

    // -----------------------------------------------------------------------
    // Test 19: Priority Pre runs before Normal
    // -----------------------------------------------------------------------
    #[test]
    fn priority_pre_runs_before_normal() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();
        let order = Rc::new(RefCell::new(Vec::new()));

        let o1 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Normal,
            None,
            Box::new(move |_| {
                o1.borrow_mut().push("normal");
            }),
        );

        let o2 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Pre,
            None,
            Box::new(move |_| {
                o2.borrow_mut().push("pre");
            }),
        );

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 0,
        });
        bus.deliver();

        assert_eq!(*order.borrow(), vec!["pre", "normal"]);
    }

    // -----------------------------------------------------------------------
    // Test 20: Priority Post runs after Normal
    // -----------------------------------------------------------------------
    #[test]
    fn priority_post_runs_after_normal() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();
        let order = Rc::new(RefCell::new(Vec::new()));

        let o1 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Post,
            None,
            Box::new(move |_| {
                o1.borrow_mut().push("post");
            }),
        );

        let o2 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Normal,
            None,
            Box::new(move |_| {
                o2.borrow_mut().push("normal");
            }),
        );

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 0,
        });
        bus.deliver();

        assert_eq!(*order.borrow(), vec!["normal", "post"]);
    }

    // -----------------------------------------------------------------------
    // Test 21: All three priorities ordered
    // -----------------------------------------------------------------------
    #[test]
    fn priority_all_three_ordered() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();
        let order = Rc::new(RefCell::new(Vec::new()));

        let o1 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Post,
            None,
            Box::new(move |_| {
                o1.borrow_mut().push("post");
            }),
        );
        let o2 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Pre,
            None,
            Box::new(move |_| {
                o2.borrow_mut().push("pre");
            }),
        );
        let o3 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Normal,
            None,
            Box::new(move |_| {
                o3.borrow_mut().push("normal");
            }),
        );

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 0,
        });
        bus.deliver();

        assert_eq!(*order.borrow(), vec!["pre", "normal", "post"]);
    }

    // -----------------------------------------------------------------------
    // Test 22: Filter passes matching events
    // -----------------------------------------------------------------------
    #[test]
    fn filter_passes_matching() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();
        let count = Rc::new(RefCell::new(0u32));

        let cc = count.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Normal,
            Some(Box::new(
                |e| matches!(e, Event::ItemProduced { quantity, .. } if *quantity > 5),
            )),
            Box::new(move |_| {
                *cc.borrow_mut() += 1;
            }),
        );

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 3,
            tick: 0,
        });
        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 10,
            tick: 1,
        });
        bus.deliver();

        assert_eq!(*count.borrow(), 1);
    }

    // -----------------------------------------------------------------------
    // Test 23: Filter blocks non-matching events
    // -----------------------------------------------------------------------
    #[test]
    fn filter_blocks_non_matching() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();
        let count = Rc::new(RefCell::new(0u32));

        let cc = count.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Normal,
            Some(Box::new(|_| false)),
            Box::new(move |_| {
                *cc.borrow_mut() += 1;
            }),
        );

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 0,
        });
        bus.deliver();

        assert_eq!(*count.borrow(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 24: No filter receives all events
    // -----------------------------------------------------------------------
    #[test]
    fn filter_none_receives_all() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();
        let count = Rc::new(RefCell::new(0u32));

        let cc = count.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Normal,
            None,
            Box::new(move |_| {
                *cc.borrow_mut() += 1;
            }),
        );

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 0,
        });
        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 2,
            tick: 1,
        });
        bus.deliver();

        assert_eq!(*count.borrow(), 2);
    }

    // -----------------------------------------------------------------------
    // Test 25: Existing on_passive unchanged behavior
    // -----------------------------------------------------------------------
    #[test]
    fn existing_on_passive_unchanged() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();
        let count = Rc::new(RefCell::new(0u32));

        let cc = count.clone();
        bus.on_passive(
            EventKind::ItemProduced,
            Box::new(move |_| {
                *cc.borrow_mut() += 1;
            }),
        );

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 0,
        });
        bus.deliver();

        assert_eq!(*count.borrow(), 1);
    }

    // -----------------------------------------------------------------------
    // Test 26: Existing on_reactive unchanged behavior
    // -----------------------------------------------------------------------
    #[test]
    fn existing_on_reactive_unchanged() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();

        bus.on_reactive(
            EventKind::RecipeCompleted,
            Box::new(|event| {
                if let Event::RecipeCompleted { node, .. } = event {
                    vec![EventMutation::RemoveNode { node: *node }]
                } else {
                    vec![]
                }
            }),
        );

        bus.emit(Event::RecipeCompleted { node, tick: 5 });
        bus.deliver();

        let mutations = bus.drain_mutations();
        assert_eq!(mutations.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Test 27: Mixed priorities and filters
    // -----------------------------------------------------------------------
    #[test]
    fn mixed_priorities_and_filters() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();
        let order = Rc::new(RefCell::new(Vec::new()));

        // Post priority, no filter
        let o1 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Post,
            None,
            Box::new(move |_| {
                o1.borrow_mut().push("post");
            }),
        );

        // Pre priority, filter passes
        let o2 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Pre,
            Some(Box::new(|_| true)),
            Box::new(move |_| {
                o2.borrow_mut().push("pre-pass");
            }),
        );

        // Pre priority, filter blocks
        let o3 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Pre,
            Some(Box::new(|_| false)),
            Box::new(move |_| {
                o3.borrow_mut().push("pre-block");
            }),
        );

        // Normal, no filter
        let o4 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Normal,
            None,
            Box::new(move |_| {
                o4.borrow_mut().push("normal");
            }),
        );

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 0,
        });
        bus.deliver();

        assert_eq!(*order.borrow(), vec!["pre-pass", "normal", "post"]);
    }

    // -----------------------------------------------------------------------
    // Test 28: Same priority preserves registration order
    // -----------------------------------------------------------------------
    #[test]
    fn same_priority_preserves_registration_order() {
        let mut bus = EventBus::new(16);
        let node = make_node_id();
        let order = Rc::new(RefCell::new(Vec::new()));

        let o1 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Normal,
            None,
            Box::new(move |_| {
                o1.borrow_mut().push('A');
            }),
        );
        let o2 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Normal,
            None,
            Box::new(move |_| {
                o2.borrow_mut().push('B');
            }),
        );
        let o3 = order.clone();
        bus.on_passive_filtered(
            EventKind::ItemProduced,
            SubscriberPriority::Normal,
            None,
            Box::new(move |_| {
                o3.borrow_mut().push('C');
            }),
        );

        bus.emit(Event::ItemProduced {
            node,
            item_type: iron(),
            quantity: 1,
            tick: 0,
        });
        bus.deliver();

        assert_eq!(*order.borrow(), vec!['A', 'B', 'C']);
    }

    // -----------------------------------------------------------------------
    // Test 29: Zero capacity is clamped to 1
    // -----------------------------------------------------------------------
    #[test]
    fn event_buffer_zero_capacity_clamped() {
        let buf = EventBuffer::new(0);
        assert_eq!(buf.capacity(), 1);
    }
}
