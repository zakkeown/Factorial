//! Integration tests for the factorial-logic circuit network module.
//!
//! These tests verify that the LogicModuleBridge integrates correctly with
//! the factorial-core engine: stepping the engine ticks the logic module,
//! inventory readers see real inventory state, and circuit controls respond
//! to signal changes over multiple ticks.

use factorial_core::engine::Engine;
use factorial_core::fixed::Fixed64;
use factorial_core::id::*;
use factorial_core::sim::SimulationStrategy;
use factorial_core::test_utils::*;
use factorial_core::transport::*;

use factorial_logic::combinator::{
    ArithmeticCombinator, ArithmeticOp, DeciderCombinator, DeciderOutput, SignalSelector,
};
use factorial_logic::condition::{ComparisonOp, Condition, InventorySource};
use factorial_logic::{LogicEvent, LogicModuleBridge, SignalSet, WireColor};

// ============================================================================
// Shared helpers
// ============================================================================

fn f(v: f64) -> Fixed64 {
    Fixed64::from_num(v)
}

/// Item type IDs for logic integration tests (200+ to avoid collisions).
fn l_iron_ore() -> ItemTypeId {
    ItemTypeId(200)
}
fn l_iron_plate() -> ItemTypeId {
    ItemTypeId(201)
}
fn l_steel() -> ItemTypeId {
    ItemTypeId(202)
}
fn l_ratio() -> ItemTypeId {
    ItemTypeId(203)
}

/// Standard item belt for connecting nodes.
fn belt() -> Transport {
    make_item_transport(8)
}

// ============================================================================
// Test 1: Circuit-controlled inserter (Factorio-style)
// ============================================================================

/// A source produces iron ore into a chest node. An inventory reader on the
/// chest broadcasts its contents on a red wire network. A downstream node
/// has a circuit control condition: "iron_ore > 10 -> enable". We run engine
/// ticks and verify the downstream node activates once enough ore accumulates.
#[test]
fn circuit_controlled_inserter_activates_on_threshold() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    engine.register_module(Box::new(LogicModuleBridge::new()));

    // Build: source -> belt -> chest (sink node that accumulates ore).
    let source = add_node(&mut engine, make_source(l_iron_ore(), 5.0), 50, 50);
    let chest = add_node(&mut engine, make_source(l_iron_ore(), 0.0), 1000, 50);
    connect(&mut engine, source, chest, belt());

    // Downstream node with circuit control -- not connected by transport,
    // just on the same wire network.
    let inserter = add_node(&mut engine, make_source(l_iron_ore(), 0.0), 50, 50);

    // Wire up: inventory reader on chest -> red network -> circuit control on inserter.
    {
        let bridge = engine.find_module_mut::<LogicModuleBridge>().unwrap();
        let red = bridge.logic_mut().create_network(WireColor::Red);
        bridge.logic_mut().add_to_network(red, chest);
        bridge.logic_mut().add_to_network(red, inserter);

        // The chest reads its own input inventory and broadcasts signals.
        bridge
            .logic_mut()
            .set_inventory_reader(chest, chest, InventorySource::Input);

        // The inserter activates when iron_ore > 10.
        bridge.logic_mut().set_circuit_control(
            inserter,
            Condition {
                left: SignalSelector::Signal(l_iron_ore()),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(f(10.0)),
            },
            WireColor::Red,
        );
    }

    // Initially the inserter should be inactive (no signals yet).
    {
        let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
        assert_eq!(bridge.logic().is_active(inserter), Some(false));
    }

    // Tick the engine until enough ore accumulates. The source produces 5/tick
    // so after a few ticks ore should exceed 10 in the chest's input inventory.
    let mut activated_tick = None;
    for tick in 1..=20 {
        engine.step();

        let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
        if bridge.logic().is_active(inserter) == Some(true) && activated_tick.is_none() {
            activated_tick = Some(tick);
        }
    }

    // Verify the inserter activated at some point.
    assert!(
        activated_tick.is_some(),
        "Inserter should have activated after ore exceeded threshold"
    );

    // Verify activation event was emitted on the activation tick.
    let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
    assert_eq!(
        bridge.logic().is_active(inserter),
        Some(true),
        "Inserter should remain active while ore > 10"
    );
}

// ============================================================================
// Test 2: Conditional production shutdown via decider combinator
// ============================================================================

