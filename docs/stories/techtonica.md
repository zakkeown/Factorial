# Techtonica: Comprehensive Game Design Research Document

## Table of Contents
1. [Game Overview](#1-game-overview)
2. [Initial Conditions](#2-initial-conditions)
3. [Core Mechanics](#3-core-mechanics)
4. [Technology/Research Tree](#4-technologyresearch-tree)
5. [Production Chains](#5-production-chains)
6. [Power Generation](#6-power-generation)
7. [Logistics Systems](#7-logistics-systems)
8. [Underground World Design](#8-underground-world-design)
9. [Combat and Threats](#9-combat-and-threats)
10. [Multiplayer Features](#10-multiplayer-features)
11. [Story and Narrative](#11-story-and-narrative)
12. [Unique Mechanics](#12-unique-mechanics)

---

## 1. Game Overview

### Basic Information

| Attribute | Details |
|-----------|---------|
| **Developer** | Fire Hose Games |
| **Release Date** | November 7, 2024 (1.0 Full Release) |
| **Early Access Start** | July 18, 2023 |
| **Platforms** | Steam, Xbox Game Pass, PlayStation 5 |
| **Genre** | First-Person Factory Automation |
| **Player Count** | 1-4 players (solo or co-op) |
| **Setting** | Underground caves on alien planet Calyx |

### Core Premise

Techtonica is a first-person factory automation game set entirely underground on the mysterious alien planet Calyx. Players wake from cryogenic suspension deep beneath the surface and must build automated production lines while exploring bioluminescent caves, uncovering the mysteries of an ancient civilization, and working to escape the depths.

### Key Differentiators from Other Factory Games

- **First-Person Perspective**: Unlike top-down or isometric factory games, Techtonica places players directly in the world
- **Underground Setting**: Entirely subterranean with bioluminescent flora providing natural lighting
- **Destructible Terrain**: Players can reshape the environment using the M.O.L.E. black hole gun
- **Vertical World Structure**: 16 floors connected by a central elevator system
- **Fully Voice-Acted Narrative**: Complete story campaign with mystery elements
- **No Combat**: Environmental challenges replace hostile creatures

---

## 2. Initial Conditions

### Starting Scenario

The player character, known as the "Groundbreaker," awakens from cryogenic suspension deep underground on the planet Calyx. Short flashbacks during awakening reveal fragments of what happened, but without context. Upon emerging into the bioluminescent caverns, the player is contacted through their spacesuit's communication system by "Sparks," a member of the original expedition to Calyx.

### Starting Equipment

Players begin with minimal equipment that must be crafted or unlocked:

| Item | How Acquired | Purpose |
|------|--------------|---------|
| Basic Mining Tool | Starting equipment | Manual ore collection |
| Scanner | Craft early (1 Copper Frame + 1 Electrical Components) | Scan fragments to unlock tech |
| Inventory | Starting equipment | Basic item storage |

### First Objectives (Tutorial Flow)

The game guides new players through a structured tutorial near Production Terminal Lima:

1. **Scan Smelter Fragments**: Locate and scan 3 Smelter Fragments near Production Terminal Lima
2. **Open Tech Tree**: Learn the research system interface
3. **Craft Research Cores**: Craft and place 7 Research Core 380nm (Purple) to unlock technologies
4. **Set Up Mining**: Craft 2 Mining Drills and place them on ore deposits
5. **Fuel Equipment**: Use gathered plantlife (Kindlevine) to fuel drills and smelters
6. **Smelt Ingots**: Craft 4 Smelters, produce Iron and Copper Ingots
7. **Supply Production Terminal**: Deliver 20 Iron Ingots and 20 Copper Ingots to Terminal Lima

### Production Terminal Lima Requirements

| Tier | Materials Required |
|------|-------------------|
| Repair | 15 Iron Ore, 15 Copper Ore |
| Electrical Components | 30 Iron Ingots, 30 Copper Ingots, 45 Conveyor Belts, 4 Inserters, 2 Containers |

### Early Game Resource Location

Mining Drill fragments can be found right next to Production Terminal Lima at the start, allowing players to immediately begin automating resource collection.

---

## 3. Core Mechanics

### 3.1 Resource Types

#### Primary Ore Resources

| Resource | Uses | Mining Method | Notes |
|----------|------|---------------|-------|
| **Iron Ore** | Iron Ingots, most recipes | Mining Drill, Blast Drill | Foundation resource |
| **Copper Ore** | Copper Ingots, electronics | Mining Drill, Blast Drill | Essential for circuits |
| **Limestone** | Building materials, Biobricks | Mining, Kindlevine processing | Can be produced infinitely |
| **Atlantum Ore** | Advanced materials | Mining, requires processing | Unlocks Tier 6 tech |
| **Carbon Veins** | Carbon Powder | Mining (1.0+) | New in full release |
| **Scrap Ore Veins** | Various materials | Mining (1.0+) | New in full release |

#### Plant-Based Resources

| Resource | Source | Products |
|----------|--------|----------|
| **Kindlevine Seeds** | Initial pickup, Thresher output | Grown in Planters |
| **Kindlevine (Plant)** | Planter output | Threshed for extracts |
| **Kindlevine Extract** | Thresher | Fuel, Limestone production |
| **Plantmatter Fiber** | Thresher (from bound sticks) | Plantmatter Frames |
| **Shiverthorn** | Planter | Cooling systems |
| **Shiverthorn Extract** | Thresher | Coolant production |

#### Special Resources (1.0)

| Resource | Location | Extraction Method |
|----------|----------|-------------------|
| **Sesamite Sand** | Desert Biome caves | Sand Pump |
| **Sesamite Gel** | Processed from sand | Assembler |

#### Resource Depletion

Resources in Techtonica deplete very slowly. A normal-sized ore node can last for hours of real-life gameplay. Unlike some factory games, resource patches are not infinite but are extremely long-lasting.

### 3.2 Mining Systems

#### Mining Drill (MK1)

| Specification | Value |
|---------------|-------|
| Power Source | Fuel (Kindlevine, Biobricks) |
| Output Rate | Base rate, upgradeable |
| Placement | On ore deposits |

#### Mining Drill MKII

| Specification | Value |
|---------------|-------|
| Output Rate | 90 ore per minute (without core boost) |
| Efficiency | Faster than MK1, more fuel efficient |

#### Blast Drill

| Specification | Value |
|---------------|-------|
| Cycle Time | 12 seconds |
| Mining Area | 3x3 area, 5 voxels deep |
| Input | Mining Charges (Blast Charges) |
| Max Charges per Cycle | 20 |
| Base Output | 125 ore per minute from 9000-ore voxel |

**Blast Drill Output Scaling (by Mining Charge research level)**:

| MC Level | Chunks per Charges | Copper Output | Limestone Output |
|----------|-------------------|---------------|------------------|
| MC 1 | 1:1 | 25 chunks | 250 |
| MC 3 | 3:2 | - | 500 |
| MC 5 | 5:3 | - | 750 |
| MC 10 | 10:4 | - | 1,000 |
| MC 15 | 15:5 | 125 chunks | 1,250 |

#### Sand Pump (1.0)

| Specification | Value |
|---------------|-------|
| Function | Drains Sesamite Sand seas |
| Appearance | Towering oil rig-like machine |
| Fuel Consumption | High |
| Location | Desert Biome caves |

### 3.3 Smelting Systems

#### Basic Smelter

| Specification | Value |
|---------------|-------|
| Function | Smelts all ores into ingots |
| Power Source | Physical fuel required |
| Connection Points | 4 (for Inserters) |
| Recipes | Ore to Ingot, Kindlevine Extract to Limestone |

#### Blast Smelter

| Specification | Value |
|---------------|-------|
| Cycle Time | 12 seconds (5 cycles per minute) |
| Max Rate | <=50/min consumption/production |
| Output Multiplier | 2x at unlock (4 chunks -> 2 slabs) |
| Total Output | 10 slabs or ingots per minute |

#### Smelter MKII

| Specification | Notes |
|---------------|-------|
| Speed | Significantly faster than MK1 |
| Efficiency | More fuel efficient for basic recipes |
| Best For | Iron Ingot, Copper Ingot, Atlantum Ingot |

#### Key Smelting Recipes

| Input | Output | Notes |
|-------|--------|-------|
| Iron Ore | Iron Ingot | Basic smelting |
| Copper Ore | Copper Ingot | Basic smelting |
| Kindlevine Extract | Limestone | Infinite limestone source |
| Atlantum Mixture | Atlantum Ingot | Advanced material |

### 3.4 Assembler Systems

#### Assembler (MK1)

| Specification | Value |
|---------------|-------|
| Output Multiplier | 2x hand-crafting amount |
| Unlock | Tech Tree (PT LIMA - T4 - Electric Components) |
| Research Cost | 2x Research Core 380nm (Purple) |

#### Assembler MKII

| Specification | Value |
|---------------|-------|
| Output Multiplier | 4x hand-crafting amount |
| Efficiency | Double the MK1 output |

#### Key Assembler Recipes

| Recipe | Inputs | Output |
|--------|--------|--------|
| Biobricks | Plantmatter, Limestone | Building material |
| Biobricks (Mass Alt) | 1 Carbon Powder Brick, 2 Sesamite Gels | Alternative recipe (1.0) |
| Atlantum Mixture | Atlantum Powder, Shiverthorn Extract, Kindlevine Extract | Pre-ingot material |
| Shiverthorn Coolant | Iron Components, Shiverthorn Extract, Limestone | Cooling systems |

### 3.5 Processing Machines

#### Thresher

| Specification | Value |
|---------------|-------|
| Function | Processes plant materials |
| Unlock | Scan fragments in Warehouse |
| Variants | MKI, MKII (faster) |

**Thresher Processing Chain**:
1. Kindlevine -> Kindlevine Extract + Stems
2. Stems -> Plant Fiber
3. Atlantum Ore -> Atlantum Powder + Limestone (byproduct)

#### Planter

| Specification | Value |
|---------------|-------|
| Capacity | Up to 4 plants |
| Unlock | Scan fragments in Warehouse |
| Variants | MKI, MKII (faster) |
| Output | 1x plant item per planted seed |

#### Core Composer

| Specification | Value |
|---------------|-------|
| Function | Automates Research Core placement |
| Variants | 2k, 4k, 8k capacity |
| Purpose | Large-scale research automation |

---

## 4. Technology/Research Tree

### 4.1 Tech Tree Structure

The tech tree is organized into tiers, each unlocked at Production Terminals. Players progress from Terminal Lima to Terminal Victor and beyond.

#### Production Terminal Progression

| Terminal | Tier Range | Unlock Requirements |
|----------|------------|---------------------|
| **Lima** | T1-T4 | Starting terminal |
| **Victor** | T5+ | Complete Lima progression |

### 4.2 Research Core System

Research Cores are crafted items that function as research currency. They come in different colors corresponding to different wavelengths of light.

#### Research Core Types

| Core Type | Wavelength | Tier Usage | Clustering Ratio |
|-----------|------------|------------|------------------|
| **Purple** | 380nm | Basic Science through Shiverthorn Processing | 10:1 |
| **Blue** | 480nm | Advanced tiers | 5:1 |
| **Additional Colors** | Various | Late-game tiers | Variable |

#### Research Core Mechanics

- Cores must be **crafted and placed** in the world to count toward research
- Placed cores add to your total at the bottom of the Tech Tree
- **Removing a core reduces your spendable amount**
- Some techs require 10 cores, others require hundreds
- Core Composers automate placement for large-scale research

### 4.3 Scanning System

Many technologies cannot be unlocked with Research Cores alone until enough related **Fragments** have been scanned.

#### Fragment Mechanics

| Aspect | Details |
|--------|---------|
| **What are Fragments** | Ruined/broken versions of machines found in the world |
| **Scanner Tool** | Introduced early, used to scan items and fragments |
| **Unlock Requirement** | Scan enough fragments to unlock the building in Tech Tree |
| **Example** | Scan 3 Smelter Fragments to unlock Smelter research |

#### Scanning Locations

| Building | Fragment Location |
|----------|-------------------|
| Mining Drill | Near Production Terminal Lima |
| Smelter | Near Production Terminal Lima |
| Assembler | Caves housing Terminal Victor |
| Thresher | Inside Warehouse |
| Planter | Inside Warehouse |

### 4.4 Tech Tree Tiers

#### Early Tiers (Production Terminal Lima)

| Tier | Name | Material Requirements |
|------|------|----------------------|
| T1 | Basic Setup | Initial resources |
| T2 | Basic Logistics | 30 Iron Ingots, 30 Copper Ingots |
| T3 | Basic Logistics | Unlocks Conveyor Belts |
| T4 | Electric Components | 30 Iron Ingots, 30 Copper Ingots, 45 Conveyor Belts, 4 Inserters, 2 Containers |

#### Mid-Game Tiers

| Tier | Name | Material Requirements |
|------|------|----------------------|
| Shiverthorn Processing | Plant Processing | 500 Iron Ingot, 500 Copper Ingot |
| Cooling Systems | Thermal Management | 2,400 Iron Ingot, 2,400 Copper Ingot, 180 Plantmatter Frames |
| Atlantum Processing | Advanced Materials | 900 Iron Ingot, 900 Copper Ingot, 900 Processor Units, 450 Atlantum Ore |

#### Late-Game Tiers

| Tier | Material Requirements |
|------|----------------------|
| Tier 7 | 4,750 Iron Frames, 4,750 Copper Frames, 500 Processor Units, 3,400 Conveyor MKIIs, 1,400 Atlantum Ingots, 320 MJ accumulated charge |

### 4.5 Power Requirements for Research

Tech Tree upgrades from **Cooling Systems tier onwards** have a power requirement. Unlike Research Cores, the power used to unlock upgrades is **not permanently consumed** - it's a threshold that must be available, not spent.

---

## 5. Production Chains

### 5.1 Basic Metal Production

#### Iron Production Chain

```
Iron Ore (Mining) -> Iron Ore -> Smelter -> Iron Ingot
```

#### Copper Production Chain

```
Copper Ore (Mining) -> Copper Ore -> Smelter -> Copper Ingot
```

### 5.2 Kindlevine Production Loop (Infinite Resources)

The Kindlevine system enables infinite production of several key resources:

```
Kindlevine Seeds -> Planter -> Kindlevine (Plant)
                                    |
                                    v
                              Thresher
                                    |
                    +---------------+---------------+
                    |                               |
                    v                               v
            Kindlevine Extract              Kindlevine Stems
                    |                               |
                    v                               v
         Smelter -> Limestone              Thresher -> Plantmatter Fiber
```

#### Infinite Ore Loop (Virtuous Metal Ore Loop)

```
Copper Ore Powder -> Copper-Infused Limestone -> Copper Ore
     ^                                              |
     |                                              v
     +------------ (Small starting amount) ---------+
                           +
                   Kindlevine -> Limestone (ongoing supply)
```

This loop generates infinite ore from a small starting amount plus continuous Kindlevine production.

### 5.3 Plantmatter and Biobrick Production

#### 5-Stage Kindlevine Processing

1. **Planting**: Kindlevine Seeds -> Planter -> Kindlevine Plant
2. **First Threshing**: Kindlevine Plant -> Thresher -> Kindlevine + Bound Sticks
3. **Second Threshing**: Bound Sticks -> Thresher -> Plantmatter Fiber
4. **Frame Production**: Plantmatter Fiber -> Assembler -> Plantmatter Frames
5. **Biobrick Assembly**: Plantmatter + Limestone -> Assembler -> Biobricks

**Challenge**: Multiple outputs from many stages must be balanced to prevent backing up.

### 5.4 Atlantum Production Chain

```
Atlantum Ore -> Thresher -> Atlantum Powder + Limestone (byproduct)
                                |
                                v
Atlantum Powder + Shiverthorn Extract + Kindlevine Extract -> Assembler -> Atlantum Mixture
                                                                              |
                                                                              v
                                                            Atlantum Mixture -> Smelter -> Atlantum Ingot
```

### 5.5 Intermediate Components

| Component | Ingredients | Use |
|-----------|-------------|-----|
| **Copper Frame** | Copper Ingots | Scanner, various machines |
| **Iron Frame** | Iron Ingots | Structural components |
| **Electrical Components** | Copper, Iron | Electronics, machines |
| **Processor Unit** | Advanced circuits | High-tier machines |
| **Processor Array** | Multiple Processor Units | Relay Circuits |
| **Relay Circuits** | Processor Arrays | Advanced automation |

---

## 6. Power Generation

### 6.1 Power System Overview

Techtonica uses a tiered power system progressing from manual power to automated water wheels.

### 6.2 Crank Generator

The first electrical generator acquired in the game.

| Specification | Value |
|---------------|-------|
| Power Output | 150 kW |
| Power Source | Manual hand crank or external connection |
| Transmission | Through building materials capable of transferring electricity |

### 6.3 Water Wheel

Automates crank generators after research unlock.

| Specification | Value |
|---------------|-------|
| Function | Automatically cranks connected generators |
| Torque Output | 100 Nm (Newton Meters) |
| MK1 Crank Power | 300 kW per wheel |
| MK2 Crank Power | 400 kW per wheel |

#### Water Wheel Ratios

| Configuration | Ratio | Notes |
|---------------|-------|-------|
| Water Wheel : Crank Generator | 1:2 | Single wheel in line |
| MK1 Crank Torque Requirement | 40 Nm | Per crank generator |
| Parallel Configuration | No limit | Additional torque scales linearly |

**Limiting Factor**: Available water space for placing Water Wheels.

### 6.4 Accumulator (Power Storage)

| Specification | Value |
|---------------|-------|
| Function | Battery for holding electrical charge |
| Base Capacity | 10 MJ |
| Max Capacity (Upgraded) | 50 MJ |
| Charging Requirement | Connected to Power Floors with power source |

#### Accumulator Mechanics

- Charges when power network has **surplus power**
- Charging rate proportional to power excess
- Visual indicator: **Spins when charging**, lights indicate fill level
- Required for **Monorail Depot** operation (energy spike for train launch)
- Power transmitted in 2 groups: Floors 1-11, then 12-16

### 6.5 Power Floors

| Specification | Details |
|---------------|---------|
| Function | Transmit power between connected buildings |
| Coverage | Every elevator floor has power floors surrounding it |
| Transmission | Power flows between floors through the elevator system |

### 6.6 Power Network Design

**Basic Power Setup**:
1. Place Crank Generators
2. Connect Water Wheels to automate cranking
3. Build Power Floors to distribute electricity
4. Place Accumulators on Power Floors for storage
5. Connect machines requiring power

---

## 7. Logistics Systems

### 7.1 Conveyor Belt System

#### Belt Tiers and Speeds

| Belt Type | Items Per Minute | Unlock |
|-----------|------------------|--------|
| Conveyor Belt (MK1) | 240/min | Lima.T3.Basic Logistics |
| Conveyor Belt MKII | 360/min | Mid-game research |
| Conveyor Belt MKIII | ~707/min | Late-game research |

### 7.2 Inserter System

Inserters transfer items between machines, belts, and containers.

#### Inserter Types and Speeds

| Inserter Type | Color | Speed (items/min) | Special Feature |
|---------------|-------|-------------------|-----------------|
| Base Inserter | Brown | 20/min | Basic transfer |
| Long Inserter | Blue | 15/min | Extended reach |
| Fast Inserter | Red | 40/min | High speed |
| Filter Inserter | Purple | 30/min | Item type selection |
| Stack Inserter | Green | 180-720/min | Multi-item grab |
| Stack Filter Inserter | - | Up to 600/min | Stack + filtering |

#### Stack Inserter Mechanics

| Aspect | Details |
|--------|---------|
| Base Stack Size | 3 items |
| Max Stack Size (Upgraded) | 12 items |
| Speed Scaling | 50 grabs/min x stack size |
| Filter Capability | Select specific item types |

**Stack Filter Inserter**: Combines stack grabbing with filtering, moves 3/6/9/12 items at 50 grabs per minute, allowing up to 600 items/min for selected item types.

### 7.3 Container System

| Feature | Details |
|---------|---------|
| Function | Buffer storage for materials |
| Slot Blocking | Can block off storage slots for control |
| Automation | Use inserters to load/unload |
| Placement | Near crafting areas for material buffers |

### 7.4 Monorail System

High-speed long-distance transport for late-game logistics.

#### Monorail Specifications

| Specification | Value |
|---------------|-------|
| Transport Capacity | 240 items per train (fully upgraded) |
| Speed | Very fast (can be problematic) |
| Power Requirement | Accumulators for energy spike at launch |

#### Monorail Components

| Component | Function |
|-----------|----------|
| Monorail Track | Rails for train movement |
| Monorail Depot | Station for loading/unloading |
| Monorail Poles | Track support structures |

#### Building Requirements

- Both poles and track segments required in inventory
- Each depot needs Power Floor network with sufficient Accumulators
- Power requirement handles energy spike for train launch

#### Monorail Limitations

| Issue | Details |
|-------|---------|
| Speed Control | No way to limit train speed |
| Bulk Sending | Sends items in large batches |
| Min/Max Control | Cannot adjust carry amounts |
| Best Use Case | Single-item long-distance transport |

### 7.5 Freight Elevator

The central logistics hub connecting all 16 floors.

| Specification | Value |
|---------------|-------|
| Floors Connected | 16 |
| Inputs/Outputs | 30 connection points |
| Function | Main bus between floors |
| Unlock | Tutorial on Floor 1 |

#### Elevator Mechanics

- Manual interaction to insert/remove 1 stack per floor
- Insert Mining Bits to dig to next floor
- Can function as vertical main bus
- Power transmission between floor groups

### 7.6 Recommended Starting Logistics

For basic automation setup:
- **Inserters**: 20-30 minimum
- **Conveyor Belts**: 50-60 minimum

---

## 8. Underground World Design

### 8.1 World Structure

#### Vertical Organization

| Aspect | Details |
|--------|---------|
| Total Floors | 16 |
| Floor Dimensions | 330 x 330 x 251 voxels each |
| Orientation | More vertical than horizontal |
| Connection | Central elevator system |

### 8.2 Biomes

#### River Biome

| Feature | Details |
|---------|---------|
| Characteristics | Water features, lush vegetation |
| Cave Count | Significant number across all levels |
| Map Variant | "The Faithless Void" (revamped version) |

#### Desert Biome

| Feature | Details |
|---------|---------|
| Characteristics | Sandy terrain, unique resources |
| Special Feature | Sesamite Sand Sea (drainable) |
| Resources | Sesamite (valuable extraction material) |

### 8.3 Map Variants (1.0)

| Map | Description |
|-----|-------------|
| **The Faithless Void** | Revamped River Biome map |
| **Mountain King Underhill** | New handcrafted map with massive caverns, spread-out resources |

### 8.4 Exploration Features

#### Bioluminescent Environment

- Flora provides natural underground lighting
- First-person perspective emphasizes immersion
- Visual discovery of hidden caves and deposits

#### Discoverable Content

| Content Type | Reward |
|--------------|--------|
| Ore Deposits | Resources for production |
| Research Facilities | Technology fragments |
| Hidden Caves | New technologies |
| Aging Artifacts | Lore and story elements |
| Fragments | Tech tree unlocks |

### 8.5 Terrain Manipulation

#### The M.O.L.E.

**M.O.L.E.** = Material, Obliteration, Leveling, and Excavation

| Specification | Value |
|---------------|-------|
| Function | Shapes terrain using focused black hole energy |
| Default Mode | Clears 5x5x5 section of voxels |
| Unlock | Progression through tech tree |

#### Terrain Features

| Feature | Details |
|---------|---------|
| Destructible | All terrain can be modified |
| Voxel-Based | World built on voxel grid |
| Tunneling | Create custom pathways |
| Flattening | New upgrade for terrain leveling |

---

## 9. Combat and Threats

### 9.1 Design Philosophy

**Techtonica has no combat system.** This is an intentional design decision by Fire Hose Games.

> "We won't ever have combat. Eventually, we will challenge players with the environment and systems in unique ways, but we won't be making you fight murdery stuff."

### 9.2 Environmental Challenges

Instead of enemies, Techtonica creates challenge through:

| Challenge Type | Description |
|----------------|-------------|
| Resource Management | Balancing production chains |
| Logistics Complexity | Multi-stage processing |
| Exploration Difficulty | Finding resources and fragments |
| Power Management | Maintaining adequate power supply |
| Vertical Logistics | Moving materials between floors |

### 9.3 Design Rationale

The developers believe combat would "clash with the experience they're giving players." The focus remains on:
- Factory building creativity
- Player expression through automation
- Exploration and discovery
- Cooperative building
- World building and atmosphere

---

## 10. Multiplayer Features

### 10.1 Co-op Specifications

| Specification | Value |
|---------------|-------|
| Player Count | 1-4 players |
| Mode | Full co-op throughout entire game |
| Solo Viability | Entirely playable alone |

### 10.2 Cooperative Gameplay

| Feature | Details |
|---------|---------|
| Shared Factory | Players work on same production systems |
| Resource Gathering | Collaborative mining and processing |
| Technology Research | Shared tech tree progression |
| Exploration | Split up or explore together |
| Building | Construct factories together |

### 10.3 Network Requirements

| Requirement | Details |
|-------------|---------|
| Internet | Required for multiplayer |
| Cross-play | Not currently available |
| Future Cross-play | Possible future addition |

### 10.4 Platform Availability

| Platform | Multiplayer Support |
|----------|---------------------|
| Steam | Yes |
| Xbox Game Pass | Yes |
| PlayStation 5 | Yes |

---

## 11. Story and Narrative

### 11.1 Narrative Structure

Techtonica features the **only factory automation game with a fully voice-acted narrative campaign**.

| Aspect | Details |
|--------|---------|
| Voice Acting | Complete voice acting throughout |
| Story Length | Full campaign with complete ending |
| Integration | Narrative drives progression forward |

### 11.2 Story Setup

#### Opening

The player character (the Groundbreaker) awakens from cryogenic suspension deep underground on planet Calyx. During awakening, short flashbacks reveal fragments of past events without context.

#### Key Characters

| Character | Role |
|-----------|------|
| **Sparks** | Expedition member who contacts player via spacesuit communication |
| **Mysterious Talking Cube** | Enlists player's help to uncover the truth |

### 11.3 Central Mystery

| Element | Details |
|---------|---------|
| Setting | Alien planet Calyx |
| Core Question | Something is wrong beneath the surface |
| Player Goal | Discover truth about what happened |
| Progression | Dig deeper, uncover secrets |

### 11.4 Story Integration with Gameplay

The narrative provides motivation for factory building:
- **Repair Production Terminals**: Story objectives
- **Unlock Technology**: Progress the mystery
- **Explore New Areas**: Discover story elements
- **Scan Artifacts**: Learn about the world's history

---

## 12. Unique Mechanics

### 12.1 First-Person Perspective

Unlike most factory games (top-down or isometric), Techtonica places the player directly in the world.

| Aspect | Impact |
|--------|--------|
| Immersion | Direct connection to environment |
| Scale Perception | Machines feel appropriately large |
| Exploration | More engaging cave discovery |
| Building | Place machines at eye level |

### 12.2 Underground Setting

The entire game takes place beneath the surface of planet Calyx.

| Feature | Design Impact |
|---------|---------------|
| Bioluminescent Flora | Natural lighting without sun |
| Vertical Structure | 16 floors of progression |
| Cave Systems | Non-linear exploration |
| Enclosed Spaces | Intimate factory building |

### 12.3 Destructible Voxel Terrain

#### M.O.L.E. System

| Feature | Details |
|---------|---------|
| Technology | Black hole energy focused to single point |
| Default Clearing | 5x5x5 voxel cube |
| Purpose | Reshape terrain for factory layout |
| Upgrade | Flatten terrain mode |

#### Terrain Manipulation Uses

- Create flat building surfaces
- Tunnel to new ore deposits
- Connect cave systems
- Customize factory layouts
- Clear obstacles

### 12.4 Elevator-Centric Progression

The freight elevator serves as both literal and metaphorical center of the game.

| Function | Details |
|----------|---------|
| Floor Access | Mining Bits unlock new floors |
| Material Transport | 30 input/output connections |
| Main Bus | Can serve as vertical logistics hub |
| Story Progression | Deeper floors reveal more story |
| Power Distribution | Connects power between floor groups |

### 12.5 Scanner-Based Unlocks

Technology unlocking requires active exploration and scanning.

| Mechanic | Traditional Factory Games | Techtonica |
|----------|---------------------------|------------|
| Tech Unlock | Spend resources | Scan fragments + spend cores |
| Discovery | Automatic availability | Must find and scan first |
| Exploration Incentive | Optional | Required for progression |

### 12.6 Research Core Placement

Unlike research packs that are consumed, Research Cores are physical objects placed in the world.

| Aspect | Details |
|--------|---------|
| Placement | Cores must be placed to count |
| Removal | Removing cores reduces research power |
| Automation | Core Composers automate placement |
| Visual Presence | Cores exist as physical objects |

### 12.7 Kindlevine Infinite Resource Loop

The ability to create infinite resources from renewable plants is a distinctive feature.

| Resource | Infinite Source |
|----------|-----------------|
| Limestone | Kindlevine Extract -> Smelter |
| Iron Ore | Virtuous Metal Ore Loop |
| Copper Ore | Virtuous Metal Ore Loop |
| Fuel | Kindlevine processing |
| Plantmatter | Kindlevine processing |

### 12.8 No Combat Design

The deliberate exclusion of combat creates a unique factory game experience focused entirely on creation rather than defense.

| Comparison | Games with Combat | Techtonica |
|------------|-------------------|------------|
| Challenge Source | Enemies, waves | Environment, logistics |
| Resource Drain | Ammo, repairs | Only production |
| Player Stress | Attack pressure | Self-imposed goals |
| Multiplayer Dynamic | Defense coordination | Pure creation |

---

## Appendix: Quick Reference Tables

### Belt Throughput Summary

| Belt Tier | Items/Minute | Research Tier |
|-----------|--------------|---------------|
| MK1 | 240 | Basic Logistics |
| MKII | 360 | Mid-game |
| MKIII | ~707 | Late-game |

### Inserter Speed Summary

| Inserter | Items/Minute | Notes |
|----------|--------------|-------|
| Base | 20 | Starting equipment |
| Long | 15 | Extended reach |
| Fast | 40 | Speed upgrade |
| Filter | 30 | Item selection |
| Stack | 180-720 | Multi-item, upgradeable |
| Stack Filter | Up to 600 | Stack + filtering |

### Power Generation Summary

| Method | Output | Notes |
|--------|--------|-------|
| Hand Crank | Variable | Manual labor |
| Crank Generator | 150 kW | First automated |
| Water Wheel (MK1) | 300 kW/wheel | Automated cranking |
| Water Wheel (MK2) | 400 kW/wheel | Upgraded |

### Production Terminal Requirements Summary

| Terminal/Tier | Key Requirements |
|---------------|------------------|
| Lima Repair | 15 Iron Ore, 15 Copper Ore |
| Lima T4 | 30 Iron Ingot, 30 Copper Ingot, 45 Belts, 4 Inserters |
| Victor Repair | 500 Iron Ingot, 500 Copper Ingot |
| Shiverthorn | 500 Iron Ingot, 500 Copper Ingot |
| Cooling Systems | 2,400 Iron, 2,400 Copper, 180 Plantmatter Frames |
| Atlantum | 900 Iron, 900 Copper, 900 Processor Units, 450 Atlantum Ore |

### Research Core Summary

| Core Color | Wavelength | Primary Use |
|------------|------------|-------------|
| Purple | 380nm | Basic through Shiverthorn |
| Blue | 480nm | Advanced tiers |

---

## Sources and References

This document was compiled from the following sources:

- [Official Techtonica Website](https://techtonicagame.com/)
- [Techtonica on Steam](https://store.steampowered.com/app/1457320/Techtonica/)
- [Techtonica Wiki (Fandom)](https://techtonica.fandom.com/wiki/Home)
- [TechRaptor Guides](https://techraptor.net/gaming/guides/techtonica-tech-tree-guide)
- [Steam Community Guides and Discussions](https://steamcommunity.com/app/1457320)
- [Fire Hose Games Official Site](https://www.firehosegames.com/)
- [Techtonica Calculator](https://www.techtonica-calculator.com/)

---

*Document compiled for game design research purposes. Data sourced from official sources, community wikis, and player documentation as of Techtonica version 1.0 (November 2024 full release).*
