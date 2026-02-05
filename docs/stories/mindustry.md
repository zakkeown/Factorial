# Mindustry: Comprehensive Game Design Research Document

## Table of Contents

1. [Overview](#overview)
2. [Initial Conditions](#initial-conditions)
3. [Core Mechanics](#core-mechanics)
   - [Resource Types](#resource-types)
   - [Drills and Mining](#drills-and-mining)
   - [Power Generation](#power-generation)
   - [Conveyors and Transportation](#conveyors-and-transportation)
   - [Factories and Production](#factories-and-production)
   - [Liquid Handling](#liquid-handling)
4. [Tower Defense Mechanics](#tower-defense-mechanics)
   - [Enemy Waves](#enemy-waves)
   - [Turret Types (Serpulo)](#turret-types-serpulo)
   - [Turret Types (Erekir)](#turret-types-erekir)
   - [Defensive Structures](#defensive-structures)
   - [Wave Mechanics](#wave-mechanics)
5. [Technology/Research Tree](#technologyresearch-tree)
6. [Production Chains](#production-chains)
7. [World/Map System](#worldmap-system)
8. [Campaign Structure](#campaign-structure)
9. [Combat Mechanics](#combat-mechanics)
   - [Units](#units)
   - [Damage Types and Status Effects](#damage-types-and-status-effects)
10. [Logic and Automation](#logic-and-automation)
11. [Multiplayer](#multiplayer)

---

## Overview

Mindustry is a free and open-source real-time strategy, factory management, and tower defense hybrid game developed by Anuken. The game combines resource extraction, logistics management, factory automation, and tower defense into a cohesive experience where players must build and defend bases against waves of enemies while expanding their industrial capacity.

**Core Gameplay Loop:**
1. Extract raw resources using drills
2. Transport materials via conveyors and logistics networks
3. Process raw materials into advanced components
4. Build defensive turrets and walls
5. Defend against enemy waves
6. Research new technologies
7. Capture new sectors and expand territory

---

## Initial Conditions

### Starting Scenarios

#### Ground Zero (Tutorial Sector)
- **Location**: Serpulo planet, Sector 0
- **Environment**: Barren, snowy chasm
- **Starting Resources**: Large patches of copper and lead, scrap slivers around ruins
- **Core Type**: Core: Shard (4,000 storage capacity per material)
- **Enemy Spawn**: Top of map, enemies take left path initially
- **Tutorial Objectives**: Must complete special objectives before waves begin

**Initial Tutorial Steps:**
1. Secure copper flow with Mechanical Drills
2. Build Duo turrets fed with copper
3. Use Routers to split conveyor lines
4. Establish lead production
5. Build basic defenses before waves commence

#### Launch Requirements
- Minimum 1,000 copper and 800 lead required to launch to new sectors
- Core: Foundation required to launch to numbered sectors

### Campaign vs Custom Maps

**Campaign Mode:**
- Pre-designed sectors with specific objectives
- Progressive difficulty scaling
- Tech tree unlocks tied to sector captures
- Persistent resource pool across all controlled sectors
- Enemy faction (Crux) with specific spawn patterns

**Custom/Sandbox Maps:**
- Player-designed or community-created maps
- Configurable wave settings
- Adjustable rules and resources
- No tech tree restrictions (optional)

---

## Core Mechanics

### Resource Types

#### Serpulo Planet Resources

**Tier 1 (Basic Materials):**
| Resource | Acquisition | Primary Uses |
|----------|-------------|--------------|
| Copper | Mined directly | Basic construction, ammunition, power nodes |
| Lead | Mined directly | Electronics, ammunition, metaglass |
| Coal | Mined directly | Power generation, graphite production |
| Sand | Mined directly | Silicon, metaglass, pyratite |
| Scrap | Found near ruins | Can be processed into various materials |

**Tier 2 (Processed Materials):**
| Resource | Recipe | Primary Uses |
|----------|--------|--------------|
| Graphite | 2 Coal -> 1 Graphite | Ammunition, construction, multi-press |
| Silicon | 1 Coal + 2 Sand -> 1 Silicon | Electronics, units, advanced buildings |
| Metaglass | Sand + Lead (+ Power) | Transparent structures, liquids |
| Pyratite | Coal + Lead + Sand | Incendiary ammunition, blast compound |

**Tier 3 (Advanced Materials):**
| Resource | Recipe | Primary Uses |
|----------|--------|--------------|
| Titanium | Mined (requires Pneumatic Drill+) | Advanced construction, ammunition |
| Plastanium | Oil + Titanium | High-tier construction, fragmentation ammo |
| Thorium | Mined (requires Laser Drill) | Nuclear power, high-damage ammunition |
| Phase Fabric | Thorium + Sand (large amounts) | Phase walls, phase conveyors |
| Surge Alloy | Copper + Lead + Titanium + Silicon | Ultimate tier construction, surge walls |
| Blast Compound | Pyratite + Spore Pods | Impact reactor fuel, explosive ammunition |
| Spore Pod | Cultivated from spores | Oil production, blast compound |

#### Erekir Planet Resources
| Resource | Description |
|----------|-------------|
| Beryllium | Primary building material |
| Tungsten | Heavy construction |
| Oxide | Chemical processes |
| Carbide | Advanced components |
| Fissile Matter | Nuclear applications |
| Dormant Cyst | Biological resource |

#### Liquids
| Liquid | Source | Uses |
|--------|--------|------|
| Water | Pumped from tiles, extracted | Cooling, steam power, boosting drills |
| Oil | Pumped from tar pits, extracted | Plastanium, power generation |
| Cryofluid | Water + Titanium (Cryofluid Mixer) | Reactor cooling, freeze effects |
| Slag | Byproduct of smelting | Damage, melting status |
| Neoplasm | Biological processes | Erekir-specific mechanics |

---

### Drills and Mining

#### Serpulo Drills

| Drill Type | Size | Extraction Rate | Max Rate | Power Required | Can Mine |
|------------|------|-----------------|----------|----------------|----------|
| Mechanical Drill | 2x2 | 0.5/s per tile | ~2/s | None | Copper, Lead, Coal, Sand |
| Pneumatic Drill | 2x2 | 0.65/s per tile | ~2.6/s | None | + Titanium |
| Laser Drill | 3x3 | 0.7/s per tile | ~6.3/s | 1.0/s | + Thorium |
| Blast Drill | 4x4 | 0.9/s per tile | ~14.4/s | 3.0/s | All ores |
| Airblast Drill | 4x4 | Higher efficiency | ~20+/s | 4.0/s | All ores (requires water) |

**Water Boost Effect:**
All drills operate at **2.56x speed** when supplied with water, making water distribution to mining operations a significant optimization.

**Drill Efficiency:**
- Drills extract faster when covering more ore tiles
- Maximum extraction occurs when all tiles under the drill contain ore
- Overlapping drill coverage on single ore tiles reduces individual drill output

#### Erekir Extraction
- Different extraction mechanics using heat-based systems
- Cliff Crusher for beryllium extraction
- Plasma Bore for tungsten

---

### Power Generation

#### Serpulo Power Sources

| Generator | Output | Fuel/Input | Notes |
|-----------|--------|------------|-------|
| Combustion Generator | ~1.0/s | Coal (0.5/s consumption) | Basic, reliable |
| Steam Generator | ~5.5/s | Coal + Water | 5.5x Combustion output, 25% faster fuel use |
| Solar Panel | ~0.1/s | None (light dependent) | Very low output, day/night dependent |
| Large Solar Panel | ~0.4/s | None (light dependent) | Better ratio, still weather-dependent |
| Differential Generator | ~18/s | Pyratite + Cryofluid | ~3.27x Steam Generator (coal equivalent) |
| RTG Generator | ~4.5/s | Thorium (slow consumption) | Expensive to build, steady output |
| Thorium Reactor | Up to 900/s | Thorium + Cryofluid | 30 power per thorium stored, EXPLODES without cooling |
| Impact Reactor | ~6,500/s | Blast Compound + Cryofluid | Requires 1,500 power to start, highest output |

**Power Distribution:**
| Component | Connections | Range | Notes |
|-----------|-------------|-------|-------|
| Power Node | 10 blocks | Short | Auto-chains when placed in lines |
| Large Power Node | 15 blocks | Extended | More range, larger footprint |
| Surge Tower | Unlimited in range | Very long | Requires surge alloy |
| Battery | N/A | Storage only | Stores excess power |
| Large Battery | N/A | Storage only | Higher capacity |
| Battery Diode | N/A | One-way flow | Prevents backflow |

**Reactor Safety:**
- Thorium Reactor requires constant Cryofluid cooling
- Explodes violently if coolant fails (damages nearby structures)
- Impact Reactor shuts down safely if fuel/coolant depleted but needs 1.5k power restart

---

### Conveyors and Transportation

#### Serpulo Conveyor Types

| Conveyor Type | Speed (items/s) | Notes |
|---------------|-----------------|-------|
| Conveyor | 8 | Basic, cheap |
| Titanium Conveyor | 11 | Standard mid-game |
| Armored Conveyor | 11 | Higher durability, same speed |
| Plastanium Conveyor | 40 (batched) | Items move in stacks, much faster throughput |

#### Transport Components

| Component | Function | Throughput Notes |
|-----------|----------|------------------|
| Junction | Cross items without mixing | 6 items per direction, ~10/s through junctions |
| Router | Splits to 3 directions | Slow, can cause backup |
| Distributor | Large router (4 outputs) | Better for high-volume |
| Sorter | Filter specific items | Same speed as conveyors |
| Inverted Sorter | Passes all except filter | Useful for overflow |
| Overflow Gate | Passes when output blocked | Prevents backup |
| Underflow Gate | Opposite of overflow | Priority routing |
| Bridge Conveyor | Spans gaps (4 tiles) | ~1.15x titanium speed |
| Phase Conveyor | Long-range bridge (12 tiles) | Requires phase fabric |

#### Storage

| Structure | Capacity | Notes |
|-----------|----------|-------|
| Container | 300 per item | Medium storage |
| Vault | 1,000 per item | Large storage |
| Core: Shard | 4,000 per item | Starting core |
| Core: Foundation | 9,000 per item | Mid-game core |
| Core: Nucleus | 14,000 per item | End-game core |
| Unloader | N/A | Extracts from storage at conveyor speed |

#### Erekir Transportation

| Component | Speed | Notes |
|-----------|-------|-------|
| Duct | 15 items/s | Faster than Serpulo conveyors |
| Armored Duct | 15 items/s | Higher durability |
| Surge Duct | 20+ items/s | Highest speed |

---

### Factories and Production

#### Production Buildings (Serpulo)

| Factory | Input | Output | Time | Power |
|---------|-------|--------|------|-------|
| Graphite Press | 2 Coal | 1 Graphite | 1.5s | None |
| Multi-Press | 3 Coal + Water | 2 Graphite | 1.0s | 1.8/s |
| Silicon Smelter | 1 Coal + 2 Sand | 1 Silicon | 0.67s | 0.5/s |
| Silicon Crucible | Coal + Sand + Pyratite | Silicon | Faster | 2.4/s |
| Kiln | 1 Sand + 1 Lead | 1 Metaglass | 0.5s | 0.6/s |
| Plastanium Compressor | 2 Titanium + 15 Oil | 1 Plastanium | 1.0s | 3.0/s |
| Phase Weaver | 4 Thorium + 10 Sand | 1 Phase Fabric | 2.0s | 5.0/s |
| Surge Smelter | Copper + Lead + Titanium + Silicon | 1 Surge Alloy | 1.2s | 4.0/s |
| Pyratite Mixer | Coal + Lead + Sand | 1 Pyratite | 0.5s | 0.2/s |
| Blast Mixer | 1 Pyratite + 1 Spore Pod | 1 Blast Compound | 0.75s | 0.4/s |
| Cryofluid Mixer | 1 Titanium + 24 Water | 24 Cryofluid | 0.5s | 1.0/s |
| Oil Extractor | 15 Sand + 15 Water | 15 Oil | 1.0s | 3.0/s |
| Coal Centrifuge | 6 Oil | 2 Coal | 1.0s | 0.7/s |
| Spore Press | 10 Spore Pods | 15 Oil | 1.0s | 0.7/s |
| Cultivator | Water | Spore Pods | Continuous | 0.8/s |

---

### Liquid Handling

#### Liquid Transportation

| Component | Throughput | Notes |
|-----------|------------|-------|
| Conduit | 14/s | Basic liquid transport |
| Pulse Conduit | 24/s | Faster, stores more |
| Plated Conduit | 24/s | Armored version |
| Liquid Router | Variable | Splits to 3 directions |
| Liquid Junction | N/A | Cross without mixing |
| Bridge Conduit | 14/s | Spans gaps |
| Phase Conduit | 14/s | Long-range bridge |

#### Pumps

| Pump Type | Output | Notes |
|-----------|--------|-------|
| Mechanical Pump | 10/s | Basic, no power |
| Rotary Pump | 12/s per tile (48/s max) | 2x2, requires power |
| Thermal Pump | 90/s | Uses heat from magma |

#### Liquid Storage

| Structure | Capacity | Notes |
|-----------|----------|-------|
| Liquid Container | 400 | Medium storage |
| Liquid Tank | 1,500 | Large storage |

**Cryofluid Production Ratios:**
- 2 Cryofluid Mixers supply 1 Impact Reactor
- 5 Cryofluid Mixers supply 4 Impact Reactors exactly
- Cryofluid Mixer requires ~60 water/s input

---

## Tower Defense Mechanics

### Enemy Waves

**Wave System:**
- Waves spawn from designated spawn points or enemy cores
- Wave timer can be automatic or player-triggered
- Wave spacing configurable (time between waves)
- Initial wave spacing (time before first wave)
- Difficulty scales with wave number

**Spawn Mechanics:**
- Each spawn entry maxes at 100 units per type
- Multiple entries for same unit type increases maximum
- Air units can spawn at map border or drop zones
- Ground units follow pathfinding to core
- Naval units spawn from water areas

**Wave Composition:**
- Early waves: T1 units (Daggers, Crawlers, Flares)
- Mid waves: T2-T3 units mixed compositions
- Late waves: T4-T5 units, multiple unit types
- Boss waves: Special compositions with high-tier units

---

### Turret Types (Serpulo)

#### Early Game Turrets

| Turret | Size | Damage | Range | Fire Rate | Ammo | Notes |
|--------|------|--------|-------|-----------|------|-------|
| Duo | 1x1 | 9/shot (27 DPS copper) | 110 | 0.4s | Copper, Lead, Graphite, Titanium, Thorium, Pyratite | Twin barrels, alternating fire |
| Scatter | 2x2 | 5/shot | 220 | 0.5s | Lead, Scrap | Anti-air flak, area damage |
| Hail | 1x1 | 10/shot | 235 | 1.5s | Graphite, Silicon | Artillery, ground only |
| Arc | 1x1 | 20 (+ 14 vs wet) | 90 | N/A | Power only | Lightning, shocks, ground only |

#### Mid Game Turrets

| Turret | Size | Damage | Range | Fire Rate | Ammo/Power | Notes |
|--------|------|--------|-------|-----------|------------|-------|
| Salvo | 2x2 | 9-23/shot | 190 | Burst | Copper-Thorium | Quick salvos |
| Wave | 2x2 | 18/s | 152 | Continuous | Liquids | Stream weapon, wets targets |
| Lancer | 2x2 | 140/shot | 165 | 1.25s charge | Power | Beam weapon, ground only |
| Swarmer | 2x2 | 12/missile | 240 | Burst | Blast Compound, Pyratite, Plastanium | Homing missiles |
| Ripple | 3x3 | 20/shell | 290 | Cluster | Graphite-Blast Compound | Artillery barrage |

#### Late Game Turrets

| Turret | Size | Damage | Range | Fire Rate | Ammo/Power | Notes |
|--------|------|--------|-------|-----------|------------|-------|
| Fuse | 3x3 | 90/shot | 110 | 0.5s | Titanium, Thorium, Pyratite | Triple piercing blast |
| Cyclone | 3x3 | 27/shot | 200 | 0.1s | Various + Power | Rapid fire, multi-ammo |
| Spectre | 4x4 | 728 DPS (graphite) | 260 | Very fast | Graphite, Titanium, Thorium, Pyratite | Rapid alternating fire |
| Meltdown | 4x4 | 1,100+ DPS | 240 | 3.83s active | Power + Water/Cryo cooling | Piercing laser beam |
| Foreshadow | 4x4 | 1,500/shot | 500 | 3s charge | Power + Items | Railgun, piercing, targets high HP |

---

### Turret Types (Erekir)

| Turret | Size | Key Feature | Requirements |
|--------|------|-------------|--------------|
| Breach | 2x2 | Rapid piercing projectiles | First available |
| Diffuse | 2x2 | High-knockback spread | Close range |
| Sublimate | 2x2 | Flame stream (540 DPS with ozone) | Bypasses armor |
| Disperse | 3x3 | Crowd control | Power + Heat |
| Afflict | 3x3 | Giant flak orb | Power + Heat |
| Malign | 4x4 | Ultimate turret | 90 Heat for full rate |
| Titan | 4x4 | Heavy damage | Carbide-based |
| Smite | 5x5 | Massive bombardment | End-game |

---

### Defensive Structures

#### Walls (Serpulo)

| Wall Type | Health | Special | Notes |
|-----------|--------|---------|-------|
| Copper Wall | 240 | None | Basic |
| Large Copper Wall | 960 | None | 2x2 (4x health) |
| Titanium Wall | 440 | None | Standard upgrade |
| Large Titanium Wall | 1,760 | None | 2x2 |
| Thorium Wall | 600 | None | High durability |
| Large Thorium Wall | 2,400 | None | 2x2 |
| Plastanium Wall | 320 | Absorbs some damage | Reduces incoming damage |
| Large Plastanium Wall | 1,280 | Absorbs some damage | 2x2 |
| Phase Wall | 360 | Reflects projectiles | Chance to bounce back |
| Large Phase Wall | 1,440 | Reflects projectiles | 2x2 |
| Surge Wall | 700 | Lightning on hit | Highest HP, emits sparks |
| Large Surge Wall | 2,800 | Lightning on hit | 2x2, best end-game |

#### Doors
- Door: Allows units through, blocks enemies
- Large Door: 2x2, equivalent to 4 doors
- Cost and health equivalent to 4 regular doors

#### Projectors
- Mend Projector: Repairs nearby blocks
- Overdrive Projector: Speeds up nearby production
- Force Projector: Energy shield bubble

---

### Wave Mechanics

**Configurable Parameters:**
- Wave Timer: Auto-spawn or player-triggered
- Wave Spacing: Time between waves
- Initial Wave Spacing: Time before first wave
- Unpredictable Wave AI: Enemies target random structures instead of core
- Enemy unit caps per spawn entry: 100 maximum

**Scaling:**
- Unit count increases per wave
- Unit tier increases at thresholds
- Multiple unit types mix in later waves
- Faster spawn intervals in harder sectors

---

## Technology/Research Tree

### Research System

**Mechanics:**
- Items spent from global resource pool (all cores on planet)
- Each tech has specific item requirements
- Some techs locked behind sector captures
- Research cost based on building cost multiplied by factors

**Unlock Methods:**
1. Spend resources from global pool
2. Capture specific named sectors
3. Meet prerequisite research
4. Achieve specific milestones

### Tech Tree Branches (Serpulo)

**Production Branch:**
- Mechanical Drill -> Pneumatic Drill -> Laser Drill -> Blast Drill -> Airblast Drill
- Graphite Press -> Multi-Press
- Silicon Smelter -> Silicon Crucible
- Various factories unlock progressively

**Defense Branch:**
- Copper Wall -> Titanium Wall -> Thorium Wall -> Phase/Surge Walls
- Duo -> Scatter/Hail -> Salvo/Wave -> Lancer -> Swarmer -> Ripple -> Fuse -> Cyclone -> Spectre/Meltdown -> Foreshadow

**Logistics Branch:**
- Conveyor -> Junction/Router -> Titanium Conveyor -> Plastanium Conveyor
- Bridge Conveyor -> Phase Conveyor
- Container -> Vault

**Power Branch:**
- Combustion Generator -> Steam Generator -> Differential Generator
- Solar Panel -> Large Solar Panel
- Thorium Reactor -> Impact Reactor

**Units Branch:**
- Ground Factory -> Air Factory -> Naval Factory
- Additive Reconstructor -> Multiplicative Reconstructor -> Exponential Reconstructor -> Tetrative Reconstructor

### Sector-Locked Technology

| Sector | Unlocks |
|--------|---------|
| Ground Zero | Basic buildings |
| Frozen Forest | Graphite Press |
| Craters | Router, Junction |
| Ruinous Shores | Naval units |
| Tar Fields | Phase Fabric |
| Stained Mountains | Vault |
| Desolate Rift | Advanced turrets |
| Nuclear Production Complex | Impact Reactor |
| Planetary Launch Terminal | Campaign completion |

---

## Production Chains

### Basic Production Chains

```
Copper Ore -> [Mechanical Drill] -> Copper
Lead Ore -> [Mechanical Drill] -> Lead
Coal Ore -> [Mechanical Drill] -> Coal
Sand -> [Mechanical Drill] -> Sand
```

### Intermediate Chains

```
Coal (2) -> [Graphite Press] -> Graphite (1)
Coal (3) + Water -> [Multi-Press] -> Graphite (2)

Coal (1) + Sand (2) -> [Silicon Smelter] -> Silicon (1)

Sand (1) + Lead (1) + Power -> [Kiln] -> Metaglass (1)

Coal + Lead + Sand -> [Pyratite Mixer] -> Pyratite
```

### Advanced Production Chains

```
Titanium Ore -> [Pneumatic/Laser Drill] -> Titanium
Thorium Ore -> [Laser Drill] -> Thorium

Titanium (2) + Oil (15) -> [Plastanium Compressor] -> Plastanium (1)

Thorium (4) + Sand (10) + Power -> [Phase Weaver] -> Phase Fabric (1)

Copper + Lead + Titanium + Silicon + Power -> [Surge Smelter] -> Surge Alloy (1)
```

### Fuel Production

```
Pyratite (1) + Spore Pod (1) -> [Blast Mixer] -> Blast Compound (1)

Water -> [Cultivator] -> Spore Pods
Spore Pods (10) -> [Spore Press] -> Oil (15)

Sand (15) + Water (15) + Power -> [Oil Extractor] -> Oil (15)
```

### Coolant Production

```
Titanium (1) + Water (24) + Power -> [Cryofluid Mixer] -> Cryofluid (24)
```

### Ammunition Effectiveness

| Ammo Type | Damage Modifier | Special Effect |
|-----------|-----------------|----------------|
| Copper | 1.0x | None |
| Lead | 1.2x | None |
| Graphite | 1.5x | None |
| Titanium | 1.8x | None |
| Thorium | 2.5x | Highest damage |
| Pyratite | 1.3x | Incendiary (Burning status) |
| Blast Compound | 2.0x | Explosive splash |
| Silicon | 1.3x | Homing capability |
| Plastanium | 1.4x | Fragmentation |

---

## World/Map System

### Sector System

**Serpulo:**
- 272 total sectors
- 29 hand-made named sectors
- Procedurally generated numbered sectors
- Hexagonal sector grid

**Sector Types:**
1. **Survival Sectors**: Defend against waves
2. **Attack Sectors**: Destroy enemy base(s)
3. **Mixed Sectors**: Both defend and attack

### Named Sectors (Serpulo Progression)

| Sector | Threat | Resources | Unlocks |
|--------|--------|-----------|---------|
| Ground Zero | None (tutorial) | Copper, Lead | Basic tech |
| Frozen Forest | Low | Copper, Lead, Coal | Graphite |
| Craters | Low | Copper, Lead, Titanium | Routing |
| Ruinous Shores | Medium | Copper, Lead, Water | Naval |
| Windswept Islands | Medium | Mixed | Water tech |
| Tar Fields | Medium | Oil, Titanium | Phase Fabric |
| Stained Mountains | High | Rich resources | Vault |
| Fungal Pass | High | Spores | Cultivator |
| Impact 0078 | High | Thorium | Advanced weapons |
| Desolate Rift | Extreme | All resources | End-game turrets |
| Nuclear Production Complex | Extreme | All resources | Impact Reactor |
| Planetary Launch Terminal | Extreme | All resources | Campaign end |

### Map Generation

**Terrain Types:**
- Desert (sand-heavy)
- Snow/Ice (coal-heavy)
- Forest (spore-heavy)
- Volcanic (slag, magma)
- Ocean (water access)
- Mixed biomes

**Resource Distribution:**
- Copper/Lead: Common in all sectors
- Coal: Common in most sectors
- Titanium: Uncommon, specific sectors
- Thorium: Rare, late-game sectors only
- Oil: Tar pits in specific sectors

---

## Campaign Structure

### Planets

#### Serpulo (Primary Campaign)
- Starting planet
- 272 sectors
- Enemy faction: Crux (red)
- Full tech tree
- Varied terrain and biomes
- Campaign ends at Planetary Launch Terminal

#### Erekir (Secondary Campaign)
- Added in Version 7.0
- More combat-focused
- Fog of war mechanic
- Heat-based systems
- Different resource set
- More structured progression
- Considered more difficult but more polished

### Progression Path

**Early Game:**
1. Ground Zero (tutorial)
2. Frozen Forest (coal, graphite)
3. Craters (routing, titanium access)

**Mid Game:**
1. Ruinous Shores (naval)
2. Tar Fields (oil, phase fabric)
3. Stained Mountains (vault, defense)

**Late Game:**
1. Impact sectors (thorium)
2. Desolate Rift (advanced tech)
3. Nuclear Production Complex (impact reactor)

**End Game:**
1. Planetary Launch Terminal (final sector)
2. Erekir access unlocked

### Cross-Sector Mechanics

- Resources shared in global pool
- Captured sectors contribute passively
- Sectors can be lost to enemy attacks
- Launch pads connect sectors
- Resource import/export between sectors

---

## Combat Mechanics

### Units

#### Unit Tiers

Units progress from Tier 1 (basic) to Tier 5 (ultimate), with each tier requiring reconstruction from the previous tier.

#### Serpulo Ground Units

| Tier | Attack | Support |
|------|--------|---------|
| T1 | Dagger (weak, cheap) | Nova (ranged support) |
| T2 | Mace | Pulsar |
| T3 | Fortress | Quasar |
| T4 | Scepter | Vela |
| T5 | Reign (ultimate ground) | Corvus |

**Crawler (T1 Special):**
- Fastest ground unit
- Suicide attack (explodes)
- Mass-producible

#### Serpulo Air Units

| Tier | Attack | Support |
|------|--------|---------|
| T1 | Flare (fast fighter) | Mono (auto-mines copper/lead) |
| T2 | Horizon | Poly (auto-repairs, auto-builds) |
| T3 | Zenith (all-round strong) | Mega (mines titanium, builds) |
| T4 | Antumbra | Quad |
| T5 | Eclipse (ultimate air) | Oct |

**Key Air Unit Notes:**
- Flare: Extremely fast, fires salvos of 3 bullets
- Mono: Automatically mines and deposits to core
- Poly: Auto-repairs structures, self-heals
- Zenith: Excellent with blast compound payload

#### Serpulo Naval Units

| Tier | Attack | Support |
|------|--------|---------|
| T1 | Risso | Retusa |
| T2 | Minke | Oxynoe |
| T3 | Bryde | Cyerce |
| T4 | Sei | Aegires |
| T5 | Omura | Navanax |

#### Unit Production

**Factories:**
- Ground Factory: Produces T1 ground units
- Air Factory: Produces T1 air units
- Naval Factory: Produces T1 naval units

**Reconstructors:**
- Additive Reconstructor: T1 -> T2
- Multiplicative Reconstructor: T2 -> T3
- Exponential Reconstructor: T3 -> T4
- Tetrative Reconstructor: T4 -> T5

**Constraints:**
- T4 and T5 units do not fit on payload conveyors
- T5 units must be fed directly from T4 reconstructor

#### Core Units (Player-Controlled)

| Unit | Unlocks With | Mining Capability |
|------|--------------|-------------------|
| Alpha | Core: Shard | Copper, Lead |
| Beta | Core: Foundation | + Coal, Sand |
| Gamma | Core: Nucleus | + Titanium, Coal |

---

### Damage Types and Status Effects

#### Status Effects (Continuous)

| Status | Source | Effect | Duration |
|--------|--------|--------|----------|
| Burning | Fire, Pyratite | Damage over time | Until extinguished |
| Freezing | Cryofluid | Slow movement | 3s |
| Wet | Water | Vulnerable to shock | 4s |
| Muddy | Mud tiles | Slow movement | While on tile |
| Melting | Slag | High damage over time | 3s |
| Sapped | Certain attacks | Reduced stats | Variable |
| Spore Slowed | Spores | Movement reduction | Variable |
| Tarred | Oil, tar tiles | Flammable, slowed | Until cleaned |
| Overdrive | Overdrive projector | Increased speed/fire rate | While in range |
| Overclock | Overdrive dome | Higher boost | While in range |
| Electrified | Surge-related | Damage over time | 2s |
| Corroded | Acid | Armor reduction | Variable |

#### Status Effects (Instantaneous)

| Status | Source | Effect |
|--------|--------|--------|
| Shocked | Lightning, Surge ammo | +14 armor-piercing damage vs Wet |
| Blasted | Explosions | Knockback |

#### Status Interactions

- **Wet + Shocked**: Massive bonus damage (14 armor-piercing)
- **Tarred + Burning**: Ignites, enhanced burn damage
- **Freezing vs Burning**: Cancel each other out
- **Wet vs Burning**: Water extinguishes fire

#### Damage Properties

| Property | Description |
|----------|-------------|
| Piercing | Passes through multiple targets |
| Splash | Area damage around impact |
| Armor Piercing | Ignores armor reduction |
| Homing | Tracks moving targets |
| Fragmentation | Breaks into smaller projectiles |
| Continuous | Sustained beam/stream damage |

---

## Logic and Automation

### Processor Types

| Processor | Size | Speed | Links | Notes |
|-----------|------|-------|-------|-------|
| Micro Processor | 1x1 | 120 ops/s | 6 | Basic automation |
| Logic Processor | 2x2 | 480 ops/s | 10 | Standard processing |
| Hyper Processor | 3x3 | 1,500 ops/s | 25 | Complex operations |

### Mindustry Logic (mlog)

**Language Type:** Assembly-like, one instruction per line

**Key Instructions:**
- `read`/`write`: Memory access
- `draw`: Graphics to displays
- `print`: Text output
- `sensor`: Read block/unit properties
- `control`: Control blocks/units
- `radar`: Find units
- `unit`: Unit commands
- `jump`: Conditional branching
- `op`: Mathematical operations
- `set`: Variable assignment

### Logic Accessories

| Block | Function |
|-------|----------|
| Memory Cell | 64 number storage |
| Memory Bank | 512 number storage |
| Logic Display | 80x80 pixel display |
| Large Logic Display | 176x176 pixel display |
| Switch | Manual boolean input |
| Message | Text display |

### Automation Examples

**Auto-Mine System:**
- Mono units auto-mine copper/lead to core
- Poly units auto-repair and build
- Mega units mine titanium

**Logic-Controlled:**
- Turret targeting priority
- Unit squad control
- Factory activation based on storage
- Defense coordination
- Resource balancing

---

## Multiplayer

### Game Modes

**Co-op:**
- Players share resources
- Collaborative building
- Joint defense against waves
- Cross-platform support

**PvP:**
- Team-based competition
- Build and defend while attacking
- Resource control objectives

### Server Types

**Local/LAN:**
- Built into game client
- "Host Multiplayer Game" option
- Steam networking support

**Dedicated Server:**
- Headless server application
- Requires Java installation
- Configurable rules and maps
- Persistent worlds

### Server Configuration

**Default Player Limit:** 16 players
**Configurable:** Unlimited (dedicated server)

**Server Commands:**
- `/help`: List commands
- `/rules`: Display/set rules
- `/ban`: Ban players
- `/kick`: Kick players
- `/admin`: Admin management

### Cross-Platform

- Full cross-platform multiplayer
- Same version required across all devices
- PC, Mobile, Steam compatible
- Any device can connect to any other

### Multiplayer Rules

- Rules apply regardless of map
- Configurable wave mechanics
- Adjustable resource multipliers
- Team balance options
- Spectator mode available

---

## Sources and References

This document was compiled from research across multiple community resources:

- [Mindustry Unofficial Wiki (Fandom)](https://mindustry-unofficial.fandom.com/)
- [Mindustry Official Wiki](https://mindustrygame.github.io/wiki/)
- [Steam Community Guides](https://steamcommunity.com/app/1127400/guides/)
- [Mindustry Encyclopedia (Miraheze)](https://mindustry.miraheze.org/)
- [Mindustry GitHub Repository](https://github.com/Anuken/Mindustry)
- [PaperNodes Beginner Guide](https://papernodes.com/a-comprehensive-guide-for-mindustry-beginners-2024/)
- [Mindustry Schematics](https://mindustryschematics.com/)
- [Mindustry Resource Calculator](https://gamertools.net/tools/9)

---

*Document Version: 1.0*
*Game Version Reference: Mindustry v7.0+*
*Last Updated: February 2026*
