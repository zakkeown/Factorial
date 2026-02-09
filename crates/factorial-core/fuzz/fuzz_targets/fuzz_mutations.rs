#![no_main]
use arbitrary::Arbitrary;
use factorial_core::engine::Engine;
use factorial_core::id::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;
use libfuzzer_sys::fuzz_target;

/// A structured mutation operation for fuzzing.
#[derive(Arbitrary, Debug)]
enum FuzzOp {
    AddNode,
    RemoveNode { index: u8 },
    Connect { from: u8, to: u8 },
    Disconnect { index: u8 },
    Step,
}

/// Top-level fuzz input: a sequence of operations.
#[derive(Arbitrary, Debug)]
struct FuzzInput {
    ops: Vec<FuzzOp>,
}

fuzz_target!(|input: FuzzInput| {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    let mut node_ids: Vec<NodeId> = Vec::new();
    let mut edge_ids: Vec<factorial_core::id::EdgeId> = Vec::new();

    // Limit operations to prevent timeouts.
    let max_ops = input.ops.len().min(200);

    for op in &input.ops[..max_ops] {
        match op {
            FuzzOp::AddNode => {
                let node = add_node(
                    &mut engine,
                    make_source(iron(), 1.0),
                    100,
                    100,
                );
                node_ids.push(node);
            }
            FuzzOp::RemoveNode { index } => {
                if !node_ids.is_empty() {
                    let idx = (*index as usize) % node_ids.len();
                    let node = node_ids.remove(idx);
                    engine.graph.queue_remove_node(node);
                    engine.graph.apply_mutations();
                }
            }
            FuzzOp::Connect { from, to } => {
                if node_ids.len() >= 2 {
                    let from_idx = (*from as usize) % node_ids.len();
                    let to_idx = (*to as usize) % node_ids.len();
                    if from_idx != to_idx {
                        let edge = connect(
                            &mut engine,
                            node_ids[from_idx],
                            node_ids[to_idx],
                            make_flow_transport(1.0),
                        );
                        edge_ids.push(edge);
                    }
                }
            }
            FuzzOp::Disconnect { index } => {
                if !edge_ids.is_empty() {
                    let idx = (*index as usize) % edge_ids.len();
                    let edge = edge_ids.remove(idx);
                    engine.graph.queue_disconnect(edge);
                    engine.graph.apply_mutations();
                }
            }
            FuzzOp::Step => {
                engine.step();
            }
        }
    }
});
