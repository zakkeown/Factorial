# Captain of Industry - Game Design Research Document

## Table of Contents

1. [Game Overview](#game-overview)
2. [Initial Conditions](#initial-conditions)
3. [Core Mechanics](#core-mechanics)
   - [Resource Types](#resource-types)
   - [Mining and Excavation](#mining-and-excavation)
   - [Terrain Modification](#terrain-modification)
   - [Power Generation](#power-generation)
   - [Conveyors and Logistics](#conveyors-and-logistics)
   - [Factories and Assembly](#factories-and-assembly)
   - [Fluid Handling](#fluid-handling)
4. [Population and City Building](#population-and-city-building)
   - [Housing Tiers](#housing-tiers)
   - [Food System](#food-system)
   - [Healthcare](#healthcare)
   - [Unity Points](#unity-points)
   - [Population Growth](#population-growth)
5. [Technology and Research Tree](#technology-and-research-tree)
6. [Production Chains](#production-chains)
   - [Smelting and Metal Processing](#smelting-and-metal-processing)
   - [Construction Materials](#construction-materials)
   - [Farming and Food Processing](#farming-and-food-processing)
   - [Fuel and Petrochemicals](#fuel-and-petrochemicals)
   - [Electronics and Advanced Materials](#electronics-and-advanced-materials)
7. [World and Map](#world-and-map)
   - [Island Maps](#island-maps)
   - [Resource Distribution](#resource-distribution)
   - [Ocean Mechanics](#ocean-mechanics)
8. [Ship Mechanics](#ship-mechanics)
   - [The Main Ship](#the-main-ship)
   - [Cargo Ships](#cargo-ships)
   - [Trading System](#trading-system)
   - [Expeditions](#expeditions)
9. [Vehicle Logistics](#vehicle-logistics)
   - [Transport Vehicles](#transport-vehicles)
   - [Mining Vehicles](#mining-vehicles)
   - [Specialized Vehicles](#specialized-vehicles)
   - [Fuel and Maintenance](#fuel-and-maintenance)
10. [Maintenance System](#maintenance-system)
11. [Waste Management and Pollution](#waste-management-and-pollution)
12. [Endgame Content](#endgame-content)

---

## Game Overview

**Captain of Industry** is a 3D factory/city builder hybrid developed by MaFi Games. Players lead a group of refugees who have survived a global catastrophe and arrived on a deserted island. The goal is to build a thriving industrial colony by mining resources, constructing factories, growing food, researching technologies, and eventually launching a space program.

The game combines elements of:
- Factory automation (conveyor systems, production chains)
- City building (housing, population needs, services)
- Resource management (finite island resources, imports)
- Exploration (world map, trading, ship upgrades)
- Terrain manipulation (mining, dumping, land reclamation)

---

## Initial Conditions

### Starting Scenario

Players begin with a shipwrecked group of refugees arriving on a deserted island. The narrative premise involves survivors of a global catastrophe seeking to rebuild civilization.

### Initial Resources

The starter settlement includes:
- **Starting Population**: 100 people
- **Initial Housing**: Shipping containers with capacity for 160 people
- **Starting Resources**: Limited stockpiles of basic materials

### Refugee Waves

- Each wave of refugees brings **16 people**
- Refugees also bring resources:
  - ~15 Iron Scraps
  - ~30 Rubber
  - ~45 Copper
  - ~45 Diesel
- Early game recommendation: No more than 3 refugee waves before expanding housing

### Tutorial and Early Game Setup

1. **Recycle the Abandoned Communication Station**: Provides initial scrap iron for smelting
2. **Repair the Shipyard**: Costs 100 construction materials
3. **Build Basic Infrastructure**:
   - Blast furnace and metal casters for iron processing
   - Concrete production
   - Basic distilling plant for diesel
   - Food production

### Early Game Priorities

- Self-sustaining concrete production
- Copper and iron smelters operational
- Basic diesel distillation
- Food supply established
- Maintenance depot functional

---

## Core Mechanics

### Resource Types

#### Mineable Resources (Non-Renewable on Island)

| Resource | Primary Use |
|----------|-------------|
| Iron Ore | Iron smelting, steel production |
| Copper Ore | Copper smelting, electronics |
| Coal | Power generation, smelting, steel |
| Limestone | Cement production, iron smelting |
| Sulfur | Chemical production, rubber |
| Quartz | Sand production, glass |
| Uranium | Nuclear power |
| Gold Ore | Advanced electronics |
| Bauxite | Aluminum production |
| Titanium Ore | Advanced materials |

#### Loose Materials

| Material | Source/Use |
|----------|------------|
| Rock/Stone | Mining byproduct, dumping |
| Dirt | Terrain modification |
| Slag | Smelting byproduct, concrete |
| Sand | Glass production (from quartz) |

#### Renewable/Infinite Resources

| Resource | Method |
|----------|--------|
| Water | Groundwater pumps, desalination |
| Seawater | Seawater pumps |
| Wood | Tree farms, harvesting |
| Crude Oil | Contracts, ocean deposits (some maps) |
| Steam | Boilers (with water + fuel) |

### Mining and Excavation

#### Mine Control Tower

- Central building that enables mining operations
- Defines control area for mining designations
- Vehicles must be assigned to the tower
- Setup requires:
  1. Place Mine Control Tower
  2. Create Mining Designations within control area
  3. Assign vehicles to the tower

#### Designation Types

| Type | Color | Function |
|------|-------|----------|
| Excavate | Orange | Remove material |
| Dump | Green | Add material |

- Both types can be set to: **Flatten**, **Ramp Up**, or **Ramp Down**
- Press **R** to toggle ramp direction
- Designation size: **4x4** (reduced from 6x6 for flexibility)
- Height number at center indicates target height level

#### Mining Behavior

- Excavators load multiple products at once
- Mining trucks can carry multiple products simultaneously
- Target height determines excavation depth (e.g., "+2" excavates everything above level 2)

### Terrain Modification

#### Terrain Stability

- Materials have "slope values" determining how they settle
- Slope = height/length ratio
- Unsupported terrain will collapse

#### Retaining Walls

- Protect terrain from falling
- Enable steeper terrain features
- Essential for deep mining operations

#### Land Reclamation

- Dump excess materials into ocean to create new land
- Required on some maps to access nearby islands (e.g., Insula Mortis)
- Dumped waste creates temporary landfill pollution

### Power Generation

#### Diesel Generators

| Building | Fuel | Workers | Output |
|----------|------|---------|--------|
| Diesel Generator | 3 Diesel/60s | 2 | 180 kW |
| Diesel Generator II | Higher consumption | - | ~5 MW (2 units = 10 MW) |

#### Coal/Steam Power

| Configuration | Coal | Water | Workers | Output |
|---------------|------|-------|---------|--------|
| Basic Coal Plant | 18/60s | 48/60s | 8 | 1.2 MW |
| Full System | 36/60s | 48/60s | 30 | 5 MW |
| With Upgraded Turbines | 30/60s | 48/60s | - | 12 MW per boiler |

#### Steam Turbine System

| Component | Input | Output |
|-----------|-------|--------|
| Coal Boiler | 18 Coal + 48 Water | 48 Steam High + 36 Exhaust |
| High-Pressure Turbine I | 24 Steam High | 1 MW Mechanical + 24 Steam Low |
| High-Pressure Turbine II | 48 Steam High | 2 MW Mechanical + 48 Steam Low |
| Low-Pressure Turbine I | 24 Steam Low | 1 MW Mechanical + Steam Depleted |
| Low-Pressure Turbine II | 48 Steam Low | 2 MW Mechanical |
| Power Generator I | 500 kW Mechanical | 300 kW Electrical (60% efficiency) |
| Power Generator II | 500 kW Mechanical | 416 kW Electrical (83% efficiency) |

**Note**: Maximum shaft capacity is 12 MW of mechanical power.

#### Nuclear Power

| Component | Details |
|-----------|---------|
| Uranium Mine Level 2 | Supports 1 power station (15 MW) |
| Nuclear Reactor | Core + blanket design |
| Fast Breeder Reactor | Requires ~40 Core Fuel to start |
| Typical Station Output | 12.7-15 MW |
| Large Designs | Up to 120 MW |

**Nuclear Fuel Cycle**:
1. Mine uranium ore
2. Process to yellowcake
3. Enrich uranium (multiple stages)
4. Create fuel rods (core fuel + blanket fuel)
5. Reprocess spent fuel
6. Blanket enriched fuel reduces uranium consumption to ~1.6/month per 4x reactor

#### Solar Power

| Configuration | Output Range |
|---------------|--------------|
| 100 Solar Panels Level 1 | 0.8 MW (rainy) - 2.4 MW (sunny) |
| 100 Solar Panels Level 2 | 1.6 MW (rainy) - 4.8 MW (sunny) |

### Conveyors and Logistics

#### Flat Conveyors

| Tier | Throughput | Power Consumption |
|------|------------|-------------------|
| Flat Conveyor I | Base rate | 1 kW + 0.20 kW/tile |
| Flat Conveyor II | 3.3x Tier I | 1 kW + 0.40 kW/tile (+40%) |
| Flat Conveyor III | Higher | 1 kW + 0.80 kW/tile |

#### Molten Channels

- Transport molten metals from blast furnaces to casters
- Directional flow
- No electricity consumption

#### Pipes

- Directional (must specify flow direction)
- Single fluid type per pipe section
- Can be used for storage (especially steam)
- No electricity consumption

#### Logistics Priority

- High-demand productions should use conveyors
- Trucks for small deliveries and short routes
- Set recipe priorities via drag-and-drop in building UI

### Factories and Assembly

#### Assembly Building Tiers

| Tier | Workers | Power | Notes |
|------|---------|-------|-------|
| Assembly I (Manual) | Base | None | Manual operation |
| Assembly II | More | Some | Electric, 2x speed |
| Assembly III | 8 | Higher | Higher throughput |
| Assembly IV (Robotic) | 2 | Highest | Reduced labor |

**Note**: Electric assemblies have twice the production speed of manual assemblies.

#### Recipe Management

- Toggle recipes on/off via "Set Recipes" button
- Drag recipes to set priority order
- Workers attempt highest priority recipe first
- Falls back to lower priority if ingredients unavailable or output full

### Fluid Handling

#### Water Sources

| Method | Notes |
|--------|-------|
| Groundwater Pump | Requires groundwater deposit, replenished by rain |
| Rainwater Harvester | Early game option |
| Seawater Pump | Max height 5 from ocean level |
| Thermal Desalinator | High fuel cost, produces brine byproduct |

**Groundwater Limits**: 2.5-3 pumps at full speed before draining aquifer. Each aquifer is independent.

#### Water Treatment

- Wastewater treatment reduces draw by 50%
- Filtered treatment reduces draw to 25%
- Required for higher housing tiers

#### Fluid Storage

- Dedicated fluid storage tanks
- Pipes can serve as storage (especially for steam)
- Some fluids cannot use standard storage

---

## Population and City Building

### Housing Tiers

| Tier | Capacity | Requirements |
|------|----------|--------------|
| Shipping Containers | 80 | Starting housing |
| Housing I | 80 | Basic construction |
| Housing II | 140 | Upgraded materials |
| Housing III | 240 | Water, Power, Household Goods |
| Housing IV | 400 | Full services |

**Housing III Requirements for 100% Unity Bonus**:
- Water supply (connected to water facility)
- Clean water input, wastewater removal
- Power connection
- Household goods supply

### Food System

#### Food Categories

1. **Carbohydrates**: Bread, Potatoes, Corn
2. **Proteins**: Meat, Eggs, Tofu, Sausages
3. **Fruits/Vegetables**: Vegetables, various crops
4. **Prepared Foods**: Food Packs, Cakes

#### Sustenance Values (People Fed Per Unit)

| Food | People Fed | Notes |
|------|------------|-------|
| Bread | 37 | Most efficient carbohydrate |
| Corn | 25 | Direct consumption |
| Potato | 17 | Less efficient but hardy |
| Grain | 56 | (24 bread = 16 flour equivalent) |
| Meat | 27 | 2 chicken carcasses = 1 meat |
| Eggs | 25 | From chicken farms |

#### Food Variety Bonus

- Multiple food types reduce individual consumption
- Increases Unity production
- Variable food bonus NOT boosted by Housing Tier
- Base food fulfillment provides 1.00 Unity (boosted by Housing Tier)

### Healthcare

#### Health Mechanics

| Health Level | Effect |
|--------------|--------|
| Positive (>0%) | Population grows 0.01% per 10 health points per month |
| Negative (<0%) | Colonists die at 0.10% per -1 health point per month |

#### Healthcare Services

- Not strictly essential (no penalty for lacking)
- Provides set Health amount per fulfilled request
- Higher tiers improve health bonuses

### Unity Points

#### Unity Generation

- Generated by Settlements based on satisfied needs
- Each satisfied resource typically provides 1-2 Unity/month
- Boosted by Housing Tier (except variable food bonus)

#### Unity Calculation

| Source | Unity | Multiplier |
|--------|-------|------------|
| Food Demand Satisfied | 1.00 | Housing Tier |
| Variable Food Types | Varies | None |
| Water Supply | Varies | Requires connection |
| Services | Varies | Per service fulfilled |

#### Unity Consumption

- Research (ongoing)
- Edicts (monthly costs)
- Cargo ship contracts (per ship, per month)
- Trade establishment fees

### Population Growth

#### Growth Methods

1. **Beacon**: Attracts refugee waves
2. **Natural Growth**: Via positive health and edicts
3. **World Map Refugees**: From exploration

#### Beacon Mechanics

- Time between waves starts at ~4 months
- Increases as population grows
- Stops functioning when gap exceeds ~24 months
- Can be paused to control population
- Refugees bring resources (Iron Scraps, Rubber, Copper, Diesel)

#### Growth Edicts

| Edict | Effect | Cost |
|-------|--------|------|
| Growth Boost I | +0.3% population growth | 1 Unity/month |
| Growth Boost II | +0.6% total (+I included) | 2 Unity/month |

---

## Technology and Research Tree

### Research System Overview

- **140+ technologies** available
- Research unlocks: buildings, vehicles, edicts, recipes, ship upgrades
- Research rate scales with population size
- Queue system for multiple technologies

### Research Lab Progression

| Building | Requirements | Unlocks |
|----------|--------------|---------|
| Research Lab (Basic) | None (starting building) | Basic technologies |
| Research Lab I | Lab Equipment supply | Early-mid game tech |
| Research Lab II | Lab Equipment II supply | Mid-game tech |
| Research Lab III | Higher requirements | Advanced tech |
| Research Lab IV | Final tier | End-game tech |

### Research Cost Scaling

| Game Phase | Cost Multiplier |
|------------|-----------------|
| Early Game | 1.0x |
| Mid Game | ~1.5x |
| End Game | ~2.0x |

### Key Technology Unlocks

#### Early Game
- Diesel Cracking
- Copper Smelting
- Automated Logistics
- Terrain Leveling
- Glass & Salt (before Research Lab II)

#### Mid Game
- Advanced Assembly
- Steel Production
- Farming Technologies
- Ship Upgrades

#### Late Game
- Aluminum and Titanium smelting
- Diamond production
- Electronics IV (Sapphire wafers + diamonds)
- Nuclear Power
- Space Program

#### Infinite Research

- 18 infinite research nodes
- Provide significant bonuses
- Allow industry overclocking

---

## Production Chains

### Smelting and Metal Processing

#### Iron Processing

```
Scrap Iron/Iron Ore + Coal
        |
        v
   Blast Furnace (24 Molten Iron/60s + Slag + Exhaust)
        |
        v
   Metal Caster (12 Molten Iron/60s = Iron Ingots)
```

**Ratio**: 1 Blast Furnace : 2 Metal Casters

#### Steel Production

```
3 Iron + 1 Coal
      |
      v
  Oxygen Furnace
      |
      v
   4 Steel
```

#### Copper Processing

```
Copper Ore
    |
    v
Blast Furnace (Molten Copper + Slag)
    |
    v
Metal Caster (Impure Copper)
    |
    v
Electrolysis (requires LOTS of power + water)
    |
    v
Pure Copper
```

### Construction Materials

#### Construction Parts Tiers

| Tier | Key Ingredients |
|------|-----------------|
| Construction Parts I | Wood, Concrete, Iron |
| Construction Parts II | CP I, Electronics, additional materials |
| Construction Parts III | CP II, Steel |
| Construction Parts IV | CP III, advanced materials |

#### Concrete Production

```
Limestone + Coal
      |
      v
  Rotary Kiln (9 Limestone + 1.5 Coal = 3 Cement)
      |
      v
  Concrete Mixer II (3 Cement = 24 Concrete Blocks)
```

**Alternative**: Bricks (temporary stopgap, less efficient than concrete + slag)

#### Advanced Concrete Slabs

- Output: 84 Concrete Slabs/min
- Side products: 48 CP I, 24 CP II, 12 CP III per minute

### Farming and Food Processing

#### Farm Mechanics

| Parameter | Details |
|-----------|---------|
| Fertility | Affects yield (Actual Yield = Recipe Yield x Fertility%) |
| Water | Fixed amount per crop |
| Monocrop Penalty | +50% fertility consumption for same crop twice |
| Growth Simulation | Daily |

#### Crop Yields (Greenhouse II, 100% Fertility)

| Crop | Yield | Water | Notes |
|------|-------|-------|-------|
| Wheat | 14.5 | 40 | Makes 21.75 bread |
| Potatoes | 29 | 45 | Direct consumption |
| Corn | Higher | - | Efficient for animal feed |
| Soybeans | 10 = 18 feed | - | More feed per unit than corn |

#### Fertility Consumption

| Crop | Fertility/Day |
|------|---------------|
| Soybeans | 0.5% |
| Potatoes | 0.35% |

#### Food Processing Recipes

| Input | Output | Notes |
|-------|--------|-------|
| Wheat | Flour | Milling |
| Flour | Bread | Baking |
| Chicken Carcass (2) | Meat (1) | Food Processor |
| Eggs | Cooked Eggs | Protein source |
| Soybeans | Tofu | Vegetarian protein |

#### Crop Rotation Strategy

- Potato + Vegetable rotation most efficient
- Avoids 50% fertility penalty
- With fertilizers, no green manure phase needed

#### Animal Farming

**Chicken Farm Requirements**:
- Water supply
- Animal Feed
- Initial chickens from village trade

**Corn vs Soybean for Feed**:
- Corn: 10 yields 11 feed (higher farm yield)
- Soybeans: 10 yields 18 feed (more feed per unit)
- Corn preferred due to higher overall farm productivity

### Fuel and Petrochemicals

#### Basic Distillation

```
Crude Oil
    |
    v
Basic Distiller
    |
    v
24 Diesel + Waste Water + Exhaust (per barrel)
```

#### Advanced Distillation

| Stage | Products | Advantages |
|-------|----------|------------|
| Stage I | Diesel + byproducts | More diesel, less energy |
| Stage II | Further refined | Better efficiency |
| Stage III | Full refining | Maximum output |

#### Byproduct Management

| Byproduct | Options |
|-----------|---------|
| Light Oil | Naphtha processing back to diesel, flare |
| Heavy Oil | Storage, flare, later processing |
| Naphtha | Rubber, plastic, fuel gas, diesel |

#### Rubber Production

```
(Diesel OR Ethanol OR Naphtha) + (Coal OR Sulfur)
                    |
                    v
               Rubber Maker
                    |
                    v
                  Rubber
```

**Uses**: Flat Conveyors, U-shape Conveyors, Electronics

#### Plastic Production

- Made from petrochemical processing
- Required for: Household Electronics, advanced items

### Electronics and Advanced Materials

#### Electronics Tiers

| Tier | Key Components |
|------|----------------|
| Electronics I | Copper, Rubber |
| Electronics II | Electronics I + additional materials |
| Electronics III | Electronics II + Microchips |
| Electronics IV | Diamonds + Lenses (Sapphire wafers) |

#### Microchip Production

```
Stage 1: Water + Acid
         |
         v
Stage 2: + Copper + Plastic
         |
         v
Stage 3: + Gold
         |
         v
      Microchips
```

**Throughput**:
- Microchip Machine I: Base rate
- Microchip Machine II: 36/min (3x Tier I)

#### Silicon Processing

- Silicon Reactor produces silicon
- Used for advanced electronics
- Quartz consumption increased in recent updates

---

## World and Map

### Island Maps

**Available at Game Start**: 8+ maps with varying:
- Difficulty levels
- Starting resources
- Land area
- Terrain types

#### Notable Maps

| Map | Features |
|-----|----------|
| Dragontail Isle | Ocean oil deposits |
| Curland | Ocean oil deposits |
| Insula Mortis | Island group, requires land bridges |

### Resource Distribution

#### On-Island Resources

- Most natural resources are **non-renewable**
- Each map has different resource abundances
- Strategic mining required to avoid depletion

#### Sustainable Resources

All resources can be sustained via:
- Outpost imports
- Trade contracts
- End-game asteroid drops

### Ocean Mechanics

#### Land Reclamation

- Dump materials into ocean to create buildable land
- Used for expansion and island connection
- Creates temporary landfill pollution

#### Ocean Resources

- Seawater (pumped for desalination, salt, glass production)
- Oil deposits (map-specific)

---

## Ship Mechanics

### The Main Ship

#### Purpose

- World Map exploration
- Transporting loot from explored locations
- Naval combat

#### Ship Upgrades

| Category | Effect |
|----------|--------|
| Weapons (Gun I, II, III) | Increase Battle Score (2 slots) |
| Armor | Increased survivability |
| Radar | Discovery range, Battle Score |
| Engine | Movement speed, fuel capacity |

**Battle Score**: Gained when at least one weapon is equipped

#### Research Requirements

- Ship technology unlocks after Research Lab II

### Cargo Ships

#### Acquisition Methods

1. **Repair Damaged Ships**: 7 shipwrecks on world map (increasing repair costs with distance)
2. **Purchase**: 600 Construction Parts III + 20 Solidarity at 4th settlement

#### Cargo Ship Function

- Automated resource transport from Map Locations to Home Island
- Requires: Cargo Depot (2 or wider), dedicated ship assignment
- Costs: Fixed Unity fee per month, per ship, per contract

### Trading System

#### Trade Types

| Type | Description |
|------|-------------|
| Quick Trade | One-time exchanges |
| Contracts | Long-term supply routes |

#### Contract Requirements

- Cargo Depot building
- Dedicated Cargo Ship
- Unity fees (monthly, per ship, per contract establishment)

#### Village Trading

- Some villages require Donation before trade
- Different villages offer different goods
- Positions randomized per world seed

### Expeditions

#### World Map Exploration

- Discover villages, resource deposits, damaged ships
- Find refugees (increases population)
- Locate Outposts for resource gathering

#### Outposts

- Gather resources for import
- Use The Ship or Cargo Ships
- Permanent resource supply once established

---

## Vehicle Logistics

### Transport Vehicles

| Vehicle | Tier | Capacity | Notes |
|---------|------|----------|-------|
| Pickup | 1 | 20 units | First transport vehicle |
| Truck | 2 | 60 units | Standard transport |
| Haul Truck (Dump) | 3 | 180 units | Loose products only |
| Haul Truck (Tank) | 3 | 180 units | Fluids only |

#### Hydrogen Variants

- Available through research
- Hydrogen trucks refuel 1.5% more often
- Diesel trucks need 15% more maintenance

### Mining Vehicles

#### Excavators

| Tier | Name |
|------|------|
| 1 | Small Excavator |
| 2 | Large Excavator |
| 3 | Mega Excavator |

#### Tree Harvesters

| Tier | Name |
|------|------|
| 1 | Tree Harvester |
| 2 | Large Tree Harvester |
| 3 | Tree Harvester II |

### Specialized Vehicles

- Assigned to specific control towers
- Mining trucks pair with excavators
- Tree harvesters convert trees to loadable wood

### Fuel and Maintenance

#### Vehicle Depot

- Central hub for vehicle management
- Fuel distribution
- Maintenance application

#### Fuel Consumption

- Significant jump from T2 to T3 vehicles
- Prepare fuel supplies before upgrading
- Idle vehicles consume less fuel

#### Edicts for Efficiency

| Edict | Effect |
|-------|--------|
| Overload Edict I | +15% load capacity |

---

## Maintenance System

### Importance

- All vehicles and many buildings consume maintenance
- Running out causes breakdowns
- Can create "death spiral" on higher difficulties

### Maintenance Depot

| Building | Notes |
|----------|-------|
| Maintenance Depot (Basic) | Converts products to maintenance, doubled throughput + 10 free maintenance |
| Upgraded Depots | Higher throughput, more coverage |

### Maintenance Requirements

| Component | Requirement |
|-----------|-------------|
| Maintenance I | Electronics + Mechanical Parts |
| Maintenance II | Higher tier components |
| Maintenance III | Electronics III + advanced parts |

### Maintenance Costs (Recent Reductions)

- Excavator and haul truck maintenance reduced
- Idle maintenance: 33% -> 20%
- Reduced workers and power in depots

### Strategy

- Set up input buffers for Electronics and Mechanical Parts
- Use Flat Belts to feed Maintenance Buildings
- Create alerts for low buffer warnings

---

## Waste Management and Pollution

### Waste Sources

- Settlement waste
- Industrial byproducts
- Without collection: recyclables and biomass become waste

### Disposal Methods

| Method | Pollution Type | Duration |
|--------|----------------|----------|
| Dump (Ocean Landfill) | Landfill Pollution | Limited (settles over years) |
| Burner (Solid) | Air Pollution | Transient |
| Incineration Plant | Capturable Exhaust | Can be scrubbed |

### Reducing Waste

- Build Recyclables Collection
- Build Biomass Collection
- These prevent materials from becoming waste

### Landfill Behavior

- Dumped waste needs years to "settle"
- No pollution after settling
- Disturbing settled waste restarts pollution

### Pollution Effects

- Air Pollution: Health impacts
- Water Pollution: Health impacts
- Can cause population decline if severe
- Exhaust scrubbing eliminates negative effects

---

## Endgame Content

### Space Program Overview

#### Purpose

- Give rockets meaningful purpose
- Motivate industrial scaling
- Extend gameplay beyond initial goals

### Space Station

#### Requirements

| Need | Description |
|------|-------------|
| Maintenance Parts | Regular deliveries |
| Crew | Must be rotated (radiation exposure limits) |
| Crew Supplies | Food, Water, Medicine |

### Rocket System

#### Rocket Launch Pad

- Enables launching rockets to space
- Rocket delivered and attached to tower
- Filled with fuel based on rocket type

#### Rocket Assembly Depot

- Constructs space rockets
- Delivers to nearest Launch Pad
- Uses specialized transporter

### Asteroid Mining (Update 3)

#### Process

1. Research and build space probes
2. Launch probes to search for asteroids
3. Select desired asteroid
4. Attach boosters
5. Controlled drop onto island

#### Benefits

- **"Refill" island mines** indefinitely
- Choose asteroid composition
- Sustainable late-game resource acquisition

### End-Game Products

| Product | Use |
|---------|-----|
| Space Probe Parts | Asteroid search |
| Station Parts | Space station construction |
| Crew Supplies | Station crew maintenance |
| Compact Reactor | Advanced power |

### Infinite Research

- 18 infinite research nodes
- Significant production bonuses
- Overclock industry capabilities

---

## Summary Statistics

### Key Numbers Reference

| Category | Value |
|----------|-------|
| Starting Population | 100 |
| Housing I Capacity | 80 |
| Housing IV Capacity | 400 |
| Refugee Wave Size | 16 people |
| Bread Sustenance | 37 people/unit |
| Truck Capacity | 60 units |
| Haul Truck Capacity | 180 units |
| Diesel Generator Output | 180 kW |
| Basic Coal Plant | 1.2 MW |
| Nuclear Station | 12-15 MW typical |
| Max Shaft Power | 12 MW |
| Technologies | 140+ |
| Island Maps | 8+ |
| Cargo Ships Available | 7 (repairable) + purchasable |

---

## Sources

- [Captain of Industry Official Wiki](https://wiki.coigame.com/)
- [Steam Community Guides](https://steamcommunity.com/app/1594320/guides/)
- [Official Website](https://www.captain-of-industry.com/)
- [Captain of Industry Calculator](https://captains-calculator.com/)
- [COI Hub Blueprints](https://hub.coigame.com/)
- [Steam Community Discussions](https://steamcommunity.com/app/1594320/discussions/)