/// A constant combinator sets a threshold. A decider combinator compares
/// the threshold against a signal, producing an enable signal when below
/// the limit. A circuit control gates a node based on the decider's output.
/// This test verifies the one-tick delay: the decider's output appears on
/// the network one tick after it is computed.
#[test]
fn decider_combinator_one_tick_delay_gates_production() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    engine.register_module(Box::new(LogicModuleBridge::new()));

    // Create nodes. We use dummy source processors because we only care
    // about the logic module's behavior, not actual production.
    let const_node = add_node(&mut engine, make_source(l_iron_ore(), 0.0), 50, 50);
    let decider_node = add_node(&mut engine, make_source(l_iron_ore(), 0.0), 50, 50);
    let gated_node = add_node(&mut engine, make_source(l_iron_ore(), 0.0), 50, 50);

    {
        let bridge = engine.find_module_mut::<LogicModuleBridge>().unwrap();
        let red = bridge.logic_mut().create_network(WireColor::Red);
        bridge.logic_mut().add_to_network(red, const_node);
        bridge.logic_mut().add_to_network(red, decider_node);
        bridge.logic_mut().add_to_network(red, gated_node);

        // Constant combinator: iron_plate = 50 (simulates "current output count").
        let mut signals = SignalSet::new();
        signals.insert(l_iron_plate(), f(50.0));
        bridge.logic_mut().set_constant(const_node, signals, true);

        // Decider: if iron_plate < 100 then output steel = 1 (enable signal).
        bridge.logic_mut().set_decider(
            decider_node,
            DeciderCombinator {
                condition: Condition {
                    left: SignalSelector::Signal(l_iron_plate()),
                    op: ComparisonOp::Lt,
                    right: SignalSelector::Constant(f(100.0)),
                },
                output: DeciderOutput::One(l_steel()),
            },
        );

        // Circuit control on gated_node: enable when steel > 0 on red wire.
        bridge.logic_mut().set_circuit_control(
            gated_node,
            Condition {
                left: SignalSelector::Signal(l_steel()),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(f(0.0)),
            },
            WireColor::Red,
        );
    }

    // Tick 1: Decider evaluates (50 < 100 => true), computes steel=1.
    // But due to one-tick delay, steel=1 is NOT on the network yet.
    // The gated_node should be INACTIVE.
    engine.step();
    {
        let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
        assert_eq!(
            bridge.logic().is_active(gated_node),
            Some(false),
            "Tick 1: gated_node should be inactive (decider output delayed by one tick)"
        );
    }

    // Tick 2: Decider's output from tick 1 is now on the network.
    // The gated_node should be ACTIVE.
    engine.step();
    {
        let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
        assert_eq!(
            bridge.logic().is_active(gated_node),
            Some(true),
            "Tick 2: gated_node should be active (decider output now visible)"
        );

        // Verify the activation event was emitted.
        let activated_events: Vec<_> = bridge
            .last_events()
            .iter()
            .filter(
                |e| matches!(e, LogicEvent::CircuitActivated { node, .. } if *node == gated_node),
            )
            .collect();
        assert_eq!(
            activated_events.len(),
            1,
            "Tick 2: should have exactly one CircuitActivated event for gated_node"
        );
    }

    // Tick 3: Still active, no new activation event (steady state).
    engine.step();
    {
        let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
        assert_eq!(bridge.logic().is_active(gated_node), Some(true));
        let activated_events: Vec<_> = bridge
            .last_events()
            .iter()
            .filter(
                |e| matches!(e, LogicEvent::CircuitActivated { node, .. } if *node == gated_node),
            )
            .collect();
        assert_eq!(
            activated_events.len(),
            0,
            "Tick 3: no new activation event in steady state"
        );
    }
}

// ============================================================================
// Test 3: Dual-network arithmetic feedback
// ============================================================================

