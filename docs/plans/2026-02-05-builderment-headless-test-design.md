# Builderment Headless Test Design

A comprehensive headless integration test that models the full Builderment production chain from raw ore through Super Computer, integrating the power, tech-tree, and stats crates.

## Goal

Prove that the Factorial engine can support a realistic factory game scenario end-to-end without any GUI, verifying:
- Deep production chains (4 tiers, ~28 nodes, ~30 edges)
- Fan-out from shared intermediate resources
- Cross-crate integration (power, tech tree, stats)
- Serialization round-trip fidelity
- Tick-by-tick determinism

## Story Selection: Builderment

Chosen because:
- No combat — pure factory optimization matching the engine's scope
- Simple linear production chains (1-4 inputs per building)
- Belt-based transport — maps to `ItemTransport`
- No fluids — avoids unimplemented systems
- Clean progression — extractors → belts → tiered processors

## Item Types (22)

```rust
// Raw Resources (7)
iron_ore       = ItemTypeId(0)
copper_ore     = ItemTypeId(1)
coal           = ItemTypeId(2)
stone          = ItemTypeId(3)
wood           = ItemTypeId(4)
tungsten_ore   = ItemTypeId(5)
uranium        = ItemTypeId(6)

// Tier 1: Furnace/Workshop outputs (8)
iron_ingot     = ItemTypeId(10)
copper_ingot   = ItemTypeId(11)
sand           = ItemTypeId(12)
glass          = ItemTypeId(13)
wood_plank     = ItemTypeId(14)
iron_gear      = ItemTypeId(15)
copper_wire    = ItemTypeId(16)
tungsten_ingot = ItemTypeId(17)  // unused but defined for completeness

// Tier 2: Machine Shop/Forge outputs (6)
motor          = ItemTypeId(20)
wood_frame     = ItemTypeId(21)
light_bulb     = ItemTypeId(22)
graphite       = ItemTypeId(23)
steel          = ItemTypeId(24)
tungsten_carbide = ItemTypeId(25)

// Tier 3: Industrial Factory outputs (3)
electric_motor = ItemTypeId(30)
circuit_board  = ItemTypeId(31)
basic_robot    = ItemTypeId(32)

// Tier 4: Manufacturer outputs (2)
computer       = ItemTypeId(40)
super_computer = ItemTypeId(41)
```

## Recipes (Faithful to Builderment)

### Furnace (1 input → 1 output)

| Input | Output | Input Qty | Output Qty | Duration |
|-------|--------|-----------|------------|----------|
| Copper Ore | Copper Ingot | 1 | 1 | 2 ticks |
| Iron Ore | Iron Ingot | 1 | 1 | 2 ticks |
| Stone | Sand | 1 | 1 | 2 ticks |
| Sand | Glass | 1 | 1 | 3 ticks |
| Tungsten Ore | Tungsten Ingot | 1 | 1 | 4 ticks |

### Workshop (1 input → 1 output)

| Input | Output | Input Qty | Output Qty | Duration |
|-------|--------|-----------|------------|----------|
| Wood | Wood Plank | 1 | 1 | 2 ticks |
| Iron Ingot | Iron Gear | 1 | 1 | 2 ticks |
| Copper Ingot | Copper Wire | 3 | 1 | 3 ticks |

### Machine Shop (2 inputs → 1 output)

| Inputs | Output | Duration |
|--------|--------|----------|
| 1 Iron Gear + 1 Copper Wire | 1 Motor | 4 ticks |
| 1 Wood Plank + 1 Iron Ingot | 1 Wood Frame | 3 ticks |
| 1 Glass + 1 Copper Wire | 1 Light Bulb | 3 ticks |
| 1 Sand + 1 Coal | 1 Graphite | 3 ticks |

### Forge (2 inputs → 1 output)

| Inputs | Output | Duration |
|--------|--------|----------|
| 10 Tungsten Ore + 1 Graphite | 1 Tungsten Carbide | 6 ticks |
| 1 Iron Ingot + 1 Coal | 1 Steel | 3 ticks |

### Industrial Factory (3 inputs → 1 output)

| Inputs | Output | Duration |
|--------|--------|----------|
| 1 Motor + 1 Steel + 1 Copper Wire | 1 Electric Motor | 6 ticks |
| 1 Glass + 1 Copper Wire + 1 Steel | 1 Circuit Board | 6 ticks |
| 1 Wood Frame + 1 Motor + 1 Light Bulb | 1 Basic Robot | 6 ticks |

### Manufacturer (4 inputs → 1 output)

| Inputs | Output | Duration |
|--------|--------|----------|
| 1 Circuit Board + 1 Electric Motor + 1 Steel + 1 Glass | 1 Computer | 8 ticks |
| 1 Computer + 1 Tungsten Carbide + 1 Electric Motor + 1 Circuit Board | 1 Super Computer | 10 ticks |

## Factory Graph

### Resource Sources (6 nodes)

