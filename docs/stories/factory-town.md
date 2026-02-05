# Factory Town - Game Design Research Document

**Game:** Factory Town
**Developer:** Erik Asmussen
**Genre:** 3D Factory/City Builder with Worker NPCs
**Platform:** PC (Steam)
**Style:** Casual, colorful, approachable automation game

---

## Table of Contents

1. [Game Overview](#game-overview)
2. [Initial Conditions](#initial-conditions)
3. [Core Mechanics](#core-mechanics)
4. [Worker System](#worker-system)
5. [Logistics and Transportation](#logistics-and-transportation)
6. [Building Types](#building-types)
7. [Technology/Research Tree](#technologyresearch-tree)
8. [Production Chains](#production-chains)
9. [World/Map System](#worldmap-system)
10. [Economy System](#economy-system)
11. [Magic/Mana System](#magicmana-system)
12. [Endgame](#endgame)
13. [Sources](#sources)

---

## Game Overview

Factory Town is a singleplayer bird's-eye view building game where the goal is to build a thriving village in the wilderness. Players produce a wide selection of food, clothing, tools, books, and magical artifacts from raw materials. The game combines classic city builder elements (units, buildings, research) with a strong focus on automation and logistics paths.

**Core Loop:**
1. Gather raw resources
2. Process them through production buildings
3. Sell finished goods to markets/houses
4. Earn coins, experience, and research points
5. Unlock new technologies and buildings
6. Expand and optimize

---

## Initial Conditions

### Starting Scenario

- **Starting Building:** Town Center (Base) - serves as the initial warehouse and central hub
- **Initial Workers:** 4 workers available (4/4 capacity with 0 houses)
- **Initial Storage:** Limited storage in the base building only
- **Starting Resources:** Access to nearby trees, stone, and basic terrain

### Tutorial Progression

1. **First Task:** Cut trees to gather wood
2. **Build Houses:** Each house provides capacity for 2 additional workers
3. **First Production:** Set up a Lumber Mill to convert wood into planks
4. **Basic Logistics:** Assign workers to harvest resources and deliver to production buildings
5. **Market Introduction:** Build a Food Market to start selling goods

### Early Game Requirements

- Workers must manually carry items between buildings initially
- Need housing to unlock additional worker capacity
- Basic production line: Forester -> Lumber Mill -> Planks
- First automation: Chutes for downhill gravity-based transport

---

## Core Mechanics

### Resource Types

#### Raw/Natural Resources

| Resource | Source | Notes |
|----------|--------|-------|
| Wood (Log) | Trees via Forester | Most common resource, base for building materials |
| Stone | Stone deposits via Mine | Basic building material |
| Iron Ore | Iron deposits via Mine | Must be smelted at Forge |
| Coal | Coal deposits via Mine | Fuel resource |
| Gold Ore | Gold deposits via Mine | Late-game accessory crafting |
| Mana Shard | Mana deposits | Main material for Mana Crystals |
| Earth Stone | Earth deposits | Elemental magic material |
| Fire Stone | Fire deposits | Elemental magic material |
| Water Stone | Water deposits | Elemental magic material |
| Air Stone | Air deposits | Elemental magic material |

#### Farm Crops

| Crop | Building | Use |
|------|----------|-----|
| Grain/Wheat | Farm | Flour, Animal Feed, direct sale |
| Cotton | Farm | Cloth production |
| Berries | Farm/Forester | Food, direct sale |
| Apples | Forester (trees) | Food, Juice |
| Carrots | Farm | Food, Vegetables |
| Tomatoes | Farm | Food, Vegetables |
| Potatoes | Farm | Food, Vegetables |
| Pears | Forester (trees) | Food, Fruit |
| Herbs | Farm | Medicine, Potions |
| Sugar | Farm | Advanced cooking |

#### Animal Products (Pasture)

| Product | Animal | Use |
|---------|--------|-----|
| Wool | Sheep | Cloth, Clothing |
| Leather | Cattle | Clothing, Crafting |
| Milk | Cattle | Food, Butter |
| Eggs | Chickens | Food |
| Fertilizer | All animals | Farm boost |
| Meat (Beef, Poultry, Mutton) | Various | Food cooking |

#### Processed Materials

| Material | Source Building | Inputs |
|----------|-----------------|--------|
| Planks | Lumber Mill | Wood |
| Paper | Lumber Mill | Wood + Water |
| Stone Brick | Stone Mason | Stone |
| Polished Stone | Stone Mason | Stone |
| Iron Plate | Forge | Iron Ore + Fuel |
| Nails | Forge | Iron Plate |
| Metal Bars | Forge | Iron Ore + Coal |
| Cloth | Workshop | Cotton or Wool |
| Flour | Food Mill | Grain |
| Bread | Kitchen | Flour + Fuel |

#### Chute-Compatible Items

These items can travel on gravity chutes:
- Wood, Stone, Iron Ore, Gold Ore, Coal
- Grain, Apples, Berries, Carrots, Cotton, Tomato, Pear, Potato, Herbs
- Flour, Animal Feed, Eggs
- Various mana-related items

---

## Worker System

### Worker Types

| Unit Type | Capacity | Speed | Special Abilities | Terrain |
|-----------|----------|-------|-------------------|---------|
| Worker | 1 item | Base | Can harvest all resources | Land only |
| Wagon | 4 items | Base | Cannot harvest | Land only |
| Harvester Drill | 10 items | Slow | Can harvest mining resources only | Land only |
| Minecart | 20 items | Fast (on rails) | Requires tracks | Rails only |
| Fishing Boat | Variable | Water speed | Harvests fish, transports items | Water only |
| Cargo Boat | 80 items | Water speed | Large capacity transport | Water only |
| Caravan | 4 types, 8 each | Fast on roads | Best on stone brick roads | Land only |
| Airship | Large | Fastest | Ignores terrain completely | Any |

### Worker Mechanics

**Pickup/Dropoff Speed:**
- Workers pick up and drop off items in **0.5 seconds**
- Maximum theoretical throughput: **1 item per second** when stationary

**Adjacent Building Efficiency:**
- "Passing" efficiency (no movement): 1 item/second
- With 1 tile separation: ~0.78 items/second (walking time reduces efficiency)

**Production Building Workers:**
- Most buildings: 1 worker required to operate
- Additional workers increase production speed
- Max workers: 5 for most buildings, 10 for mines/farms/mana buildings
- Efficiency note: 2 buildings with 1 worker each > 1 building with 2 workers

**Pathfinding Behavior:**
- Workers prefer roads/paths when available
- Will take longer road routes over shorter pathless routes
- Path choice algorithm updates when entering new tiles
- Known quirk: Workers may take convoluted routes if roads exist

### Speed Bonuses

**Road Types (Worker Speed Multipliers):**
| Path Type | Speed Bonus |
|-----------|-------------|
| No path | 1.0x (base) |
| Foot Path | ~1.5x |
| Stone Road | ~2.0x |
| Stone Brick Road | ~2.5x (best for Caravans) |

---

## Logistics and Transportation

### Conveyor Belts

| Belt Type | Speed (items/sec) | Research Level | Notes |
|-----------|-------------------|----------------|-------|
| Cloth Conveyor Belt | 1.215 | Early | Starter belt, versatile |
| Metal Conveyor Belt | 1.911 | Mid | Good mid-game option |
| Magic Conveyor Belt | 3.686 | Late | Highest throughput |

**Belt Advantages:**
- Can transport items uphill
- Can carry any item type
- Can build bridges over pathways with structural blocks
- No restrictions on turns or directions

### Chutes

| Configuration | Speed (items/sec) | Notes |
|---------------|-------------------|-------|
| Level Chute | ~2.0 | Equal to Metal Belt |
| Downhill Chute | 3.0+ | Faster with steeper slopes |
| Straight Downhill | 4.0+ | Maximum gravity boost |

**Chute Limitations:**
- Only works flat or downhill
- Cannot turn corners
- Limited item types (mostly raw resources)
- Cannot go uphill

### Rail System

| Rail Type | Notes |
|-----------|-------|
| Basic Rails | Manual minecart movement |
| Mechanical Rails | Powered by Rotational Power, minimum speed guarantee |
| Magic Rails | Accelerates to max speed gradually |

**Rail Vehicles:**

| Vehicle | Capacity | Requirements |
|---------|----------|--------------|
| Minecart | 20 items | Basic rails |
| Freight Hopper Car | 100 items (loose bulk) | Advanced rails |
| Steam Locomotive | Pulls multiple cars | Fuel + Water, Train Stations |
| Train (full) | 100-200 items/sec | Load/unload at Train Stations |

**Rail Advantages:**
- Higher throughput over same area (each cart holds many items)
- Workers can walk over tracks
- Continuous loop delivery to multiple destinations
- Packager building can load crates for even higher capacity

### Pipe Systems

| Pipe Type | Contents | Special Features |
|-----------|----------|------------------|
| Fluid Pipes | Water, Ether, Milk, Juice, Potions, Fish Oil | Can go underground, ignores gravity |
| Steam Pipes | Steam Power | Crossover arrangement possible |
| Mana Pipes | Mana Crystals | Long-distance crystal transport without rails |
| OmniPipes | Any item | End-game, infinite speed upgrades |

### Logistics Buildings

| Building | Function |
|----------|----------|
| Grabber | Pulls items from buildings onto conveyors; can be filtered |
| Sorter | Redirects filtered items; allows passthrough if destination full |
| Filter | Whitelist or blacklist specific items |
| Splitter | Divides items between adjacent conveyors/buildings |
| Pusher | Pushes items off conveyors into buildings |
| Packager | Packages items into crates for rail transport |

---

## Building Types

### Resource Gathering Buildings

| Building | Function | Outputs |
|----------|----------|---------|
| Forester | Gathers from trees | Wood, Apples, Pears, Berries |
| Farm | Grows ground crops | Grain, Cotton, Vegetables, Herbs |
| Mine | Extracts minerals | Stone, Iron Ore, Coal, Gold Ore |
| Mine Shaft | Underground mining | Renewable ore access |
| Pasture | Raises livestock | Wool, Leather, Milk, Eggs, Fertilizer |
| Fishery | Catches fish | Fish |
| Well/Water Pump | Extracts water | Water |

### Processing Buildings

| Building | Inputs | Outputs |
|----------|--------|---------|
| Lumber Mill | Wood, (Water) | Planks, Paper |
| Stone Mason | Stone | Stone Brick, Polished Stone |
| Forge | Iron Ore, Coal/Fuel | Iron Plate, Nails, Metal Bars |
| Food Mill | Grain | Flour |
| Kitchen | Various food items | Bread, Meals, Gourmet Food |
| Workshop | Planks, Cloth, etc. | Cloth, Tools, Reinforced Planks |
| Tailor | Cloth, Wool, Leather | Shirts, Cloaks, Clothing |
| Medicine Hut | Herbs, Cloth | Health Potions, Antidotes |
| Machine Shop | Planks, Metal | Rails, Conveyor Belts, Machinery |

### Market Buildings

| Building | Goods Accepted | Coin Type |
|----------|----------------|-----------|
| Food Market | Food items (Grain, Vegetables, Bread, etc.) | Yellow |
| General Store | Tools, Materials, Basic Goods | Red |
| Apothecary | Medicines, Potions | Blue |
| Tavern | Gourmet Food, Drinks | Blue |
| Specialty Goods | Jewelry, Magic Items, Luxury Goods | Purple |

**Market Mechanics:**
- Markets consume goods when "satisfaction" meter drops
- Provides town-wide Happiness production bonus
- Higher tier demands unlock with upgraded nearby houses
- Satisfaction depletes faster at higher tiers (more sales/sec)
- Tier requirements: I, II, III, IV, V (matched by item tier)

### Housing

| Level | Population | Happiness Cap | Upgrades With |
|-------|------------|---------------|---------------|
| 1 | 2 workers | Low | Experience from sales |
| 5 | ~6 workers | Medium | Continued sales |
| 10 (Max) | ~12 workers | High | Full market satisfaction |

**Housing Features:**
- Each house adds population capacity
- House level increases with experience from nearby market sales
- Visual appearance changes with upgrades
- Proximity to markets affects which tiers are unlocked

### Research Buildings

| Building | Function | Outputs |
|----------|----------|---------|
| School | Sells Books/Paper/Tomes to houses | Research Points |
| Laboratory | Creates books | Natural + Industrial Research Points |
| Mage Tower | Creates magical books | Magic Research Points |

### Magic Buildings

| Building | Function |
|----------|----------|
| Magic Forge | Produces mana-related items |
| Mana Transmitter | Delivers Mana Crystals to provide Mana Power |
| Mana Receiver | Receives Mana Power at buildings |
| Mana Recharger | Recharges depleted mana crystals |
| Elemental Refinery | Converts Elemental Stones to Ether |
| Enchanter | Creates enchanted items, jewelry |
| Air/Earth/Fire/Water Shrine | Elemental power buildings |

### Power Buildings

| Building | Function |
|----------|----------|
| Steam Generator | Produces Steam from Water + Fuel |
| Steam Engine | Converts Steam to Rotational Power |
| Water Pump (Powered) | More efficient water extraction with Rotational Power |

### Special Buildings

| Building | Function |
|----------|----------|
| Town Center (Base) | Central warehouse, starting building |
| Barn | Additional storage |
| Silo | Bulk storage for specific items |
| Crate | Small storage container |
| Airship Dock | Airship management |
| Train Station | Rail vehicle loading/unloading |
| OmniTemple | End-game building, infinite item sink |

---

## Technology/Research Tree

### Research System Overview

- Open Research panel with **R key**
- Purchase tech when you have enough resources and prerequisites
- Research Points come in different "flavors": General, Industry, Nature, Magic

### Research Point Sources

| Point Type | Earned From |
|------------|-------------|
| General Research | Selling Books, Tomes to houses |
| Industrial Research | Laboratory production |
| Natural Research | Laboratory production |
| Magic Research | Mage Tower production |

### Research Tiers (Examples)

| Research | Cost | Unlocks |
|----------|------|---------|
| Farming | 100 General + 100 Yellow Coins | Farms, Crops |
| Forester | ~50 General | Forester building |
| Mining | ~150 General | Mine building |
| Pastures | ~200 General | Pasture, Animals |
| Kitchen | ~100 General + 50 Red | Kitchen building |
| Workshop | ~150 General | Workshop building |
| Tailor | ~200 General + 100 Red | Tailor building |
| Rails | ~300 Industrial | Rail tiles, Minecarts |
| Steam Power | ~400 Industrial | Steam Generator, Engine |
| Mana Purification | ~300 Magic | Magic Forge, Mana basics |
| Elemental Research | ~500 Magic | Elemental buildings |
| Trains | ~600 Industrial | Steam Locomotive |
| Enchanting | ~400 Magic | Enchanter building |
| Jewelry | ~300 General + Magic | Gold crafting |
| OmniTemple | Late game | End-game content |

### Research Cost Multiplier

- Default setting: 100% = 100 research points per tech
- Adjustable in custom games
- Affects all research costs proportionally

### Infinite Research

Once OmniTemple is unlocked:
- New infinite research appears at School
- Consumes Stars and high-end ingredients
- Can be repeated infinitely
- Gets more expensive each repetition
- Includes "House Maximum" for unlimited house building

---

## Production Chains

### Basic Production Chains

**Wood Processing:**
```
Trees -> Forester -> Wood
                      |
                      v
               Lumber Mill -> Planks -> Workshop (various)
                      |
                      v (+ Water)
                   Paper -> School, Books
```

**Stone Processing:**
```
Stone Deposits -> Mine -> Stone
                           |
                           v
                    Stone Mason -> Stone Brick -> Roads, Buildings
                           |
                           v
                    Polished Stone -> Decorative
```

**Iron Processing:**
```
Iron Ore Deposits -> Mine -> Iron Ore
                              |
Coal Deposits -> Mine -> Coal |
                         |    |
                         v    v
                        Forge -> Iron Plate -> Nails, Tools
                              -> Metal Bars -> Advanced crafting
```

**Food Chain (Basic):**
```
Farm -> Grain -> Food Mill -> Flour -> Kitchen (+ Fuel) -> Bread
                                |
                                v
                         Animal Feed -> Pasture
```

**Clothing Chain:**
```
Farm -> Cotton -> Workshop -> Cloth -> Tailor -> Shirts, Cloaks
                                |
Pasture -> Wool ----------------+
        -> Leather -> Tailor -> Leather goods
```

### Key Recipes

**Workshop Recipes:**
| Recipe | Inputs | Output |
|--------|--------|--------|
| Cloth | Cotton or Wool | Cloth |
| Reinforced Plank | Planks + Nails | Reinforced Plank |
| Wood Wheel | Planks | Wood Wheel |
| Iron Wheel | Iron Plate | Iron Wheel |
| Wood Axe | Planks + Iron Plate | Wood Axe |
| Pickaxe | Planks + Iron Plate | Pickaxe |
| Cloth Conveyor Belt | Cloth + Planks | Belt segments |
| Wood Rail | Planks + Iron Plate | Rail tiles |

**Kitchen Recipes:**
| Recipe | Inputs | Output |
|--------|--------|--------|
| Bread | Flour + Fuel | Bread |
| Fish Stew | Fish + Vegetables | Fish Stew |
| Apple Pie | Apples + Flour + Sugar | Apple Pie |
| Cake | Flour + Eggs + Sugar + Milk | Cake |
| Butter | Milk | Butter |
| Gourmet Meal | Multiple high-tier ingredients | Gourmet Food |

**Tailor Recipes:**
| Recipe | Inputs | Output |
|--------|--------|--------|
| Shirt | Cloth | Shirt |
| Cloak | Cloth + Wool | Cloak |
| Leather Boots | Leather + Nails | Boots |
| Warm Coat | Cloth + Wool + Leather | Warm Coat |
| Fine Clothing | Multiple fabrics | Luxury Clothing |

### Goods Tiers

| Tier | Examples | Market Type | Coin Reward |
|------|----------|-------------|-------------|
| I | Grain, Berries, Stone | Food Market | Low Yellow |
| II | Bread, Planks, Tools | General Store | Yellow + Red |
| III | Clothing, Processed Food | Various | Red + Blue |
| IV | Gourmet Food, Medicine | Tavern, Apothecary | Blue |
| V | Enchanted Items, Jewelry | Specialty | Purple |

### Production Ratios (Approximate)

- 1 Forester supports ~2-3 Lumber Mills
- 1 Farm (grain) supports ~1 Food Mill
- 1 Mine (iron) supports ~1-2 Forges
- 1 Pasture supports ~1 Tailor (wool) or Kitchen (milk/eggs)

---

## World/Map System

### Map Generation

- Procedurally generated using **Perlin noise layering**
- Adjustable parameters for custom games
- Multiple **biomes** selectable per map
- Default starting biome: **Plains and Rivers**

### Biome Types

| Biome | Characteristics |
|-------|-----------------|
| Plains and Rivers | Flat terrain, water access, balanced resources |
| Mountains | High elevation, more minerals, difficult terrain |
| Forest | Dense trees, more wood, limited flat space |
| Desert | Scarce water, unique resources |
| Swamp | Abundant water, challenging logistics |
| Islands | Water-heavy, requires boats |

### Terrain Features

**Elevation:**
- Multiple height levels
- Affects building placement
- Critical for chute logistics (gravity-based)
- Can be modified with terraforming (costs red coins)

**Water:**
- Rivers and lakes provide water access
- Lumber Mills need 40+ connected water tiles for full draw rate
- Water draw rate: 1 water/second (not affected by workers)
- Partial draw if < 40 tiles (proportional to tiles/40)

**Resources Distribution:**
- Tree clusters near spawn
- Stone deposits scattered across map
- Iron/Coal often in hillier terrain
- Gold Ore in specific deposits
- Mana Shards in magical locations
- Elemental Stones in specific biome areas

### Map Expansion

- Initial play area is limited
- Scroll to map edge to see purchase options
- Expand territory using Yellow Coins
- **Elemental Temples** hidden in purchasable territory
- Must buy surrounding lands to find all four temples

### Terraforming

| Action | Cost | Notes |
|--------|------|-------|
| Raise Terrain | Red Coins | Creates elevation |
| Lower Terrain | Red Coins | Removes elevation |
| Place Water | Red Coins | Creates water tiles |
| Remove Water | Red Coins | Drains water tiles |

- Disabled in some campaign missions
- Available by default in custom games
- Can be toggled off for challenge

---

## Economy System

### Coin Types

| Coin | Color | Primary Sources | Primary Uses |
|------|-------|-----------------|--------------|
| Yellow | Yellow | Food sales, basic goods | Building costs, land purchase, early research |
| Red | Red | Tools, materials, General Store | Advanced buildings, terraforming |
| Blue | Blue | Medicine, Gourmet Food | Magic research, advanced tech |
| Purple | Purple | Luxury goods, enchanted items | End-game content |

### Coin Generation Strategy

**Yellow Coins (Early Game):**
- Food items produced "for free" from farms
- Best early items: Berries + Vegetables + Flour
- Upgraded: Switch flour for Bread (Kitchen)
- Profit margin: Highest for raw -> basic processed

**Red Coins (Mid Game):**
- Shirts sell for 10 red coins each
- Iron-based tools and goods
- General Store sales

**Blue Coins (Late Game):**
- Medicine Hut products
- Gourmet food from Tavern
- Advanced processed goods

**Purple Coins (End Game):**
- Enchanted items
- Jewelry
- Specialty luxury goods

### Profit Scaling

- Higher tier products have small extra cost vs. basic items
- More complex products = more profit margin
- Processing raw goods always more valuable than selling raw

### Experience Points

| Source | XP Earned |
|--------|-----------|
| Selling items to markets | Item XP value |
| Higher tier items | More XP |
| Market variety bonus | Additional XP |

**Progression:**
- Houses level up with XP
- House levels increase population cap
- House levels increase happiness cap
- Visual house appearance changes with level
- Max house level: 10

### Happiness System

**Happiness Sources:**
- Market satisfaction (variety of goods)
- Town infrastructure
- Building upgrades

**Happiness Effects:**
- Global production speed buff for town buildings
- **+10% speed per 100 happiness**
- 1000 happiness = **2x production speed**
- School receives **3x** the happiness bonus
- Multiplicative with other bonuses (workers, upgrades)

**Market Happiness Bonus:**
- Wide variety of goods = more happiness
- All market categories satisfied = maximum bonus
- Higher tier goods = extra happiness contribution

---

## Magic/Mana System

### Mana Progression Overview

**Step 1: Research Mana Purification**
- First required research for any mana content
- Unlocks Magic Forge

**Step 2: Produce Basic Mana Items**
- Magic Forge creates mana-related components
- Mine Mana Shards from deposits
- Create Mana Crystals

**Step 3: Mana Power Distribution**
- Deliver Mana Crystals to Mana Transmitter
- Connect via Mana Pipes to buildings
- Buildings receive Mana Power buff

**Step 4: Elemental Refinement**
- Mine Elemental Stones (Earth, Fire, Water, Air)
- Send to Elemental Refinery
- Produces Elemental Ether

**Step 5: Elemental Crystals**
- Ship Elemental Ether + 2 Mana Crystals to Elemental Temple
- Temple outputs specific Elemental Crystal
- Each element has its own temple

### Elemental Temples

| Temple | Location | Input | Output |
|--------|----------|-------|--------|
| Earth Temple | Hidden in purchasable land | Earth Ether + 2 Mana Crystals | Earth Crystal |
| Fire Temple | Hidden in purchasable land | Fire Ether + 2 Mana Crystals | Fire Crystal |
| Water Temple | Hidden in purchasable land | Water Ether + 2 Mana Crystals | Water Crystal |
| Air Temple | Hidden in purchasable land | Air Ether + 2 Mana Crystals | Air Crystal |

**Finding Temples:**
- Cannot be built - must be discovered
- Located randomly outside initial map bounds
- Must purchase territory to reveal
- Each temple in different location per game

### Magic Buildings Details

**Magic Forge:**
- Unlocked with Mana Purification research
- Produces all mana-related items
- Central to magic production chain

**Mana Transmitter:**
- Receives Mana Crystals
- Outputs Mana Power through Mana Pipes
- Powers connected buildings

**Mana Recharger:**
- Recharges Depleted Mana Crystals
- Enables crystal recycling
- Critical for sustained mana production

**Elemental Refinery:**
- Converts Elemental Stones to Ether
- Outputs Depleted Mana Crystal (extract separately)
- Requires elemental research

**Enchanter:**
- Unlocks late (requires Mana Power)
- Creates enchanted items
- Recipes require Elemental Crystals
- Produces items for Specialty Goods market

### Mana Products

| Product | Ingredients | Use |
|---------|-------------|-----|
| Mana Crystal | Mana Shards | Power, Elemental Crystals |
| Elemental Ether | Elemental Stone | Temple input |
| Elemental Crystal | Ether + Mana Crystals | Enchanting |
| Magic Conveyor Belt | Cloth Belt + Mana Crystal | Fast transport |
| Magic Rails | Rails + Mana Crystal | Accelerating rails |
| Enchanted Book | Book + Mana Crystal | Base upgrades |
| Enchanted Jewelry | Gold + Elemental Crystal | Specialty sales |

### Shrines

| Shrine | Element | Function |
|--------|---------|----------|
| Earth Shrine | Earth | Recharges ore deposits |
| Fire Shrine | Fire | Heat-based bonuses |
| Water Shrine | Water | Water production |
| Air Shrine | Air | Speed bonuses |

---

## Endgame

### OmniTemple

**Building Requirements:**
- Size: 5x5 tiles (massive)
- Construction cost: Very high
- Resource delivery: Huge amounts to construction site
- Late-game research required

**Functionality:**
- Infinite item sink
- Randomly requests Offerings
- 4 different high-end items per cycle
- Various stack amounts required
- Earn 1 Star per completed cycle
- Recipe changes every 100 cycles

### Stars

**Earning:**
- Complete OmniTemple offering cycles
- Each cycle = 1 Star
- Town level milestones (every 5th level)

**Spending:**
- Infinite research at School
- Global perks
- Permanent upgrades

### Infinite Research

| Research | Effect | Cost Scaling |
|----------|--------|--------------|
| Production Speed | Global speed boost | Increases per purchase |
| House Maximum | More houses allowed | Increases per purchase |
| Worker Capacity | More workers per house | Increases per purchase |
| Transport Speed | Faster logistics | Increases per purchase |

### Victory Conditions

**Customizable Goals (set at game creation):**
- Minimum happiness level
- Required building types (specific counts)
- Required player items
- Minimum items produced (even if consumed)
- Minimum base level

**Default Victory:**
- OmniTemple completion goals
- Developer intention: OmniTemple as default win condition

### Endgame Loop

1. Build and supply OmniTemple
2. Earn Stars from offerings
3. Spend Stars on infinite research
4. Research bonuses increase production speed
5. Faster production supplies more to OmniTemple
6. Positive feedback loop until extreme speeds achieved

### OmniPipes

- End-game transport option
- Extremely expensive
- Transports ANY item through pipes
- Can research infinite speed upgrades
- Ultimate logistics solution

### Completion Estimates

- Dedicated play: 1-2 weeks to reach level 50
- Casual play: Several weeks to months
- True completion (all infinite research): Ongoing

---

## Sources

### Official Resources
- [Official Factory Town Wiki - Buildings](https://factorytown.fandom.com/wiki/Buildings)
- [Official Factory Town Wiki - Items](https://factorytown.fandom.com/wiki/Items)
- [Official Factory Town Wiki - Workers](https://factorytown.fandom.com/wiki/Workers)
- [Official Factory Town Wiki - Research](https://factorytown.fandom.com/wiki/Research)
- [Official Factory Town Wiki - Getting Started](https://factorytown.fandom.com/wiki/Getting_Started_/_How_to_Play)
- [Official Factory Town Wiki - Map](https://factorytown.fandom.com/wiki/Map)
- [Official Factory Town Wiki - Tutorial](https://factorytown.fandom.com/wiki/Tutorial)
- [Official Factory Town Wiki - Basic Logistics Guide](https://factorytown.fandom.com/wiki/Basic_Logistics_Guide)
- [Official Factory Town Wiki - Market demand and happiness](https://factorytown.fandom.com/wiki/Market_demand_and_happiness)
- [Official Factory Town Wiki - Production Speed Bonuses](https://factorytown.fandom.com/wiki/Production_Speed_Bonuses)
- [Official Factory Town Wiki - Item Transport](https://factorytown.fandom.com/wiki/Item_Transport)
- [Official Factory Town Wiki - OmniTemple](https://factorytown.fandom.com/wiki/OmniTemple)

### Steam Community
- [Steam Community - Factory Town](https://steamcommunity.com/app/860890)
- [Guide: Making Sense of Magic](https://steamcommunity.com/sharedfiles/filedetails/?id=1700164197)
- [General Guide (WIP)](https://steamcommunity.com/sharedfiles/filedetails/?id=1498953499)
- [Campaign Guide and Tips for v1.0](https://steamcommunity.com/sharedfiles/filedetails/?id=2697624855)
- [Logistics and Computing Guide](https://steamcommunity.com/sharedfiles/filedetails/?id=2023835110)

### Developer Updates
- [Factory Town - Research Overhaul](https://store.steampowered.com/news/app/860890/view/3028089305142633608)
- [Factory Town - OmniTemple and Infinite Research](https://store.steampowered.com/news/app/860890/view/3931035846876453232)
- [Factory Town - Trains Update](https://steamdb.info/patchnotes/5209605/)
- [Factory Town - Pipes Update](https://store.steampowered.com/news/app/860890/view/3931035846876452973)
- [Factory Town - Cargo Boats and Caravans](https://store.steampowered.com/news/app/860890/view/3931035846876453470)
- [Factory Town - Mining Overhaul](https://store.steampowered.com/news/app/860890/view/1670199476441803668)
- [Factory Town - Freight Rail Cars](https://store.steampowered.com/news/app/860890/view/2930113183954904081)

### Other Resources
- [Factory Town Beginners Guide - Number13](https://en.number13.de/factory-town-beginners-guide/)
- [Factory Town Campaign Guide - KosGames](https://kosgames.com/factory-town-campaign-guide-and-tips-for-v1-0-13586/)
- [Factory Town Production Calculator - GitHub](https://github.com/AlyxMoon/factory-town-production-calculator)
- [Factory Town Planner](https://gc-locks.github.io/ftplanner/dist/index.html)
- [Newbie's Guide to Factory Town - Steam Solo](https://steamsolo.com/guide/a-newbie-s-guide-to-factory-town-factory-town/)

---

*Document compiled from web research. Some specific numbers may vary with game updates. Last researched: February 2026.*
