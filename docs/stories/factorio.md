# Factorio: Comprehensive Game Design Research Document

## Table of Contents
1. [Initial Conditions](#1-initial-conditions)
2. [Core Mechanics](#2-core-mechanics)
3. [Technology/Research Tree](#3-technologyresearch-tree)
4. [Production Chains](#4-production-chains)
5. [World/Map](#5-worldmap)
6. [Combat/Survival](#6-combatsurvival)
7. [Endgame](#7-endgame)
8. [Multiplayer and Mods](#8-multiplayer-and-mods)

---

## 1. Initial Conditions

### Crash Landing Scenario

The player begins Factorio as a survivor of a space crash landing on an alien planet. The narrative establishes that the player's spacecraft has crashed, leaving them stranded with limited supplies scattered across the crash site. The primary objective is to survive, harvest local resources, and ultimately build a rocket to escape the planet (or in Space Age, expand across multiple worlds).

### Starting Inventory

In **Freeplay mode** (the standard game mode), the player starts with minimal equipment:

| Item | Quantity | Purpose |
|------|----------|---------|
| Burner Mining Drill | 1 | Initial resource extraction |
| Stone Furnace | 1 | Smelting ore into plates |
| Wood/Coal | Small amount | Initial fuel source |

The crash site may also contain additional items scattered nearby that can be recovered, though players must be careful not to ignite themselves on burning wreckage.

### Initial Map State

- The player spawns in a guaranteed **starting area** that contains:
  - Iron ore deposit
  - Copper ore deposit
  - Coal deposit
  - Stone deposit
- **Excluded from starting area**: Uranium ore and crude oil (to prevent early-game complexity)
- The starting area is typically free of enemy nests within a configurable radius
- Basic trees and rocks are present for emergency wood and stone collection

### Tutorial and Campaign Structure

Factorio offers multiple entry points for new players:

1. **Interactive Tutorial**: A drip-fed introduction that teaches core mechanics progressively
2. **Freeplay Mode**: The main sandbox experience with full freedom
3. **Scenario Campaigns**: Pre-built scenarios with specific objectives
4. **Demo**: A free limited version introducing basic concepts

The game does not force tutorial completion - experienced players can jump directly into freeplay.

---

## 2. Core Mechanics

### 2.1 Resource Types

Factorio features several raw resource types, each with specific properties and uses:

#### Primary Resources

| Resource | Mining Time | Uses | Notes |
|----------|-------------|------|-------|
| **Iron Ore** | 1 second | Smelts to iron plates; foundation of most recipes | Most consumed resource |
| **Copper Ore** | 1 second | Smelts to copper plates; electronics and wiring | Second most consumed |
| **Coal** | 1 second | Fuel for boilers, smelters; plastic/explosives production | Dual-purpose resource |
| **Stone** | 1 second | Stone bricks, walls, furnaces, rails | Defensive and infrastructure |
| **Crude Oil** | N/A (pumped) | Petroleum products, plastics, sulfur | Requires pumpjacks |
| **Uranium Ore** | 2 seconds | Nuclear fuel, atomic weapons | Requires sulfuric acid to mine |
| **Water** | N/A (infinite) | Steam power, oil processing, concrete | Extracted via offshore pumps |

#### Secondary/Processed Resources

- **Petroleum Gas**: Primary oil product; plastics, sulfur
- **Light Oil**: Solid fuel (most efficient), rocket fuel
- **Heavy Oil**: Lubricant, can be cracked down
- **Sulfuric Acid**: Uranium mining, battery production, blue circuits
- **Lubricant**: Required for express belts, electric engines

### 2.2 Mining and Drilling Systems

#### Mining Drill Types

| Drill Type | Mining Speed | Coverage Area | Power | Pollution | Notes |
|------------|--------------|---------------|-------|-----------|-------|
| **Burner Mining Drill** | 0.25/s | 2x2 tiles | 150 kW (fuel) | 12/min | Starting equipment |
| **Electric Mining Drill** | 0.5/s | 5x5 tiles | 90 kW | 10/min | Standard automation |
| **Big Mining Drill** (Space Age) | 2.5/s | 13x13 tiles | 300 kW | 40/min | 50% resource drain |

#### Mining Formula

```
Production Rate = Mining Speed / Mining Time
```

For hand mining:
```
Production Rate = (1 + Mining Speed Modifier) x 0.5 / Mining Time
```

#### Resource Depletion

- Resources have finite quantities per tile
- Burner and electric drills: 100% resource drain (1:1 output to depletion)
- Big mining drill: 50% resource drain (doubles effective patch life)
- Mining productivity research increases output without additional drain

#### Special Mining Requirements

- **Uranium ore**: Requires sulfuric acid fed to electric mining drills
- **Crude oil**: Requires pumpjacks; yield decreases to minimum 20% over time

### 2.3 Power Generation

#### Steam Power (Coal/Solid Fuel)

| Component | Output/Consumption | Ratio |
|-----------|-------------------|-------|
| Offshore Pump | 1,200 water/second | 1 pump : 20 boilers |
| Boiler | Consumes 1.8 MW fuel, produces steam | 1 boiler : 2 steam engines |
| Steam Engine | 900 kW output | - |

**Standard Setup**: 1 offshore pump feeds 20 boilers and 40 steam engines = 36 MW

#### Solar Power

| Component | Specification |
|-----------|---------------|
| Solar Panel | 60 kW peak output (daytime only) |
| Accumulator | 5 MJ storage capacity |

**Optimal Ratio**: 0.84 accumulators per solar panel (approximately 21:25)

**For 1 MW continuous power**:
- 23.8 solar panels
- 20 accumulators (approximately)

**Day/Night Cycle** (default):
- Day: 12,500/60 seconds
- Dawn/Dusk: 5,000/60 seconds each
- Night: 2,500/60 seconds

#### Nuclear Power

| Component | Specification |
|-----------|---------------|
| Nuclear Reactor | 40 MW base output |
| Heat Exchanger | Consumes 10 MW heat, produces 103 steam/s |
| Steam Turbine | 5.82 MW output |

**Key Ratios**:
- 1 reactor : 4 heat exchangers (without neighbor bonus)
- 1 heat exchanger : ~1.72 steam turbines
- 2 offshore pumps : 233 heat exchangers : 400 steam turbines (large setup)

**Neighbor Bonus**: Each adjacent active reactor provides +100% output
- 2x2 reactor grid: 480 MW total (120 MW each with 2 neighbors)

**Fuel Cycle**:
- Uranium processing: 10,000 ore yields ~7 U-235 and 993 U-238
- Fuel cell: 1 U-235 + 19 U-238 + 10 iron plates = 10 fuel cells
- Each fuel cell: 8 GJ energy, consumed in 200 seconds per reactor

**Kovarex Enrichment Process**:
- Input: 40 U-235 + 5 U-238
- Output: 41 U-235 + 2 U-238
- Time: 60 seconds
- Net effect: Converts 3 U-238 into 1 U-235

### 2.4 Inserters and Belt Systems

#### Inserter Types

| Inserter | Rotation Speed | Items/Second (Chest-to-Chest) | Power Drain |
|----------|---------------|-------------------------------|-------------|
| Burner Inserter | 0.013 rot/tick | 0.79 | Fuel-based |
| Inserter | 0.014 rot/tick | 0.86 | 4.2 kW idle |
| Long-handed Inserter | 0.02 rot/tick | 1.2 | 4.2 kW idle |
| Fast Inserter | 0.04 rot/tick | 2.5 | 13.3 kW idle |
| Bulk Inserter | 0.04 rot/tick | 4.8 | 13.3 kW idle |
| Stack Inserter (Space Age) | 0.04 rot/tick | Variable | 96 kW active |

**Capacity Bonuses**: Research can increase stack size for bulk/stack inserters up to 12 items per swing.

#### Belt Transport System

| Belt Type | Speed (items/sec) | Underground Max Distance | Research Required |
|-----------|-------------------|-------------------------|-------------------|
| Transport Belt (Yellow) | 15 | 4 tiles | None |
| Fast Transport Belt (Red) | 30 | 6 tiles | Logistics 2 |
| Express Transport Belt (Blue) | 45 | 8 tiles | Logistics 3 |
| Turbo Transport Belt (Green) | 60 | 10 tiles | Space Age DLC |

**Belt Properties**:
- All belts have 2 lanes
- Fully compressed belt holds 8 items per tile
- Splitters divide input 1:1 between outputs (configurable priority/filtering)

#### Underground Belts

Allow belt lines to pass under obstacles:
- Maintain same throughput as surface belts
- Can cross other underground belts perpendicularly
- Entrance/exit pairs must match tier

### 2.5 Assembling Machines

| Machine | Crafting Speed | Module Slots | Power |
|---------|---------------|--------------|-------|
| Assembling Machine 1 | 0.5x | 0 | 75 kW |
| Assembling Machine 2 | 0.75x | 2 | 150 kW |
| Assembling Machine 3 | 1.25x | 4 | 375 kW |

**Crafting Time Calculation**:
```
Actual Time = Recipe Time / (Crafting Speed x (1 + Speed Bonus))
```

**Maximum Crafting Speed** (with 12 beacons, Speed Module 3): 11.25x

**Modules**:
- **Speed Modules**: +20%/+30%/+50% speed, increased power consumption
- **Efficiency Modules**: -30%/-40%/-50% power consumption
- **Productivity Modules**: +4%/+6%/+10% free output, speed penalty, power increase

### 2.6 Fluid Handling

#### Components

| Component | Function | Throughput |
|-----------|----------|------------|
| Offshore Pump | Extracts water | 1,200/s |
| Pumpjack | Extracts crude oil | Variable (yield-dependent) |
| Pipe | Transports fluids | ~1,000/s (short distance) |
| Underground Pipe | Passes under obstacles | Max 10 tiles |
| Storage Tank | Stores 25,000 fluid | Buffer/pressure management |
| Pump | Boosts flow, prevents backflow | 12,000/s |

#### Fluid Mechanics

- Fluids flow based on pressure differentials
- Longer pipe runs reduce throughput
- **Rule of thumb**: Place pumps every 17 pipes to maintain 1,200/s flow
- Pumps act as one-way valves when unpowered

#### Oil Refinery

| Recipe | Input | Output | Time |
|--------|-------|--------|------|
| Basic Oil Processing | 100 crude oil | 45 petroleum gas | 5s |
| Advanced Oil Processing | 100 crude oil + 50 water | 25 heavy + 45 light + 55 petroleum | 5s |
| Coal Liquefaction | 10 coal + 25 heavy oil + 50 steam | 90 heavy + 20 light + 10 petroleum | 5s |

#### Cracking Recipes (Chemical Plant)

| Recipe | Input | Output | Time |
|--------|-------|--------|------|
| Heavy Oil Cracking | 40 heavy oil + 30 water | 30 light oil | 2s |
| Light Oil Cracking | 30 light oil + 30 water | 20 petroleum gas | 2s |

**Optimal Petroleum Ratio**: 20:5:17 (advanced processing : heavy cracking : light cracking)

### 2.7 Logistics Systems (Robots)

#### Robot Types

| Robot | Speed | Purpose |
|-------|-------|---------|
| Logistic Robot | 3 tiles/s (base) | Item transport between chests |
| Construction Robot | 3.6 tiles/s (base) | Building, repairing, deconstructing |

**Robot Energy**: 1.5 MJ storage, 3 kW flight consumption + 5 kJ per tile traveled

#### Chest Types

| Chest | Color | Function | Priority |
|-------|-------|----------|----------|
| Passive Provider | Red | Supplies items when requested | Lowest |
| Active Provider | Purple | Forces immediate emptying | Highest |
| Storage | Yellow | General bot-accessible storage | Medium |
| Requester | Blue | Requests specific items | N/A (destination) |
| Buffer | Green | Hybrid requester/provider | Medium |

**Priority Order** (source): Active Provider > Storage > Buffer > Passive Provider

#### Roboport

| Specification | Value |
|--------------|-------|
| Logistics Area | 50x50 tiles |
| Construction Area | 110x110 tiles |
| Charging Slots | 4 |
| Charging Rate | 1 MW per slot |
| Internal Battery | 100 MJ |
| Robots Charged/Minute | 50-70 |

### 2.8 Train Systems

#### Components

- **Locomotive**: Provides movement power
- **Cargo Wagon**: Holds 40 item stacks
- **Fluid Wagon**: Holds 25,000 fluid
- **Artillery Wagon**: Mobile artillery platform

#### Signals

| Signal Type | Function |
|-------------|----------|
| Rail Signal | Creates block boundary; trains wait if block occupied |
| Chain Signal | Reserves path through multiple blocks; prevents intersection blocking |

**Golden Rule**: "Chain signal before intersection, rail signal after intersection"

**Block Spacing**: Minimum distance between signals should fit the longest train

#### Train Scheduling

**Wait Conditions** (15 types including):
- Time passed
- Cargo full/empty
- Item/fluid count threshold
- Circuit condition
- Inactivity

**Train Limits**: Stations can limit concurrent trains to prevent congestion

**Interrupts** (Space Age): Global conditions that override normal schedules

### 2.9 Circuit Network

#### Wires

- **Red Wire**: Carries signals (no power)
- **Green Wire**: Carries signals (no power)
- Signals on same-color wires are combined
- Different wire colors keep signals separate

#### Combinators

| Combinator | Function |
|------------|----------|
| Constant Combinator | Outputs fixed signals continuously |
| Arithmetic Combinator | Performs math operations (+, -, *, /, %, ^, shifts) |
| Decider Combinator | Outputs based on conditional logic (>, <, =, !=) |

#### Common Applications

- Controlling inserters based on chest contents
- Train station management
- Power monitoring and switching
- Belt balancing and overflow detection
- Display systems (using lamps)

---

## 3. Technology/Research Tree

### 3.1 Science Pack Types

| Science Pack | Tier | Crafting Time | Recipe |
|--------------|------|---------------|--------|
| Automation (Red) | I | 5 seconds | 1 copper plate + 1 iron gear wheel |
| Logistic (Green) | II | 6 seconds | 1 transport belt + 1 inserter |
| Military (Black) | III | 10 seconds | 1 piercing rounds + 1 grenade + 2 walls = **2 packs** |
| Chemical (Blue) | IV | 24 seconds | 1 sulfur + 3 advanced circuits + 2 engine units = **2 packs** |
| Production (Purple) | V | 21 seconds | 30 rails + 1 electric furnace + 1 productivity module = **3 packs** |
| Utility (Yellow) | VI | 21 seconds | 2 processing units + 1 flying robot frame + 3 low density structures = **3 packs** |
| Space (White) | VII | N/A | 1,000 packs per satellite launch |

### 3.2 Raw Material Costs Per Science Pack

| Science Pack | Iron | Copper | Coal | Stone | Crude Oil |
|--------------|------|--------|------|-------|-----------|
| Automation | 2 | 1 | - | - | - |
| Logistic | 5.5 | 1.5 | - | - | - |
| Military | 5.75 | 0.5 | 5 | 10 | - |
| Chemical | 12 | 7.5 | 1.5 | - | 38.46 |
| Production | 52.5 | 19.17 | 3.33 | 11.67 | 68.38 |
| Utility | 33.33 | 49.83 | 3.83 | - | 106.84 |
| Space | 57.54 | 101.79 | 9.95 | - | 306.92 |

### 3.3 Space Age Expansion Science Packs

| Science Pack | Planet | Recipe | Crafting Time |
|--------------|--------|--------|---------------|
| Metallurgic | Vulcanus | 200 molten copper + 3 tungsten carbide + 2 tungsten plates | 10s |
| Electromagnetic | Fulgora | 1 accumulator + 25 electrolyte + 25 holmium solution + 1 supercapacitor | 10s |
| Agricultural | Gleba | 1 bioflux + 1 pentapod egg | 4s |
| Cryogenic | Aquilo | 6 fluoroketone (cold) + 3 ice + 1 lithium plate | 20s |
| Promethium | Space Platform | 10 biter eggs + 25 promethium chunks + 1 quantum processor = **10 packs** | 5s |

### 3.4 Key Technology Unlocks

#### Early Game (Red + Green Science)

| Technology | Unlocks | Packs Required |
|------------|---------|----------------|
| Automation | Assembling Machine 1 | 10 Red |
| Logistics | Inserters, Belts | 20 Red |
| Electronics | Lab upgrades | 30 Red |
| Steel Processing | Steel plates | 50 Red + 50 Green |
| Advanced Material Processing | Steel furnaces | 75 Red + 75 Green |
| Oil Processing | Refinery, Chemical Plant | 100 Red + 100 Green |

#### Mid Game (+ Blue Science)

| Technology | Unlocks | Packs Required |
|------------|---------|----------------|
| Advanced Oil Processing | Cracking, better yields | 75 each |
| Electric Engine | Express belts, bots | 50 each |
| Robotics | Construction/Logistic bots | 100 each |
| Logistics 3 | Express belts | 300 each |
| Nuclear Power | Reactors, centrifuges | 1,000+ each |

#### Late Game (+ Purple + Yellow Science)

| Technology | Unlocks | Packs Required |
|------------|---------|----------------|
| Kovarex Enrichment | Efficient U-235 production | High |
| Rocket Silo | Win condition structure | Very High |
| Spidertron | Ultimate combat vehicle | Very High |
| Artillery | Long-range nest destruction | High |

### 3.5 Infinite Research Technologies

These technologies can be researched indefinitely, following mathematical progressions:

| Technology | Effect Per Level | Progression Type |
|------------|-----------------|------------------|
| Mining Productivity | +10% miner output | Arithmetic |
| Follower Robot Count | +5 robots | Arithmetic |
| Worker Robot Speed | +65% speed | Geometric (powers of 2) |
| Physical/Laser/Artillery Damage | Variable % | Geometric |
| Artillery Range | +30% range | Geometric |
| Research Speed | +60% lab speed | Geometric |

**Hard Cap** (Space Age): 300% maximum productivity on all recipes (prevents infinite resource exploits)

---

## 4. Production Chains

### 4.1 Circuit Production

#### Electronic Circuit (Green Circuit)

```
Recipe: 1 iron plate + 3 copper cables = 1 green circuit (0.5s)
Copper Cable: 1 copper plate = 2 copper cables (0.5s)
```

**Production Ratio**: 3 copper cable assemblers : 2 green circuit assemblers

**Raw Materials per Green Circuit**: 1 iron plate + 1.5 copper plates

#### Advanced Circuit (Red Circuit)

```
Recipe: 2 green circuits + 2 plastic bars + 4 copper cables = 1 red circuit (6s)
```

**Raw Materials per Red Circuit**:
- 4 iron plates
- 8 copper plates
- Plastic (petroleum-based)

#### Processing Unit (Blue Circuit)

```
Recipe: 20 green circuits + 2 red circuits + 5 sulfuric acid = 1 processing unit (10s)
```

**Raw Materials per Blue Circuit**:
- 20 green circuits (20 iron, 30 copper)
- 2 red circuits (8 iron, 16 copper, plastic)
- 5 sulfuric acid (iron, sulfur)

**Practical Ratios**: For 1 blue circuit/second, need 20 red/s and 200 green/s feeding the line

### 4.2 Rocket Components

#### Low Density Structure

```
Recipe: 20 copper plates + 2 plastic bars + 2 steel plates = 1 LDS (20s)
```

#### Rocket Fuel

```
Recipe: 10 light oil = 1 solid fuel (2s)
Recipe: 10 solid fuel = 1 rocket fuel (30s)
```

#### Rocket Control Unit

```
Recipe: 1 processing unit + 1 speed module = 1 RCU (30s)
```

#### Rocket Part

```
Recipe: 10 LDS + 10 rocket fuel + 10 processing units = 1 rocket part (3s)
```

**Total Rocket Requirements** (Base Game - 100 parts):
- 1,000 Low Density Structures
- 1,000 Rocket Fuel
- 1,000 Processing Units (as part of Rocket Control Units)

**Space Age** (50 parts): Requirements halved

### 4.3 Satellite Production

```
Recipe: 100 LDS + 100 solar panels + 100 accumulators + 5 processing units + 100 radar = 1 satellite
```

### 4.4 Key Production Ratios

| Production | Optimal Ratio |
|------------|---------------|
| Copper Cable : Green Circuit | 3:2 |
| Iron Plate : Steel Plate | 5:1 (time-based) |
| Oil Refinery : Heavy Cracking : Light Cracking | 20:5:17 |
| Boiler : Steam Engine | 1:2 |
| Solar Panel : Accumulator | 25:21 (close to 1:0.84) |
| Nuclear Reactor : Heat Exchanger (no bonus) | 1:4 |
| Heat Exchanger : Steam Turbine | 1:1.72 |

---

## 5. World/Map

### 5.1 World Generation Parameters

#### Resource Settings

| Parameter | Effect | Default |
|-----------|--------|---------|
| Frequency | Number of patches per area | 100% |
| Size | Surface area of each patch | 100% |
| Richness | Yield per tile | 100% |

**Note**: Richness naturally increases with distance from spawn

#### Terrain Settings

| Parameter | Effect |
|-----------|--------|
| Water Scale | Size of water bodies |
| Water Coverage | Percentage of map covered by water |
| Moisture Bias | Grass vs desert ratio |
| Terrain Type Bias | Red desert vs sand distribution |
| Cliff Frequency | Number of cliff formations |
| Cliff Continuity | Length of cliff lines |

### 5.2 Biome Types

| Biome | Characteristics |
|-------|-----------------|
| Grassland | Green terrain, abundant trees |
| Desert | Sandy/red terrain, sparse vegetation |
| Red Desert | Rust-colored terrain |
| Water | Lakes, oceans, rivers |

### 5.3 Resource Distribution

**Guaranteed Starting Resources**:
- Iron ore
- Copper ore
- Coal
- Stone

**Excluded from Starting Area**:
- Uranium ore
- Crude oil

**Distance Scaling**: Resources become richer further from spawn point

### 5.4 Map Size and Generation

| Specification | Value |
|--------------|-------|
| Maximum Map Size | 2,000 x 2,000 kilometers |
| Maximum Tiles | ~4 trillion |
| Generation Method | Procedural, seed-based |
| Chunk Size | Generates in chunks as explored |
| Preload Radius | 3 chunks around explored areas |

**Practical Limits**: RAM consumption from generated chunks typically limits practical map size before reaching theoretical maximum.

### 5.5 Map Generation Presets

| Preset | Description |
|--------|-------------|
| Default | Standard balanced settings |
| Rich Resources | Higher richness for abundant materials |
| Marathon | Extended recipe costs, longer gameplay |
| Death World | 200% enemy frequency, faster evolution |
| Death World Marathon | Combined difficulty |
| Rail World | Larger resource patches, spread apart |
| Ribbon World | 128-tile height limit, 300% resources |
| Island | Single island surrounded by ocean |

---

## 6. Combat/Survival

### 6.1 Enemy Types

#### Biters (Nauvis)

| Type | Health | Damage | Evolution Threshold |
|------|--------|--------|---------------------|
| Small Biter | 15 | 7 | 0.0 |
| Medium Biter | 75 | 15 | 0.2 |
| Big Biter | 375 | 30 | 0.4 |
| Behemoth Biter | 3,000 | 90 | 0.9 |

#### Spitters (Nauvis)

| Type | Health | Evolution Threshold |
|------|--------|---------------------|
| Small Spitter | 10 | 0.0 |
| Medium Spitter | 50 | 0.2 |
| Big Spitter | 200 | 0.4 |
| Behemoth Spitter | 1,500 | 0.9 |

**Spitter Mechanics**: Ranged acid attacks with predictive aiming, create damage-over-time puddles

#### Worms

| Type | Health | Range | Evolution Threshold |
|------|--------|-------|---------------------|
| Small Worm | 200 | 20 tiles | 0.0 |
| Medium Worm | 400 | 24 tiles | 0.3 |
| Big Worm | 750 | 28 tiles | 0.5 |
| Behemoth Worm | 1,500 | 32 tiles | 0.9 |

**Worm Mechanics**: Static defensive creatures with high regeneration and fire resistance

### 6.2 Evolution Mechanics

Evolution factor ranges from 0 to 1, affecting enemy spawn ratios.

#### Evolution Sources

| Source | Rate | Notes |
|--------|------|-------|
| Time | 0.000004 per second | ~1.5% per hour |
| Pollution | 0.0000009 per unit produced | Global, not just absorbed |
| Nest Destruction | 0.002 per nest | Spawners and worms |

**Diminishing Returns Formula**:
```
Actual Increase = Base Increase x (1 - evolution_factor)^2
```

This means evolution slows as it approaches 1.0.

### 6.3 Pollution Mechanics

#### Pollution Generation

| Source | Pollution/Minute |
|--------|------------------|
| Boiler | 30 |
| Burner Mining Drill | 12 |
| Electric Mining Drill | 10 |
| Stone Furnace | 2 |
| Steel Furnace | 4 |
| Assembling Machine 1/2/3 | 4/3/2 |
| Oil Refinery | 6 |
| Chemical Plant | 4 |

#### Pollution Spread and Absorption

- Spreads across chunks at configurable rate
- Trees absorb pollution (and may die from excess)
- Enemy nests absorb pollution to spawn attack parties
- Attacks launched every 1-10 minutes (random) from polluted nests

### 6.4 Defensive Structures

#### Walls

| Structure | Health |
|-----------|--------|
| Stone Wall | 350 |
| Gate | 350 (opens for players/vehicles) |

#### Turrets

| Turret | Range | Damage | Notes |
|--------|-------|--------|-------|
| Gun Turret | 18 tiles | Variable (ammo-dependent) | Highest single-target DPS |
| Laser Turret | 24 tiles | 20/shot | No ammo, uses power |
| Flamethrower Turret | 30 tiles (min 6) | Highest total damage | Uses fluid fuel, area damage |
| Artillery Turret | 224 tiles (min 32) | Massive | Cannot auto-target units |

#### Dragon's Teeth Defense Pattern

Standard defensive layout:
1. Outer wall with gaps (slows enemies)
2. Inner wall (solid barrier)
3. Mixed turret line behind walls
4. Flame turrets for area denial
5. Artillery for nest destruction

### 6.5 Military Technology Progression

#### Personal Weapons

| Weapon | Tier | Notes |
|--------|------|-------|
| Pistol | Starting | Weak but reliable |
| Submachine Gun | Early | Rapid fire |
| Shotgun | Early | Close range burst |
| Combat Shotgun | Mid | Improved shotgun |
| Rocket Launcher | Mid | Area damage |
| Flamethrower | Mid | Area denial |
| Atomic Bomb | Late | Massive destruction |

#### Vehicles

| Vehicle | Purpose |
|---------|---------|
| Car | Fast personal transport |
| Tank | Heavy combat vehicle |
| Spidertron | Ultimate all-terrain combat vehicle, can use atomic bombs |

#### Armor Progression

1. Light Armor
2. Heavy Armor
3. Modular Armor (equipment grid)
4. Power Armor
5. Power Armor MK2

#### Equipment Grid Items

- Personal Roboport
- Exoskeletons (speed boost)
- Personal Laser Defense
- Energy Shield
- Night Vision
- Battery/Fusion Reactor

---

## 7. Endgame

### 7.1 Rocket Silo

#### Base Game Requirements

| Specification | Value |
|--------------|-------|
| Size | 9x9 tiles |
| Power | 4 MW |
| Rocket Parts Required | 100 |
| Crafting Time per Part | 3 seconds |

#### Rocket Part Recipe

```
10 Low Density Structure + 10 Rocket Fuel + 10 Processing Unit = 1 Rocket Part
```

**Total for One Rocket**:
- 1,000 Low Density Structures
- 1,000 Rocket Fuel
- 1,000 Processing Units

### 7.2 Satellite Launch

- Launching a satellite returns **1,000 Space Science Packs**
- Packs arrive at cargo landing pad ~29 seconds after launch
- Output slot maximum: 2,000 packs (excess lost)
- Can enable "Send to orbit automatically" for hands-off operation

### 7.3 Victory Condition (Base Game)

Launching a single rocket with a satellite triggers the victory screen. Players can continue playing indefinitely afterward.

### 7.4 Space Age Expansion Endgame

#### Space Platforms

- Flying factories for interplanetary travel
- Must defend against asteroids
- Process asteroid chunks for fuel and ammunition
- Transport cargo and the player between planets

#### New Planets

| Planet | Unique Mechanics | Key Resources |
|--------|------------------|---------------|
| **Vulcanus** | Volcanic, waterless, lava pools | Tungsten, molten metals |
| **Fulgora** | Lightning storms, scrap recycling | Holmium, scrap from dead civilization |
| **Gleba** | Humid marshlands, organic tech | Bioflux, pentapod eggs, spoilage mechanics |
| **Aquilo** | Frozen ammonia ocean, heat management | Lithium, cryogenic materials |

#### Quality System

Items and machines can have 5 quality levels:
- Normal
- Uncommon
- Rare
- Epic
- Legendary

Higher quality provides:
- Faster machine operation
- More powerful equipment
- Better crafting results

### 7.5 Infinite Research

Post-launch gameplay focuses on infinite research:

| Research | Benefit |
|----------|---------|
| Mining Productivity | +10% per level (no resource drain increase) |
| Robot Speed | Faster logistics/construction |
| Weapon Damage | Stronger military |
| Artillery Range | Extended nest clearance |

---

## 8. Multiplayer and Mods

### 8.1 Multiplayer Mechanics

#### Architecture

- **Deterministic Simulation**: All clients simulate identical game state
- **Server Role**: Proxies player inputs, ensures tick synchronization
- **No State Transfer**: Game state too large/dynamic; only inputs transmitted

#### Connection Types

| Type | Description |
|------|-------------|
| Direct Connect | Join via IP address |
| Steam Friends | Join through Steam |
| Public Server List | Browse available games |
| Dedicated Server | Headless server (no graphics) |

#### Latency Management

**Latency Hiding** (since v0.12):
- Simulates player actions locally
- Masks network delay for common interactions
- Server arbitrates conflicts

**Negotiation**:
- Server estimates round-trip delay per client
- Adjusts latency compensation every 5 seconds
- Built on custom reliable UDP layer

#### Synchronization

- If server lacks a player's input, proceeds without it
- Lagging players don't slow down others
- Client catches up when connection stabilizes

### 8.2 Modding Ecosystem

#### Mod Portal

- Official distribution platform: [mods.factorio.com](https://mods.factorio.com)
- In-game mod manager
- Automatic dependency resolution
- Version compatibility checking

#### Modding API

**Three Stages**:

| Stage | Purpose | Timing |
|-------|---------|--------|
| Settings | Configure mod options | Game launch |
| Prototype | Define items, recipes, entities | Before game load |
| Runtime | Event handlers, gameplay logic | During play |

**Language**: Custom Lua implementation

#### Mod Categories

| Category | Count (Approximate) | Examples |
|----------|---------------------|----------|
| Total Conversion | 144 | Space Exploration, Krastorio 2 |
| Quality of Life | 1,095 | Squeak Through, Even Distribution |
| New Resources/Machines | 495 | Angel's Ores, Bob's Mods |
| Production Chains | 1,233 | Extended vanilla production |
| Libraries | 258 | Shared code for other mods |

#### Popular Mod Combinations

**Bob's + Angel's**: Massively expanded production chains
**Krastorio 2**: Rebalanced progression with new mechanics
**Space Exploration**: Post-rocket interplanetary gameplay (pre-Space Age)
**Industrial Revolution**: Steampunk-themed progression

---

## Appendix: Quick Reference Tables

### Belt Throughput Summary

| Belt | Items/Second | Items/Minute |
|------|--------------|--------------|
| Yellow | 15 | 900 |
| Red | 30 | 1,800 |
| Blue | 45 | 2,700 |
| Turbo (Space Age) | 60 | 3,600 |

### Power Generation Summary

| Method | Output | Space Efficiency |
|--------|--------|------------------|
| Steam (Coal) | 900 kW per engine | Medium |
| Solar | 60 kW peak per panel | Low |
| Nuclear | 40+ MW per reactor | High |
| Fusion (Space Age) | Variable | Very High |

### Science Pack Production Rates

| Science Pack | Base Craft Time | Packs per Craft |
|--------------|-----------------|-----------------|
| Automation (Red) | 5s | 1 |
| Logistic (Green) | 6s | 1 |
| Military (Black) | 10s | 2 |
| Chemical (Blue) | 24s | 2 |
| Production (Purple) | 21s | 3 |
| Utility (Yellow) | 21s | 3 |

---

*Document compiled for game design research purposes. Data sourced from official Factorio Wiki, community resources, and in-game mechanics as of version 2.0 (Space Age expansion).*