| Node | Item | Rate |
|------|------|------|
| Iron Ore Source | iron_ore | 5/tick |
| Copper Ore Source | copper_ore | 5/tick |
| Coal Source | coal | 5/tick |
| Stone Source | stone | 3/tick |
| Wood Source | wood | 2/tick |
| Tungsten Ore Source | tungsten_ore | 3/tick |

### Tier 1 Processing (7 nodes)

| Node | Recipe |
|------|--------|
| Iron Furnace | Iron Ore → Iron Ingot |
| Copper Furnace | Copper Ore → Copper Ingot |
| Stone Furnace | Stone → Sand |
| Glass Furnace | Sand → Glass |
| Plank Workshop | Wood → Wood Plank |
| Gear Workshop | Iron Ingot → Iron Gear |
| Wire Workshop | 3 Copper Ingot → 1 Copper Wire |

### Tier 2 Processing (6 nodes)

| Node | Recipe |
|------|--------|
| Motor Shop | Iron Gear + Copper Wire → Motor |
| Wood Frame Shop | Wood Plank + Iron Ingot → Wood Frame |
| Light Bulb Shop | Glass + Copper Wire → Light Bulb |
| Graphite Shop | Sand + Coal → Graphite |
| Steel Forge | Iron Ingot + Coal → Steel |
| Tungsten Carbide Forge | 10 Tungsten Ore + Graphite → Tungsten Carbide |

### Tier 3 Processing (3 nodes)

| Node | Recipe |
|------|--------|
| Electric Motor Factory | Motor + Steel + Copper Wire → Electric Motor |
| Circuit Board Factory | Glass + Copper Wire + Steel → Circuit Board |
| Basic Robot Factory | Wood Frame + Motor + Light Bulb → Basic Robot |

### Tier 4 Processing (2 nodes)

| Node | Recipe |
|------|--------|
| Computer Manufacturer | Circuit Board + Electric Motor + Steel + Glass → Computer |
| Super Computer Manufacturer | Computer + Tungsten Carbide + Electric Motor + Circuit Board → Super Computer |

### Sinks (2 nodes)

| Node | Purpose |
|------|---------|
| Computer Sink | Consumes Computers (Earth Teleporter analog) |
| Super Computer Sink | Consumes Super Computers |

### Total: 26 production nodes + 2 sinks = 28 nodes

## Transport Connections (~30 edges)

All use `ItemTransport` (8 slots, speed 1.0, 1 lane) matching Builderment's discrete belt system.

```
// Raw → Tier 1
Iron Ore Source       → Iron Furnace
Copper Ore Source     → Copper Furnace
Stone Source          → Stone Furnace
Coal Source           → Graphite Shop
Coal Source           → Steel Forge
Wood Source           → Plank Workshop
Tungsten Ore Source   → Tungsten Carbide Forge

// Tier 1 → Tier 1 (chained processing)
Stone Furnace         → Glass Furnace        (Sand → Glass chain)

// Tier 1 → Tier 2
Iron Furnace          → Gear Workshop
Iron Furnace          → Steel Forge
Iron Furnace          → Wood Frame Shop
Copper Furnace        → Wire Workshop
Glass Furnace         → Light Bulb Shop
Glass Furnace         → Circuit Board Factory
Glass Furnace         → Computer Manufacturer
Stone Furnace         → Graphite Shop        (Sand for Graphite)
Plank Workshop        → Wood Frame Shop
Gear Workshop         → Motor Shop

// Tier 1 → Tier 3 (Wire fan-out)
Wire Workshop         → Motor Shop
Wire Workshop         → Light Bulb Shop
Wire Workshop         → Electric Motor Factory
Wire Workshop         → Circuit Board Factory

// Tier 2 → Tier 3
Motor Shop            → Electric Motor Factory
Motor Shop            → Basic Robot Factory
Steel Forge           → Electric Motor Factory
Steel Forge           → Circuit Board Factory
Steel Forge           → Computer Manufacturer
Wood Frame Shop       → Basic Robot Factory
Light Bulb Shop       → Basic Robot Factory
Graphite Shop         → Tungsten Carbide Forge

// Tier 3 → Tier 4
Electric Motor Factory → Computer Manufacturer
Electric Motor Factory → Super Computer Manufacturer
Circuit Board Factory  → Computer Manufacturer
Circuit Board Factory  → Super Computer Manufacturer

// Tier 4 → Tier 4 (chained)
Computer Manufacturer  → Super Computer Manufacturer

// Tier 4 → Sinks
Computer Manufacturer       → Computer Sink
Super Computer Manufacturer → Super Computer Sink

// Tungsten Carbide → Tier 4
Tungsten Carbide Forge → Super Computer Manufacturer
```

### Key Fan-Out Nodes

