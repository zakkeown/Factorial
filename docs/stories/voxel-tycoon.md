# Voxel Tycoon: Comprehensive Game Design Research Document

## Table of Contents
1. [Overview](#1-overview)
2. [Initial Conditions](#2-initial-conditions)
3. [Core Mechanics](#3-core-mechanics)
4. [Resource System](#4-resource-system)
5. [Production Chains](#5-production-chains)
6. [Transportation Systems](#6-transportation-systems)
7. [Train System Details](#7-train-system-details)
8. [Research/Technology Tree](#8-researchtechnology-tree)
9. [Buildings Reference](#9-buildings-reference)
10. [World/Map Generation](#10-worldmap-generation)
11. [Economic Simulation](#11-economic-simulation)
12. [Unique Mechanics](#12-unique-mechanics)
13. [Modding Support](#13-modding-support)

---

## 1. Overview

Voxel Tycoon is a management simulation game set in an infinite, procedurally-generated voxel world. The game combines elements from transportation tycoons (like Transport Tycoon Deluxe) with factory-building games (like Factorio), creating a unique hybrid where players mine resources, build custom factories, establish supply chains, and grow cities.

### Key Distinguishing Features

| Feature | Description |
|---------|-------------|
| **Custom Factory Building** | Unlike traditional tycoons with prebuilt industries, players design and construct their own factories |
| **Infinite Voxel World** | Procedurally generated, endless world with unique biomes from deserts to arctic tundra |
| **Physics Simulation** | Vehicle behavior affected by weight, cargo, motive power, and terrain grades |
| **Full Terraforming** | Ability to reshape terrain down to individual voxels |
| **Free Camera Mode** | First-person perspective, ride-along on trains, immersive exploration |

### Development Status

- **Developer**: Voxel Tycoon Devs
- **Platform**: PC (Steam)
- **Status**: Early Access
- **Latest Version**: 0.89.x (as of late 2024)

---

## 2. Initial Conditions

### Starting Capital and Difficulty Settings

Voxel Tycoon offers configurable difficulty settings when creating a new game:

| Setting | Options | Notes |
|---------|---------|-------|
| **Starting Capital** | Configurable | Enough to link towns/mines in starting region by road OR build one short railway |
| **Maximum Loan** | Configurable | Adjustable loan ceiling |
| **Loan Interest** | Configurable | Rate charged on borrowed money |
| **Inflation** | On/Off | Prices increase over time if enabled |
| **Bankruptcy** | On/Off | Company fails after 3 months of negative balance |

### Starting Region

| Aspect | Details |
|--------|---------|
| **Unlocked Region** | One large region unlocked at game start |
| **Guaranteed Resources** | Iron ore, wood, stone, and coal (Tier 0/1 basics) |
| **Starting Cities** | At least one city with initial demands |
| **Rare Resources** | Copper, sand, and advanced materials require region expansion |

### Sandbox/Cheat Options

| Mode | Effect |
|------|--------|
| **Research All Done** | All technologies pre-unlocked |
| **Infinite Money** | No financial constraints |
| **Just Let Me Build** | Free construction (mod-enabled) |

### Initial Vehicles

Players begin with access to **Steam-era vehicles** only:
- Steam locomotives (available immediately)
- Basic trucks
- Diesel and Electric vehicles require research unlocks

---

## 3. Core Mechanics

### 3.1 Mining and Resource Extraction

#### Mining Facility Types

| Facility | Resource | Output Rate | Placement |
|----------|----------|-------------|-----------|
| **Coal Mine** | Coal | 0.25 items/s (1 per 4s) | On coal deposits |
| **Iron Mine** | Iron Ore | 0.25 items/s | On iron deposits |
| **Copper Mine** | Copper Ore | 0.25 items/s | On copper deposits |
| **Stone Quarry** | Stone | 0.25 items/s | On stone deposits |
| **Sand Quarry** | Sand | 0.25 items/s | On sand deposits |
| **Sawmill** | Wood | 0.25 items/s | On forest areas |

**Key Mining Mechanics**:
- All mines produce at a base rate of **0.25 items/second** (as of v0.85.1)
- Multiple mines can be placed on a single deposit
- Optimal mine placement maximizes extraction efficiency
- Resources are **finite** - deposits deplete when fully mined
- Mines must be placed **within** the deposit area

#### Resource Depletion

- Deposits contain finite amounts of resources
- Once depleted, mines stop producing
- Players must expand to new regions for fresh deposits
- Strategic planning required for long-term resource sustainability

### 3.2 Conveyor Belt System

#### Belt Specifications

| Specification | Value |
|---------------|-------|
| **Throughput** | 1.25 items/second |
| **Items per Minute** | 75 |
| **Lanes** | Single lane |

#### Conveyor Components

| Component | Function | Visual Indicator |
|-----------|----------|------------------|
| **Input Connector** | Receives items | Blue color |
| **Output Connector** | Sends items | Orange color |
| **Filter** | Sorts items | Green (selected) / Orange (rejected) |
| **Splitter** | Divides flow | Arrows show direction |
| **Merger** | Combines flows | Arrows converge |

#### Conveyor Mechanics

- Arrows on belts indicate flow direction
- Splitting requires no extra device - just branch the conveyor with opposite-facing arrows
- "Flip Conveyors" tool changes connector modes
- Conveyors can connect directly to warehouses, mines, and factories

### 3.3 Warehouse System

#### Warehouse Functions

| Function | Description |
|----------|-------------|
| **Storage** | Buffer for goods between production stages |
| **Automatic Logistics** | When near mines, automatically collects output |
| **Hub Integration** | Connects to stations for vehicle loading/unloading |
| **Factory Supply** | Input warehouse feeds factory, output stores products |

**Basic Factory Configuration**:
1. Input Warehouse (receives raw materials)
2. Conveyors connecting to processing devices
3. Processing machines (furnaces, saws, etc.)
4. Conveyors to output
5. Output Warehouse (stores finished products)

### 3.4 Transport Hubs

A **Transport Hub** is a group of adjacent buildings that can transfer goods without direct conveyor connections.

#### Hub Formation

- Build a truck station adjacent to a train station
- Warehouses can join hubs when placed nearby
- Hub modules share inventory access

#### Hub Workflow Example

```
Train Station -> Train Unloads -> Warehouse (buffer) -> Truck Station -> Truck Loads -> Delivery
```

**Benefits**:
- Trains unload directly to warehouses, reducing station dwell time
- Smoother transfers between rail and road networks
- Efficient multi-modal logistics

---

## 4. Resource System

### 4.1 Resource Tiers

| Tier | Resources | Availability |
|------|-----------|--------------|
| **Tier 0** | Coal, Iron Ore, Wood | Starting region (guaranteed) |
| **Tier 1** | Copper Ore, Sand, Stone | Starting region or nearby |
| **Tier 2+** | Advanced materials | Further regions required |

### 4.2 Complete Cargo List

The game features **29 types of cargo**:

#### Raw Materials (Tier 0-1)

| Resource | Tier | Source |
|----------|------|--------|
| Iron Ore | 0 | Iron Mine |
| Coal | 0 | Coal Mine |
| Wood | 0 | Sawmill |
| Stone | 1 | Stone Quarry |
| Sand | 1 | Sand Quarry |
| Copper Ore | 1 | Copper Mine |

#### Processed Materials

| Material | Tier | Processing Device |
|----------|------|-------------------|
| Gravel | 1 | Crusher |
| Wood Beam | 1 | Circular Saw |
| Wood Plank | 1 | Circular Saw |
| Iron Bar | 1 | Alloy Smelter |
| Copper Bar | 2 | Alloy Smelter |
| Steel Bar | 2 | Alloy Smelter |
| Glass Tube | 2 | Glass Furnace |
| Glass Pane | 2 | Glass Furnace |
| Stone Brick | 1 | Press |
| Concrete Beam | 2 | Concrete Mixer |

#### Advanced Products

| Product | Tier | Components |
|---------|------|------------|
| Iron Parts | 2 | Iron Bar |
| Iron Plate | 2 | Iron Bar |
| Copper Wire | 2 | Copper Bar |
| Steel Beam | 3 | Steel Bar |
| Reinforced Concrete Beam | 3 | Concrete Beam |
| Wood Frame | 2 | Wood Beam, Wood Plank |
| Advanced Wood Frame | 3 | Wood Frame |
| Vacuum Tube | 3 | Glass Tube, Copper Wire |
| Circuit | 3 | Copper Wire, Iron Parts |
| Furniture | 2 | Wood Plank |
| Advanced Furniture | 3 | Wood Frame |
| Radio | 4 | Vacuum Tube, Circuit |
| TV | 4 | Circuit, Glass Pane |

---

## 5. Production Chains

### 5.1 Processing Devices

| Device | Input | Output | Notes |
|--------|-------|--------|-------|
| **Alloy Smelter** | Ore + Coal | Metal Bars | Core smelting |
| **Crusher** | Stone | Gravel | Basic processing |
| **Circular Saw** | Wood | Beams/Planks | Wood processing |
| **Glass Furnace** | Sand + Coal | Glass products | Glass production |
| **Concrete Mixer** | Gravel + Water | Concrete | Construction materials |
| **Press** | Stone | Stone Brick | Brick production |
| **Carpentry** | Wood products | Furniture | Consumer goods |
| **Electronics Assembler** | Multiple | Electronics | Advanced products |

### 5.2 Key Production Ratios

#### Iron Bar Production

```
Input: Iron Ore + Coal
Ratio: 5 Iron Ore conveyors : 1 Coal conveyor
Smelters: 8 Alloy Smelters to saturate 1 Iron Bar belt
Optimal: 35 smelters with 5 iron inputs, 5 outputs per coal input
```

#### Steel Bar Production

```
Input: Iron Bar
Smelters: 19 Alloy Smelters to fully consume Iron Bar belt and saturate Steel Bar belt
```

#### Copper Production

```
Input: Copper Ore + Coal
Smelters: 5 Alloy Smelters to saturate 1 Copper Bar belt
```

### 5.3 Recipe Ratio Management

**General Rule**: If a recipe requires 5 units of Resource A and 2 units of Resource B, supply them at a **5:2 ratio** on conveyor lines.

**Factory Design Considerations**:
- Match conveyor input rates to recipe ratios
- Consider vehicle capacities when choosing processing location
- Some materials more efficient to process at source, others at destination

---

## 6. Transportation Systems

### 6.1 Vehicle Categories

| Category | Types | Unlocks |
|----------|-------|---------|
| **Rail** | Locomotives, Railcars | Steam available initially; Diesel/Electric via research |
| **Road** | Trucks, Buses | Progressive unlocks through eras |
| **Conveyor** | Belts, Connectors | Available from start |

### 6.2 Vehicle Eras

The game features **50+ vehicles** spanning multiple technological eras:

| Era | Characteristics | Examples |
|-----|-----------------|----------|
| **Steam** | Cheap to buy, expensive to maintain, least efficient | Steam locomotives |
| **Diesel** | Rugged, high pulling power, poor acceleration | Diesel locomotives, trucks |
| **Electric** | Most powerful, best acceleration, most expensive, requires infrastructure | Electric locomotives, multi-unit trains |

### 6.3 Locomotive Types

#### Steam Locomotives

| Aspect | Details |
|--------|---------|
| **Availability** | From game start |
| **Purchase Cost** | Lowest |
| **Running Costs** | Highest |
| **Efficiency** | Lowest |
| **Best For** | Early game, short routes |

#### Diesel Locomotives

| Aspect | Details |
|--------|---------|
| **Availability** | Research required |
| **Pulling Power** | Massive loads |
| **Acceleration** | Poor |
| **Best For** | Long-haul freight |

#### Electric Locomotives

| Aspect | Details |
|--------|---------|
| **Availability** | Research required |
| **Infrastructure** | Requires electrified rails (overhead lines) |
| **Power** | Most powerful in game |
| **Acceleration** | Exceptional |
| **Cost** | Most expensive |
| **Pathfinding** | Will not route through unelectrified segments |

### 6.4 Road Vehicles

#### Trucks

| Property | Description |
|----------|-------------|
| **Capacity** | MaxWeight determines cargo units based on item weight |
| **Speed** | VelocityLimit in km/h |
| **Variants** | Includes semi-trailers for increased capacity |

#### Buses

| Property | Description |
|----------|-------------|
| **Function** | Passenger transport between cities |
| **City Coverage** | Bus stops increase passenger transport coverage |
| **Growth Effect** | Active bus routes accelerate city growth |

### 6.5 Vehicle Physics

| Factor | Effect |
|--------|--------|
| **Weight** | Heavier vehicles accelerate slower |
| **Cargo Load** | Full vehicles behave differently than empty |
| **Grades** | Hills affect speed and fuel consumption |
| **Motive Power** | Different engine types have varying performance curves |

### 6.6 Vehicle Maintenance (Wear and Tear System)

| Aspect | Description |
|--------|-------------|
| **Wear Over Time** | Vehicles wear based on usage |
| **Increased Maintenance** | Older vehicles require more frequent service |
| **Service Intervals** | Configurable per vehicle |
| **Depot Tiers** | Tier II depots offer lower maintenance costs |
| **Full Wear** | Fully worn vehicles demand frequent services, higher costs, reduced performance |

**Recent Improvements** (v0.89):
- Vehicles attempt maintenance only from stations
- Option to skip depot stops if no maintenance needed

---

## 7. Train System Details

### 7.1 Rail Network Components

| Component | Function |
|-----------|----------|
| **Rail Track** | Basic infrastructure for train movement |
| **Electrified Rail** | Includes overhead lines for electric trains |
| **Train Depot** | Purchase, store, and maintain trains |
| **Freight Station** | Loading/unloading cargo; customizable length and platforms |
| **Passenger Station** | Embarking/disembarking passengers |
| **Signals** | Control train movement and block division |

### 7.2 Signal System

#### Block Signals

Signals divide the railroad into **blocks** - sections of connected rail between signals or track ends.

**Core Rule**: The game does not allow multiple trains on one block under normal operating conditions.

#### Signal Types

| Signal Type | Function | Placement |
|-------------|----------|-----------|
| **Normal Signal** | Checks if next block is clear; allows train to pass if clear | Exits of intersections |
| **Pre-Signal** | Checks multiple blocks ahead before allowing passage | Entrances to intersections, between intersections |

#### Pre-Signal Chain Mechanics

Pre-signals can be "chained" together:
- First pre-signal reads the second
- Second pre-signal reads the third
- Chain continues until a normal signal
- Allows trains to reserve paths through complex junctions

#### Signal Placement Rules

| Scenario | Signal Type | Position |
|----------|-------------|----------|
| **Intersection entrance** | Pre-signal | Before the junction |
| **Between intersections** | Pre-signal | Maintains chain |
| **Intersection exit** | Normal signal | After the junction |
| **Two-way track junctions** | Pre-signal going in, Normal signal coming out | At one-way/two-way transitions |

#### Common Signal Patterns

**One-Way Main Line**:
```
[Pre-Signal] -> [Junction] -> [Normal Signal]
```

**Double-Track Main Line**:
- Pre-signals on approaches
- Normal signals on departures
- Block spacing should fit longest train

### 7.3 Train Scheduling

#### Schedule Operations

| Operation | Description |
|-----------|-------------|
| **Full Load** | Wait until all cargo spaces filled |
| **Full Unload** | Wait until all cargo emptied |
| **Load Specific** | Load particular cargo types |
| **Unload Specific** | Unload particular cargo types |
| **Wait Time** | Wait for specified duration |

#### Advanced Scheduling Features

| Feature | Description |
|---------|-------------|
| **Track Selection** | Choose exact platform/track at stations |
| **Railcar-Specific Orders** | Designate which cars load/unload at each stop |
| **Train Duplication** | Copy fully-configured trains including orders |
| **Multi-Stop Routes** | Complex routes with multiple loading/unloading points |

### 7.4 Electrification System

| Component | Description |
|-----------|-------------|
| **Overhead Lines** | Catenary wires above electrified track |
| **Poles** | Support structures (side configurable) |
| **Power Grid** | Must connect to electrical source |
| **Power Source** | Required to energize the rail network |

**Electric Train Behavior**:
- Will not pathfind through unelectrified segments
- Can coast through short unelectrified gaps with momentum
- Most powerful and efficient for electrified networks

### 7.5 Bridges and Tunnels

#### Bridges

| Specification | Value |
|---------------|-------|
| **Minimum Height** | 3 voxels above crossing track/road |
| **Cost** | Expensive |
| **Construction** | Start at crossing point for proper support gaps |

#### Tunnels

| Specification | Value |
|---------------|-------|
| **Maximum Depth** | 3 voxels (default) |
| **Cursor Adjustment** | X (increase) / Z (decrease) height |
| **Cost** | Price per section + maintenance |

---

## 8. Research/Technology Tree

### 8.1 Laboratory System

| Building | Function |
|----------|----------|
| **Laboratory** | Conducts research when supplied with materials and money |

#### Research Requirements

| Requirement | Description |
|-------------|-------------|
| **Daily Money** | PricePerDay - ongoing financial cost |
| **Daily Items** | ItemsPerDay - materials delivered to labs |
| **Duration** | Number of days to complete |
| **Prerequisites** | Some research locked behind other technologies |

### 8.2 Technology Epochs

Technologies are organized into **technological epochs**, with:
- Vehicles and buildings split into "current" and "obsolete" categories
- Progression requires increasingly complex materials
- Higher tiers unlock more advanced production chains

### 8.3 Research Categories

| Category | Unlocks |
|----------|---------|
| **Vehicles** | New locomotives, trucks, buses |
| **Buildings** | New factories, stations, infrastructure |
| **Recipes** | New production chain options |
| **Eras** | Diesel and Electric vehicle access |

### 8.4 Research Progression

| Tier | Materials Required | Typical Unlocks |
|------|-------------------|-----------------|
| **Early** | Basic materials (coal, iron ore, wood) | Basic vehicles, simple factories |
| **Mid** | Processed materials (iron bars, wood planks) | Diesel vehicles, advanced factories |
| **Late** | Advanced products (circuits, electronics) | Electric vehicles, complex production |

### 8.5 Accessing Research

1. Click the **Flask icon** in the bottom right
2. Left panel shows completed (checkmark) and available (flask) research
3. Select research to view requirements
4. Supply laboratory with required materials
5. Fund the daily research cost

---

## 9. Buildings Reference

### 9.1 Mining Facilities

| Building | Function | Output |
|----------|----------|--------|
| Coal Mine | Extracts coal | 0.25/s |
| Iron Mine | Extracts iron ore | 0.25/s |
| Copper Mine | Extracts copper ore | 0.25/s |
| Stone Quarry | Extracts stone | 0.25/s |
| Sand Quarry | Extracts sand | 0.25/s |
| Sawmill | Harvests wood | 0.25/s |

### 9.2 Processing Factories

| Building | Input | Output |
|----------|-------|--------|
| Alloy Smelter | Ore + Coal | Metal Bars |
| Crusher | Stone | Gravel |
| Circular Saw | Wood | Beams, Planks |
| Glass Furnace | Sand + Coal | Glass products |
| Concrete Mixer | Gravel + Water | Concrete |
| Press | Stone | Stone Brick |
| Carpentry | Wood products | Furniture |
| Electronics Assembler | Various | Electronics |

### 9.3 Transportation Infrastructure

| Building | Function |
|----------|----------|
| **Freight Station** | Train cargo loading/unloading |
| **Passenger Station** | Passenger boarding |
| **Train Depot** | Train purchase and maintenance |
| **Garage** | Truck/bus purchase and maintenance |
| **Bus Stop** | Passenger pickup in cities |
| **Truck Station** | Road freight loading/unloading |

### 9.4 Logistics Buildings

| Building | Function |
|----------|----------|
| **Warehouse** | Storage and hub integration |
| **Conveyor** | Item transport |
| **Connector** | Conveyor input/output points |
| **Filter** | Item sorting |

### 9.5 Infrastructure

| Building | Function |
|----------|----------|
| **Signals** | Train traffic control |
| **Waypoints** | Route definition points |
| **Laboratory** | Research facility |

---

## 10. World/Map Generation

### 10.1 World Structure

| Aspect | Description |
|--------|-------------|
| **Size** | Infinite (procedurally generated) |
| **Division** | Split into purchasable regions |
| **Starting Region** | One large region unlocked free |
| **Expansion** | Buy adjacent regions to unlock |

### 10.2 Region System

| Aspect | Details |
|--------|---------|
| **Purchase Cost** | Largest regions cost 2+ million |
| **Contents** | Each region spawns cities and resources |
| **Resource Distribution** | More abundant resources further from start |
| **Rare Resources** | Found in distant regions |

### 10.3 Biomes

The world contains **unique biomes** including:

| Biome | Characteristics |
|-------|-----------------|
| Grassland | Standard terrain, abundant resources |
| Desert | Sandy terrain |
| Arctic/Tundra | Snow and ice |
| Forest | Dense tree coverage |

**Biome Settings**:
- Biomes can be excluded from generation (Hidden property)
- Distribution affects resource availability
- Terrain flatness configurable in settings

### 10.4 World Generation Settings

| Setting | Effect |
|---------|--------|
| **Region Size** | Configurable area per region |
| **Terrain Flatness** | How hilly/flat the world generates |
| **Seed** | Determines procedural generation output |

### 10.5 Terraforming

| Feature | Description |
|---------|-------------|
| **Voxel Manipulation** | Edit individual voxels |
| **Dig to Bedrock** | Full terrain excavation possible |
| **Leveling** | Flatten terrain for construction |
| **Cost** | Very expensive, especially near buildings |

---

## 11. Economic Simulation

### 11.1 City Demand System

#### Store Tiers and Limits

| Rule | Description |
|------|-------------|
| **Maximum per Tier** | 2 demands of each tier per city |
| **Tier Appearance** | Higher tier demands appear as city grows |
| **Demand Upgrades** | Satisfied demands "level up" to accept more cargo |
| **Population Requirement** | Each demand upgrade requires higher population |

#### Demand Mechanics

| Aspect | Details |
|--------|---------|
| **Satisfaction** | Provide required cargo quantities monthly |
| **Growth Bonus** | Satisfied demands accelerate city growth |
| **Increasing Requirements** | Growing towns need more goods |
| **Decay** | Unsatisfied demands lose levels, may close |

### 11.2 Supply and Demand Pricing

| Principle | Effect |
|-----------|--------|
| **Oversupply Penalty** | More goods to one location = less money per unit |
| **Storage Limits** | Demands have maximum storage; excess refused |
| **Diversification Benefit** | Spread deliveries across multiple demands/cities |

### 11.3 City Growth

#### Growth Factors

| Factor | Impact |
|--------|--------|
| **Demand Satisfaction** | Each satisfied demand adds small growth bonus |
| **Passenger Coverage** | Bus stops and stations accelerate growth |
| **City Type** | Industrial vs Tourist cities have different drivers |

#### City Types

| Type | Primary Growth Driver |
|------|----------------------|
| **Industrial** | Supplying stores and industries |
| **Tourist** | Passenger transport coverage and satisfaction |

### 11.4 Financial Management

| Aspect | Details |
|--------|---------|
| **Loans** | Available up to configurable maximum |
| **Interest** | Charged on outstanding loans |
| **Inflation** | Optional; prices increase over time |
| **Bankruptcy** | 3 consecutive months of negative balance (if enabled) |

### 11.5 Passenger Transportation Economics

| Aspect | Details |
|--------|---------|
| **Income Source** | Fares from passenger transport |
| **Multi-Modal** | Passengers can transfer (bus to train to bus) |
| **Coverage Calculation** | Based on stop/station placement |
| **Satisfaction** | Affects income and city growth |

---

## 12. Unique Mechanics

### 12.1 Distinguishing Features from Competitors

#### vs. Transport Tycoon / OpenTTD

| Feature | Voxel Tycoon | Transport Tycoon |
|---------|--------------|------------------|
| **Industries** | Player-built custom factories | Pre-spawned, fixed |
| **World** | 3D voxel, infinite | 2D tile-based, finite |
| **Terraforming** | Full voxel manipulation | Limited tile modification |
| **Production** | Conveyor-based factory design | Industry accepts/produces automatically |

#### vs. Factorio

| Feature | Voxel Tycoon | Factorio |
|---------|--------------|----------|
| **Focus** | Transportation + city supply | Factory automation + defense |
| **Cities** | Dynamic demand system | No cities |
| **Combat** | None | Core mechanic |
| **Vehicle Physics** | Realistic weight/grade simulation | Simplified |
| **Transportation** | Primary gameplay | Supporting mechanic |

### 12.2 Voxel World Interaction

| Feature | Description |
|---------|-------------|
| **Digging** | Excavate terrain to any depth |
| **Explosives** | Blow up terrain quickly |
| **Building Integration** | Structures integrate with voxel terrain |
| **Underground** | Full underground construction possible |

### 12.3 Camera and Perspective

| Mode | Description |
|------|-------------|
| **Standard View** | Traditional top-down tycoon perspective |
| **Orthographic/Isometric** | Locks to 45-degree angles, top-down (v0.88.8+) |
| **Free Camera** | First-person flight, immersive exploration |
| **Ride-Along** | Follow vehicles from their perspective |

### 12.4 Physics-Based Vehicles

| Property | Effect |
|----------|--------|
| **Weight** | Affects acceleration and braking |
| **Cargo** | Changes handling based on load |
| **Grades** | Hills impact speed realistically |
| **Power** | Engine type affects pulling capability |

---

## 13. Modding Support

### 13.1 Modding Capabilities

| Aspect | Moddable |
|--------|----------|
| **Config Files** | Yes - modify existing values |
| **New Vehicles** | Yes - locomotives, trucks, buses |
| **New Buildings** | Yes - factories, stations, devices |
| **New Recipes** | Yes - production chains |
| **Custom Logic** | Yes - C# API for game logic |
| **Biomes** | Yes - can exclude or add |

### 13.2 Steam Workshop Integration

| Feature | Description |
|---------|-------------|
| **Distribution** | Official Steam Workshop support |
| **Discovery** | Browse and subscribe to mods in-game |
| **Installation** | Automatic download and activation |
| **Collections** | Curated mod packs |

### 13.3 Modding Documentation

| Resource | Description |
|----------|-------------|
| **Official Docs** | docs.voxeltycoon.xyz |
| **GitHub Repository** | Sample mods and source code |
| **mod.json** | Mod configuration file format |
| **Publishing Guide** | Workshop upload instructions |

### 13.4 Mod Types

| Type | Examples |
|------|----------|
| **Content Mods** | New vehicles, buildings, production chains |
| **QoL Mods** | Interface improvements, automation helpers |
| **Sandbox Mods** | Free building modes, infinite resources |
| **Visual Mods** | New skins, decorations |

### 13.5 Creating Mods

#### Locomotive Mod Structure
- Define vehicle properties (speed, power, weight)
- Create visual assets
- Configure mod.json metadata
- Test in-game
- Publish to Workshop

#### Production Chain Mod Structure
- Define new item types
- Create recipe specifications
- Design processing buildings
- Balance input/output ratios
- Integrate with research tree (optional)

---

## Appendix A: Quick Reference Tables

### Conveyor Throughput

| Metric | Value |
|--------|-------|
| Items per Second | 1.25 |
| Items per Minute | 75 |

### Mine Output Rates

| All Mines | Output |
|-----------|--------|
| Base Rate | 0.25 items/s |
| Per Minute | 15 items |
| Per Hour | 900 items |

### Smelter Ratios

| Production | Smelters per Belt |
|------------|-------------------|
| Iron Bar | 8 Alloy Smelters |
| Copper Bar | 5 Alloy Smelters |
| Steel Bar | 19 Alloy Smelters |

### Vehicle Era Comparison

| Era | Buy Cost | Running Cost | Efficiency | Acceleration |
|-----|----------|--------------|------------|--------------|
| Steam | Low | High | Low | Medium |
| Diesel | Medium | Medium | Medium | Low |
| Electric | High | Low | High | High |

### City Demand Limits

| Rule | Value |
|------|-------|
| Same-tier demands per city | Maximum 2 |
| Demand level-up requirement | Population + satisfaction |

---

## Appendix B: Planned Features

According to developer communications, planned additions include:

| Feature | Status |
|---------|--------|
| **Multiplayer** | Planned |
| **Railway Shunting** | Planned |
| **Weather Conditions** | Planned |
| **Sea Transport** | Planned |
| **Air Transport** | Planned |
| **Rail Crossings** | In development (rails/roads rework) |
| **Vehicle Obsolescence** | In development |
| **Contracts System** | In development |

---

## Appendix C: Version History Highlights

### Version 0.89.x
- Cargo assignment improvements for vehicle replacement
- "Do not change" option for replacement recipes
- Skip depot stops if no maintenance needed
- Passenger/mail income affected by inflation (fix)

### Version 0.88.x
- Improved orthographic (isometric) mode
- Camera bookmarks system
- Enhanced HUD controls

### Version 0.85.x
- Passengers 2.0 update
- Multi-modal passenger transfers
- Enhanced passenger route planning

---

*Document compiled for game design research purposes. Data sourced from official Voxel Tycoon Wiki, Voxel Tycoon Guru, Steam Community guides, official developer devlogs, and modding documentation.*

**Primary Sources**:
- [Official Voxel Tycoon Wiki](https://voxeltycoon.fandom.com/wiki/Voxel_Tycoon_Wiki)
- [Voxel Tycoon Official Site](https://voxeltycoon.xyz/)
- [Voxel Tycoon Guru](https://voxeltycoon.guru/)
- [Steam Community Guides](https://steamcommunity.com/app/732050/guides/)
- [Voxel Tycoon Modding Documentation](https://docs.voxeltycoon.xyz/)
