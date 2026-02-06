//! Input command queue for externally-submitted engine mutations.
//!
//! Commands are queued by the game client (UI, scripting, network) and
//! executed at tick boundaries to maintain simulation determinism.
//! Each command represents a single atomic operation on the engine.

use crate::id::{BuildingTypeId, EdgeId, NodeId};
use crate::item::Inventory;
use crate::processor::{Modifier, Processor};
use crate::transport::Transport;

// ---------------------------------------------------------------------------
// Command enum
// ---------------------------------------------------------------------------

/// A single command that can be submitted to the engine.
///
/// Commands are queued and executed at the start of the next tick
/// (during the pre-tick phase) to maintain determinism.
#[derive(Debug, Clone)]
pub enum Command {
    /// Add a new node with the given building type.
    AddNode { building_type: BuildingTypeId },
    /// Remove an existing node and all its edges.
    RemoveNode { node: NodeId },
    /// Connect two nodes with a new edge.
    Connect { from: NodeId, to: NodeId },
    /// Disconnect an existing edge.
    Disconnect { edge: EdgeId },
    /// Set the processor for a node.
    SetProcessor { node: NodeId, processor: Processor },
    /// Set the transport for an edge.
    SetTransport { edge: EdgeId, transport: Transport },
    /// Set the input inventory for a node.
    SetInputInventory { node: NodeId, inventory: Inventory },
    /// Set the output inventory for a node.
    SetOutputInventory { node: NodeId, inventory: Inventory },
    /// Set the modifiers for a node.
    SetModifiers {
        node: NodeId,
        modifiers: Vec<Modifier>,
    },
}

// ---------------------------------------------------------------------------
// CommandQueue
// ---------------------------------------------------------------------------

