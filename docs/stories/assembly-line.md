# Assembly Line: Comprehensive Game Design Research Document

## Table of Contents
1. [Overview](#1-overview)
2. [Initial Conditions](#2-initial-conditions)
3. [Core Mechanics](#3-core-mechanics)
4. [Machines](#4-machines)
5. [Resources and Products](#5-resources-and-products)
6. [Blueprints and Recipes](#6-blueprints-and-recipes)
7. [Progression System](#7-progression-system)
8. [Level Design and Space Management](#8-level-design-and-space-management)
9. [Unique Mechanics](#9-unique-mechanics)
10. [Monetization Model](#10-monetization-model)
11. [Assembly Line 2 Differences](#11-assembly-line-2-differences)

---

## 1. Overview

### Game Identity

**Assembly Line** is an idle/tycoon incremental game developed by **Joao Reis** (published under Olympus). The game combines elements from idle games and factory simulation, where players design and optimize production lines to manufacture products and maximize profit.

| Specification | Value |
|---------------|-------|
| Developer | Joao Reis |
| Publisher | Olympus |
| Platforms | iOS, Android, Steam (Assembly Line 2) |
| Genre | Idle/Tycoon/Factory Simulation |
| Release | Assembly Line 1: 2018, Assembly Line 2: 2023 |

### Core Design Philosophy

Unlike traditional factory games like Factorio or Satisfactory that feature exploration and combat, Assembly Line is a pure puzzle-optimization game focused on:

- **Constrained Space**: Limited grid area forces efficient layouts
- **No Combat**: Pure production optimization
- **Idle Progression**: Factory continues generating income offline
- **Incremental Unlocks**: New machines and blueprints unlock through progression

### What Distinguishes Assembly Line

1. **No Premium Currency**: Unlike many mobile idle games
2. **Optional Ads Only**: Ads only play when player chooses to double earnings
3. **Pure Factory Design**: No exploration, combat, or survival elements
4. **Compact Puzzle Focus**: Small grid spaces require creative solutions
5. **Multiple Assembly Lines**: Players can own and optimize multiple separate production facilities

---

## 2. Initial Conditions

### Starting State

When players first launch Assembly Line, they begin with:

| Resource | Amount | Purpose |
|----------|--------|---------|
| Starting Cash | Small amount | Purchase first machines |
| Assembly Lines | 1 | Single production facility |
| Grid Space | 6x6 tiles (AL1) / 10x10 chunk (AL2) | Initial building area |
| Starters Limit | 8 maximum | Resource generation cap |
| Machines Unlocked | Starter, Seller, Roller | Basic production flow |

### Tutorial and Onboarding

Assembly Line uses an implicit tutorial approach:

1. **First Action**: Purchase a Starter (resource generator)
2. **Second Action**: Connect to a Seller via Roller
3. **Goal Communication**: See money accumulating in real-time
4. **Progression Hint**: Unlock buttons visible with costs shown

The game does not force tutorial completion - players learn through experimentation with immediate visual and financial feedback.

### First Mechanics Introduced

| Order | Mechanic | Learning Objective |
|-------|----------|-------------------|
| 1 | Starter | Resources are generated, not gathered |
| 2 | Roller | Items must flow through the factory |
| 3 | Seller | Items must reach a destination to profit |
| 4 | Direction | Arrows indicate flow direction |
| 5 | Furnace | Processing increases item value |

### Early Game Strategy

The recommended early progression:

1. **Phase 1**: Buy maximum starters (8), point into rollers, connect to single seller
2. **Phase 2**: Unlock Furnace, process raw materials before selling
3. **Phase 3**: Upgrade to Hydraulic Press (plates sell for $250 each)
4. **Phase 4**: Begin crafting circuits and simple products

---

## 3. Core Mechanics

### 3.1 Resource Flow System

Assembly Line uses a grid-based conveyor system where items flow from generators to processors to sellers.

#### Flow Properties

| Property | Behavior |
|----------|----------|
| Direction | Each machine/roller has a directional arrow |
| Speed | Determined by machine processing speed and upgrades |
| Collision | Items queue when destination is occupied |
| Merging | Multiple inputs can feed single machine |
| Splitting | Requires splitter machines or robotic arms |

#### Grid Movement

- Items move tile-by-tile along rollers
- Processing machines consume inputs and output products
- Dead ends cause backup and halt production
- Every production chain must terminate at a Seller

### 3.2 Operating Costs

All machines have operating costs that affect net profit:

| Cost Type | Description | Impact |
|-----------|-------------|--------|
| Purchase Cost | One-time machine placement | Capital expenditure |
| Electricity | Per-operation power cost | Ongoing expense |
| Space | Tiles occupied | Opportunity cost |

#### Default Operating Costs

| Machine | Base Electricity Cost | Upgradeable To |
|---------|----------------------|----------------|
| Starter | $5 per activation | $1 per activation |
| Crafter | $5 per use | $1 per use |
| Roller | $1 per item moved | - |
| Furnace | Variable | Reduced via upgrades |

### 3.3 Speed and Timing

#### Base Production Rates

| Machine | Default Speed | Upgraded Speed |
|---------|---------------|----------------|
| Starter | 1 item per 3 seconds | 1 item per 1 second |
| Crafter | 1 item per 3 seconds | 1 item per 1 second |
| Refinery | 1 item per 5 seconds | 1 item per 1 second |
| Cable Maker | 1 item per 5 seconds | 1 item per 1 second |

#### Assembly Line 2 Starter Rates

| Upgrade Level | Spawn Rate |
|---------------|------------|
| Base | 1 per 5 seconds |
| Maximum | 4 per second |

#### Maximum Items Per Activation

Starters can be upgraded to output multiple items per activation:

| Upgrade Level | Items Per Activation |
|---------------|---------------------|
| Base | 1 |
| Maximum | 3 |

### 3.4 Profit Calculation

```
Net Profit = Sell Price - (Resource Cost + Operating Costs)
```

Where:
- **Sell Price**: Fixed value per item type (see Profit Chart)
- **Resource Cost**: $5 base per starter activation (reducible to $1)
- **Operating Costs**: Electricity for all machines in the chain

#### Profit Efficiency Metric

Players optimize using **Profit Per Starter**:

```
Efficiency = (Sell Price - Operating Costs) / Number of Starters Used
```

---

## 4. Machines

### 4.1 Machine Categories

Assembly Line machines fall into distinct functional categories:

| Category | Purpose | Examples |
|----------|---------|----------|
| **Generators** | Create raw resources | Starter, Radioactive Starter |
| **Processors** | Transform resources | Furnace, Hydraulic Press, Wire Drawer, Cutter |
| **Crafters** | Combine resources into products | Crafter, Crafter MK1-MK4 |
| **Transportation** | Move items between locations | Roller, Transporter, Robotic Arm |
| **Output** | Convert items to money | Seller |
| **Utility** | Special functions | Filter, Importer |

### 4.2 Complete Machine List (Assembly Line 1)

Assembly Line 1 features **12-18 different machines** (varies by platform):

#### Generation Machines

| Machine | Function | Output |
|---------|----------|--------|
| **Starter** | Generates basic raw resources | Copper, Iron, Gold, Aluminum, Diamond (selectable) |
| **Radioactive Starter** | Generates radioactive resources | Uranium, Plutonium |

#### Processing Machines

| Machine | Input | Output | Processing |
|---------|-------|--------|------------|
| **Furnace** | Raw ore | Ingot/Liquid | Smelts materials |
| **Hydraulic Press** | Raw ore or Ingot | Plate | Compresses materials |
| **Wire Drawer/Maker** | Raw ore or Ingot | Wire | Draws into wire form |
| **Cutter** | Raw ore or Ingot | Gear | Cuts into gear shape |
| **Cable Maker** | 3x Wire (same type) | Cable | Combines wires (AL2 only) |
| **Refinery** | Uranium/Plutonium | Refined Uranium/Plutonium | Processes radioactive materials |

#### Crafting Machines

| Machine | Function | Inputs | Notes |
|---------|----------|--------|-------|
| **Crafter** | Combines items per blueprint | 2-3 items | Base crafter |
| **Crafter MK1** | Assembly Line 2 tier | 2 specific resources | AL2 exclusive |
| **Crafter MK2** | Higher tier crafting | More complex recipes | AL2 exclusive |

#### Transportation Machines

| Machine | Function | Special Properties |
|---------|----------|--------------------|
| **Roller** | Moves items in direction | $1 per item, unlocked by default |
| **Transporter Input** | Sends items to another line | Paired with Output via ID (0-99) |
| **Transporter Output** | Receives items from Input | 1:1 pairing required |
| **Advanced Transporter** | Cross-line transport | AL2: Only version that works between lines |
| **Robotic Arm** | Picks and places items | Can filter specific items |
| **Filtered Robotic Arm** | Selectively moves items | Choose which items to handle |

#### Output Machines

| Machine | Function | Notes |
|---------|----------|-------|
| **Seller** | Converts items to money | All chains must end here |

### 4.3 Machine Upgrade System

Most machines can be upgraded to improve performance:

| Upgrade Type | Effect | Typical Progression |
|--------------|--------|---------------------|
| Speed | Reduces processing time | 3s -> 2s -> 1s |
| Electricity | Reduces operating cost | $5 -> $3 -> $1 |
| Capacity | Increases output per activation | 1 -> 2 -> 3 items |

#### Starter Upgrade Example

| Upgrade | Base Value | Fully Upgraded |
|---------|------------|----------------|
| Spawn Time | 3 seconds | 1 second |
| Electricity Cost | $5 | $1 |
| Items Per Spawn | 1 | 3 |
| Maximum Starters | 8 | 56 |

---

## 5. Resources and Products

### 5.1 Basic Resources

Assembly Line features 5 basic resources plus 2 radioactive resources (AL2):

#### Standard Resources

| Resource | Starter Type | Base Sell Price | Notes |
|----------|--------------|-----------------|-------|
| **Copper** | Standard | Low | Most versatile |
| **Iron** | Standard | Low | Foundation material |
| **Gold** | Standard | Medium | Valuable base resource |
| **Aluminum** | Standard | Medium | Used in advanced products |
| **Diamond** | Standard | High | Premium resource |

#### Radioactive Resources (Assembly Line 2)

| Resource | Starter Type | Notes |
|----------|--------------|-------|
| **Uranium** | Radioactive Starter | Requires Refinery processing |
| **Plutonium** | Radioactive Starter | Requires Refinery processing |

### 5.2 Processed Materials

Raw resources can be processed into intermediate materials:

| Process | Input | Output | Machine | Sell Value Increase |
|---------|-------|--------|---------|---------------------|
| Smelting | Raw Ore | Ingot/Liquid | Furnace | Moderate |
| Pressing | Raw/Ingot | Plate | Hydraulic Press | High ($250/plate) |
| Drawing | Raw/Ingot | Wire | Wire Drawer | Moderate |
| Cutting | Raw/Ingot | Gear | Cutter | Moderate |

### 5.3 Material Forms

Each basic resource can exist in multiple forms:

| Form | Created By | Example |
|------|------------|---------|
| Raw | Starter | Raw Iron, Raw Copper |
| Ingot | Furnace | Iron Ingot, Copper Ingot |
| Liquid | Furnace | Liquid Gold, Liquid Aluminum |
| Plate | Hydraulic Press | Iron Plate, Copper Plate |
| Wire | Wire Drawer | Copper Wire, Gold Wire |
| Gear | Cutter | Iron Gear, Gold Gear |
| Cable | Cable Maker | Copper Cable, Gold Cable |

### 5.4 Crafted Products Hierarchy

Products increase in complexity and value:

#### Tier 1: Basic Products (Crafter)

| Product | Recipe | Sell Value | Notes |
|---------|--------|------------|-------|
| Circuit | 2 Copper Wire + 1 Raw Gold | Medium | Most frequently needed |
| Engine | 2 Iron Gear + 1 Gold Gear | Medium | Foundation for vehicles |
| Heater Plate | 1 Diamond + 1 Copper + 1 Copper Wire | Medium | Heating component |
| Cooler Plate | 1 Diamond + 1 Gold + 1 Gold Wire | Medium | Cooling component |

#### Tier 2: Intermediate Products

| Product | Recipe | Notes |
|---------|--------|-------|
| Toaster | 1 Heater Plate + 1 Aluminum + 1 Copper | Appliance |
| Grill | Uses Heater Plate | Appliance |
| Light Bulb | Basic components | Lighting |
| Clock | Time-keeping device | |
| Antenna | Communication component | |
| Battery | 1 Circuit + 1 Raw Aluminum + 1 Liquid Aluminum | Power storage |

#### Tier 3: Advanced Products

| Product | Recipe | Notes |
|---------|--------|-------|
| Advanced Engine | 1 Engine + 1 Circuit | Vehicle component |
| Generator | 4 Engines + 5 Gold Plates + 5 Copper Plates | Power generation |
| Electric Generator | Engine + Gold/Copper Plates + Circuit + Battery | Advanced power |
| Processor | Complex circuit-based | $1,320 sell, 8 starters optimal |
| Power Supply | Circuit + Copper/Iron Wire + Aluminum | Computer component |
| Tablet | Computing device | |

#### Tier 4: Complex Products

| Product | Recipe | Notes |
|---------|--------|-------|
| Computer | Power Supply + Processor + Aluminum + other components | High value |
| Speaker | Multi-component | $3,300 sell, 14 starters optimal |
| Drone | 2 Batteries + 2 Processors + 4 Aluminum Plates | **Most efficient product** |

#### Tier 5: End-Game Products

| Product | Recipe | Notes |
|---------|--------|-------|
| Server Rack | 10 Aluminum + 20 Aluminum Plates | Data center component |
| Super Computer | 50 Computers + 10 Server Racks | Highest value product |
| AI Processor | Super Computer + components | AI technology |
| AI Robot Body | Complex manufacturing | 1,462 starters required |
| AI Robot Head | AI Processor + 200 Aluminum | $1,163.22 per starter |
| AI Robot | AI Robot Body + AI Robot Head | 2,888 starters for 1/second |

---

## 6. Blueprints and Recipes

### 6.1 Blueprint System

Crafters require **blueprints** to produce items:

| Blueprint Property | Description |
|--------------------|-------------|
| Unlock Cost | Cash required to purchase |
| Crafter Requirement | Which crafter tier can use it |
| Input Requirements | Required items and quantities |
| Output | Product created |

#### Blueprint Tiers

| Tier | Crafter | Blueprint Examples |
|------|---------|-------------------|
| Basic | Crafter | Circuit, Engine |
| MK1 | Crafter MK1 | All require 2 items |
| MK2 | Crafter MK2 | More complex recipes |
| MK3 | Crafter MK3 | Advanced products |
| MK4 | Crafter MK4 | End-game products |

### 6.2 Key Recipe Details

#### Circuit (Most Important Early Product)

```
Input: 2 Copper Wire + 1 Raw Gold
Output: 1 Circuit
Crafter: Basic
Time: 3 seconds (base)
```

**Production Setup:**
- Copper starter outputs 3 pieces per second
- Gold starter outputs 2 pieces per second
- Wire drawer converts copper to wire
- Single crafter combines

#### Engine

```
Input: 2 Iron Gear + 1 Gold Gear
Output: 1 Engine
Crafter: Basic
```

**Recommended Setup:**
- 2 Iron starters feeding cutters
- 1 Gold starter feeding cutter
- All gears merge into crafter

#### Drone (Optimal Efficiency)

```
Input: 2 Batteries + 2 Processors + 4 Aluminum Plates
Output: 1 Drone
```

**Why Most Efficient:**
- Highest profit-per-starter ratio in game
- Compact 6x6 or 6x7 production layouts possible

### 6.3 MK1 Blueprint Pattern

All MK1 blueprints follow a consistent pattern:

| Rule | Description |
|------|-------------|
| Input Count | Always 2 items |
| Input Type | 1 processed material + 1 raw (except Server Rack) |
| Processed Forms | Liquid, Wire, or Gear |

---

## 7. Progression System

### 7.1 Unlock Progression

Assembly Line uses a **cascading unlock system** where machines and blueprints become available as players accumulate wealth:

| Unlock Stage | Typical Unlocks | Cash Threshold |
|--------------|-----------------|----------------|
| Starting | Starter, Seller, Roller | $0 |
| Early | Furnace, Wire Drawer | ~$1,000 |
| Early-Mid | Hydraulic Press, Cutter | ~$10,000 |
| Mid | Crafter, Basic Blueprints | ~$50,000 |
| Mid-Late | Advanced machines | ~$500,000 |
| Late | Transporters, Radioactive | ~$5,000,000+ |

### 7.2 Upgrade Shop

The Upgrade Shop allows permanent improvements:

| Upgrade Category | Effect | Progression |
|------------------|--------|-------------|
| Starter Speed | Faster resource generation | Multiple levels |
| Starter Capacity | More items per activation | 1 -> 2 -> 3 |
| Starter Limit | Maximum starters per line | 8 -> 16 -> ... -> 56 |
| Machine Efficiency | Reduced electricity costs | Per-machine upgrades |
| Processing Speed | Faster machine operation | Per-machine upgrades |

### 7.3 Technology Tree (Assembly Line 2)

Assembly Line 2 introduces **technology branches**:

| Branch | Focus | End Product |
|--------|-------|-------------|
| Standard | Traditional products | Super Computer |
| AI Robot | Artificial intelligence | AI Robot |
| AI Robot Bomber | Military AI | AI Robot Bomber (uses radioactive) |

### 7.4 Difficulty Curve

| Phase | Challenges | Player Goals |
|-------|-----------|--------------|
| Early | Limited space, few machines | Maximize basic output |
| Mid | Complex recipes, balancing ratios | Efficient production chains |
| Late | Multiple resource types, optimization | Profit per starter maximization |
| End-Game | Massive production requirements | AI Robots, Super Computers |

---

## 8. Level Design and Space Management

### 8.1 Grid System

#### Assembly Line 1

| Specification | Value |
|---------------|-------|
| Starting Area | 6x6 tiles |
| Maximum Area | 16x16 tiles (256 total) |
| Expansion Unit | Chunk purchase |

#### Assembly Line 2

| Specification | Value |
|---------------|-------|
| Grid Unit | 1x1 tiles |
| Chunk Size | 10x10 tiles |
| Starting Chunks | 1 |
| Expansion Direction | Adjacent only (not diagonal), right/up only |

### 8.2 Space Expansion Costs

Assembly Line uses **exponential scaling** for space expansion:

| Chunk Number | Cost | Multiplier |
|--------------|------|------------|
| 1st expansion | $100,000 | Base |
| 2nd expansion | $300,000 | 3x |
| 3rd expansion | $900,000 | 3x |
| 4th expansion | $2,700,000 | 3x |
| nth expansion | Previous x 3 | 3x |

### 8.3 Multiple Assembly Lines

Players can own multiple separate assembly lines:

| Line Feature | Details |
|--------------|---------|
| Separate Production | Each line runs independently |
| Shared Cash | All lines contribute to same wallet |
| Inter-Line Transport | Transporters can move items between lines |
| Expansion Costs | Very high (trillions for late expansions) |

**Line Expansion Costs (Assembly Line 2):**

| Expansion Type | Cost Range |
|----------------|------------|
| Cheapest line expansion | ~$6.5 trillion |
| Most expensive line expansion | ~$124 trillion |

### 8.4 Layout Design Patterns

#### Basic Production Chain

```
[Starter] -> [Roller] -> [Roller] -> [Seller]
```

#### Processed Material Chain

```
[Starter] -> [Furnace] -> [Roller] -> [Seller]
```

#### Crafting Chain (Circuit Example)

```
[Copper Starter] -> [Wire Drawer] ─┐
                                   ├-> [Crafter] -> [Seller]
[Gold Starter] ───────────────────┘
```

#### Advanced Multi-Product Layout

Players must balance:
- **Throughput matching**: Inputs must match crafter consumption rate
- **Space efficiency**: Minimize wasted tiles
- **Upgrade scalability**: Leave room for future improvements

---

## 9. Unique Mechanics

### 9.1 What Distinguishes Assembly Line from Other Factory Games

| Feature | Assembly Line | Factorio | Satisfactory |
|---------|---------------|----------|--------------|
| Map Size | Fixed small grid | Infinite procedural | Large 3D world |
| Combat | None | Aliens attack | Hostile wildlife |
| Exploration | None | Required for resources | Required |
| Resource Gathering | Starters auto-generate | Mining drills on patches | Miners on nodes |
| Idle Progression | Yes (offline earning) | No | No |
| Complexity Scale | Compact optimization | Massive automation | Large-scale building |

### 9.2 Transporter System

Transporters enable multi-line factories:

| Property | Value |
|----------|-------|
| ID Range | 0-99 |
| Pairing | 1:1 (one input to one output) |
| Cross-Line | Only Advanced Transporters (AL2) |
| Duplicate IDs | Not allowed |

**Performance Note:** Transporter performance between lines can be inconsistent due to polling-based calculation when lines aren't actively loaded.

### 9.3 Robotic Arms

Robotic arms provide flexible item manipulation:

| Arm Type | Function |
|----------|----------|
| Standard Robotic Arm | Picks and places items |
| Filtered Robotic Arm | Only handles selected item types |

**Known Bug:** Robotic arm can duplicate items when positioned over a seller while another arm is ready to pick up.

### 9.4 Profit Per Starter Optimization

The core meta-game is maximizing **profit efficiency**:

| Product | Starters Required | Sell Price | Profit/Starter |
|---------|-------------------|------------|----------------|
| Processor | 8 | $1,320 | $165 |
| Speaker | 14 | $3,300 | $235 |
| Drone | Variable | High | **Highest** |
| AI Robot Head | 1,428 | High | $1,163.22 |

### 9.5 Offline Earnings

Assembly Line generates income while players are away:

| Feature | Behavior |
|---------|----------|
| Production | Continues at calculated rate |
| Income Accumulation | Based on active production lines |
| Return Bonus | Option to watch ad to double offline earnings |
| Calculation | Estimates based on production efficiency |

---

## 10. Monetization Model

### 10.1 Core Monetization Philosophy

Assembly Line uses a **player-friendly free-to-play model**:

| Principle | Implementation |
|-----------|----------------|
| No Premium Currency | All progression uses in-game cash only |
| No Forced Ads | Zero interrupting advertisements |
| Optional Ad Rewards | Watch ads voluntarily for bonuses |
| No Pay-to-Win | Cannot purchase gameplay advantages |

### 10.2 Ad Integration

| Ad Type | Trigger | Reward |
|---------|---------|--------|
| Offline Earnings Double | Return from idle | 2x accumulated earnings |
| Voluntary Bonus | Player-initiated | Cash bonus |

**Player Sentiment:** Community praises the non-intrusive approach. Players note there is "one tiny screen icon that allows you to watch an ad for $$" but it's never forced.

### 10.3 Revenue Model Comparison

| Monetization Element | Assembly Line | Typical Idle Game |
|---------------------|---------------|-------------------|
| Premium Currency | No | Yes |
| Energy System | No | Often |
| Paywall Content | No | Common |
| Forced Ads | No | Frequent |
| IAP Packs | Minimal/None | Extensive |
| Subscription | No | Sometimes |

---

## 11. Assembly Line 2 Differences

### 11.1 New Features in AL2

| Feature | Description |
|---------|-------------|
| **Cable Maker** | New machine: 3 wires -> 1 cable |
| **Radioactive Resources** | Uranium and Plutonium |
| **Radioactive Starter** | Generates radioactive materials |
| **Refinery** | Processes radioactive materials |
| **Crafter Tiers** | MK1, MK2 separate crafters |
| **21 Machines** | Expanded from original 12-18 |
| **~50 Resources** | More crafting options |
| **Technology Branches** | Multiple progression paths |

### 11.2 Removed Features

Some features from AL1 are absent in AL2:

| Removed Feature | Impact |
|-----------------|--------|
| Inter-factory Transfer | Must use Advanced Transporters |
| Transport Arms | Different arm mechanics |
| Detailed Splitting Control | Changed resource splitting |
| Starter Production Rates | Different upgrade system |

### 11.3 Platform-Specific Changes

| Platform | Optimization |
|----------|--------------|
| PC (Steam) | Keyboard/mouse UI, redesigned editing tools |
| Mobile | Touch-optimized interface |

### 11.4 Visual Improvements

- Better textures
- Clearer resource flow visualization
- Improved graphics overall
- Enhanced UI clarity

---

## Appendix: Quick Reference Tables

### Machine Processing Summary

| Machine | Base Time | Upgraded Time | Base Cost | Upgraded Cost |
|---------|-----------|---------------|-----------|---------------|
| Starter | 3s | 1s | $5 | $1 |
| Crafter | 3s | 1s | $5 | $1 |
| Refinery | 5s | 1s | - | - |
| Cable Maker | 5s | 1s | - | - |
| Roller | Instant | - | $1/item | - |

### Profit Optimization Priorities

| Game Stage | Focus | Target |
|------------|-------|--------|
| Early | Raw materials | Maximize starters |
| Early-Mid | Processing | Plates ($250) |
| Mid | Basic crafting | Circuits |
| Late | Complex crafting | Drones |
| End-Game | AI Products | AI Robot Head |

### Space Costs Quick Reference

| Expansion | Cost |
|-----------|------|
| 1st Chunk | $100,000 |
| 2nd Chunk | $300,000 |
| 3rd Chunk | $900,000 |
| Formula | Previous x 3 |

### Product Complexity Tiers

| Tier | Starters for 1/sec | Examples |
|------|-------------------|----------|
| 1 | 1-10 | Basic processed materials |
| 2 | 10-50 | Circuits, Engines |
| 3 | 50-200 | Computers, Generators |
| 4 | 200-1000 | Super Computers |
| 5 | 1000+ | AI Robots |

---

## Sources

Research compiled from:
- [Assembly Line Wiki (Fandom)](https://assembly-line.fandom.com/wiki/Assembly_Line_Wiki)
- [AppGamer Assembly Line Guide](https://www.appgamer.com/assembly-line/strategy-guide/)
- [Google Play Store - Assembly Line](https://play.google.com/store/apps/details?id=com.olympus.assemblyline)
- [Apple App Store - Assembly Line](https://apps.apple.com/us/app/assembly-line/id1339770318)
- [Steam - Assembly Line 2 Mobile Version](https://store.steampowered.com/app/2691010/Assembly_Line_2_Mobile_Version/)
- [Incremental DB - Assembly Line 2](https://www.incrementaldb.com/game/assembly-line-2)

*Document compiled for game design research purposes. Data sourced from community wikis, official app store listings, and player strategy guides.*
