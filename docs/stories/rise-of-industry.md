# Rise of Industry: Comprehensive Game Design Research Document

## Table of Contents
1. [Game Overview](#1-game-overview)
2. [Initial Conditions](#2-initial-conditions)
3. [Core Mechanics](#3-core-mechanics)
4. [Research/Tech Tree](#4-researchtech-tree)
5. [Building Types](#5-building-types)
6. [Transportation Systems](#6-transportation-systems)
7. [Town/City Demand System](#7-towncity-demand-system)
8. [Economic Simulation](#8-economic-simulation)
9. [Map Generation and Regions](#9-map-generation-and-regions)
10. [Game Modes](#10-game-modes)
11. [Pollution and Environment](#11-pollution-and-environment)
12. [DLC Content](#12-dlc-content)

---

## 1. Game Overview

### Basic Information

| Attribute | Value |
|-----------|-------|
| **Developer** | Dapper Penguin Studios (Spanish indie studio) |
| **Publisher** | Kasedo Games |
| **Release Date** | May 2, 2019 |
| **Platforms** | PC (Steam, GOG) |
| **Genre** | Strategic Tycoon / Supply Chain Management |
| **Setting** | 1930s procedurally generated world |

### Premise

Build and manage an industrial empire in a living, procedurally generated world that evolves based on your playstyle. The game emphasizes supply chain management, transportation logistics, and strategic expansion with over 150 products and numerous factory types.

### Key Features

- Over 150 products across multiple industry categories
- Procedurally generated maps with resource deposits
- Dynamic town growth and demand system
- Multiple transportation options (trucks, trains, boats, zeppelins)
- Competitor AI and contract system
- Pollution and environmental mechanics

---

## 2. Initial Conditions

### Starting Capital by Difficulty

| Difficulty | Starting Capital | Upkeep Modifier | Transport Cost | Pollution | Product Value |
|------------|-----------------|-----------------|----------------|-----------|---------------|
| Newcomer | Standard | 50% | 50% | 50% | Increased |
| Veteran | Standard | 100% | 100% | 100% | Standard |
| Startup | $5,000,000 | 200% | 200% | 200% | Standard |
| Custom | Variable | 0-300% | 0-300% | 0-300% | Variable |

### First Buildings

1. **Headquarters (HQ)**: Center of your industrial empire
   - First permit is FREE when placing HQ
   - Handles Tech Tree access and loan management
   - Upgrades visually based on research progress (10 unlocks triggers Tier 1 model change)

2. **Three Free Research Unlocks**: Players receive 3 free tech tree unlocks at game start

### Recommended Starting Strategy

- Place HQ centrally between multiple towns to minimize transport costs
- Avoid mountainous/hilly regions (terraforming is expensive)
- Start with basic extraction: water siphons, orchards, or crop farms
- "Farm spam" strategy: Check Farmer's Market prices, build farms for highest-priced produce

### Map State at Start

- Procedurally generated terrain with resource deposits
- Multiple settlements (towns/cities) already present
- Resource deposits distributed across regions
- Settlements have existing shops with initial demand

---

## 3. Core Mechanics

### 3.1 Supply Chain System

The core gameplay loop:

```
Extraction → Processing → Manufacturing → Distribution → Sales
(Gatherers)   (Factories)  (Factories)    (Transport)    (Town Shops)
```

### 3.2 Production Flow

```
Raw Resources → Components → End Products → Luxury Items
(Gatherers)     (Factories)   (Factories)    (Mega-Factories)
```

### 3.3 Resource Categories

| Category | Description | Examples |
|----------|-------------|----------|
| Raw Resources | Harvested directly from map/farms | Coal, Iron, Oil, Gas, Sand, Wood, Water, Fish, Crops |
| Components | Intermediate products | Steel, Glass, Plastic, Fabric, Leather, Paper |
| End Products | Sellable finished goods | Bread, Beer, Furniture, Clothes, Books |
| Luxury Items | Complex high-value products | Automobiles, Computers, Pre-packed Meals |

### 3.4 Logistics and Distribution

Products move through the supply chain via:

| Method | Description |
|--------|-------------|
| Local Delivery | Automatic trucks within building range |
| Destination Trucks | Single-unit carriers that return to origin |
| Trade Routes | Long-distance transport between depots |

**Key Rule**: Build tightly-knit blocks of buildings to minimize transport costs and travel time.

---

## 4. Research/Tech Tree

### 4.1 Structure

| Property | Details |
|----------|---------|
| Tiers | Multiple tiers of progression (Tier 1, 2, 3+) |
| Initial Unlocks | 3 free unlocks across first two tiers |
| Cost | Money and time |
| Building Access | All buildings visible, recipes must be researched |

### 4.2 Specializations

Players choose one specialism at career start:

| Specialism | Focus | Bonus |
|------------|-------|-------|
| Gathering | Resource extraction | More R&D points from gathering |
| Farming | Agricultural production | More R&D points from farming |
| Industry | Manufacturing | More R&D points from factories |
| Logistics | Transportation | More R&D points from logistics |

### 4.3 Industry Categories

- Food
- Carpentry
- Jewelry
- Metallurgy
- Drinks
- Livestock
- Glass & Metallurgy
- Home Goods
- Petrochemicals
- Heavy Industry

### 4.4 HQ Upgrades

When 10 unlocks are researched within a category:
- HQ model changes to represent that industry
- **Rural Model**: Drinks, Food, Livestock
- **Heavy Industry Model**: Glass & Metallurgy, Home Goods, Petrochemicals
- Grants specific industry bonuses (two upgrade tiers per industry)

### 4.5 Efficiency Technologies

| Efficiency Level | Output | Upkeep | Pollution |
|-----------------|--------|--------|-----------|
| 100% (Base) | 1x | 1x | 1x |
| 125% | 1.25x | 1.25x | ~1.56x |
| 150% | 1.5x | 1.5x | ~2.25x |
| 200% | 2x | 2x | 4x |

**Note**: Efficiency upgrades must be manually applied per building after research.

---

## 5. Building Types

### 5.1 Gatherers

Gatherers extract raw resources from map deposits. Structure: 1 main building + up to 4 harvester units.

#### Land-Based Gatherers

| Building | Resource | Requires Deposit | Requires Road |
|----------|----------|------------------|---------------|
| Lumber Yard | Wood | Forest tiles | Yes |
| Coal Mine | Coal | Coal deposit | No |
| Iron Mine | Iron Ore | Iron deposit | No |
| Copper Mine | Copper | Copper deposit | No |
| Oil Drill | Oil | Oil deposit | No |
| Gas Pump | Gas | Gas deposit | No |
| Sand Quarry | Sand | Sand/Beach tiles | Yes |

#### Coastal Gatherers

| Building | Resource | Location |
|----------|----------|----------|
| Water Siphon | Water | Any water source |
| Water Well | Water | Inland (lower output) |
| Fisherman Dock | Fish | Coastal water |

### 5.2 Farms

Farms generate components when supplied with water.

#### Crop Farms and Orchards

| Farm Type | Products | Input |
|-----------|----------|-------|
| Apple Orchard | Apples | Water |
| Orange Orchard | Oranges | Water |
| Grape Vineyard | Grapes | Water |
| Wheat Farm | Wheat | Water |
| Cotton Farm | Cotton | Water |
| Hop Farm | Hops | Water |
| Sugar Plantation | Sugar | Water |
| Vegetable Farm | Vegetables | Water |
| Cocoa Plantation | Cocoa | Water |
| Berry Farm | Berries | Water |
| Potato Farm | Potatoes | Water |

#### Livestock Farms

| Animal | Products | Inputs |
|--------|----------|--------|
| Chickens | Meat, Eggs | Water, Wheat |
| Cows | Meat, Leather, Milk | Water, Wheat |
| Sheep | Meat, Leather, Wool | Water, Wheat |

### 5.3 Factories

#### Light Industry

| Factory | Products | Category |
|---------|----------|----------|
| Brewery | Beer | Drinks |
| Distillery | Whiskey, Vodka, Wine, Cider | Drinks |
| Carpentry | Wood Furniture, Outdoor Furniture | Home Goods |
| Papermill | Paper, Cardboard | Components |
| Textile Factory | Fabric, Clothes | Textiles |

#### Heavy Industry

| Factory | Products | Category |
|---------|----------|----------|
| Glassworks and Smelter | Steel, Glass, Cans | Metallurgy |
| Petrochemical Plant | Plastics, Rubber | Petrochemicals |
| Chemical Plant | Dyes, Chemicals | Chemicals |

#### Food Industry

| Factory | Products | Category |
|---------|----------|----------|
| Food Factory | Bread, Soup, Chocolate, Cakes | Food |
| Preservation Factory | Canned Fish, Corned Meat, Marmalade | Food |

#### Mega-Factories (End Game)

| Mega-Factory | Prototype Product | Supply Chain Depth |
|--------------|-------------------|-------------------|
| Automobile Factory | Cars | Extensive (metals, parts, assembly) |
| Computer Factory | Computers | Extensive (electronics, plastics) |
| Food Processing Plant | Pre-packed Meals | Extensive (multiple food chains) |

### 5.4 Logistics Buildings

| Building | Capacity | Function |
|----------|----------|----------|
| Warehouse | 750 units | Auto-collects/distributes within range |
| Truck Depot | 240 units | Inter-city trade routes |
| Train Terminal | 240 units | Long-distance rail transport |
| Boat Depot | 180 units | Water-based transport |
| Zeppelin Hangar | Variable | Air transport for distant/difficult terrain |

---

## 6. Transportation Systems

### 6.1 Vehicle Types

| Vehicle | Capacity | Cost per Trip | Best Use Case |
|---------|----------|---------------|---------------|
| Regular Truck | 1 unit | $100 | Local delivery |
| Trade Truck | 2 units | $250 | Inter-depot routes |
| Train | 6 units | $500 | Long-distance, high-volume |
| Boat | 8 units | $750 | Coastal/water routes |
| Zeppelin | 4 units | $2,500 | Remote/mountainous areas |

### 6.2 Destination Trucks

- Carry 1 unit of product
- Travel from building to building
- Return to origin after delivery
- Best for short distances within production clusters

### 6.3 Trade Trucks

- Carry 2-3 units per truck
- Travel only between truck depots
- Despawn at final destination (don't return)
- More cost-effective for long distances

### 6.4 Trains

- Can carry multiple products (1 product per wagon)
- Serve multiple stations per route
- Require track infrastructure investment
- Tracks are one-way only
- Cannot cross tracks except via tunnel/bridge
- Maximum 45-degree turns

### 6.5 Trade Route Setup

1. Open Trade Route Panel
2. Click "Add Route"
3. Select vehicle type
4. Pick depot and product
5. Add stops with modes:
   - **Green (Pickup)**: Load products
   - **Red (Drop Off)**: Unload products
   - **Black (Keep)**: Pass through without action

---

## 7. Town/City Demand System

### 7.1 Settlement Structure

Towns contain various shop types that create demand:

| Shop Type | Product Category |
|-----------|-----------------|
| Farmer's Market | Raw farm products, basic food |
| Hardware Store | Tools, basic materials |
| Grocery Store | Processed food |
| Liquor Store | Alcoholic beverages |
| Deli | Specialty foods |
| Diner | Prepared meals |
| Construction Goods | Building materials |
| Home Goods | Furniture, housewares |
| Clothing Store | Apparel |
| Book Store | Books, newspapers |

### 7.2 Demand Mechanics

| Mechanic | Effect |
|----------|--------|
| Global Supply/Demand | Product prices set by global availability |
| Scarcity Premium | Towns pay more for scarce products |
| Variety Requirement | Each shop needs at least 2 different products to be "happy" |
| Demand Growth | Every 50k population, product requests increase |

### 7.3 Town Growth

| Population Milestone | Effect |
|---------------------|--------|
| Every 50k | Increased demand for products |
| Every 100k | New advanced tier store built |
| 100-300k increments | Town wants to "advance" (level up) |

---

## 8. Economic Simulation

### 8.1 Pricing System

| Mechanic | Effect |
|----------|--------|
| Dynamic Pricing | Based on global supply/demand |
| Abundance Effect | More supply = lower prices |
| Scarcity Premium | Towns pay extra for rare products |

### 8.2 Loan System

| Loan Type | Interest | Notes |
|-----------|----------|-------|
| Starter Loan | 0% | No interest, no urgency to repay |
| Standard Loans | Variable | Available through HQ building |

### 8.3 Contract System

**Contract Components**:
- Objective: Deliver Y units of Product to Settlement by Date X
- Time Limit: Displayed in Contract Tab
- Rewards: Influence, Money, or temporary price increases
- Penalties: For failure to deliver

**Contract Mechanics**:
- Maximum 3 active contracts at once
- Auction system for competitive bidding
- Reward formula: Baseline Price x RNG(1-3)

### 8.4 Permit System (Regional Access)

| Permit Type | Allows | Use Case |
|-------------|--------|----------|
| Full Build Permit | All buildings | Production regions |
| Logistics Permit | Roads, rails only | Transport corridors |

**Permit Costs Based On**:
- Number of permits already owned
- Region size
- Resources in region
- Town growth level (grows over time)

---

## 9. Map Generation and Regions

### 9.1 Procedural Generation

- Terrain generated from seed
- Resource deposits distributed by type
- Multiple climate zones
- Towns placed with initial shops

### 9.2 Resource Distribution

| Resource | Location Pattern |
|----------|------------------|
| Coal | Clustered deposits |
| Iron | Scattered deposits |
| Oil | Specific geological formations |
| Gas | Often near oil |
| Sand | Coastal and desert areas |
| Wood | Forest regions |
| Water | Rivers, lakes, coastlines |

### 9.3 Regional Permits

Each region requires a permit to build. Costs increase with:
- Existing permit count
- Region value (resources, towns)
- Town growth level

---

## 10. Game Modes

### 10.1 Career Mode

| Property | Details |
|----------|---------|
| Starting Conditions | Limited buildings, starting capital, chosen specialism |
| Progression | Unlock buildings through XP and research |
| End Goal | Build a mega-factory producing Automobiles, Computers, or Pre-packed Meals |
| Victory | Creating a full prototype triggers win screen with score |

### 10.2 Sandbox Mode

- All buildings unlocked from start
- Unlimited funds option
- No progression requirements
- Full creative freedom

---

## 11. Pollution and Environment

### 11.1 Pollution Mechanics

| Property | Details |
|----------|---------|
| Sources | Factories and fossil fuel gatherers |
| Spread | Pollution spreads once a tile reaches 100% blight |
| Efficiency Trade-off | 200% efficiency = 400% pollution |

### 11.2 Pollution Mitigation

| Method | Effect |
|--------|--------|
| Trees | Passive pollution reduction |
| Cleaners | -5% effect per tile from source |
| Distance | Place polluters far from sensitive areas |

---

## 12. DLC Content

### 2130 DLC Expansion

**Release**: October 24, 2019
**Price**: $9.99

Set 200 years after the base game in a dystopian future:

| Feature | Description |
|---------|-------------|
| Setting | Dystopian future world |
| New Tech Tree | Futuristic technologies |
| New Resources | Future materials and components |
| Maglev Trains | High-speed rail transport |
| Cargo Boats | Enhanced water transport |
| Dropships | Advanced air transport |
| Pollution Harnessing | Turn pollution into resources |
| Scavenging | Extract resources from ruined cities |

---

## Appendix: Quick Reference

### Production Chain Examples

**Beer Production**:
```
Water Siphon → Water
Hop Farm + Water → Hops
Wheat Farm + Water → Wheat
Brewery + Hops + Wheat → Beer
```

**Furniture Production**:
```
Lumber Yard → Wood
Carpentry + Wood → Wood Furniture
```

**Steel Production**:
```
Coal Mine → Coal
Iron Mine → Iron Ore
Glassworks + Coal + Iron Ore → Steel
```

### Cost-Effectiveness Summary

| Transport | Cost/Unit | Best Distance |
|-----------|-----------|---------------|
| Local Truck | $100 | < 1 region |
| Trade Truck | $125 | 1-2 regions |
| Train | $83 | 2+ regions |
| Boat | $94 | Water routes |
| Zeppelin | $625 | Remote only |

---

*Document compiled for game design research purposes. Data sourced from Rise of Industry Wiki, Steam Community guides, and official documentation.*
