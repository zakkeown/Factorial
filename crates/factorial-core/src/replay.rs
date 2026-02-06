//! Replay recording and playback for debugging and multiplayer verification.
//!
//! Records a sequence of commands applied to an engine, starting from a
//! serialized snapshot. The replay can be played back to reproduce the exact
//! same simulation state, with optional hash verification at checkpoints.

use crate::engine::Engine;
use crate::id::{BuildingTypeId, EdgeId, NodeId};
use crate::item::Inventory;
use crate::processor::{Modifier, Processor};
use crate::serialize::{DeserializeError, SerializeError};
use crate::transport::Transport;

// ---------------------------------------------------------------------------
// ReplayCommand
// ---------------------------------------------------------------------------

/// A command that can be recorded and replayed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ReplayCommand {
    Advance {
        dt: u64,
    },
    Step,
    QueueAddNode {
        building_type: BuildingTypeId,
    },
    QueueRemoveNode {
        node: NodeId,
    },
    QueueConnect {
        from: NodeId,
        to: NodeId,
    },
    QueueDisconnect {
        edge: EdgeId,
    },
    SetProcessor {
        node: NodeId,
        processor: Processor,
    },
    SetInputInventory {
        node: NodeId,
        inventory: Inventory,
    },
    SetOutputInventory {
        node: NodeId,
        inventory: Inventory,
    },
    SetModifiers {
        node: NodeId,
        modifiers: Vec<Modifier>,
    },
    SetTransport {
        edge: EdgeId,
        transport: Transport,
    },
    ApplyMutations,
}

// ---------------------------------------------------------------------------
// ReplayMismatch
// ---------------------------------------------------------------------------

/// Details about where replay verification failed.
#[derive(Debug, Clone)]
pub struct ReplayMismatch {
    /// The command index where the mismatch was detected.
    pub command_index: usize,
    /// Expected hash from the recording.
    pub expected_hash: u64,
    /// Actual hash from the replay.
    pub actual_hash: u64,
}

// ---------------------------------------------------------------------------
// ReplayLog
// ---------------------------------------------------------------------------

/// A recorded sequence of commands starting from a snapshot.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReplayLog {
    /// Serialized engine state at the start of recording.
    pub initial_snapshot: Vec<u8>,
    /// Recorded commands in order.
    pub commands: Vec<ReplayCommand>,
    /// Hash checkpoints: (command_index, state_hash).
    /// Used for verification during playback.
    pub hash_checkpoints: Vec<(usize, u64)>,
}

impl ReplayLog {
    /// Create a new replay log, capturing the current engine state as the initial snapshot.
    pub fn new(engine: &Engine) -> Result<Self, SerializeError> {
        let initial_snapshot = engine.serialize()?;
        Ok(Self {
            initial_snapshot,
            commands: Vec::new(),
            hash_checkpoints: Vec::new(),
        })
    }

    /// Record a command.
    pub fn record(&mut self, cmd: ReplayCommand) {
        self.commands.push(cmd);
    }

    /// Record a command with a hash checkpoint.
    pub fn record_with_hash(&mut self, cmd: ReplayCommand, hash: u64) {
        let index = self.commands.len();
        self.commands.push(cmd);
        self.hash_checkpoints.push((index, hash));
    }

