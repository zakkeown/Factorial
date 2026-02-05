# Satisfactory: Comprehensive Game Design Research Document

## Table of Contents
1. [Initial Conditions](#1-initial-conditions)
2. [Core Mechanics](#2-core-mechanics)
3. [Technology/Research Tree](#3-technologyresearch-tree)
4. [Production Chains](#4-production-chains)
5. [World/Map](#5-worldmap)
6. [Combat/Survival](#6-combatsurvival)
7. [Endgame](#7-endgame)
8. [Multiplayer](#8-multiplayer)

---

## 1. Initial Conditions

### Drop Pod Landing Scenario

The player begins Satisfactory as a FICSIT employee (called a "Pioneer") who has been deployed to an alien planet to establish industrial operations. The player arrives via drop pod, which serves as the initial hub for operations. Unlike crash-landing scenarios, this is a deliberate corporate deployment with the goal of exploiting the planet's resources for Project Assembly.

### Starting Inventory

In **standard play**, the player starts with minimal equipment:

| Item | Quantity | Purpose |
|------|----------|---------|
| Xeno-Zapper | 1 | Melee weapon for self-defense |
| HUB Parts | 1 set | Components to build the initial HUB |

The drop pod landing provides the components necessary to construct the HUB, which becomes the central progression structure.

### Initial Map State

- The player selects one of **four starting areas** before landing:
  - Grass Fields (recommended for beginners)
  - Rocky Desert (strong resource availability)
  - Northern Forest (highest resource concentration)
  - Dune Desert (difficult start, best late-game potential)
- All starting areas guarantee access to:
  - Iron ore nodes
  - Copper ore nodes
  - Limestone deposits
  - Coal (nearby, for early power transition)
- **Excluded from starting areas**: Uranium, Bauxite, and Nitrogen Gas (late-game resources)

### Tutorial and Onboarding Structure

Satisfactory uses a guided onboarding system through ADA (the AI assistant):

1. **Onboarding Phase**: After exiting the drop pod, ADA guides players through basic objectives
2. **Tier 0 HUB Upgrades**: Six sequential upgrades that teach core mechanics
3. **Freeplay Progression**: After Tier 0, players have freedom to pursue milestones in any order within unlocked tiers
4. **Skip Option**: Experienced players can skip the onboarding tutorial

#### Tier 0 HUB Upgrade Requirements

| HUB Upgrade | Requirements | Unlocks |
|-------------|--------------|---------|
| HUB Upgrade 1 | 10 Iron Ingots | First Biomass Burner (attached to HUB) |
| HUB Upgrade 2 | 20 Iron Rods, 10 Iron Plates | Equipment Workshop |
| HUB Upgrade 3 | 20 Iron Plates, 20 Iron Rods, 20 Cable | Portable Miner, Object Scanner |
| HUB Upgrade 4 | 75 Iron Plates, 20 Cables, 10 Concrete | Conveyor Belts Mk.1, Power Poles |
| HUB Upgrade 5 | 200 Iron Rods, 200 Iron Plates, 100 Cable, 50 Concrete | Miner Mk.1, Second Biomass Burner |
| HUB Upgrade 6 | 100 Iron Rods, 100 Iron Plates, 100 Cable, 50 Concrete | FICSIT Freighter, Space Elevator |

---

## 2. Core Mechanics

### 2.1 Resource Types

Satisfactory features multiple resource categories with specific extraction methods and uses:

#### Solid Resources (Mined)

| Resource | Primary Uses | Node Distribution | Notes |
|----------|--------------|-------------------|-------|
| **Iron Ore** | Iron Ingots, Steel | 127 total nodes | Most abundant, foundational resource |
| **Copper Ore** | Copper Ingots, Wire, Cable | 55 total nodes | Electronics foundation |
| **Limestone** | Concrete, building materials | 94 total nodes | Structural applications |
| **Coal** | Power generation, Steel production | 62 total nodes | Dual-purpose resource |
| **Caterium Ore** | Quickwire, advanced electronics | 24 total nodes | MAM research required |
| **Raw Quartz** | Crystal components, Silica | 26 total nodes | MAM research required |
| **Sulfur** | Explosives, batteries | 19 total nodes | Combat and power applications |
| **Bauxite** | Aluminum production | 17 total nodes | Complex processing chain |
| **Uranium** | Nuclear power | 5 total nodes | Radioactive, late-game |
| **SAM (Strange Alien Metal)** | Endgame alien technology | Limited nodes | Tier 9 applications |

#### Fluid Resources

| Resource | Extraction Method | Primary Uses |
|----------|-------------------|--------------|
| **Water** | Water Extractor (from bodies of water) | Coal power, oil processing, concrete |
| **Crude Oil** | Oil Extractor (from oil nodes) | Plastic, Rubber, Fuel |
| **Nitrogen Gas** | Resource Well Pressurizer | Tier 8+ advanced recipes |

### 2.2 Mining and Extraction Systems

#### Resource Node Purity

All minable resource nodes have one of three purity levels affecting output:

| Purity | Output Multiplier | Relative Frequency |
|--------|-------------------|-------------------|
| Impure | 0.5x (half rate) | Common |
| Normal | 1.0x (base rate) | Common |
| Pure | 2.0x (double rate) | Rare |

#### Miner Types and Extraction Rates

| Miner | Base Rate | Impure Node | Normal Node | Pure Node | Unlock |
|-------|-----------|-------------|-------------|-----------|--------|
| **Portable Miner** | 20/min | 20/min | 40/min | 80/min | Tier 0 |
| **Miner Mk.1** | 60/min | 30/min | 60/min | 120/min | Tier 0 |
| **Miner Mk.2** | 120/min | 60/min | 120/min | 240/min | Tier 2 |
| **Miner Mk.3** | 240/min | 120/min | 240/min | 480/min | Tier 6 |

**Overclocking**: Miners can be overclocked up to 250% using Power Shards:
- Miner Mk.3 on Pure Node at 250% = 1,200 items/min

#### Fluid Extraction

| Extractor | Base Rate | Power | Notes |
|-----------|-----------|-------|-------|
| **Water Extractor** | 120 m³/min | 20 MW | Placed on water bodies |
| **Oil Extractor** | 120 m³/min (normal) | 40 MW | Placed on oil nodes |
| **Resource Well Pressurizer** | 60 m³/min per satellite | 150 MW | For pressurized resources |

### 2.3 Power Generation

#### Biomass Burner (Early Game)

| Specification | Value |
|--------------|-------|
| Power Output | 30 MW (standalone) / 20 MW (HUB-attached) |
| Fuel Types | Leaves, Wood, Biomass, Solid Biofuel, Packaged Liquid Biofuel |
| Automation | Cannot be automated; requires manual refueling |

**Fuel Burn Times** (at 30 MW):
- Leaves: 4.5 seconds
- Wood: 7.5 seconds
- Biomass: 22.5 seconds
- Solid Biofuel: 60 seconds

#### Coal-Powered Generator

| Specification | Value |
|--------------|-------|
| Power Output | 75 MW |
| Coal Consumption | 15 items/min |
| Water Consumption | 45 m³/min |
| Unlock | Tier 3 |

**Optimal Ratio**: 3 Water Extractors : 8 Coal Generators (360 m³/min water : 360 m³/min consumption)

**Fuel Alternatives**:
- Compacted Coal: 7.143/min (more efficient)
- Petroleum Coke: 25/min (less efficient)

#### Fuel-Powered Generator

| Specification | Value |
|--------------|-------|
| Power Output | 250 MW |
| Fuel Consumption | Variable by fuel type |
| Unlock | Tier 5 |

**Fuel Options**:
| Fuel Type | Consumption Rate | Energy Content |
|-----------|-----------------|----------------|
| Fuel | 15 m³/min | 750 MJ/m³ |
| Turbofuel | 4.5 m³/min | 2,000 MJ/m³ |
| Liquid Biofuel | 15 m³/min | 750 MJ/m³ |
| Rocket Fuel | 3 m³/min | 3,000 MJ/m³ |
| Ionized Fuel | 1.5 m³/min | 6,000 MJ/m³ |

#### Geothermal Generator

| Specification | Value |
|--------------|-------|
| Power Output (Impure Geyser) | 50-150 MW (avg 100 MW) |
| Power Output (Normal Geyser) | 100-300 MW (avg 200 MW) |
| Power Output (Pure Geyser) | 200-600 MW (avg 400 MW) |
| Fuel | None (passive) |
| Unlock | Tier 5 |

**Note**: Output fluctuates cyclically; geysers emit steam approximately every 15 seconds.

#### Nuclear Power Plant

| Specification | Value |
|--------------|-------|
| Power Output | 2,500 MW |
| Fuel | Uranium Fuel Rod or Plutonium Fuel Rod |
| Uranium Fuel Rod Duration | 300 seconds |
| Waste Produced | 10 Uranium Waste per rod (500/min processing to Plutonium) |
| Unlock | Tier 8 |

**Fuel Cycle**:
1. Uranium Ore + Sulfuric Acid → Uranium → Encased Uranium Cell → Uranium Fuel Rod
2. Uranium Waste → Plutonium Pellet → Encased Plutonium Cell → Plutonium Fuel Rod
3. Plutonium Waste → Ficsonium (Tier 9, eliminates waste)

### 2.4 Conveyor Belt System

#### Belt Throughput Tiers

| Belt Type | Throughput | Unlock | Build Cost |
|-----------|------------|--------|------------|
| **Conveyor Mk.1** | 60 items/min | Tier 0 | 1 Iron Plate |
| **Conveyor Mk.2** | 120 items/min | Tier 2 | 1 Reinforced Iron Plate |
| **Conveyor Mk.3** | 270 items/min | Tier 4 | 1 Steel Beam |
| **Conveyor Mk.4** | 480 items/min | Tier 5 | 1 Encased Industrial Beam |
| **Conveyor Mk.5** | 780 items/min | Tier 7 | 1 Alclad Aluminum Sheet |
| **Conveyor Mk.6** | 1,200 items/min | Tier 9 | 1 Fused Modular Frame |

**Note**: Above 780 items/min, engine calculations may cause item loss in certain configurations.

#### Conveyor Lifts

Vertical item transport matching belt speeds:
- Mk.1 through Mk.6 available
- Bi-directional (items flow based on connection)
- Same throughput as corresponding belt tier

#### Splitters and Mergers

| Type | Function | Unlock |
|------|----------|--------|
| **Splitter** | Divides 1 input into 3 outputs (even distribution) | Tier 1 |
| **Merger** | Combines 3 inputs into 1 output | Tier 1 |
| **Smart Splitter** | Programmable filtering with overflow support | Tier 4 (Caterium research) |
| **Programmable Splitter** | Up to 64 filter rules across outputs | Tier 5 |

**Smart Splitter Features**:
- Item-specific routing
- "Any Undefined" category for unfiltered items
- Overflow handling (routes to designated output when others are full)

### 2.5 Production Buildings

#### Processing Machines

| Machine | Inputs | Outputs | Power | Unlock |
|---------|--------|---------|-------|--------|
| **Smelter** | 1 solid | 1 solid | 4 MW | Tier 0 |
| **Foundry** | 2 solids | 1 solid | 16 MW | Tier 3 |
| **Constructor** | 1 solid | 1 solid | 4 MW | Tier 0 |
| **Assembler** | 2 solids | 1 solid | 15 MW | Tier 2 |
| **Manufacturer** | 3-4 solids | 1 solid | 55 MW | Tier 5 |
| **Refinery** | 1-2 fluids/solids | 1-2 fluids/solids | 30 MW | Tier 5 |
| **Blender** | 2-4 fluids/solids | 1-2 fluids/solids | 75 MW | Tier 7 |
| **Packager** | 1 fluid + container | 1 packaged item | 10 MW | Tier 5 |
| **Particle Accelerator** | Variable | Variable | 250-1500 MW | Tier 8 |
| **Quantum Encoder** | Variable + Photonic Matter | Variable + Dark Matter | 500-2000 MW | Tier 9 |
| **Converter** | Ore + Reanimated SAM | Different Ore/Ingot | 200 MW | Tier 9 |

#### Somersloop Production Amplification

Select machines can use Somersloops for output doubling:

| Somersloop Requirement | Buildings |
|----------------------|-----------|
| 1 Somersloop | Smelter, Constructor |
| 2 Somersloops | Assembler, Foundry, Refinery, Converter |
| 4 Somersloops | Manufacturer, Blender, Particle Accelerator, Quantum Encoder |

**Effect**: 100% increased output, 0% increased input, ~300% increased power consumption

### 2.6 Fluid Handling

#### Pipeline System

| Component | Throughput | Function |
|-----------|------------|----------|
| **Pipeline Mk.1** | 300 m³/min | Basic fluid transport |
| **Pipeline Mk.2** | 600 m³/min | High-capacity fluid transport |
| **Pipeline Pump Mk.1** | +20m head lift | Vertical fluid transport |
| **Pipeline Pump Mk.2** | +50m head lift | Extended vertical transport |

**Fluid Physics**:
- Fluids flow based on gravity (downhill preference)
- Head lift required for upward transport
- Machines have built-in head lift (Refinery: 10m output)
- Pumps consume 4 MW (Mk.1) or 8 MW (Mk.2)

#### Fluid Storage

| Structure | Capacity |
|-----------|----------|
| **Industrial Fluid Buffer** | 400 m³ |
| **Fluid Freight Platform** | 2,400 m³ |

### 2.7 Logistics Systems

#### Vehicles

| Vehicle | Unlock | Inventory | Characteristics |
|---------|--------|-----------|-----------------|
| **Tractor** | Tier 3 | 25 slots | Basic transport, autopilot capable |
| **Truck** | Tier 5 | 48 slots | Heavy cargo transport |
| **Explorer** | MAM (Quartz) | 24 slots | Fast, agile, rough terrain |
| **Cyber Wagon** | MAM (Caterium) | 12 slots | Premium personal transport |
| **Factory Cart** | Tier 2 | 16 slots | Indoor factory transport |

**Automation**: Tractors and Trucks can record and loop paths for automated delivery.

#### Train System

| Component | Function | Capacity |
|-----------|----------|----------|
| **Electric Locomotive** | Provides movement | N/A |
| **Freight Car** | Solid cargo | 32 stacks |
| **Freight Platform** | Station loading/unloading | 48 stacks buffer |
| **Fluid Freight Platform** | Liquid cargo | 2,400 m³ buffer |

**Train Station Load/Unload Time**: 27.08 seconds

**Railway Signals**:
- Path Signals: Reserve entire path through blocks
- Block Signals: Simple block occupancy detection

#### Drone Network

| Component | Function | Unlock |
|-----------|----------|--------|
| **Drone** | Automated point-to-point transport | Tier 8 |
| **Drone Port** | Endpoint for drone routes | Tier 8 |

**Drone Specifications**:
- 9 inventory slots
- 60 m/s flight speed
- Requires batteries for fuel
- No intermediate infrastructure needed

### 2.8 Hypertube System

Player-only rapid transit using tubes and accelerators:

| Component | Function |
|-----------|----------|
| **Hypertube** | Transport tube segment |
| **Hypertube Entrance** | Entry point (self-powered acceleration) |
| **Hypertube Support** | Structural support |

**Speed**: Accumulates with multiple entrances; can achieve extremely high velocities.

---

## 3. Technology/Research Tree

### 3.1 Milestone System Overview

Satisfactory uses a tiered milestone system unlocked at the HUB Terminal. Higher tiers require Space Elevator deliveries.

| Tier | Unlock Requirement | Milestone Count |
|------|-------------------|-----------------|
| Tier 0 | Game Start | 6 (HUB Upgrades) |
| Tiers 1-2 | Complete Tier 0 | 3 each |
| Tiers 3-4 | Space Elevator Phase 1 | 4 each |
| Tiers 5-6 | Space Elevator Phase 2 | 4-5 each |
| Tiers 7-8 | Space Elevator Phase 3 | 4 each |
| Tier 9 | Space Elevator Phase 5 | 5 |

### 3.2 Space Elevator Project Phases

#### Phase 1: Send Smart Components

| Item | Quantity | Recipe |
|------|----------|--------|
| **Smart Plating** | 50 | 1 Reinforced Iron Plate + 1 Rotor |

**Unlocks**: Tiers 3 and 4

#### Phase 2: Establish Global Network

| Item | Quantity | Recipe |
|------|----------|--------|
| **Smart Plating** | 500 | 1 Reinforced Iron Plate + 1 Rotor |
| **Versatile Framework** | 500 | 1 Modular Frame + 12 Steel Beam |
| **Automated Wiring** | 100 | 1 Stator + 20 Cable |

**Unlocks**: Tiers 5 and 6

#### Phase 3: Expand Continental Network

| Item | Quantity | Recipe |
|------|----------|--------|
| **Versatile Framework** | 2,500 | 1 Modular Frame + 12 Steel Beam |
| **Modular Engine** | 500 | 2 Motor + 15 Rubber + 2 Smart Plating |
| **Adaptive Control Unit** | 100 | 15 Automated Wiring + 2 Circuit Board + 2 Heavy Modular Frame + 2 Computer |

**Unlocks**: Tiers 7 and 8

#### Phase 4: Transition to Interstellar

| Item | Quantity | Recipe |
|------|----------|--------|
| **Assembly Director System** | 4,000 | 2 Adaptive Control Unit + 1 Supercomputer |
| **Magnetic Field Generator** | 4,000 | 5 Versatile Framework + 2 Electromagnetic Control Rod + 10 Battery |
| **Nuclear Pasta** | 1,000 | 200 Copper Powder + 1 Pressure Conversion Cube |
| **Thermal Propulsion Rocket** | 1,000 | 5 Modular Engine + 2 Turbo Motor + 6 Cooling System + 2 Fused Modular Frame |

**Unlocks**: Tier 9

#### Phase 5: Project Assembly Launch

Final delivery to initiate game ending (specific requirements after completing Tier 9).

### 3.3 MAM Research Tree

The MAM (Molecular Analysis Machine) provides parallel research tracks unlocked by finding specific materials:

#### Research Trees

| Tree | Unlock Material | Key Unlocks |
|------|-----------------|-------------|
| **Alien Organisms** | Hog Remains, Plasma Spitter Remains | Alien Protein, Biomass recipes, combat upgrades |
| **Caterium** | Caterium Ore | Quickwire, Smart Splitters, Zipline, Power Storage |
| **Flower Petals** | Flower Petals | Color Cartridges, decorative items |
| **Mycelia** | Mycelia | Fabric, Parachute, Medicinal Inhaler, Gas Mask |
| **Nutrients** | Bacon Agaric, Beryl Nut, Paleberry | Health items, consumables |
| **Power Slugs** | Power Slug (Blue/Yellow/Purple) | Power Shards, Overclocking |
| **Quartz** | Raw Quartz | Silica, Crystal Oscillators, Explorer vehicle |
| **Sulfur** | Sulfur | Black Powder, Nobelisks, Batteries |

#### Power Shard Research

| Slug Type | Shards Produced | Research Cost |
|-----------|-----------------|---------------|
| Blue Power Slug | 1 Power Shard | 1 Blue Slug |
| Yellow Power Slug | 2 Power Shards | 1 Yellow Slug |
| Purple Power Slug | 5 Power Shards | 1 Purple Slug |

### 3.4 Hard Drive Alternate Recipes

Crash Sites scattered across the map contain Hard Drives that unlock alternate recipes.

#### Hard Drive Statistics

| Specification | Value |
|--------------|-------|
| Total Crash Sites | 118 |
| Total Alternate Recipes | 113 (108 via Hard Drive + 5 via MAM) |
| Choices per Hard Drive | 2 random options (from unlocked pool) |
| Research Time | 10 minutes |

#### Notable Alternate Recipes

**Smelter/Foundry Alternates**:
| Recipe | Standard | Alternate | Benefit |
|--------|----------|-----------|---------|
| Iron Alloy Ingot | N/A | Foundry: Iron + Copper → 5 Ingots | 3x output vs standard |
| Copper Alloy Ingot | N/A | Foundry: Iron + Copper → 5 Ingots | 3x output vs standard |
| Steel Ingot (Coke) | N/A | Foundry: Iron + Petroleum Coke | No coal required |
| Steel Ingot (Compacted) | N/A | Foundry: Iron + Compacted Coal | More efficient |

**Constructor Alternates**:
| Recipe | Standard | Alternate | Benefit |
|--------|----------|-----------|---------|
| Steel Screw | 6 Iron Rod → 24 Screws | 5 Steel Beam → 260 Screws | 52 screws from 1 beam |
| Iron Wire | N/A | Iron Ingot → 9 Wire | Copper-free wire |
| Cast Screw | N/A | Iron Ingot → 20 Screws | Skip iron rod step |

**High-Value Alternates**:
| Recipe | Benefit |
|--------|---------|
| Stitched Iron Plate | Wire instead of Screws for Reinforced Iron Plate |
| Steel Rotor | All-iron rotor production |
| Pure Aluminum Ingot | More efficient bauxite processing |
| Turbo Blend Fuel | Most efficient fuel production |

---

## 4. Production Chains

### 4.1 Basic Component Production

#### Iron Processing

| Recipe | Machine | Input | Output | Time |
|--------|---------|-------|--------|------|
| Iron Ingot | Smelter | 1 Iron Ore | 1 Iron Ingot | 2s (30/min) |
| Iron Plate | Constructor | 3 Iron Ingot | 2 Iron Plate | 6s (20/min) |
| Iron Rod | Constructor | 1 Iron Ingot | 1 Iron Rod | 4s (15/min) |
| Screw | Constructor | 1 Iron Rod | 4 Screw | 6s (40/min) |

**Production Ratios**:
- 1 Smelter (30 ingots/min) → 1.5 Constructors for Plates (45 ingots/min needed for 30 plates)
- 1 Smelter (30 ingots/min) → 2 Constructors for Rods (30 rods/min)
- 1 Rod Constructor → 1 Screw Constructor (balanced at 15/min)

#### Copper Processing

| Recipe | Machine | Input | Output | Time |
|--------|---------|-------|--------|------|
| Copper Ingot | Smelter | 1 Copper Ore | 1 Copper Ingot | 2s (30/min) |
| Copper Sheet | Constructor | 2 Copper Ingot | 1 Copper Sheet | 6s (10/min) |
| Wire | Constructor | 1 Copper Ingot | 2 Wire | 4s (30/min) |
| Cable | Constructor | 2 Wire | 1 Cable | 2s (30/min) |

#### Reinforced Iron Plate

| Recipe | Machine | Input | Output | Time |
|--------|---------|-------|--------|------|
| Standard | Assembler | 6 Iron Plate + 12 Screw | 1 Reinforced Plate | 12s (5/min) |
| Stitched (Alt) | Assembler | 10 Iron Plate + 20 Wire | 3 Reinforced Plate | 32s (5.625/min) |
| Bolted (Alt) | Assembler | 18 Iron Plate + 50 Screw | 3 Reinforced Plate | 24s (7.5/min) |

### 4.2 Steel Production

#### Steel Ingot Production

| Recipe | Machine | Input | Output | Rate |
|--------|---------|-------|--------|------|
| Standard | Foundry | 3 Iron Ore + 3 Coal | 3 Steel Ingot | 45/min |
| Coke Steel (Alt) | Foundry | 15 Iron Ore + 15 Petroleum Coke | 20 Steel Ingot | 100/min |
| Compacted Steel (Alt) | Foundry | 6 Iron Ore + 3 Compacted Coal | 10 Steel Ingot | 37.5/min |

#### Steel Components

| Recipe | Machine | Input | Output | Rate |
|--------|---------|-------|--------|------|
| Steel Beam | Constructor | 4 Steel Ingot | 1 Steel Beam | 15/min |
| Steel Pipe | Constructor | 3 Steel Ingot | 2 Steel Pipe | 20/min |
| Encased Industrial Beam | Assembler | 3 Steel Beam + 6 Concrete | 1 | 6/min |

### 4.3 Motor and Rotor Production

#### Rotor

| Recipe | Machine | Input | Output | Rate |
|--------|---------|-------|--------|------|
| Standard | Assembler | 5 Iron Rod + 25 Screw | 1 Rotor | 4/min |
| Steel Rotor (Alt) | Assembler | 2 Steel Pipe + 6 Wire | 1 Rotor | 5/min |

#### Stator

| Recipe | Machine | Input | Output | Rate |
|--------|---------|-------|--------|------|
| Standard | Assembler | 3 Steel Pipe + 8 Wire | 1 Stator | 5/min |

#### Motor

| Recipe | Machine | Input | Output | Rate |
|--------|---------|-------|--------|------|
| Standard | Assembler | 2 Rotor + 2 Stator | 1 Motor | 5/min |

### 4.4 Modular Frame Production

#### Modular Frame

| Recipe | Machine | Input | Output | Rate |
|--------|---------|-------|--------|------|
| Standard | Assembler | 3 Reinforced Iron Plate + 12 Iron Rod | 2 | 2/min |
| Steeled Frame (Alt) | Assembler | 2 Reinforced Iron Plate + 10 Steel Pipe | 2 | 3/min |

#### Heavy Modular Frame

| Recipe | Machine | Input | Output | Rate |
|--------|---------|-------|--------|------|
| Standard | Manufacturer | 5 Modular Frame + 15 Steel Pipe + 5 Encased Industrial Beam + 100 Screw | 1 | 2/min |
| Heavy Encased Frame (Alt) | Manufacturer | 8 Modular Frame + 10 Encased Industrial Beam + 36 Steel Pipe + 22 Concrete | 3 | 2.8125/min |

### 4.5 Electronics Production

#### Circuit Board

| Recipe | Machine | Input | Output | Rate |
|--------|---------|-------|--------|------|
| Standard | Assembler | 2 Copper Sheet + 4 Plastic | 1 | 7.5/min |

#### Computer

| Recipe | Machine | Input | Output | Rate |
|--------|---------|-------|--------|------|
| Standard | Manufacturer | 10 Circuit Board + 9 Cable + 18 Plastic + 52 Screw | 1 | 2.5/min |

#### Supercomputer

| Recipe | Machine | Input | Output | Rate |
|--------|---------|-------|--------|------|
| Standard | Manufacturer | 2 Computer + 2 AI Limiter + 3 High-Speed Connector + 28 Plastic | 1 | 1.875/min |

### 4.6 Oil Processing

#### Primary Oil Recipes

| Recipe | Machine | Input | Output |
|--------|---------|-------|--------|
| Plastic | Refinery | 30 m³ Crude Oil | 20 Plastic + 10 m³ Heavy Oil Residue |
| Rubber | Refinery | 30 m³ Crude Oil | 20 Rubber + 20 m³ Heavy Oil Residue |
| Fuel | Refinery | 60 m³ Crude Oil | 40 m³ Fuel + 30 Polymer Resin |

#### Byproduct Processing

| Recipe | Machine | Input | Output |
|--------|---------|-------|--------|
| Residual Fuel | Refinery | 60 m³ Heavy Oil Residue | 40 m³ Fuel |
| Residual Plastic | Refinery | 60 m³ Heavy Oil Residue + 20 Polymer Resin | 20 Plastic |
| Residual Rubber | Refinery | 40 m³ Heavy Oil Residue + 40 Polymer Resin | 20 Rubber |
| Petroleum Coke | Refinery | 40 m³ Heavy Oil Residue | 120 Petroleum Coke |

### 4.7 Aluminum Production Chain

Aluminum requires a complex multi-step process:

1. **Alumina Solution**: Bauxite + Water → Alumina Solution + Silica (Refinery)
2. **Aluminum Scrap**: Alumina Solution + Coal → Aluminum Scrap + Water (Refinery)
3. **Aluminum Ingot**: Aluminum Scrap + Silica → Aluminum Ingot (Foundry)

| Step | Input | Output | Machine |
|------|-------|--------|---------|
| Alumina Solution | 120 Bauxite + 180 m³ Water | 120 m³ Alumina + 50 Silica | Refinery |
| Aluminum Scrap | 240 m³ Alumina + 120 Coal | 360 Aluminum Scrap + 120 m³ Water | Refinery |
| Aluminum Ingot | 90 Aluminum Scrap + 75 Silica | 60 Aluminum Ingot | Foundry |

### 4.8 Nuclear Production Chain

#### Uranium Processing

| Step | Recipe | Machine |
|------|--------|---------|
| 1 | Uranium Ore + Sulfuric Acid → Uranium Pellet + Sulfuric Acid | Refinery |
| 2 | Uranium Pellet + Concrete → Encased Uranium Cell | Assembler |
| 3 | Encased Uranium Cell + Electromagnetic Control Rod → Uranium Fuel Rod | Manufacturer |

#### Waste Processing

| Step | Recipe | Machine |
|------|--------|---------|
| 1 | Uranium Waste + Silica → Non-Fissile Uranium | Blender |
| 2 | Non-Fissile Uranium → Plutonium Pellet | Particle Accelerator |
| 3 | Plutonium Pellet → Encased Plutonium Cell → Plutonium Fuel Rod | Assembler, Manufacturer |
| 4 (Tier 9) | Plutonium Waste → Ficsonium (eliminates waste) | Converter |

---

## 5. World/Map

### 5.1 World Specifications

| Specification | Value |
|--------------|-------|
| Map Size | 47.1 km² (7.972 km x 6.8 km) |
| Generation Type | Hand-crafted (no procedural generation) |
| Total Biomes | 22 named regions |
| Cave Systems | 52+ caves |
| Vertical Range | Significant elevation changes, supports vertical factory building |

### 5.2 Starting Area Biomes

| Starting Area | Difficulty | Characteristics | Resource Highlights |
|---------------|------------|-----------------|---------------------|
| **Grass Fields** | Easy | Flat terrain, abundant coal, nearby lake | Good balance, easy coal power setup |
| **Rocky Desert** | Easy-Medium | Open terrain, good resources | Strong mid-game potential |
| **Northern Forest** | Medium | Dense vegetation, highest resource concentration | Best early expansion |
| **Dune Desert** | Hard | Hostile fauna, limited early resources | Most Pure nodes, best mega-factory location |

### 5.3 Biome Regions

| Biome | Characteristics | Unique Resources |
|-------|-----------------|------------------|
| Grass Fields | Flat, green, beginner-friendly | Standard ores |
| Rocky Desert | Orange terrain, open spaces | Abundant iron/copper |
| Northern Forest | Dense trees, caves | Caterium, Quartz |
| Dune Desert | Sand dunes, hostile creatures | Pure node concentration |
| Blue Crater | Distinctive blue coloring | Uranium |
| Swamp | Waterlogged, poisonous flora | Uranium, Nitrogen Wells |
| Spire Coast | Tall rock formations | Oil |
| Red Bamboo Fields | Red vegetation | Sulfur |
| Titan Forest | Massive trees | Caterium |
| Abyss Cliffs | Deep chasms | SAM |

### 5.4 Resource Node Distribution

| Resource | Impure | Normal | Pure | Total Nodes |
|----------|--------|--------|------|-------------|
| Iron Ore | 39 | 42 | 46 | 127 |
| Copper Ore | 13 | 29 | 13 | 55 |
| Limestone | 15 | 50 | 29 | 94 |
| Coal | 15 | 31 | 16 | 62 |
| Caterium | 4 | 12 | 8 | 24 |
| Raw Quartz | 5 | 14 | 7 | 26 |
| Sulfur | 3 | 10 | 6 | 19 |
| Bauxite | 5 | 6 | 6 | 17 |
| Uranium | 3 | 2 | 0 | 5 |
| Crude Oil | 10 | 12 | 8 | 30 |

**Maximum Theoretical Resource Extraction** (all nodes at 250% overclock with Mk.3 miners):
- Iron Ore: ~152,400/min
- Copper Ore: ~66,000/min
- Coal: ~74,400/min

### 5.5 Geyser Distribution

| Purity | Count | Average Output |
|--------|-------|----------------|
| Impure | 9 | 100 MW |
| Normal | 8 | 200 MW |
| Pure | 1 | 400 MW |

**Total Potential**: ~2.2 GW from all geysers

### 5.6 Exploration Features

| Feature | Description |
|---------|-------------|
| **Crash Sites** | 118 sites containing Hard Drives |
| **Somersloops** | 106 collectible alien artifacts |
| **Mercer Spheres** | Collectible for Dimensional Depot upgrades |
| **Power Slugs** | Found throughout world, unlock overclocking |
| **Collectible Nuts/Berries** | Health items in the wild |

---

## 6. Combat/Survival

### 6.1 Hostile Creatures

#### Hogs (Fluffy-tailed Hog variants)

| Type | Health | Damage | Behavior |
|------|--------|--------|----------|
| Fluffy-tailed Hog | 20 | 10 | Charges at player |
| Alpha Hog | 60 | 20 | Stronger charge attack |
| Cliff Hog | 40 | 15 | Found on cliffs/elevated areas |

#### Spitters (Plasma Spitter variants)

| Type | Health | Damage | Range |
|------|--------|--------|-------|
| Spitter (small) | 25 | 10 (fireball) | Medium |
| Spitter (large) | 50 | 15 (fireball) | Long |
| Alpha Spitter | 75 | 20 (fireball) | Long |

**Behavior**: Lob fireballs with predictive aiming; create damage-over-time puddles.

#### Stingers (Spider-like creatures)

| Type | Health | Damage | Location |
|------|--------|--------|----------|
| Stinger (small) | 25 | 10 | Caves, dark areas |
| Stinger (large) | 50 | 15 | Caves, dark areas |
| Stinger (elite) | 100 | 25 | Deep caves |

**Behavior**: Very fast movement, climb walls, attack in swarms, retreat when stamina depleted.

**Note**: Stingers and Hogs are hostile to each other.

### 6.2 Creature Hostility Settings

| Setting | Behavior |
|---------|----------|
| Default | Creatures attack when player enters line of sight |
| Passive | Creatures never attack (peaceful mode) |
| Retaliate | Creatures only attack when provoked |

### 6.3 Player Equipment

#### Weapons

| Weapon | Unlock | Damage | Characteristics |
|--------|--------|--------|-----------------|
| **Xeno-Zapper** | Starting | Low | Melee, infinite use |
| **Xeno-Basher** | Tier 2 | Medium | Melee, knockback |
| **Rebar Gun** | Tier 3 | High | Single-shot projectile |
| **Rifle** | Tier 5 (Sulfur research) | Variable | Automatic, uses ammo types |
| **Nobelisk Detonator** | Tier 5 (Sulfur research) | High | Remote explosives |

#### Nobelisk Types

| Type | Effect |
|------|--------|
| Standard | Direct damage explosion |
| Gas | Poison cloud |
| Shock | Electric damage |
| Cluster | Multiple smaller explosions |
| Nuke | Massive destruction radius |

#### Protective Equipment

| Equipment | Slot | Effect | Unlock |
|-----------|------|--------|--------|
| **Blade Runners** | Legs | +50% speed, +100% jump, fall damage reduction | Tier 3 |
| **Gas Mask** | Head | Poison immunity | MAM (Mycelia) |
| **Hazmat Suit** | Body | Radiation protection | Tier 7 |
| **Jetpack** | Body | Flight (fuel-based) | Tier 6 |
| **Hover Pack** | Body | Wireless powered flight near power lines | Tier 7 |

### 6.4 Health and Recovery

| Item | Effect | Source |
|------|--------|--------|
| Paleberry | Restore 10 HP | Forage |
| Beryl Nut | Restore 20 HP | Forage |
| Bacon Agaric | Restore 30 HP | Forage |
| Medicinal Inhaler | Restore 100 HP (rechargeable) | Craft (MAM) |

**Base Health**: 100 HP
**Regeneration**: Slow passive regeneration when out of combat

---

## 7. Endgame

### 7.1 Victory Condition

Satisfactory's endgame culminates in completing **Project Assembly** via the Space Elevator. After delivering all Phase 5 requirements and completing Tier 9 content, players can initiate the ending sequence.

### 7.2 Phase 5 and Ending Sequence

1. Complete all Tier 9 milestones
2. Deliver Phase 5 requirements to Space Elevator
3. Pull the final lever at the Space Elevator
4. Watch the ending cinematic: Ship assembly, wormhole creation, and departure
5. Credits roll
6. Continue playing in sandbox mode

### 7.3 Tier 9 Content

#### Key Unlocks

| Milestone | Unlocks |
|-----------|---------|
| Matter Conversion | Converter building (ore transmutation) |
| Quantum Encoding | Quantum Encoder building |
| Spatial Energy Regulation | Alien Power Augmenter |
| Peak Efficiency | Conveyor Belt Mk.6, Pipeline Mk.2 |
| Portal Infrastructure | Satellite Portal, Dimensional Depot |

#### Late-Game Buildings

| Building | Function | Power |
|----------|----------|-------|
| **Converter** | Transmute ores using Reanimated SAM | 200 MW |
| **Quantum Encoder** | Produce quantum-level components | 500-2000 MW |
| **Alien Power Augmenter** | +500 MW + 10% circuit boost | Uses Somersloops |
| **Satellite Portal** | Fast travel network | Variable |
| **Dimensional Depot** | Cloud storage accessible anywhere | Building + Uploaders |

### 7.4 Nuclear Waste Elimination

Tier 9 introduces **Ficsonium**, allowing complete nuclear waste elimination:

1. Process Plutonium Waste in Converter
2. Produce Ficsonium (radioactive but creates no waste)
3. Use Ficsonium Fuel Rods in Nuclear Power Plants
4. Achieve waste-free nuclear power

### 7.5 Alien Technology Integration

| Artifact | Total Available | Primary Use |
|----------|-----------------|-------------|
| **Somersloop** | 106 | Production amplification, Alien Power Augmenter |
| **Mercer Sphere** | Multiple | Dimensional Depot upgrades |
| **Power Slugs** | Many (respawning) | Power Shards for overclocking |

**Production Amplification Effects**:
- 100% output increase (double production)
- No input increase required
- ~300% power consumption increase

### 7.6 Post-Game Objectives

After the ending, players can continue with self-imposed challenges:

- Maximize theoretical production output
- Achieve 100% resource node utilization
- Complete all alternate recipe unlocks
- Build aesthetic/artistic factories
- Minimize pollution/environmental impact
- Achieve specific items/minute targets
- Complete the game again with different starting conditions

---

## 8. Multiplayer

### 8.1 Multiplayer Architecture

| Specification | Value |
|--------------|-------|
| Default Player Limit | 4 players |
| Maximum Players (PC config edit) | 127 (theoretical) |
| Console Crossplay Limit | 4 players (cannot be increased) |

### 8.2 Session Types

| Type | Description |
|------|-------------|
| **Player-Hosted** | One player hosts, others join |
| **Dedicated Server** | Headless server (PC only) |
| **Invite-Only** | Private session with invitations |
| **Friends-Only** | Visible to friends list |

### 8.3 Multiplayer Features

- **Shared World**: All players build in the same world
- **Shared Resources**: Resources and inventory accessible to all
- **Independent Progression**: Each player maintains their own unlocks (synced to host on join)
- **Ping System**: Mark locations for other players
- **Voice/Text Chat**: Built-in communication

### 8.4 Dedicated Server Setup

Available on PC for always-online worlds:
- Requires config file editing for increased player counts
- Can run on separate hardware
- World persists when players disconnect
- Admin controls for player management

### 8.5 Crossplay

| Platform Combination | Supported |
|---------------------|-----------|
| PC (Steam) ↔ PC (Epic) | Yes |
| PC ↔ Console | Yes |
| Console ↔ Console | Yes |

**Note**: Console crossplay limited to 4 players maximum regardless of host configuration.

---

## Appendix: Quick Reference Tables

### Belt Throughput Summary

| Belt | Items/Minute | Typical Use Case |
|------|--------------|------------------|
| Mk.1 | 60 | Early game, low-volume lines |
| Mk.2 | 120 | Single miner output (normal node) |
| Mk.3 | 270 | Multiple miner merges |
| Mk.4 | 480 | High-volume production |
| Mk.5 | 780 | Main bus systems |
| Mk.6 | 1,200 | Maximum throughput applications |

### Power Generation Summary

| Method | Output per Unit | Space Efficiency | Automation |
|--------|-----------------|------------------|------------|
| Biomass Burner | 30 MW | Low | Manual only |
| Coal Generator | 75 MW | Medium | Full |
| Fuel Generator | 250 MW | High | Full |
| Geothermal | 100-600 MW | High | Passive |
| Nuclear | 2,500 MW | Very High | Full |

### Miner Output by Purity (at 100%)

| Miner | Impure | Normal | Pure |
|-------|--------|--------|------|
| Mk.1 | 30/min | 60/min | 120/min |
| Mk.2 | 60/min | 120/min | 240/min |
| Mk.3 | 120/min | 240/min | 480/min |
| Mk.3 @250% | 300/min | 600/min | 1,200/min |

### Machine Power Consumption

| Machine | Power (MW) |
|---------|------------|
| Smelter | 4 |
| Constructor | 4 |
| Assembler | 15 |
| Foundry | 16 |
| Manufacturer | 55 |
| Refinery | 30 |
| Blender | 75 |
| Packager | 10 |
| Particle Accelerator | 250-1500 |
| Quantum Encoder | 500-2000 |
| Converter | 200 |

### Space Elevator Phase Requirements Summary

| Phase | Key Items | Total Items | Unlocks |
|-------|-----------|-------------|---------|
| 1 | Smart Plating | 50 | Tiers 3-4 |
| 2 | Smart Plating, Versatile Framework, Automated Wiring | 1,100 | Tiers 5-6 |
| 3 | Versatile Framework, Modular Engine, Adaptive Control Unit | 3,100 | Tiers 7-8 |
| 4 | Assembly Director System, Magnetic Field Generator, Nuclear Pasta, Thermal Propulsion Rocket | 10,000 | Tier 9 |
| 5 | Final delivery | Variable | Game Ending |

---

*Document compiled for game design research purposes. Data sourced from official Satisfactory Wiki, community resources, and in-game mechanics as of version 1.0 (September 2024 release).*
