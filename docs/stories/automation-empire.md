# Automation Empire - Game Design Research Document

## Table of Contents

1. [Game Overview](#game-overview)
2. [Initial Conditions](#initial-conditions)
3. [Core Mechanics](#core-mechanics)
4. [Resource System](#resource-system)
5. [Mining and Extraction](#mining-and-extraction)
6. [Power Generation](#power-generation)
7. [Conveyor System](#conveyor-system)
8. [Factory Buildings](#factory-buildings)
9. [Vehicle Logistics](#vehicle-logistics)
10. [Train System](#train-system)
11. [Monorail System](#monorail-system)
12. [Building Types](#building-types)
13. [Technology and Research](#technology-and-research)
14. [Production Chains](#production-chains)
15. [Water System](#water-system)
16. [World and Maps](#world-and-maps)
17. [Shipping and Export](#shipping-and-export)
18. [Endgame](#endgame)
19. [Sources](#sources)

---

## Game Overview

**Developer:** Dog Hoggler
**Release:** November 2019
**Platform:** PC (Steam)
**Genre:** Factory simulation/management game

Automation Empire is a simulation/management game focused on efficiency and expansion. Players start with nothing and build up a massive interconnected industrial network of factories and machines. The game emphasizes logistics and transportation optimization over complex crafting trees.

### Key Characteristics
- No survival elements or combat
- No expansive tech tree or exponentially complex resource chains
- Appeal lies in simplicity and logistics optimization
- 3D graphics with lovingly animated machines and vehicles
- Focus on economical transportation of resources
- Resources are infinite (no depletion mechanics)

### Known Limitations
- No in-game tutorial or manual
- No plot or enemies
- No speed selection for time management
- Limited optimization in later game stages
- No in-game music
- Development appears to have ended with the Monorail update

---

## Initial Conditions

### Game Start Options

When starting a new game, players must:
1. Name their save file
2. Choose game mode options:
   - **Starter Mode:** Recommended for beginners; provides better starting funds (selling all placed starter items returns full purchase value, often resulting in more total funds than non-starter mode)
   - **Challenge Mode:** Introduces a tax mechanic; described as "bizarre and sadistic" by the community

### Starting Scenario

**Sandbox Mode:** Achieved by selecting "Challenge Mode - OFF"
- Drastically lower taxes
- Faster item purchasing
- More relaxed progression

**Standard Mode:**
- Initial cash allocation for basic infrastructure
- Access to basic buildings (mines, basic conveyors)
- Four base resources available on all maps from the start

### Initial Goals
- First milestone: Ship 30,000 kg within a 3-month period
- Second milestone: Ship 45,000 kg within a 3-month period

---

## Core Mechanics

### Fundamental Gameplay Loop

1. **Extract** raw resources from the ground (mining/drilling)
2. **Refine** raw ores into processed materials
3. **Combine** processed materials into advanced products
4. **Transport** goods using various logistics methods
5. **Ship** products to earn money
6. **Research** to unlock new technologies
7. **Expand** production capacity

### Time System
- Game operates on a monthly cycle for shipping quotas
- 3-month periods for milestone tracking
- Real-time production with specific timing for each machine

### Economy
- Currency symbol: Ʀ (appears to be a stylized R)
- Full refund on sold buildings (no depreciation)
- Resource bonuses decline over time (see Shipping section)
- Taxes in Challenge Mode

### Controls
- Hold TAB to see inside all factories simultaneously
- Hotkeys 1, 2, 3 to switch track heights in build mode
- Book icon at top of screen shows recipes

---

## Resource System

### Base Raw Resources (Ores)

All maps contain four base resource types:

| Resource | Refined Weight | Refine Time | Notes |
|----------|---------------|-------------|-------|
| Coal Ore | 10 kg (as Coal) | 10 seconds | Used for fuel and carbon |
| Iron Ore | 10 kg (as Iron) | 11 seconds | Primary construction material |
| Gold Ore | 10 kg (as Gold) | 13 seconds | High-value processing |
| Crude Oil | Varies | 15 seconds | Latest to unlock, used for fuel/ethanol |

### Refined Resources

| Resource | Weight | Source |
|----------|--------|--------|
| Coal | 10 kg | Coal Ore refining |
| Iron | 10 kg | Iron Ore refining |
| Gold | 10 kg | Gold Ore refining |
| Oil | - | Crude Oil refining |
| Carbon | - | Secondary Coal refining (Coal + Water) |

### Combined/Compound Resources

| Resource | Weight | Recipe | Notes |
|----------|--------|--------|-------|
| Steel Plates | 13 kg (x2 = 26 kg total) | 10 kg Coal + 10 kg Iron | Net weight gain of 6 kg |
| Gold Fuel | 13 kg | Gold + Coal | Requires refined materials |
| Capacitors | - | 2 Gold + 2 Iron | Requires refined materials |
| Carbon Meal | ~4 kg actual | Advanced coal processing | Listed as 7 kg but actual is 4 kg |
| Mega Fuel | 16 kg | Advanced fuel variant | Highest weight product |
| Grain Meal | ~4 kg actual | Green Grain + Red Grain | Listed as 7 kg |
| Ethanol | - | Oil + Red Grain | Good sell value |

### Farming Resources

| Resource | Source | Production Rate |
|----------|--------|-----------------|
| Red Grass | Farm/Greenhouse | 1 mat per 7.5 seconds (6 mats per 45 seconds) |
| Green Grass | Farm/Greenhouse | 1 mat per 7.5 seconds |
| Red Grain | Refined Red Grass | Secondary processing |
| Green Grain | Refined Green Grass | Secondary processing |

### Resource Weight Summary

**Important:** Weights are NOT affected by bonuses and remain constant throughout the game.

---

## Mining and Extraction

### Mine Buildings
- Vertical machines that drill and bring up resources
- Resources are infinite (no depletion)
- One mining rig feeds approximately one refiner (matched production rates)

### Production Ratios
- 1 Mining Rig : 1 Refiner (optimal ratio)
- Mining speed can be increased with water connection (+30% speed bonus)

### Oil Derricks
- Unlocked later in game progression
- Extracts crude oil
- Oil refines at 1 unit per 15 seconds (slowest refining rate)

---

## Power Generation

### Power Station
- Each power station produces **40 units of power**
- Multiple stations can connect to the same power grid (combined output)
- **No fuel required** - automated power generation
- Can be placed together or spread across the map
- Power is required for factory operations

### Power Grid
- Factories connect to power grid
- Water connection to factories reduces energy costs

---

## Conveyor System

### Step Conveyors

**Speed:** Move crates in 1-second timed intervals
**Effective Throughput:** 1 crate every 2 seconds (due to spacing logic)
**Unlock Cost:** 45,000 Ʀ

Step conveyor output is constant at 1 crate every 2 seconds, allowing for standardized factory flow design.

### Unloading Stations/Belts
- Only conveyor belt type that can be placed outdoors
- Used with claw trains for loading/unloading
- Interfaces between indoor and outdoor logistics

### Load Belts
- Indoor conveyor for receiving crates from claw trains
- Should be long enough to handle full claw train cargo
- Feeds onto step conveyors

---

## Factory Buildings

### Crate Maker

**Production Rate:**
- Creates 1 empty crate in 1 second
- Fills crate with commodity in 3 seconds
- **Effective rate: 1 crate every 4 seconds**
- 3 crates every 12 seconds

**Capacity:** Maximum 30 crates

**Ratios:**
- 1 Crate Maker can supply approximately:
  - 2.5 Coal Refiners
  - 2.75 Iron Refiners
  - 3.25 Refiners (general average)

### Refiner

**Unlock Cost:** 20,000 Ʀ

**Refining Times:**
| Resource | Time |
|----------|------|
| Coal | 10 seconds |
| Iron | 11 seconds |
| Gold | 13 seconds |
| Oil | 15 seconds |

**Requirements:**
- Water connection for compound resource production
- Water connection reduces energy costs

### Combiner

**Unlock Cost:** 6,000 kg shipped + 50 Iron RSp + 50 Coal RSp + 500,000 Ʀ

**Production Rate:** 1 combination every 9 seconds

**Recipes:**
- Steel Plates: 1 Iron + 1 Coal = 2 Steel Plates
- Gold Fuel: 1 Gold + 1 Coal = Gold Fuel
- Capacitors: 2 Gold + 2 Iron = Capacitors
- Ethanol: Oil + Red Grain (operates at 60% capacity due to oil's slower production)

**Note:** Combiner recipes require REFINED materials, not raw ore. Using wrong materials produces "ash."

### Water Reservoir

**Production:** 40 units of water

**Unlock Requirements:**
- 100 kg of gold refined
- 11,000 kg shipped within 3-month period

### Container/Silo
- Storage for resources
- Used for buffering between production stages
- "Suckers" pull resources from crates into containers

---

## Vehicle Logistics

### Drones/Bots

**Role:** Early-game transport system

**Characteristics:**
- Free movement (cannot be programmed or assigned areas)
- AI described as "dumb" - bots bump into each other
- Useful for loading trucks at game start
- Eventually replaced by claw trains

**Strategy:** Avoid sending drones on long journeys - travel time significantly impacts efficiency.

**Upgrades Available:**
- Drone speed (up to Tier V)

### Trucks

**Capacity:** Departs when loaded with **6 crates**

**Characteristics:**
- Automated departure when full
- Uses road network
- One-way roads can be built for traffic optimization
- Multiple loading stations increase throughput

**Comparison to Trains:**
- 3-4 truck stops often more efficient than 6 train cars
- Faster loading/unloading than trains

**Upgrades Available:**
- Truck speed
- Truck intervals (Tier V costs included in 5,750,000 Ʀ total upgrades)

### Claw Trains (Clawtrains)

**Unlock Cost:** 1,500 kg shipped + 80,000 Ʀ

**Capacity:** 1 crate per claw (compared to 4 crates per minecart)

**Characteristics:**
- Hook railway system picks up crates from unload belts
- Drops crates onto load belts in other factories
- Loading/unloading is the biggest bottleneck
- Requires multiple claw trains to be effective
- Claw tracks can be difficult to delete

**Upgrades Available:**
- Clawtrain speed (up to Tier V)
- Clawtrain capacity (up to Tier V)
- Crane speed
- Crane limit

**Clawtrack Truck Loader Unlock:** 4,000 kg + 25 Coal RSp + 150,000 Ʀ

### Cargo Rockets

**Unlock Cost:** 22,000 kg shipped + 300 Steel Plates RSp + 300 Gold Fuel RSp + 300 Capacitors RSp + 300 Ethanol RSp + 5,000,000 Ʀ

**Bonus:** 45% cash and weight bonus

**Role:** End-game shipping solution - provides the best bonuses for export.

---

## Train System

### Minecarts

**Capacity:** 4 crates worth of resources

**Track Types:**
| Track Type | Speed | Use Case |
|------------|-------|----------|
| Mine Track | Standard | Short-distance, within factories |
| Highway Track | Faster | Long-distance transport |

**Characteristics:**
- Ideal for long-distance transport due to capacity
- Can transfer resources between step conveyor and mine track
- Can run entire map without crate makers (train-only logistics)

**Adding Minecarts:**
- Select built track
- Press "Add" button to purchase
- Hotkeys 1, 2, 3 to switch track heights

### Freight Trains

**Unlock Cost:** 8,000 kg shipped + 100 Steel Plates RSp + 50 Gold RSp + 800,000 Ʀ

**Characteristics:**
- Slow to load but carry enormous amounts
- Far greater capacity than trucks
- Output using trains significantly exceeds trucks

**Wagon Configuration:**
- Wagons determined by silos over track
- 6 silos = 8 train cars
- Can exceed 50 wagon limit
- 8 cars fills and delivers 1 load per month

**Recommendations:**
- 6 train cars less efficient than 3-4 truck stops
- Recommend 12-18 cars minimum if space available

**Upgrades Available:**
- Train intervals (up to Tier V)
- Maximum vehicles on line (research)

### Track Building
- Use hotkeys 1, 2, 3 for different track heights
- Mine track for standard speed
- Highway track for faster minecart travel

### Loading/Unloading
- Pick up/drop stations required
- Underneath needs load/unload station on factory floor
- Load belt should be long enough for full cargo
- Content extraction works same as minecart system

---

## Monorail System

**Unlock:** Final update content (Monorail & Modding Update - November 2020)

### Characteristics
- Elevated rail system
- **Capacity:** 5x the resources of standard minecarts
- Loading/unloading is **instantaneous**
- Cars do not stop - collect/deliver cargo while passing over loaders/unloaders

### Benefits
- Massively boosts long-distance transport efficiency
- Very fast transfer speed
- No loading delay

### Known Issues
- Final update from developer (no further fixes)
- Reported bugs on certain maps (e.g., Glacier Zone)
- Some players find it comes too late in progression to be valuable

---

## Building Types

### Extraction Buildings

| Building | Function | Notes |
|----------|----------|-------|
| Mine | Extracts ore from ground | Vertical drilling machine |
| Oil Derrick | Extracts crude oil | Late-game unlock (12,000 kg + 100 Gold Fuel RSp + 2,000,000 Ʀ) |
| Farm | Grows grass | Outdoor agriculture |
| Greenhouse | Grows grass faster | 6 mats per 45 seconds, unlock: 18,000 kg + 300 Grain Meal RSp + 800,000 Ʀ |

### Processing Buildings

| Building | Function | Unlock Cost |
|----------|----------|-------------|
| Refiner | Converts ore to refined materials | 20,000 Ʀ |
| Combiner | Creates compound resources | 6,000 kg + 50 Iron + 50 Coal + 500,000 Ʀ |
| Crate Maker | Packages resources for transport | Base building |

### Logistics Buildings

| Building | Function | Unlock Cost |
|----------|----------|-------------|
| Container/Silo | Resource storage | Base building |
| Step Conveyor | Indoor transport | 45,000 Ʀ |
| Unloading Station | Outdoor conveyor for claw trains | Included with claw system |
| Truck Stop | Truck loading/departure point | Base building |
| Clawtrain Station | Pick up/drop points | Part of clawtrain unlock |
| Remote Connector | Long-distance connections | 9,000 kg + 500,000 Ʀ |

### Infrastructure Buildings

| Building | Function | Notes |
|----------|----------|-------|
| Power Station | Generates 40 power units | No fuel required |
| Water Reservoir | Produces 40 water units | Unlock: 100 kg gold + 11,000 kg shipped |
| Research Bay | Processes resources for research | Claims research points |

### Control Buildings

| Building | Function | Unlock Cost |
|----------|----------|-------------|
| Crate Gate | Controls crate flow | 3,000 kg + 25 Iron RSp + 150,000 Ʀ |

---

## Technology and Research

### Research System

**Research Bays:**
- Process resources for research points
- Resources are "claimed" after processing 20 crates total
- Claimed research applies to ALL unlocks needing that resource simultaneously

**Research Points:**
- Raw ore provides fewer research points (e.g., 2 points)
- Refined materials provide more points (e.g., 4 points)

**Important:** Research unlocks require REFINED materials, not raw ore. Researching coal ore only provides base research points, not material requirements for tech unlocks.

### Unlock Requirements Structure

Unlocks typically require:
1. **Minimum kilograms shipped** (cumulative)
2. **Research points** from specific refined materials (RSp)
3. **Currency cost** (Ʀ)
4. **Minimum 3-month shipping quota** (often required)

### Complete Unlock Tree

| Technology | kg Shipped | Research Materials | Cost (Ʀ) |
|------------|------------|-------------------|----------|
| Refiner | - | - | 20,000 |
| Step-Conveyor | - | - | 45,000 |
| Clawtrain | 1,500 | - | 80,000 |
| Crate Gate | 3,000 | 25 Iron RSp | 150,000 |
| Clawtrack Truck Loader | 4,000 | 25 Coal RSp | 150,000 |
| Combiner | 6,000 | 50 Iron + 50 Coal RSp | 500,000 |
| Freight Train | 8,000 | 100 Steel Plates + 50 Gold RSp | 800,000 |
| Remote Connector | 9,000 | - | 500,000 |
| Waterworks | 11,000 | 100 Gold RSp | 600,000 |
| Oil Derrick | 12,000 | 100 Gold Fuel RSp | 2,000,000 |
| Farming | 15,000 | 200 Oil RSp | 200,000 |
| Greenhouse | 18,000 | 300 Grain Meal RSp | 800,000 |
| Cargo Rocket | 22,000 | 300 Steel Plates + 300 Gold Fuel + 300 Capacitors + 300 Ethanol RSp | 5,000,000 |

### Vehicle/Machine Upgrades

All upgrades to Tier V cost a combined **5,750,000 Ʀ** total:
- Minecart Speed (Tiers I-V)
- Clawtrain Speed (Tiers I-V)
- Drone Speed (Tiers I-V)
- Minecart Capacity (Tiers I-V)
- Clawtrain Capacity (Tiers I-V)
- Train Intervals (Tiers I-V)
- Truck Intervals (Tiers I-V)

---

## Production Chains

### Basic Processing Chain

```
[Mining Rig] → [Ore] → [Crate Maker] → [Refiner] → [Refined Material] → [Export]
```

### Steel Plates Production

```
Coal Ore → Refine (10s) → Coal (10 kg)
                                        ↘
                                         → Combiner (9s) → Steel Plates (2x 13 kg = 26 kg)
                                        ↗
Iron Ore → Refine (11s) → Iron (10 kg)
```

**Efficiency:** 20 kg input produces 26 kg output (30% weight gain)

### Gold Fuel Production

```
Coal Ore → Refine (10s) → Coal (10 kg)
                                        ↘
                                         → Combiner (9s) → Gold Fuel (13 kg)
                                        ↗
Gold Ore → Refine (13s) → Gold (10 kg)
```

### Capacitor Production

```
Gold Ore → Refine (13s) → Gold (10 kg) × 2
                                           ↘
                                            → Combiner (9s) → Capacitors
                                           ↗
Iron Ore → Refine (11s) → Iron (10 kg) × 2
```

### Carbon Production (Secondary Refining)

```
Coal Ore → Refine → Coal → Refine again (with water) → Carbon → Process → Carbon Meal
```

**Requirement:** Water must be connected to factory AND crates must be circulating (not fresh from crate maker)

### Ethanol Production

```
Crude Oil → Refine (15s) → Oil
                               ↘
                                → Combiner (9s) → Ethanol
                               ↗
Red Grass → Refine → Red Grain
```

**Note:** Combiner operates at 60% capacity (9 seconds work every 15 seconds due to slower oil production)

**Recommended Setup:** 7 Combiners + 9 Red Greenhouses + 7 Refiners

### Grain Meal Production

```
Red Grass → Refine → Red Grain
                               ↘
                                → Combiner → Grain Meal (4 kg actual)
                               ↗
Green Grass → Refine → Green Grain
```

### Optimal Production Ratios

| Equipment Ratio | Notes |
|-----------------|-------|
| 1 Mining Rig : 1 Refiner | Matched production rates |
| 1 Crate Maker : 2.5 Coal Refiners | Based on refine times |
| 1 Crate Maker : 2.75 Iron Refiners | Based on refine times |
| 1 Crate Maker : ~3.25 Refiners (avg) | General guideline |

---

## Water System

### Water Reservoir
- Produces **40 units of water**
- Unlocked at: 100 kg gold refined + 11,000 kg shipped in 3-month period

### Water Connections

Run a pipe from water tank or water-connected structure into the side of buildings.

### Water Bonuses by Building Type

| Building | Bonus |
|----------|-------|
| Factory | Reduced energy cost |
| Mine | +30% speed |
| Research Bay | +10% speed |

### Compound Resources Requirement

All compound resources REQUIRE water connection to the factory for production.

---

## World and Maps

### Map Overview
- **Total Maps:** 14 maps available
- **Planets:** 7 different planets with unique biomes

### Biome Variety
Each planet presents unique landforms that must be factored into base layout. However, core gameplay mechanics function identically across all maps.

### Known Map Names
- Compact Canyon
- Middle Mountain
- Resource Rich
- Glacier Zone (note: has reported monorail bugs)

### Resource Distribution
- All four base resources (Coal Ore, Iron Ore, Gold Ore, Crude Oil) available on all maps
- Layout differs between maps
- Resources are infinite (no depletion)

### Map Characteristics
- Terrain affects building placement
- Elevation changes require track height adjustments
- Different maps offer varying difficulty based on resource accessibility

---

## Shipping and Export

### Shipping Methods

| Method | Capacity | Bonus | Notes |
|--------|----------|-------|-------|
| Trucks | 6 crates | None | Early-game shipping |
| Trains | Large (varies) | None | Mid-game, high volume |
| Cargo Rockets | High | 45% cash + weight | End-game, best returns |

### Value Bonus System

**Initial Bonus:** 20% on first shipments of each resource type

**Decay Rate:** -0.08% per crate shipped

**Bonus Duration:** Reaches 0% after 250 crates

**Important:** Weights are NOT affected by bonuses - only monetary value changes.

### Shipping Goals/Milestones

| Milestone | Requirement | Period |
|-----------|-------------|--------|
| First | 30,000 kg | 3 months |
| Second | 45,000 kg | 3 months |

### Optimal Export Resources

Based on community optimization:
- Carbon Meal: ~4.47 per second recommended
- Steel Plates: High weight (26 kg per combo)
- Gold Fuel: Good value/weight ratio
- Capacitors: Required for rocket unlock
- Ethanol: Good sell price

---

## Endgame

### Final Technologies

1. **Cargo Rockets** - The ultimate shipping method
   - 45% cash and weight bonus
   - Requires most advanced research (Steel Plates, Gold Fuel, Capacitors, Ethanol)
   - Cost: 5,000,000 Ʀ

2. **Monorail** - Final update content
   - 5x minecart capacity
   - Instantaneous loading/unloading
   - Best for long-distance, high-volume transport

### Endgame Goals

- Maximize kg/month output
- Optimize production chains for highest-value resources
- Fully upgrade all vehicle tiers (5,750,000 Ʀ total)
- Achieve consistent shipping quotas

### Late-Game Strategy

1. **Prioritize Rockets** for the 45% bonus on all shipments
2. **Use Monorails** for bulk resource transport (if available on map)
3. **Focus on Heavy Resources:**
   - Mega Fuel (16 kg)
   - Steel Plates (13 kg × 2)
   - Gold Fuel (13 kg)
4. **Optimize Combiner Efficiency** - they're often the bottleneck at 9 seconds per operation
5. **Scale Horizontally** - multiple parallel production lines rather than single large chains

### Completion State

The game lacks a definitive "win" state beyond achieving shipping milestones. Players typically set personal goals for:
- Maximum kg/month throughput
- Aesthetic factory designs
- Minimal building count challenges
- All-technology unlocks

---

## Sources

### Steam Community Guides
- [Automation Empire User Guide](https://steamcommunity.com/sharedfiles/filedetails/?id=2686035048)
- [Steam Community Hub](https://steamcommunity.com/app/1112790)

### Game Information
- [Automation Empire on Steam](https://store.steampowered.com/app/1112790/Automation_Empire/)
- [Automation Empire - Metacritic](https://www.metacritic.com/game/automation-empire/)

### Beginner Guides
- [Automation Empire Beginner's Guide - MGW](https://guides.magicgameworld.com/automation-empire-beginners-guide/)
- [Automation Empire Tips & Tricks - Magic Game World](https://www.magicgameworld.com/automation-empire-useful-tips-tricks-controls/)

### Community Discussions
- [Production Chart Discussion](https://steamcommunity.com/app/1112790/discussions/0/1740008576841973089/)
- [Crate maker / refiner ratio](https://steamcommunity.com/app/1112790/discussions/0/1751268210977073516/)
- [Best way to optimise output](https://steamcommunity.com/app/1112790/discussions/0/3345546172888447268/)
- [Research Unlock Requirements](https://steamcommunity.com/app/1112790/discussions/0/1661194916736480117/)
- [Train Cart Amounts](https://steamcommunity.com/app/1112790/discussions/0/1661194916739967938/)
- [Monorail Discussion](https://steamcommunity.com/app/1112790/discussions/0/3114769644700385968/)

### Reviews and Impressions
- [Automation Empire: First Impressions - David Rector](https://blog.rectorsquid.com/automation-empire-first-impressions/)
- [PCGamingWiki - Automation Empire](https://www.pcgamingwiki.com/wiki/Automation_Empire)

### Update Notes
- [Monorail & Modding Update (November 2020)](https://steamdb.info/patchnotes/5789981/)
- [Compound Resources Update](https://store.steampowered.com/news/app/1112790/view/1698352047863246064)

---

*Document compiled from community resources, Steam discussions, and game guides. Some values may vary based on game version or community interpretation.*