/// Red and green wire networks with separate node sets. Red carries a constant
/// signal (simulating inventory). A bridge node reads from red and an arithmetic
/// combinator on a green-only node computes a ratio. A circuit control on the
/// green network enables a consumer when the ratio exceeds a threshold.
/// This verifies wire color independence and multi-tick signal propagation.
#[test]
fn dual_network_arithmetic_feedback() {
    let mut engine = Engine::new(SimulationStrategy::Tick);
    engine.register_module(Box::new(LogicModuleBridge::new()));

    // Nodes:
    //   const_node  -- constant combinator, on red only
    //   bridge_node -- on red AND green, relays signal via a second constant
    //   arith_node  -- arithmetic combinator, on green only (output stays on green)
    //   consumer    -- circuit control, on green only
    let const_node = add_node(&mut engine, make_source(l_iron_ore(), 0.0), 50, 50);
    let bridge_node = add_node(&mut engine, make_source(l_iron_ore(), 0.0), 50, 50);
    let arith_node = add_node(&mut engine, make_source(l_iron_ore(), 0.0), 50, 50);
    let consumer = add_node(&mut engine, make_source(l_iron_ore(), 0.0), 50, 50);

    {
        let bridge = engine.find_module_mut::<LogicModuleBridge>().unwrap();

        // Red network: const_node and bridge_node.
        let red = bridge.logic_mut().create_network(WireColor::Red);
        bridge.logic_mut().add_to_network(red, const_node);
        bridge.logic_mut().add_to_network(red, bridge_node);

        // Green network: bridge_node, arith_node, and consumer.
        let green = bridge.logic_mut().create_network(WireColor::Green);
        bridge.logic_mut().add_to_network(green, bridge_node);
        bridge.logic_mut().add_to_network(green, arith_node);
        bridge.logic_mut().add_to_network(green, consumer);

        // Constant on red: iron_plate = 200.
        let mut signals = SignalSet::new();
        signals.insert(l_iron_plate(), f(200.0));
        bridge.logic_mut().set_constant(const_node, signals, true);

        // Bridge node also has a constant that relays iron_plate onto green.
        // This simulates a node that reads red and forwards to green.
        let mut bridge_signals = SignalSet::new();
        bridge_signals.insert(l_iron_plate(), f(200.0));
        bridge
            .logic_mut()
            .set_constant(bridge_node, bridge_signals, true);

        // Arithmetic combinator on arith_node (green only): ratio = iron_plate / 10.
        // arith_node sees green network signals which include bridge_node's constant.
        bridge.logic_mut().set_arithmetic(
            arith_node,
            ArithmeticCombinator {
                left: SignalSelector::Signal(l_iron_plate()),
                op: ArithmeticOp::Divide,
                right: SignalSelector::Constant(f(10.0)),
                output: l_ratio(),
            },
        );

        // Circuit control on consumer: enable when ratio > 15 on green wire.
        bridge.logic_mut().set_circuit_control(
            consumer,
            Condition {
                left: SignalSelector::Signal(l_ratio()),
                op: ComparisonOp::Gt,
                right: SignalSelector::Constant(f(15.0)),
            },
            WireColor::Green,
        );
    }

    // Tick 1: Arithmetic combinator evaluates (200/10 = 20) but output is delayed.
    // Green network has no ratio signal yet. Consumer should be INACTIVE.
    engine.step();
    {
        let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
        assert_eq!(
            bridge.logic().is_active(consumer),
            Some(false),
            "Tick 1: consumer inactive (arithmetic output delayed)"
        );

        // Red network should have iron_plate = 400 (const_node=200 + bridge_node=200).
        let red_net_id = bridge
            .logic()
            .networks
            .values()
            .find(|n| n.color == WireColor::Red)
            .unwrap()
            .id;
        let red_signals = bridge.logic().network_signals(red_net_id).unwrap();
        assert_eq!(
            red_signals.get(&l_iron_plate()),
            Some(&f(400.0)),
            "Red network should carry iron_plate=400 (both constants merged)"
        );

        // Green network should NOT have ratio yet (one-tick delay).
        let green_net_id = bridge
            .logic()
            .networks
            .values()
            .find(|n| n.color == WireColor::Green)
            .unwrap()
            .id;
        let green_signals = bridge.logic().network_signals(green_net_id).unwrap();
        assert!(
            green_signals.get(&l_ratio()).is_none()
                || green_signals.get(&l_ratio()) == Some(&f(0.0)),
            "Green network should not have ratio on tick 1"
        );
    }

    // Tick 2: Arithmetic output from tick 1 (ratio=20) now on green.
    // Consumer should be ACTIVE (20 > 15).
    engine.step();
    {
        let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
        assert_eq!(
            bridge.logic().is_active(consumer),
            Some(true),
            "Tick 2: consumer active (ratio=20 > 15 on green)"
        );

        // Green network should now carry ratio=20.
        let green_net_id = bridge
            .logic()
            .networks
            .values()
            .find(|n| n.color == WireColor::Green)
            .unwrap()
            .id;
        let green_signals = bridge.logic().network_signals(green_net_id).unwrap();
        assert_eq!(
            green_signals.get(&l_ratio()),
            Some(&f(20.0)),
            "Green network should carry ratio=20 on tick 2"
        );

        // Red network should NOT carry ratio (arith_node is not on red).
        let red_net_id = bridge
            .logic()
            .networks
            .values()
            .find(|n| n.color == WireColor::Red)
            .unwrap()
            .id;
        let red_signals = bridge.logic().network_signals(red_net_id).unwrap();
        assert!(
            red_signals.get(&l_ratio()).is_none(),
            "Red network should NOT carry ratio signal (wire color independence)"
        );
    }

    // Tick 3: Verify steady state -- consumer remains active, no new activation event.
    engine.step();
    {
        let bridge = engine.find_module::<LogicModuleBridge>().unwrap();
        assert_eq!(bridge.logic().is_active(consumer), Some(true));
        let activated: Vec<_> = bridge
            .last_events()
            .iter()
            .filter(|e| matches!(e, LogicEvent::CircuitActivated { node, .. } if *node == consumer))
            .collect();
        assert_eq!(
            activated.len(),
            0,
            "No new activation event in steady state"
        );
    }
}
