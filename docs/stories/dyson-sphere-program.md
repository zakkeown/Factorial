# Dyson Sphere Program - Game Design Research Document

## Table of Contents

1. [Game Overview](#game-overview)
2. [Initial Conditions](#initial-conditions)
3. [Core Mechanics](#core-mechanics)
   - [Resource Types](#resource-types)
   - [Mining Systems](#mining-systems)
   - [Power Generation](#power-generation)
   - [Conveyor and Logistics Systems](#conveyor-and-logistics-systems)
   - [Building Machines](#building-machines)
   - [Fluid Handling](#fluid-handling)
   - [Planetary Logistics](#planetary-logistics)
   - [Interstellar Logistics](#interstellar-logistics)
4. [Technology and Research Tree](#technology-and-research-tree)
   - [Research Matrix Types](#research-matrix-types)
   - [Tech Tree Structure](#tech-tree-structure)
   - [Research Mechanics](#research-mechanics)
5. [Production Chains](#production-chains)
   - [Smelting Recipes](#smelting-recipes)
   - [Oil Processing](#oil-processing)
   - [Chemical Plant Recipes](#chemical-plant-recipes)
   - [Key Production Ratios](#key-production-ratios)
6. [World and Map](#world-and-map)
   - [Planet Types](#planet-types)
   - [Star Types](#star-types)
   - [Resource Distribution](#resource-distribution)
   - [Rare Resources](#rare-resources)
   - [Galaxy Generation](#galaxy-generation)
7. [Dyson Sphere Construction](#dyson-sphere-construction)
   - [Dyson Swarm vs Dyson Sphere](#dyson-swarm-vs-dyson-sphere)
   - [Construction Requirements](#construction-requirements)
   - [Power Output Mechanics](#power-output-mechanics)
8. [Icarus Mech Mechanics](#icarus-mech-mechanics)
   - [Core Abilities](#core-abilities)
   - [Upgrades](#upgrades)
   - [Energy and Fuel](#energy-and-fuel)
9. [Combat and Dark Fog](#combat-and-dark-fog)
10. [Endgame](#endgame)
    - [Victory Conditions](#victory-conditions)
    - [Infinite Research](#infinite-research)
    - [Post-Game Content](#post-game-content)

---

## Game Overview

Dyson Sphere Program is a sandbox factory simulation game developed by Youthcat Studio. Set in a science fiction universe where human consciousness has been digitized and uploaded to a supercomputer called the "CentreBrain," players control a mech called Icarus tasked with building an interstellar network of factories across multiple planetary systems. The ultimate goal is constructing a Dyson Sphere - a megastructure that completely encapsulates a star to capture its entire energy output.

The game combines elements of:
- Factory building and automation
- Resource management and logistics
- Space exploration
- Technology progression
- Combat (with the "Rise of the Dark Fog" expansion)

---

## Initial Conditions

### Starting Scenario

- Players begin on a randomly generated starting planet within a star system
- The starting system is always a habitable Mediterranean-type planet
- Icarus (the mech) starts with basic inventory and construction capabilities
- Initial resources must be gathered manually before automation can begin
- Players receive a seed number that determines galaxy generation

### Starting Planet Selection

The starting system has specific constraints:
- Cannot have a sulfuric ocean
- Cannot have planetary rare resources (except Fire Ice on Gas Giants)
- Gas Giants most often have Fire Ice, rarely Deuterium
- Rotational traits are rare in starting systems

### Ideal Starting Conditions

A good starting seed should have:
- Deuterium availability in the system
- Fire Ice deposits
- Good wind/solar ratios on the starting planet
- Multiple planets for expansion
- Nearby O or B-type stars for optimal Dyson Sphere construction

### Notable Seeds

| Seed | Features |
|------|----------|
| 81488271 | Best documented starting seed |
| 74564148 | Tidally locked Lava Planet, Gas Giant with Fire Ice |
| 33420870 | Tidally locked inner planet, ice giant with Fire Ice |
| 52967102 | Multiple blue giants, abundant Fire Ice |

### Initial Mech Capabilities

Icarus starts with:
- Basic movement (walking)
- Manual resource gathering
- Handheld crafting capability
- Medium-range laser weapon
- Small inventory capacity
- Construction drone fleet (limited)
- Basic energy core requiring fuel

---

## Core Mechanics

### Resource Types

#### Basic Ores
| Resource | Description | Common Locations |
|----------|-------------|------------------|
| Iron Ore | Primary metal ore | Most planets |
| Copper Ore | Electrical component ore | Most planets |
| Stone | Non-metallic building material | Barren planets (high) |
| Coal | Fossil fuel, energy source | Habitable planets |
| Silicon Ore | Electronics production | Gobi planets |
| Titanium Ore | Advanced metal | Ice/Lava planets |
| Crude Oil | Liquid fuel resource | Habitable planets |
| Water | Processing liquid | Ocean planets |

#### Processed Materials
| Material | Recipe | Facility |
|----------|--------|----------|
| Iron Ingot | 1 Iron Ore | Smelter |
| Copper Ingot | 1 Copper Ore | Smelter |
| Stone Brick | 1 Stone | Smelter |
| High-Purity Silicon | 2 Silicon Ore | Smelter |
| Titanium Ingot | 2 Titanium Ore | Smelter |
| Magnet | 1 Iron Ore | Smelter |
| Glass | 2 Stone | Smelter |
| Steel | 3 Iron Ingot | Smelter |
| Energetic Graphite | 2 Coal | Smelter |
| Diamond | 1 Energetic Graphite | Smelter |

#### Rare Resources
| Resource | Source | Primary Use |
|----------|--------|-------------|
| Fire Ice | Ice planets, Gas Giants | Graphene, Hydrogen bypass |
| Organic Crystal | Special veins | Yellow Science shortcut |
| Kimberlite | Special veins | Diamond alternative |
| Fractal Silicon | Gobi planets | Silicon shortcut |
| Optical Grating Crystal | Special veins | Casimir Crystal |
| Spiniform Stalagmite Crystal | Prairie/Ocean | Carbon Nanotube |
| Unipolar Magnet | Special locations | Particle Container |

### Mining Systems

#### Mining Machine
- Extracts ore from veins at a base rate
- Can cover multiple vein nodes
- Connects directly to conveyor belts
- Power consumption scales with vein count covered

#### Advanced Mining Machine
- Faster extraction rate
- Larger coverage area
- Higher power consumption
- Unlocked later in tech tree

#### Oil Extractor
- Extracts Crude Oil from oil seeps
- Power consumption: 840 kW
- Production rate: ~2.5 items/second
- Oil seeps deplete over time but never fully exhaust

#### Water Pump
- Extracts water from ocean tiles
- Power consumption: 300 kW
- Production rate: 7.2 items/second
- Infinite resource (water doesn't deplete)

#### Orbital Collector
- Collects resources from Gas Giants
- Icarus cannot land on Gas Giants
- Collects Fire Ice, Hydrogen, or Deuterium
- Operates in low orbit

### Power Generation

#### Early Game Power Sources

| Structure | Power Output | Notes |
|-----------|--------------|-------|
| Wind Turbine | Up to 300 kW | Scales with planet's Wind Energy Ratio |
| Solar Panel | Up to 360 kW | Only works in sunlight, scales with Solar Energy Ratio |
| Thermal Power Station | 2.16 MW | Requires constant fuel supply |

#### Mid Game Power Sources

| Structure | Power Output | Notes |
|-----------|--------------|-------|
| Mini Fusion Power Plant | 9 MW (15 MW at 100% load) | Uses Deuteron Fuel Rods only |

#### Late Game Power Sources

| Structure | Power Output | Notes |
|-----------|--------------|-------|
| Ray Receiver (Power Mode) | 5-15 MW base | Requires Dyson Swarm/Sphere line of sight |
| Ray Receiver (Photon Mode) | 48-120 MW effective | Produces Critical Photons, 8x multiplier |
| Ray Receiver + Graviton Lens | Up to 240 MW | 2x additional multiplier with lens |
| Artificial Star | 75 MW | Uses Antimatter Fuel Rods |

#### Fuel Energy Values

| Fuel Type | Energy (MJ) | Notes |
|-----------|-------------|-------|
| Plant Fuel | 0.35 | -30% efficiency penalty |
| Log | ~0.5 | Basic fuel |
| Coal | 2.70 | Common early fuel |
| Energetic Graphite | 5.4 | Refined from coal |
| Combustible Unit | 7.776 | Processed fuel |
| Hydrogen | 8.0 | Burns at 80% efficiency |
| Hydrogen Fuel Rod | 54 | Efficient fuel rod |
| Deuteron Fuel Rod | 600 | Mini Fusion fuel |
| Antimatter Fuel Rod | 375+ | Artificial Star fuel |

#### Power Transmission

| Structure | Function | Coverage |
|-----------|----------|----------|
| Tesla Tower | Basic power distribution | Spherical radius |
| Wireless Power Tower | Extended range, charges Icarus | Extended sphere |
| Satellite Substation | Large area coverage | Cylindrical (10x Tesla area) |
| Energy Exchanger | Charges/discharges Accumulators | Grid interface |

### Conveyor and Logistics Systems

#### Conveyor Belt Tiers

| Belt Type | Speed (items/sec) | Speed (items/min) |
|-----------|-------------------|-------------------|
| Conveyor Belt Mk.I (Yellow) | 6 | 360 |
| Conveyor Belt Mk.II (Green) | 12 | 720 |
| Conveyor Belt Mk.III (Blue) | 30 | 1,800 |

#### Sorter Types

| Sorter Type | Base Speed (1 grid) | Power | Stacking |
|-------------|---------------------|-------|----------|
| Sorter Mk.I | 1.5 items/sec | Low | No |
| Sorter Mk.II | 3 items/sec | Medium | No |
| Sorter Mk.III | 6 items/sec | High | Yes (up to 4) |

#### Mk.III Sorter with Stacking Upgrades

| Upgrade Level | Speed at 1 Grid |
|---------------|-----------------|
| Level 1 | 9.25 items/sec |
| Level 2 | 11.2 items/sec |
| Level 3 | 12.6 items/sec |
| Level 4 | 13.6 items/sec |
| Level 5 | 14.4 items/sec |

#### Pile Sorter
- Latest sorter type
- When fully upgraded: 120 items/sec
- Outputs 30 full quad stacks per second
- Can fully saturate a Mk.III belt

#### Sorter Mechanics
- Sorters combine insert, remove, and filter functions
- Energy per item moved is constant across all tiers
- Only Mk.III sorters can stack items
- Stacked items on belts travel as single units

### Building Machines

#### Assembling Machines

| Type | Speed Multiplier | Notes |
|------|------------------|-------|
| Assembling Machine Mk.I | 0.75x | Basic assembler |
| Assembling Machine Mk.II | 1.0x | Standard speed |
| Assembling Machine Mk.III | 1.5x | Fast assembler |
| Assembling Machine Mk.IV | 2.0x+ | Fastest tier |

#### Smelters

| Type | Function | Power |
|------|----------|-------|
| Arc Smelter | Basic ore processing | Standard |
| Plane Smelter | Advanced smelting | 4-5x Arc Smelter |

#### Specialized Facilities

| Facility | Function | Key Recipes |
|----------|----------|-------------|
| Oil Refinery | Oil processing | Plasma Refining, X-Ray Cracking |
| Chemical Plant | Chemical synthesis | Plastic, Graphene, Sulfuric Acid |
| Quantum Chemical Plant | Advanced chemistry | Higher speed chemical processing |
| Miniature Particle Collider | Particle physics | Strange Matter, Deuterium, Antimatter |
| Matrix Lab | Research/Production | All Science Matrices |
| Fractionator | Hydrogen processing | Deuterium extraction (1% rate) |

### Fluid Handling

#### Key Mechanics
- **No pipes exist** - all fluids transported via belts in containers
- Fluids are encapsulated and moved like solid items
- Storage Tanks connect at the bottom when stacked
- Stacked tanks function as a single unit

#### Storage Tank Behavior
- Input/output only from bottom tank
- Auto-fills from bottom up
- Can create loopback systems for overflow handling

#### Fluid Storage Encapsulation
- Allows liquid items to be transported on conveyor belts
- Each "fluid" item represents a unit of liquid
- Refined Oil, Hydrogen, Sulfuric Acid, Water all belt-transportable

### Planetary Logistics

#### Planetary Logistics Station
- Transports resources within a single planet
- Uses Logistics Drones for transport
- Can hold up to 50 drones
- Supports 5 item slots for supply/demand
- Modes: Local Supply, Local Demand, Storage

#### Logistics Drone Mechanics
- Carry items between stations on same planet
- Fully simulated flight paths
- Speed upgradeable through research
- Power consumption during flight

### Interstellar Logistics

#### Interstellar Logistics Station
- Transports resources between planets and star systems
- Uses both Drones (local) and Vessels (remote)
- Can hold 50 drones + 10 vessels
- Larger footprint than Planetary stations
- Higher power consumption

#### Logistics Vessel Mechanics
| Property | Base Value | Notes |
|----------|------------|-------|
| Capacity | 200 items | Upgradeable |
| Minimum Load | 200 items | Won't transport less |
| Base Speed | ~1,600 m/s | Upgradeable |
| Max Speed (upgraded) | 8,700+ m/s | With research |
| Warp Speed | 0.73+ ly/sec | Requires Space Warpers |

#### Warp Mechanics
- Requires Space Warpers loaded in station (50 max)
- Must enable warp in station settings
- Dramatically reduces interstellar travel time
- Increased power load during warp

---

## Technology and Research Tree

### Research Matrix Types

#### Electromagnetic Matrix (Blue Cube)
| Property | Value |
|----------|-------|
| Recipe | 1 Circuit Board + 1 Magnetic Coil |
| Crafting Time | 3 seconds |
| Production Rate | 20/minute per lab |
| Facility | Matrix Lab |
| Purpose | Basic technology research |

#### Energy Matrix (Red Cube)
| Property | Value |
|----------|-------|
| Recipe | 2 Energetic Graphite + 2 Hydrogen |
| Crafting Time | 6 seconds |
| Production Rate | 10/minute per lab |
| Facility | Matrix Lab |
| Purpose | Energy and power research |

#### Structure Matrix (Yellow Cube)
| Property | Value |
|----------|-------|
| Recipe | 1 Diamond + 1 Titanium Crystal |
| Crafting Time | 8 seconds |
| Production Rate | 7.5/minute per lab |
| Facility | Matrix Lab |
| Purpose | Structural and logistics research |

#### Information Matrix (Purple Cube)
| Property | Value |
|----------|-------|
| Recipe | 2 Processor + 1 Particle Broadband |
| Crafting Time | 10 seconds |
| Production Rate | 6/minute per lab |
| Facility | Matrix Lab |
| Purpose | Advanced computing research |

#### Gravity Matrix (Green Cube)
| Property | Value |
|----------|-------|
| Recipe | 1 Graviton Lens + 1 Quantum Chip |
| Crafting Time | 24 seconds |
| Production Rate | 2.5/minute per lab |
| Facility | Matrix Lab |
| Purpose | Space-warping technology |

#### Universe Matrix (White Cube)
| Property | Value |
|----------|-------|
| Recipe | 1 of each other Matrix + Antimatter |
| Crafting Time | 15+ seconds |
| Production Rate | Variable |
| Facility | Matrix Lab |
| Purpose | End-game infinite research |

### Tech Tree Structure

#### Progression Stages

| Stage | Matrix Required | Focus Areas |
|-------|-----------------|-------------|
| Tier 1-2 | Blue | Basic automation, smelting, assembly |
| Tier 3-4 | Blue + Red | Oil processing, fluid automation |
| Tier 5-6 | Blue + Red + Yellow | Interplanetary, logistics |
| Tier 7-8 | All previous + Purple | Complex automation, computing |
| Tier 9-10 | All previous + Green | Dyson construction, warping |
| End-game | White | Infinite upgrades, optimization |

#### Key Technology Unlocks

| Technology | Matrix Cost | Unlocks |
|------------|-------------|---------|
| Electromagnetic Matrix | Starting | Blue cube production |
| Basic Logistics System | Blue | Sorters, Belts Mk.I |
| Plasma Extract Refining | Blue + Red | Oil Refinery, Hydrogen |
| Interplanetary Logistics | Yellow | Planetary Logistics Station |
| Interstellar Logistics | Purple | Interstellar Logistics Station |
| Gravitational Wave Refraction | Green | Space Warper, Warp Drive |
| Vertical Launching Silo | Purple (576k hashes) | Small Carrier Rockets |
| Dyson Sphere Stress System | Green | Dyson Sphere framework |

### Research Mechanics

#### Matrix Lab Functions
1. **Production Mode**: Creates science matrices from components
2. **Research Mode**: Consumes matrices to generate hashes

#### Hash Generation
| Property | Value |
|----------|-------|
| Base Rate | 60 hashes/second per lab |
| Research Speed Upgrade | +60 hashes/sec per level |
| Multiple Labs | Stack for higher hash rate |

#### Research Requirements
- Labs must have ALL required matrix types
- Partial matrix availability = no research
- Labs can be stacked vertically
- Research mode labs pass matrices upward

---

## Production Chains

### Smelting Recipes

| Product | Input | Time | Rate/min |
|---------|-------|------|----------|
| Iron Ingot | 1 Iron Ore | 1s | 60 |
| Copper Ingot | 1 Copper Ore | 1s | 60 |
| Magnet | 1 Iron Ore | 1.5s | 40 |
| Stone Brick | 1 Stone | 1s | 60 |
| Glass | 2 Stone | 2s | 30 |
| High-Purity Silicon | 2 Silicon Ore | 2s | 30 |
| Titanium Ingot | 2 Titanium Ore | 2s | 30 |
| Steel | 3 Iron Ingot | 3s | 20 |
| Energetic Graphite | 2 Coal | 2s | 30 |
| Diamond | 1 Energetic Graphite | 2s | 30 |
| Crystal Silicon | 1 High-Purity Silicon | 2s | 30 |

### Oil Processing

#### Plasma Extract Refining (Basic)
- **Input**: 2 Crude Oil
- **Output**: 2 Refined Oil + 1 Hydrogen
- **Time**: 4 seconds
- **Rate**: 30 Crude/min = 30 Refined + 15 Hydrogen

#### X-Ray Cracking
- **Input**: 2 Refined Oil + 1 Hydrogen
- **Output**: 3 Hydrogen + 1 Energetic Graphite
- **Time**: 4 seconds
- **Net Gain**: +2 Hydrogen per cycle

#### Reforming Refine
- **Input**: 2 Hydrogen + 1 Coal + 2 Refined Oil
- **Output**: 3 Refined Oil + 1 Graphite
- **Time**: 4 seconds
- **Net Gain**: +1 Refined Oil

### Chemical Plant Recipes

| Product | Inputs | Time | Notes |
|---------|--------|------|-------|
| Plastic | 2 Refined Oil + 1 Energetic Graphite | 3s | Basic polymer |
| Sulfuric Acid | Refined Oil + Stone + Water | 6s | Industrial acid |
| Organic Crystal | Plastic + Refined Oil + Water | 6s | Alternative to veins |
| Graphene | 3 Energetic Graphite + 1 Sulfuric Acid | 3s | Carbon material |
| Carbon Nanotube | 3 Graphene + 1 Titanium Ingot | 4s | Advanced material |

### Key Production Ratios

#### Smelter Ratios
- 1 Smelter : 2 Mining veins (for Iron/Copper)
- 60 ore/min consumption per smelter
- 1 Iron Ingot = 1 second processing

#### Matrix Lab Ratios (Production)
| Matrix | Labs Needed (per belt) |
|--------|------------------------|
| Blue | 3 labs |
| Red | 6 labs |
| Yellow | 8 labs |
| Purple | 10 labs |
| Green | 12 labs |

#### Oil Refinery Ratios
- 3 Oil Refineries per Oil Extractor (recommended)
- 1 Refinery per 0.5 oil/second extraction rate
- 1 Plasma Refinery feeds 2 X-Ray Cracking Refineries

---

## World and Map

### Planet Types

#### Habitable Planets (In Habitable Zone)

| Type | Characteristics | Key Resources |
|------|-----------------|---------------|
| Mediterranean | Starting world type | Iron, Copper, Oil, Coal, Stone |
| Prairie | Grassland planet | Coal, possible Spiniform Crystal |
| Oceanic Jungle | Tropical water world | Crude Oil, possible organic resources |
| Waterworld | Mostly water | High Crude Oil, Spiniform Crystal |

#### Hot Planets (Close to Star)

| Type | Characteristics | Key Resources |
|------|-----------------|---------------|
| Lava | Volcanic, molten surface | Iron, Copper, Titanium (high) |
| Volcanic Ash | Ash-covered, sulfur ocean | Iron, Copper, Titanium, Sulfur |
| Arid Desert | Hot, dry | Copper, Titanium |

#### Cold Planets (Far from Star)

| Type | Characteristics | Key Resources |
|------|-----------------|---------------|
| Ice Field Gelisol | Frozen surface | Titanium (high), Fire Ice |
| Glacieon | Ice covered | Fire Ice veins |
| Frozen Tundra | Permafrost | Crystalline resources |

#### Barren Planets (Anywhere)

| Type | Characteristics | Key Resources |
|------|-----------------|---------------|
| Barren Desert | No liquids, no wind | Stone (extreme), minimal others |
| Gobi | Rocky desert | Copper, Silicon, Fractal Silicon |
| Red Stone | Iron-rich surface | Iron deposits |
| Rocky Salt Lake | Salt deposits | Varied minerals |

#### Gas Giants
- Cannot land on surface
- Orbit for resource collection only
- Resources: Fire Ice, Hydrogen, Deuterium
- Require Orbital Collectors

### Star Types

| Type | Color | Luminosity | Dyson Sphere Quality |
|------|-------|------------|---------------------|
| O | Blue | Highest | Best |
| B | Light Blue | Very High | Excellent |
| A | Blue-White | High | Very Good |
| F | Yellow-White | Medium-High | Good |
| G | Yellow | Medium | Average |
| K | Orange | Medium-Low | Below Average |
| M | Red | Low | Poor |
| White Dwarf | White | Low | Poor |
| Neutron Star | - | Very Low | Very Poor |
| Black Hole | - | 0.1 | Negligible |

#### Optimal Stars for Dyson Spheres
1. **O-Type**: Largest radius, highest luminosity
2. **B-Type**: Second choice, more common than O
3. **A-Type**: Good balance of availability and output

### Resource Distribution

#### By Planet Category
| Category | Typical Resources |
|----------|-------------------|
| Habitable | Coal, Crude Oil, Organic materials |
| Hot | Iron, Copper, Titanium, Sulfur |
| Frozen | Fire Ice, Titanium, Crystalline |
| Barren | Stone, Silicon, Non-metals |

### Rare Resources

| Resource | Planet Types | Use Case |
|----------|--------------|----------|
| Fire Ice | Ice planets, Gas Giants | Bypasses oil infrastructure |
| Organic Crystal | Special veins | Yellow science shortcut |
| Optical Grating Crystal | Special locations | Casimir Crystal production |
| Spiniform Stalagmite | Prairie, Ocean | Carbon Nanotube production |
| Unipolar Magnet | Special locations | Particle Container |
| Fractal Silicon | Gobi planets | Silicon shortcut (minor) |
| Kimberlite | Special veins | Diamond alternative (minor) |

#### Rare Resource Value Ranking
1. **Fire Ice** - Essential, saves entire oil production chain
2. **Organic Crystal** - Very useful for yellow science
3. **Optical Grating Crystal** - Valuable for Casimir
4. **Unipolar Magnet** - Useful for late game
5. **Spiniform Stalagmite** - Helpful for nanotubes
6. **Fractal Silicon** - Minor benefit
7. **Kimberlite** - Minor benefit

### Galaxy Generation

#### Seed System
- Each galaxy generated from a numeric seed
- Seed determines all star/planet placement
- Resources, planet types, rare materials fixed per seed
- Same seed = identical galaxy

#### Galaxy Size Options
- Small clusters to large galaxies
- More stars = more variety but longer travel

---

## Dyson Sphere Construction

### Dyson Swarm vs Dyson Sphere

#### Dyson Swarm
- Loose collection of Solar Sails orbiting a star
- Created by launching sails from EM-Rail Ejectors
- Temporary - sails eventually decay
- Can be built without Sphere framework
- Lower initial investment

#### Dyson Sphere
- Permanent rigid structure around a star
- Requires framework built from Carrier Rockets
- Solar Sails integrate into framework cells
- Higher power output potential
- Permanent installation

### Construction Requirements

#### EM-Rail Ejector (Solar Sails)
| Property | Value |
|----------|-------|
| Launch Rate | 20 sails/minute (40 with proliferator) |
| Placement | Best at planet poles for continuous operation |
| Requirement | Line of sight to swarm orbit point |
| Product | Solar Sail |

#### Vertical Launching Silo (Rockets)
| Property | Value |
|----------|-------|
| Launch Rate | 5 rockets/minute (10 with proliferator) |
| Product | Small Carrier Rocket |
| Unlock | Purple science (576k hashes) |

#### Solar Sail Recipe
- Graphene + Photon Combiner
- Launched into swarm orbit
- Decays over time in swarm form
- Permanent when absorbed into sphere

#### Small Carrier Rocket Recipe
Components required:
- Dyson Sphere Component (80 assemblers worth)
- Deuterium Fuel Rod
- Quantum Chip

### Sphere Construction Process

1. **Design Phase**: Create sphere blueprint in editor
2. **Framework**: Launch rockets to build nodes and frames
3. **Shells**: Sails absorb into completed framework sections
4. **Completion**: Framework + shells = power generation

#### Structure Points
- 1 Small Carrier Rocket = 1 Structure Point
- Each node requires 30 Structure Points (30 rockets)

#### Cell Points (Sails)
- 1 Solar Sail = 1 Cell Point
- Sails absorb through completed nodes
- Absorption rate: 30 sails/minute per available node
- Each node can request 120 sails at a time

### Power Output Mechanics

Dyson Sphere power output depends on:
1. **Star Luminosity**: Higher = more power (multiplier effect)
2. **Structure Points**: Total rockets launched
3. **Cell Points**: Total sails absorbed
4. **Sphere Radius**: Larger stars allow larger spheres

#### Ray Receiver Power Collection
| Mode | Base Power | With Graviton Lens |
|------|------------|-------------------|
| Power Generation | 6-15 MW | N/A |
| Photon Generation | 48-120 MW | 96-240 MW |

---

## Icarus Mech Mechanics

### Core Abilities

#### Movement
| Ability | Description | Unlock |
|---------|-------------|--------|
| Walking | Basic ground movement | Starting |
| Low-Altitude Flight | Hover above ground | Drive Engine 1 |
| Planetary Flight | Sustained atmospheric flight | Drive Engine 1 |
| Space Flight | Interplanetary travel | Drive Engine 2 |
| Warp | Interstellar travel | Warp research + Space Warpers |

#### Production
- Hand-crafting at same speed as facilities
- Some recipes cannot be hand-crafted
- Consumes inventory materials directly

#### Construction
- Deploys construction drone fleet
- Drones build/deconstruct autonomously
- Number and speed upgradeable

#### Combat
- Equipped with medium-range laser weapon
- Multiple ammunition slots
- Auto or manual fire modes

### Upgrades

#### Upgrade Categories

| Category | Function | Max Level |
|----------|----------|-----------|
| Mecha Core | Energy storage, durability, laser | 7+ |
| Drive Engine | Movement speed, flight capability | 6+ |
| Energy Circuit | Power generation efficiency | Variable |
| Drone Engine | Drone movement speed | Variable |
| Communication Control | Number of drones | Variable |
| Mass Construction | Multi-building placement | Variable |
| Inventory Capacity | Storage space | Variable |
| Mechanical Frame | Physical capabilities | Variable |
| Energy Shield | Damage absorption | 26+ (infinite after) |

#### Key Upgrade Unlocks
| Upgrade | Effect |
|---------|--------|
| Mecha Core 1 | Increases max energy storage |
| Drive Engine 1 | Enables sustained flight |
| Mecha Core 2 | Required for interplanetary |
| Drive Engine 2 | Enables space flight |

### Energy and Fuel

#### Mech Power System
- Internal Mecha Core consumes any fuel
- 100% fuel efficiency
- Slow base power generation (upgradeable)
- Wireless charging from Power Towers

#### Charging Methods
1. **Fuel Consumption**: Any burnable fuel
2. **Wireless Power Towers**: Stand nearby
3. **Signal Towers**: Rapid charging when close

#### Fuel Priority for Mech
Highest to lowest energy density:
1. Antimatter Fuel Rod
2. Deuteron Fuel Rod
3. Hydrogen Fuel Rod
4. Energetic Graphite
5. Coal
6. Plant materials

---

## Combat and Dark Fog

### Overview (Rise of the Dark Fog Update)
- Added hostile NPC faction: The Dark Fog
- Self-replicating enemy force
- Born as a "system error" in the simulation
- Attacks player factories periodically

### Enemy Types

#### Space Units
- Dark Fog space fleets
- Attack orbital and ground installations
- Spawn from Hive structures

#### Ground Units
- Emerge from planetary bases
- Target production facilities
- Scale with game progression

### Defense Structures

| Structure | Function | Range |
|-----------|----------|-------|
| Missile Turret | Anti-air/space missiles | Planetary |
| Plasma Turret | Energy weapon | Local area |
| Signal Tower | Coordinates defenses | Planet-wide |

### Dark Fog Structures

#### Relay
- Digs planetary boreholes
- Constructs ground bases
- Transports matter to space core

#### Hive
- Central space structure
- Spawns attack fleets
- Must be destroyed for elimination

### Combat Tips
- 10-20 missile turrets sufficient for most bases
- Signal towers coordinate turret fire
- Build thorough planetary defense systems
- Consider difficulty settings for combat intensity

---

## Endgame

### Victory Conditions

#### Mission Complete
- Research the final technology in the tech tree
- Requires producing 4000 white science cubes (Universe Matrices)
- Unlocks "Mission Accomplished" achievement

#### Requirements
- Full production chain for all 6 matrix types
- Antimatter production capability
- Sufficient power generation
- Interstellar logistics network

### Infinite Research

After completing the tech tree, infinite upgrades become available:

| Upgrade Type | Effect | Resource |
|--------------|--------|----------|
| Research Speed | +60 hashes/sec per level | White matrices |
| Vessel Capacity | Increased cargo | White matrices |
| Mech Upgrades | Various improvements | White matrices |
| Energy Shield | Damage resistance | White matrices |

#### Icarus, PhD Achievement
- Complete ALL research including infinite techs
- Requires at least level 1 of all infinite research
- Energy Shield requires level 26 for infinite tier

### Post-Game Content

#### After Mission Complete
- Game continues as sandbox
- Build additional Dyson Spheres
- Optimize production efficiency
- Achieve 100% achievements
- Challenge runs (speedrun, minimal resources)

#### Dark Matter Integration
- Destroy Dark Fog hives
- Collect Dark Matter
- Power additional research
- Scale to higher production

### Scaling Challenges
- White matrix production requires all previous matrices
- Antimatter production requires functional Dyson Sphere
- Each infinite upgrade costs progressively more
- Logistics become primary bottleneck

---

## Proliferator System

### Overview
Proliferators coat items to provide production bonuses when used in recipes.

### Proliferator Tiers

| Tier | Production Bonus | Speed Bonus | Sprays per Unit |
|------|------------------|-------------|-----------------|
| Mk.I | +12.5% products | +25% speed | 12 |
| Mk.II | +20% products | +50% speed | 24 |
| Mk.III | +25% products | +100% speed | 60 (75 if self-sprayed) |

### Spray Modes
1. **Extra Products**: Chance for bonus output items
2. **Production Speedup**: Faster recipe completion

### Key Rules
- ALL inputs must be proliferated for effect
- Partial proliferation = wasted resources
- Self-spraying proliferators increases charges
- Cannot use both modes simultaneously

### Spray Coater
- Automated spraying building
- Place on belt to coat passing items
- Requires proliferator supply

---

## Summary Statistics

### Production Building Count Reference

| Building | Size | Input Slots | Output |
|----------|------|-------------|--------|
| Arc Smelter | 3x3 | 1 | 1 |
| Assembler Mk.I-IV | 3x3 | 4 max | 1 |
| Chemical Plant | 3x3 | 3 | 2 |
| Oil Refinery | 3x3 | 2 | 2 |
| Matrix Lab | 3x3 | 6 | 1 |
| Particle Collider | 3x3 | 2 | 2 |

### Power Reference

| Source | Output | Fuel/Input |
|--------|--------|------------|
| Wind Turbine | 0-300 kW | None |
| Solar Panel | 0-360 kW | Sunlight |
| Thermal Power | 2.16 MW | Any fuel |
| Mini Fusion | 9-15 MW | Deuteron Rods |
| Ray Receiver | 5-240 MW | Dyson light |
| Artificial Star | 75 MW | Antimatter Rods |

### Belt Throughput Reference

| Belt | Items/sec | Items/min |
|------|-----------|-----------|
| Mk.I | 6 | 360 |
| Mk.II | 12 | 720 |
| Mk.III | 30 | 1,800 |

---

*Document compiled from community wikis, guides, and game data. Values may be subject to game updates and patches.*