| Node | Downstream Count | Feeds |
|------|-----------------|-------|
| Wire Workshop | 4 | Motor Shop, Light Bulb Shop, Electric Motor Factory, Circuit Board Factory |
| Iron Furnace | 3 | Gear Workshop, Steel Forge, Wood Frame Shop |
| Steel Forge | 3 | Electric Motor Factory, Circuit Board Factory, Computer Manufacturer |
| Glass Furnace | 3 | Light Bulb Shop, Circuit Board Factory, Computer Manufacturer |
| Motor Shop | 2 | Electric Motor Factory, Basic Robot Factory |
| Electric Motor Factory | 2 | Computer Manufacturer, Super Computer Manufacturer |
| Circuit Board Factory | 2 | Computer Manufacturer, Super Computer Manufacturer |

## Cross-Crate Integration

### Power Module

- Create 1 power network
- Add 1 producer (Coal Power Plant, capacity 100W)
- Register all production buildings as consumers (demand ~5W each)
- **Brownout test**: Remove producer at tick 200, verify `PowerGridBrownout` event
- **Recovery test**: Re-add producer at tick 250, verify `PowerGridRestored` event
- Game logic: read `satisfaction()` each tick, apply speed modifier of 0.0 when brownout

### Tech Tree Module

Register 5 technologies:

| Tech ID | Name | Prerequisites | Cost (Items model) | Unlocks |
|---------|------|--------------|-------------------|---------|
| 0 | basic_smelting | None | 10 Iron Ingot | Building::Furnace |
| 1 | workshops | basic_smelting | 20 Iron Gear, 10 Copper Wire | Building::Workshop |
| 2 | machine_shops | workshops | 15 Motor | Building::MachineShop |
| 3 | industrial | machine_shops | 10 Steel, 10 Circuit Board | Building::IndustrialFactory |
| 4 | manufacturing | industrial | 5 Computer | Building::Manufacturer |

Test: Feed produced items into tech tree, verify events fire in order.

### Stats Module

- Window: 50 ticks, history capacity: 10
- Feed engine events each tick
- After steady-state (~500 ticks), assert:
  - Iron Furnace production rate ≈ 0.5 items/tick
  - Wire Workshop stall ratio > other Tier 1 buildings (bottleneck)
  - Total iron ingot production > total copper wire production (3:1 effect)

## Test Structure

```rust
// tests/builderment_headless.rs

mod builderment_headless {
    // Shared factory builder
    fn build_factory() -> Engine { ... }

    // Item type constants
    fn iron_ore() -> ItemTypeId { ItemTypeId(0) }
    // ... etc

    #[test]
    fn full_chain_produces_computers()
    // Run 500 ticks, assert Computer Sink has received computers

    #[test]
    fn full_chain_produces_super_computers()
    // Run 1000 ticks, assert Super Computer Sink has received super computers

    #[test]
    fn copper_wire_bottleneck_visible_in_stats()
    // Run 500 ticks with stats tracking, verify Wire Workshop stall ratio

    #[test]
    fn power_brownout_and_recovery()
    // Run 200 ticks normal, remove power, run 50, restore, run 50 more

    #[test]
    fn tech_tree_progression()
    // Run factory while feeding items to tech tree, verify unlock order

    #[test]
    fn serialize_round_trip_full_factory()
    // Run 250, serialize, deserialize, run 250 more, compare hash with 500 straight

    #[test]
    fn determinism_full_factory()
    // Two identical factories, 500 ticks, tick-by-tick hash comparison
}
```

## Expected Outcomes

1. **Computers produced**: After ~50-100 ticks of pipeline warmup, computers should flow steadily
2. **Super Computers produced**: After ~100-200 ticks of warmup (deeper chain), super computers should appear
3. **Copper Wire is the bottleneck**: The 3:1 input ratio creates natural starvation downstream
4. **Power brownout works**: Production halts during brownout, resumes on recovery
5. **Tech tree respects ordering**: Can't research manufacturing before industrial
6. **Deterministic**: Identical setups produce identical state hashes every tick
7. **Serialization preserves state**: Save/load mid-simulation produces identical final state

## What This Tests About the Engine

| Capability | How It's Tested |
|-----------|----------------|
| Source processors | 6 resource extractors running continuously |
| Fixed recipe processors | 18 different recipes across 4 tiers |
| Item transport (discrete belts) | 30 belt connections with slot-based movement |
| Fan-out from shared nodes | Wire Workshop → 4 consumers, Iron Furnace → 3 consumers |
| Deep dependency chains | Stone → Sand → Glass → Circuit Board → Computer → Super Computer |
| Backpressure / stalling | Wire Workshop stalls on full output when consumers are slow |
| Multi-input recipes | 2-input, 3-input, and 4-input recipes |
| Topological ordering | Engine processes nodes in correct dependency order |
| State hashing | Determinism verification across 500+ ticks |
| Serialization | Round-trip through 28-node factory |
| Power module | Brownout detection and recovery events |
| Tech tree module | Item-cost research with prerequisites |
| Stats module | Per-node rates, stall ratios, edge throughput |