    /// Number of recorded commands.
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Serialize the replay log to bytes (using bitcode).
    pub fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        bitcode::serialize(self).map_err(|e| SerializeError::Encode(e.to_string()))
    }

    /// Deserialize a replay log from bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, DeserializeError> {
        bitcode::deserialize(data).map_err(|e| DeserializeError::Decode(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// ReplayResult
// ---------------------------------------------------------------------------

/// The result of replaying a log.
#[derive(Debug)]
pub struct ReplayResult {
    /// Number of commands executed.
    pub commands_executed: usize,
    /// Whether all hash checkpoints matched.
    pub is_verified: bool,
    /// First mismatch encountered (if any).
    pub first_mismatch: Option<ReplayMismatch>,
}

// ---------------------------------------------------------------------------
// Replay execution
// ---------------------------------------------------------------------------

/// Apply a single command to an engine.
fn apply_command(engine: &mut Engine, cmd: &ReplayCommand) {
    match cmd {
        ReplayCommand::Advance { dt } => {
            engine.advance(*dt);
        }
        ReplayCommand::Step => {
            engine.step();
        }
        ReplayCommand::QueueAddNode { building_type } => {
            engine.graph.queue_add_node(*building_type);
        }
        ReplayCommand::QueueRemoveNode { node } => {
            engine.graph.queue_remove_node(*node);
        }
        ReplayCommand::QueueConnect { from, to } => {
            engine.graph.queue_connect(*from, *to);
        }
        ReplayCommand::QueueDisconnect { edge } => {
            engine.graph.queue_disconnect(*edge);
        }
        ReplayCommand::SetProcessor { node, processor } => {
            engine.set_processor(*node, processor.clone());
        }
        ReplayCommand::SetInputInventory { node, inventory } => {
            engine.set_input_inventory(*node, inventory.clone());
        }
        ReplayCommand::SetOutputInventory { node, inventory } => {
            engine.set_output_inventory(*node, inventory.clone());
        }
        ReplayCommand::SetModifiers { node, modifiers } => {
            engine.set_modifiers(*node, modifiers.clone());
        }
        ReplayCommand::SetTransport { edge, transport } => {
            engine.set_transport(*edge, transport.clone());
        }
        ReplayCommand::ApplyMutations => {
            engine.graph.apply_mutations();
        }
    }
}

/// Replay a log and verify hash checkpoints.
pub fn replay_and_verify(log: &ReplayLog) -> Result<ReplayResult, DeserializeError> {
    let mut engine = Engine::deserialize(&log.initial_snapshot)?;

    let mut first_mismatch: Option<ReplayMismatch> = None;
    let mut checkpoint_idx = 0;

    for (i, cmd) in log.commands.iter().enumerate() {
        apply_command(&mut engine, cmd);

        // Check if this command index has a hash checkpoint.
        while checkpoint_idx < log.hash_checkpoints.len()
            && log.hash_checkpoints[checkpoint_idx].0 == i
        {
            let (_, expected_hash) = log.hash_checkpoints[checkpoint_idx];
            let actual_hash = engine.state_hash();
            if actual_hash != expected_hash && first_mismatch.is_none() {
                first_mismatch = Some(ReplayMismatch {
                    command_index: i,
                    expected_hash,
                    actual_hash,
                });
            }
            checkpoint_idx += 1;
        }
    }

    Ok(ReplayResult {
        commands_executed: log.commands.len(),
        is_verified: first_mismatch.is_none(),
        first_mismatch,
    })
}

/// Replay a log without verification, returning the final engine state.
pub fn replay(log: &ReplayLog) -> Result<Engine, DeserializeError> {
    let mut engine = Engine::deserialize(&log.initial_snapshot)?;
    for cmd in &log.commands {
        apply_command(&mut engine, cmd);
    }
    Ok(engine)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::SimulationStrategy;
    use crate::test_utils::*;

    fn make_test_engine() -> Engine {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        let src = add_node(&mut engine, make_source(iron(), 3.0), 100, 100);
        let consumer = add_node(
            &mut engine,
            make_recipe(vec![(iron(), 2)], vec![(gear(), 1)], 5),
            100,
            100,
        );
        connect(&mut engine, src, consumer, make_flow_transport(5.0));
        engine
    }

    // -----------------------------------------------------------------------
    // Test 1: Replay log captures initial state
    // -----------------------------------------------------------------------
    #[test]
    fn replay_log_captures_initial_state() {
        let engine = make_test_engine();
        let log = ReplayLog::new(&engine).unwrap();
        assert!(!log.initial_snapshot.is_empty());
        assert_eq!(log.commands.len(), 0);
        assert_eq!(log.hash_checkpoints.len(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 2: Record commands
    // -----------------------------------------------------------------------
    #[test]
    fn replay_log_record_commands() {
        let engine = make_test_engine();
        let mut log = ReplayLog::new(&engine).unwrap();
        log.record(ReplayCommand::Step);
        log.record(ReplayCommand::Step);
        log.record(ReplayCommand::Step);
        assert_eq!(log.command_count(), 3);
    }

    // -----------------------------------------------------------------------
    // Test 3: Empty log returns initial state
    // -----------------------------------------------------------------------
    #[test]
    fn replay_empty_log_returns_initial_state() {
        let engine = make_test_engine();
        let original_hash = engine.state_hash();
        let log = ReplayLog::new(&engine).unwrap();
        let replayed = replay(&log).unwrap();
        assert_eq!(replayed.state_hash(), original_hash);
    }

    // -----------------------------------------------------------------------
    // Test 4: Replay step advances tick
    // -----------------------------------------------------------------------
    #[test]
    fn replay_step_advances_tick() {
        let engine = make_test_engine();
        let mut log = ReplayLog::new(&engine).unwrap();
        log.record(ReplayCommand::Step);
        log.record(ReplayCommand::Step);
        log.record(ReplayCommand::Step);

        let replayed = replay(&log).unwrap();
        assert_eq!(replayed.sim_state.tick, 3);
    }

    // -----------------------------------------------------------------------
    // Test 5: Replay verify passes for deterministic replay
    // -----------------------------------------------------------------------
    #[test]
    fn replay_verify_passes() {
        let mut engine = make_test_engine();
        let mut log = ReplayLog::new(&engine).unwrap();

        for _ in 0..5 {
            engine.step();
            log.record_with_hash(ReplayCommand::Step, engine.state_hash());
        }

        let result = replay_and_verify(&log).unwrap();
        assert!(result.is_verified);
        assert_eq!(result.commands_executed, 5);
        assert!(result.first_mismatch.is_none());
    }

    // -----------------------------------------------------------------------
    // Test 6: Replay verify detects mismatch
    // -----------------------------------------------------------------------
    #[test]
    fn replay_verify_detects_mismatch() {
        let engine = make_test_engine();
        let mut log = ReplayLog::new(&engine).unwrap();

        // Record steps with a deliberately wrong hash.
        log.record(ReplayCommand::Step);
        log.hash_checkpoints.push((0, 0xDEADBEEF));

        let result = replay_and_verify(&log).unwrap();
        assert!(!result.is_verified);
        assert!(result.first_mismatch.is_some());
        let mismatch = result.first_mismatch.unwrap();
        assert_eq!(mismatch.command_index, 0);
        assert_eq!(mismatch.expected_hash, 0xDEADBEEF);
    }

    // -----------------------------------------------------------------------
    // Test 7: Replay round-trip serialize
    // -----------------------------------------------------------------------
    #[test]
    fn replay_round_trip_serialize() {
        let mut engine = make_test_engine();
        let mut log = ReplayLog::new(&engine).unwrap();

        for _ in 0..3 {
            engine.step();
            log.record(ReplayCommand::Step);
        }

        let bytes = log.serialize().unwrap();
        let restored = ReplayLog::deserialize(&bytes).unwrap();

        assert_eq!(restored.command_count(), 3);
        assert_eq!(restored.initial_snapshot.len(), log.initial_snapshot.len());
    }

    // -----------------------------------------------------------------------
    // Test 8: Replay with mutations
    // -----------------------------------------------------------------------
    #[test]
    fn replay_with_mutations() {
        let engine = Engine::new(SimulationStrategy::Tick);
        let mut log = ReplayLog::new(&engine).unwrap();

        // Queue add node, apply mutations via step
        log.record(ReplayCommand::QueueAddNode {
            building_type: crate::id::BuildingTypeId(0),
        });
        log.record(ReplayCommand::ApplyMutations);
        log.record(ReplayCommand::Step);

        let replayed = replay(&log).unwrap();
        assert_eq!(replayed.node_count(), 1);
        assert_eq!(replayed.sim_state.tick, 1);
    }

    // -----------------------------------------------------------------------
    // Test 9: Replay set processor and transport
    // -----------------------------------------------------------------------
    #[test]
    fn replay_set_processor_and_transport() {
        let mut engine = Engine::new(SimulationStrategy::Tick);
        // Add two nodes and connect them
        let pending_a = engine.graph.queue_add_node(building());
        let pending_b = engine.graph.queue_add_node(building());
        let result = engine.graph.apply_mutations();
        let a = result.resolve_node(pending_a).unwrap();
        let b = result.resolve_node(pending_b).unwrap();
        let pending_e = engine.graph.queue_connect(a, b);
        let result = engine.graph.apply_mutations();
        let edge = result.resolve_edge(pending_e).unwrap();

        let mut log = ReplayLog::new(&engine).unwrap();

        let src_proc = make_source(iron(), 5.0);
        let transport = make_flow_transport(3.0);

        log.record(ReplayCommand::SetProcessor {
            node: a,
            processor: src_proc,
        });
        log.record(ReplayCommand::SetInputInventory {
            node: a,
            inventory: simple_inventory(100),
        });
        log.record(ReplayCommand::SetOutputInventory {
            node: a,
            inventory: simple_inventory(100),
        });
        log.record(ReplayCommand::SetTransport { edge, transport });
        log.record(ReplayCommand::Step);
        log.record(ReplayCommand::Step);

        let replayed = replay(&log).unwrap();
        assert_eq!(replayed.sim_state.tick, 2);
        // Source should have produced items
        assert!(replayed.get_output_inventory(a).is_some());
    }

    // -----------------------------------------------------------------------
    // Test 10: Replay complex scenario matches original
    // -----------------------------------------------------------------------
    #[test]
    fn replay_complex_scenario() {
        let mut engine = make_test_engine();
        let mut log = ReplayLog::new(&engine).unwrap();

        // Run 10 steps recording each with hash
        for _ in 0..10 {
            engine.step();
            log.record_with_hash(ReplayCommand::Step, engine.state_hash());
        }

        let final_hash = engine.state_hash();

        // Verify replay produces same state
        let result = replay_and_verify(&log).unwrap();
        assert!(result.is_verified);
        assert_eq!(result.commands_executed, 10);

        // Also check final state matches
        let replayed = replay(&log).unwrap();
        assert_eq!(replayed.state_hash(), final_hash);
        assert_eq!(replayed.sim_state.tick, engine.sim_state.tick);
    }
}