/// A queue of commands waiting to be executed at the next tick boundary.
///
/// Supports optional history tracking for replay and debugging.
pub struct CommandQueue {
    /// Commands waiting to be executed.
    pending: Vec<Command>,
    /// History of executed commands: (tick, command).
    history: Vec<(u64, Command)>,
    /// Maximum history entries to retain. 0 = no history.
    max_history: usize,
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandQueue {
    /// Create a new empty command queue with no history tracking.
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            history: Vec::new(),
            max_history: 0,
        }
    }

    /// Create a new command queue that retains up to `max_history` entries.
    pub fn with_max_history(max_history: usize) -> Self {
        Self {
            pending: Vec::new(),
            history: Vec::new(),
            max_history,
        }
    }

    /// Push a single command onto the queue.
    pub fn push(&mut self, command: Command) {
        self.pending.push(command);
    }

    /// Push multiple commands onto the queue at once.
    pub fn push_batch(&mut self, commands: impl IntoIterator<Item = Command>) {
        self.pending.extend(commands);
    }

    /// Drain all pending commands, moving them to history with the given tick.
    /// Returns the drained commands in submission order.
    pub fn drain(&mut self, tick: u64) -> Vec<Command> {
        let commands: Vec<Command> = self.pending.drain(..).collect();

        if self.max_history > 0 {
            for cmd in &commands {
                self.history.push((tick, cmd.clone()));
            }
            // Trim history if over limit
            let excess = self.history.len().saturating_sub(self.max_history);
            if excess > 0 {
                self.history.drain(..excess);
            }
        }

        commands
    }

    /// Number of commands waiting to be executed.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Whether the queue has no pending commands.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Get the command history (tick, command) pairs.
    pub fn history(&self) -> &[(u64, Command)] {
        &self.history
    }

    /// Clear all history entries.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_node_id() -> NodeId {
        let mut sm = SlotMap::<NodeId, ()>::with_key();
        sm.insert(())
    }

    fn make_edge_id() -> EdgeId {
        let mut sm = SlotMap::<EdgeId, ()>::with_key();
        sm.insert(())
    }

    fn make_add_node_cmd() -> Command {
        Command::AddNode {
            building_type: BuildingTypeId(0),
        }
    }

    fn make_remove_node_cmd() -> Command {
        Command::RemoveNode {
            node: make_node_id(),
        }
    }

    fn make_connect_cmd() -> Command {
        Command::Connect {
            from: make_node_id(),
            to: make_node_id(),
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: new_queue_is_empty
    // -----------------------------------------------------------------------
    #[test]
    fn new_queue_is_empty() {
        let queue = CommandQueue::new();
        assert_eq!(queue.pending_count(), 0);
        assert!(queue.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 2: push_increments_pending
    // -----------------------------------------------------------------------
    #[test]
    fn push_increments_pending() {
        let mut queue = CommandQueue::new();
        queue.push(make_add_node_cmd());
        queue.push(make_remove_node_cmd());
        queue.push(make_connect_cmd());
        assert_eq!(queue.pending_count(), 3);
    }

    // -----------------------------------------------------------------------
    // Test 3: push_batch
    // -----------------------------------------------------------------------
    #[test]
    fn push_batch() {
        let mut queue = CommandQueue::new();
        let commands = vec![
            make_add_node_cmd(),
            make_add_node_cmd(),
            make_remove_node_cmd(),
            make_connect_cmd(),
            Command::Disconnect {
                edge: make_edge_id(),
            },
        ];
        queue.push_batch(commands);
        assert_eq!(queue.pending_count(), 5);
    }

    // -----------------------------------------------------------------------
    // Test 4: drain_returns_all_pending
    // -----------------------------------------------------------------------
    #[test]
    fn drain_returns_all_pending() {
        let mut queue = CommandQueue::new();
        queue.push(make_add_node_cmd());
        queue.push(make_remove_node_cmd());
        queue.push(make_connect_cmd());

        let drained = queue.drain(0);
        assert_eq!(drained.len(), 3);
        assert_eq!(queue.pending_count(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 5: drain_preserves_order
    // -----------------------------------------------------------------------
    #[test]
    fn drain_preserves_order() {
        let mut queue = CommandQueue::new();
        queue.push(Command::AddNode {
            building_type: BuildingTypeId(1),
        });
        queue.push(Command::RemoveNode {
            node: make_node_id(),
        });
        queue.push(Command::Connect {
            from: make_node_id(),
            to: make_node_id(),
        });

        let drained = queue.drain(0);
        assert_eq!(drained.len(), 3);
        assert!(matches!(drained[0], Command::AddNode { .. }));
        assert!(matches!(drained[1], Command::RemoveNode { .. }));
        assert!(matches!(drained[2], Command::Connect { .. }));
    }

    // -----------------------------------------------------------------------
    // Test 6: history_tracking
    // -----------------------------------------------------------------------
    #[test]
    fn history_tracking() {
        let mut queue = CommandQueue::with_max_history(100);
        queue.push(make_add_node_cmd());
        queue.push(make_remove_node_cmd());

        let _drained = queue.drain(42);

        let history = queue.history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].0, 42);
        assert_eq!(history[1].0, 42);
        assert!(matches!(history[0].1, Command::AddNode { .. }));
        assert!(matches!(history[1].1, Command::RemoveNode { .. }));
    }

    // -----------------------------------------------------------------------
    // Test 7: history_trimming
    // -----------------------------------------------------------------------
    #[test]
    fn history_trimming() {
        let mut queue = CommandQueue::with_max_history(3);

        // Push and drain 5 commands across two ticks.
        queue.push(make_add_node_cmd());
        queue.push(make_add_node_cmd());
        queue.push(make_add_node_cmd());
        let _drained = queue.drain(1);

        queue.push(make_remove_node_cmd());
        queue.push(make_connect_cmd());
        let _drained = queue.drain(2);

        // Max history is 3, so oldest entries should be trimmed.
        assert_eq!(queue.history().len(), 3);
    }

    // -----------------------------------------------------------------------
    // Test 8: no_history_by_default
    // -----------------------------------------------------------------------
    #[test]
    fn no_history_by_default() {
        let mut queue = CommandQueue::new();
        queue.push(make_add_node_cmd());
        queue.push(make_remove_node_cmd());
        let _drained = queue.drain(10);

        assert!(queue.history().is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 9: is_empty_after_drain
    // -----------------------------------------------------------------------
    #[test]
    fn is_empty_after_drain() {
        let mut queue = CommandQueue::new();
        queue.push(make_add_node_cmd());
        assert!(!queue.is_empty());

        let _drained = queue.drain(0);
        assert!(queue.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 10: clear_history
    // -----------------------------------------------------------------------
    #[test]
    fn clear_history() {
        let mut queue = CommandQueue::with_max_history(100);
        queue.push(make_add_node_cmd());
        queue.push(make_remove_node_cmd());
        let _drained = queue.drain(5);

        assert!(!queue.history().is_empty());

        queue.clear_history();
        assert!(queue.history().is_empty());
    }
}
