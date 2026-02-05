# Little Big Workshop: Comprehensive Game Design Research Document

## Table of Contents
1. [Game Overview](#1-game-overview)
2. [Initial Conditions](#2-initial-conditions)
3. [Core Mechanics](#3-core-mechanics)
4. [Worker System](#4-worker-system)
5. [Workstations and Machines](#5-workstations-and-machines)
6. [Blueprint and Recipe System](#6-blueprint-and-recipe-system)
7. [Order and Contract System](#7-order-and-contract-system)
8. [Factory Layout and Space Management](#8-factory-layout-and-space-management)
9. [Resource Procurement](#9-resource-procurement)
10. [Research and Tech Tree](#10-research-and-tech-tree)
11. [Progression Systems](#11-progression-systems)
12. [Unique Mechanics](#12-unique-mechanics)

---

## 1. Game Overview

### Developer and Publisher
- **Developer**: Mirage Game Studios (Swedish studio)
- **Publisher**: HandyGames (THQ Nordic subsidiary)
- **Release Date**: October 17, 2019 (PC), 2020 (Consoles)
- **Platforms**: PC (Steam, GOG, Epic), PlayStation 4, Xbox One, Nintendo Switch

### Core Concept

Little Big Workshop is a factory management simulation where players oversee a miniature workshop that appears as a diorama on a tabletop. Players manage cute, cartoon-style workers to produce over 50 different product types, from simple furniture to complex electronics and toys.

The game combines workshop simulation with tycoon elements, requiring players to:
- Design efficient production layouts
- Manage worker assignments and happiness
- Research new technologies and machines
- Fulfill market orders and client contracts
- Compete against a rival corporation (Nemesis Inc.)

### Visual Style

The game's most distinctive feature is its **diorama aesthetic**:
- The factory exists on what appears to be a workshop table or drafting board
- Coffee mugs and blueprints lie around the virtual table's edge
- Workers are depicted as small, gnome-like figures with rounded, cartoon features
- The tabletop miniature presentation creates a cozy, toy-like atmosphere
- Complete day/night cycle with environmental lighting changes

---

## 2. Initial Conditions

### Starting Resources

| Resource | Amount | Purpose |
|----------|--------|---------|
| Starting Money | ~$20,000 | Initial capital for equipment and materials |
| Starting Workers | 2-3 operators | Basic workforce for production |
| Factory Rooms | 3 separate rooms | Pre-divided workspace |

### Bankruptcy Threshold

- **Game Over Condition**: Net balance falls below -$5,000
- No loan system available - players must work within their capital
- No way to recover from bankruptcy state

### First Available Equipment

Players begin with access to basic Tier 0 workstations:

| Workstation | Cost | Size | Operations |
|-------------|------|------|------------|
| Woodworking Station | $2,475 | 6x4 | 6 different wood operations |
| Metalworking Station | $2,060 | 4x4 | Multiple metal operations |
| Assembly Station | $2,805 | 7x4 | General assembly |
| Glue Station | $3,120 | - | Adhesive operations |

### Tutorial Flow

1. Introduction to the diorama workshop concept
2. Basic room navigation and camera controls
3. Placing first workstation (typically Woodworking Station)
4. Accepting first market order
5. Creating first production plan/blueprint
6. Assigning workers to production
7. Material procurement basics
8. Delivery and payment collection

### First Recommended Products

- Simple wooden items (stools, shelves)
- Basic plastic toys (injection molding)
- Products with minimal operation steps
- Items using only Tier 0 workstations

---

## 3. Core Mechanics

### 3.1 Production Flow

The fundamental production loop follows this sequence:

```
Raw Materials → Storage → Workstation Input → Processing → Intermediate Storage → Assembly → Export Zone → Delivery
```

### 3.2 Planning Mode

Building a product begins with the **Planning Mode**, which defines:

1. **Product Selection**: Choose what to manufacture
2. **Material Choices**: Select materials for each component (wood, metal, plastic, leather)
3. **Workstation Assignment**: Link operations to specific workstations or billboards
4. **Quality Targets**: Set minimum attribute requirements (durability, style, comfort)

**Key Planning Features**:
- Plans can be duplicated and modified for variations
- Each material choice affects final product attributes
- More complex blueprints offer more material/equipment choices
- Plans can be named for easy identification (e.g., "C4 D24 S17" for Comfort 4, Durability 24, Style 17)

### 3.3 Production Attributes

Products have three main quality attributes:

| Attribute | Description | Affected By |
|-----------|-------------|-------------|
| **Durability** | How long the product lasts | Metal content, material quality |
| **Style** | Aesthetic appeal | Material choices, design options |
| **Comfort** | User comfort level | Padding, material softness |

**Attribute Examples**:
- Small Drawer with metal handles: Higher durability, lower style
- Small Drawer with wood knobs: Lower durability, higher style
- Material quality directly impacts attribute values

### 3.4 Worker AI and Task Assignment

Workers operate autonomously based on:

1. **Task Priority**: Jobs are processed in queue order
2. **Proximity**: Workers prefer nearby tasks
3. **Availability**: Workers take the next available task when free
4. **Specialization**: Specialists prioritize their specialty machines

**Worker Task Flow**:
1. Check for available jobs in queue
2. Retrieve required materials from storage/input zones
3. Transport materials to assigned workstation
4. Perform operation (time varies by complexity and worker skill)
5. Deposit finished component in storage zone
6. Return to task queue check

### 3.5 Workstation Queue System

Each workstation maintains a job queue:
- Jobs sorted with upcoming job at top
- Following jobs in descending order
- Jobs can be reordered manually
- Jobs can be moved between workstations
- **Billboards** allow automatic distribution across multiple workstations

### 3.6 Billboard System

Billboards serve as **workstation group managers**:

| Function | Description |
|----------|-------------|
| Work Distribution | Evenly distributes jobs across linked workstations |
| Queue Management | Single queue point for multiple machines |
| Load Balancing | Prevents one station from being overloaded while others idle |
| Plan Linking | Plans link to billboard instead of individual stations |

**Billboard Setup**:
1. Purchase and place billboard
2. Link billboard to workstations (right-click menu)
3. Link production plans to billboard
4. Billboard automatically delegates to available workstations

**Example**: 5 woodworking stations linked to 1 billboard ensures all 5 receive evenly distributed work.

---

## 4. Worker System

### 4.1 Worker Types

| Type | Function | Cost | Speed | Special Abilities |
|------|----------|------|-------|-------------------|
| **Operator** | Production work, machine operation | Base salary | Normal | Can perform all basic tasks |
| **Hauler** | Material transport between zones | Higher salary | Faster while carrying | Prioritizes loading/unloading trucks |
| **Technician** | Maintenance and repairs | Higher salary | Fast repairs | Auto-repairs machines |
| **Specialist** | Advanced machine operation | Highest salary | Same as operator | Required for Tier 2+ machines |

### 4.2 Worker Roles Detail

**Operators**:
- Primary production workers
- Retrieve materials from input zones
- Operate workstations
- Deposit finished items in storage zones
- Transport to workstations (NOT between zones)

**Haulers**:
- Move items between zones only
- Load/unload delivery trucks
- Do NOT deliver to workstations
- Faster movement speed when carrying items
- Higher idle time than operators

**Technicians**:
- Automatically repair machines when available
- Much faster repair speed than operators
- Unlocked through R&D progression
- Essential for maintaining large factories

### 4.3 Worker Star Levels

Workers gain experience and star levels through work:

| Level | Stars | Requirements | Benefits |
|-------|-------|--------------|----------|
| Novice | 0 | Starting level | Base efficiency |
| Level 1 | 1 star | Work experience + happiness | +Efficiency |
| Level 2 | 2 stars | Continued work + happiness | ++Efficiency |
| Level 3 | 3 stars | Maximum base level | +++Efficiency |

**Leveling Requirements**:
- Worker must be happy (high mood)
- Worker must be active (consistent work)
- Both conditions must be sustained over time

### 4.4 Specialist System

After reaching certain star levels, workers can specialize:

| Specialist Type | R&D Requirement | Research Cost | Function |
|-----------------|-----------------|---------------|----------|
| Wood Specialist | Wood Production #2 | 2 Points | Operates advanced wood machines |
| Metal Specialist | Metal Production #2 | 3 Points | Operates advanced metal machines |
| Plastic Specialist | Plastic Production #2 | 3 Points | Operates advanced plastic machines |
| Assembly Specialist | Assembly Specialist perk | 2 Points | Advanced assembly operations |

**Important Notes**:
- Specialists can gain up to 3 additional stars (6 total with base)
- Switching specialization resets specialist experience
- Specialists do NOT move faster or interact faster
- Specialists are required for certain high-tier machines
- Specialists still work on non-specialty tasks when their machines are idle

### 4.5 Worker Energy and Breaks

Workers have an energy meter that depletes during work:

**Energy Mechanics**:
- Energy depletes while working
- Low energy reduces work efficiency
- Zero energy causes worker to collapse
- Workers automatically seek break rooms when tired

**Break Room Capacity** (measured in Cups):
- Maximum: 10 Cups per break room
- Each break consumes 1 Cup
- Cups regenerate over time
- Workers recover all energy per break

**Ideal Break Room Setup** (10 Cups):
- 2 Coffee Machines
- 1 Vending Machine
- Total = 10 Cups exactly

### 4.6 Worker Mood System

Mood affects work duration before needing breaks:

| Factor | Effect on Mood |
|--------|----------------|
| Machine Noise | Negative (varies by machine) |
| Large Room | -25 (400-600 tiles) |
| Huge Room | -50 (600+ tiles) |
| Decorations | Positive (diminishing returns) |
| Plants | Positive mood boost |

**Mood Formula**:
```
Room Mood = Decoration Comfort Value - Machine Noise Value - Room Size Penalty
```

**Decoration Diminishing Returns**:
- First item: 100% effectiveness
- Second identical item: 50% effectiveness
- Each additional identical item: Halved again
- Example: Second arcade machine gives only +30 mood instead of +60

### 4.7 Worker Pathfinding

**Movement Behavior**:
- Workers calculate shortest path to destination
- Obstacles (machines, storage) block direct paths
- Pathways should be clear for efficient movement
- Workers store items in nearest legal zone

**Optimization Tips**:
- Place machines against walls
- Leave clear pathways in front of machines
- Centralize storage zones in room middle
- Minimize walking distance between operations

---

## 5. Workstations and Machines

### 5.1 Workstation Categories

| Category | Description | Efficiency |
|----------|-------------|------------|
| **Workbenches** | Versatile, cheap, multiple operations | Low |
| **Light Machinery** | Specialized, single operation type | Medium |
| **Heavy Machinery** | Advanced, requires specialists | High |

### 5.2 Tier System

| Tier | Research Requirement | Efficiency | Specialist Required |
|------|---------------------|------------|---------------------|
| Tier 0 | None (starting) | 100% (base) | No |
| Tier 1 | R&D Rows 1-2 | 100% | No |
| Tier 2 | R&D Row 4 | 200% | Some machines |
| Tier 3 | Factory Focus (last row) | 250% | Yes |

### 5.3 Complete Workstation List

#### Tier 0 - Workbenches (Starting)

| Workstation | Cost | Size | Operations | Notes |
|-------------|------|------|------------|-------|
| Woodworking Station | $2,475 | 6x4 | Sawing, Carving, Sanding, etc. (6 types) | Jack-of-all-trades for wood |
| Metalworking Station | $2,060 | 4x4 | Various metal operations | Versatile metal processing |
| Assembly Station | $2,805 | 7x4 | Assembly, Basic construction | Used in most blueprints |
| Glue Station | $3,120 | - | Adhesive bonding | Common requirement |

#### Tier 1 - Light Machinery

| Workstation | Cost | Size | Operations | Notes |
|-------------|------|------|------------|-------|
| Circlesaw | Lower cost | Compact | Straight wood cuts | More efficient than bandsaw for common cuts |
| Bandsaw | Higher cost | - | Jigsaw/curved cuts | Expensive but needed for complex shapes |
| Lathe (Wood) | $2,895+ | 4x4 | Wood turning | Needed for certain products |
| Metal Lathe | Similar | 4x4 | Metal turning | Less commonly used than wood lathe |
| Milling Machine | $2,895 | - | Wood carving | Cold mechanical precision |
| Small Forge | - | - | Small metal molding | Cannot be replaced by Big Forge for small items |
| Sheet Machine | $3,990 | 5x4 | Sheet metal forming | |
| Form Press | $5,120 | 4x5 | Press forming | |
| Metal Bender | $4,740 | - | Metal bending | |
| Foam Injection Machine | $4,900 | - | Foam products | |
| Injection Press | Compact | Small | Plastic injection molding | Very profitable early game |
| Paint Station | - | - | Painting/coloring | Mostly for plastics |
| Welding Station | - | - | Metal welding | Not very common in blueprints |
| Plasma Cutter | $3,900 | 4x4 | Precision metal cutting | |

#### Tier 2 - Heavy Machinery

| Workstation | Cost | Size | Operations | Notes |
|-------------|------|------|------------|-------|
| Big Forge | Expensive | Large | Large metal molding | Does NOT replace Small Forge |
| Plastic Extrusion Machine | $7,130 | 6x4 | Plastic extrusion | |
| Advanced machinery | Various | Large | Specialist operations | Requires specialists |

#### Tier 3 - Advanced Machinery

- Unlocked through Factory Focus research
- 250% efficiency
- All require specialist operators
- Highest cost and space requirements

### 5.4 Workstation Operations

Common operations across product types:

| Operation | Workstations |
|-----------|--------------|
| Sawing (straight) | Circlesaw, Woodworking Station |
| Sawing (curved) | Bandsaw, Woodworking Station |
| Milling | Milling Machine |
| Turning | Lathe (Wood/Metal) |
| Forging (small) | Small Forge |
| Forging (large) | Big Forge |
| Welding | Welding Station |
| Assembly | Assembly Station |
| Gluing | Glue Station |
| Painting | Paint Station |
| Injection Molding | Injection Press |
| Extrusion | Plastic Extrusion Machine |
| Sheet Forming | Sheet Machine |
| Bending | Metal Bender |
| Cutting (metal) | Plasma Cutter |

### 5.5 Workstation Maintenance

**Durability System**:
- All workstations deteriorate over time
- Efficiency decreases as durability drops
- Severely worn machines operate painfully slow
- Completely broken machines cannot operate

**Repair Status Indicator Colors**:
| Color | Status |
|-------|--------|
| Green (bright) | Excellent condition |
| Green (pale) | Good - consider servicing |
| Yellow | Needs service soon |
| Orange/Red | Critical - service immediately |

**Servicing Process**:
1. Open workstation context menu
2. Click service button
3. Available operator/technician performs repair
4. Machine returns to full efficiency

**Technician Advantage**:
- Auto-repair when not busy
- Much faster repair speed
- Higher priority for manual repairs

### 5.6 Overdrive Mode

Workstations can be set to **Overdrive**:

| Benefit | Penalty |
|---------|---------|
| +50% efficiency | 2x wear rate |
| Faster production | More frequent repairs needed |

**Best Used For**:
- Rushing critical orders
- Bottleneck machines
- Short-term production boosts

### 5.7 Machine Explosions

**Catastrophic Failure**:
- Can occur from severe neglect
- Random chance increases with low durability
- Results in blackened, smoking rubble
- Machine must be fully repaired before use
- Potential production delays

---

## 6. Blueprint and Recipe System

### 6.1 Product Categories

The Market organizes products into three tiers:

| Category | Unlock Requirement | Complexity | Attribute Demands |
|----------|-------------------|------------|-------------------|
| **Basic** | Starting | Simple, few operations | Low |
| **Medium** | Bronze Milestone | More operations, better workstations | Medium |
| **Advanced** | Silver Milestone | Multiple sub-assemblies | High |

### 6.2 Product Examples

#### Basic Products
- Back Scratcher
- Barbara Doll
- Bar Stool
- Barbells
- Dala Horse
- Doggo House
- Double-Oh-Quack (Rubber Duck)
- Fancy Hat
- Pitchfork
- Round Table
- Sandbox Kit
- Scarecrow
- Shovel
- Simple Chair
- Skateboard
- Small Shelf
- Stool
- Toy Food

#### Medium Products
- Adjustable Table
- Barbecue Wagon
- Bedside Table
- Game Console
- Huge Teddybear
- Large Shelf
- Orcish Multitool
- Skis
- Small Drawer
- Snowboard
- Square Table

#### Advanced Products
- Complex electronics
- Drones
- Electric guitars
- Scooters
- Bicycles
- Robots
- Vehicles

**Total Product Types**: 50+ unique products

### 6.3 Blueprint Components

Each blueprint defines:

1. **Raw Materials Required**
   - Wood planks
   - Metal sheets
   - Plastic pellets
   - Leather pieces
   - Foam
   - Paint

2. **Component Parts**
   - Individual pieces that make up the product
   - Each part may have material options

3. **Operations Sequence**
   - Step-by-step manufacturing process
   - Each step requires specific workstation type

4. **Assembly Requirements**
   - How components combine
   - Sub-assemblies for complex products

### 6.4 Material Selection

For each component, players choose:

| Material Type | Cost | Durability | Style |
|---------------|------|------------|-------|
| Wood | Cheapest | Medium | Higher |
| Metal | More expensive | Highest | Lower |
| Plastic | Variable | Variable | Variable |
| Leather | Expensive | Medium | High |

**Material Quality Tiers**:
- Basic materials (starting)
- Standard materials
- Premium materials (R&D unlock)

### 6.5 Plan Management

**Creating Plans**:
1. Select product from market
2. Choose material for each component
3. Assign workstations/billboards to operations
4. Save plan with descriptive name

**Plan Naming Convention** (Community Standard):
```
[Product Name] - [Comfort][Durability][Style]
Example: "Chair - C4 D24 S17"
```

**Plan Duplication**:
- Duplicate existing plans
- Modify one or two operations
- Quickly create variants for different requirements

---

## 7. Order and Contract System

### 7.1 Market System

The market serves as the primary source of income:

**Market Structure**:
- Three tabs: Basic, Medium, Advanced
- Each product shows current demand and price
- Historical price/demand graphs available
- Demand fluctuates over market cycles

**Demand Levels**:
| Range | Classification |
|-------|----------------|
| 15-25 | Low demand |
| 26-35 | Moderate demand |
| 36+ | High demand |

### 7.2 Market Saturation

**Saturation Mechanics**:
- Each product type has maximum market capacity
- Producing beyond capacity saturates the market
- Saturated products cannot be sold until cycle reset
- Market cycle resets demand periodically

**Saturation Strategy**:
- Higher demand = longer to saturate
- Low demand items saturate quickly
- Monitor demand before committing to large production runs

### 7.3 Contract System

Contracts are **special orders from specific clients**:

| Aspect | Description |
|--------|-------------|
| Payment | Generally higher than market |
| Deadline | Must complete within time limit |
| Specifications | Specific attribute requirements |
| Reputation | Affects client relationship |

**Contract Flow**:
1. Client offers contract
2. Review specifications and deadline
3. Accept or decline
4. Create/modify plan to meet specs
5. Produce required quantity
6. Deliver before deadline
7. Receive payment and reputation

### 7.4 Client System

**Named Clients**:
- Crazy Steve Enterprises
- IncoInc
- Mitzurella
- Others

**Client Characteristics**:
| Client | Reputation Gain | Notes |
|--------|-----------------|-------|
| Standard Clients | 3-4 orders per level | Normal progression |
| IncoInc | 7-10 orders per level | Demanding, slower reputation |
| Mitzurella | Normal | Has ultimate challenge at Level 5 |

### 7.5 Reputation Levels

Each client has independent reputation:

| Level | Benefits |
|-------|----------|
| Level 1 | Basic contracts |
| Level 2 | Better paying contracts |
| Level 3 | Special challenges unlocked |
| Level 4 | Premium contracts |
| Level 5 | Ultimate challenge available |
| Max | No further reputation gains |

**Building Reputation**:
- Complete contracts successfully
- Meet or exceed deadlines
- Match product specifications
- Higher complexity = more reputation

### 7.6 Challenges

**Challenge Types**:
- Regular challenges: Well-paid, time-limited
- Special challenges: Unlocked at higher reputation
- Ultimate challenges: Final challenge at max reputation (gold star)

**Challenge Rewards**:
- Higher payment than standard contracts
- Significant reputation boost
- Unlock new market products
- Progress toward milestones

### 7.7 Deadline Management

**Deadline Penalties**:
- Missing deadline: Lose potential reputation gain
- No financial penalty beyond lost payment
- Contract marked as failed
- Affects milestone progress

**Deadline Tips**:
- Check estimated completion time
- Account for material delivery time
- Consider worker availability
- Factor in potential machine breakdowns

---

## 8. Factory Layout and Space Management

### 8.1 Room System

The factory is divided into rooms:

**Starting Configuration**: 3 separate rooms

**Room Functions**:
- Production rooms (workstations)
- Storage rooms (materials)
- Break rooms (worker rest)
- Mixed-use rooms

### 8.2 Room Size Penalties

| Room Size (tiles) | Mood Penalty |
|-------------------|--------------|
| Up to 400 | None |
| 401-600 | -25 (Large Room) |
| 600+ | -50 (Huge Room) |

**Mitigation**:
- Add dividing walls within large rooms
- Increase decorations
- Accept the penalty for mega-factories

### 8.3 Recommended Room Sizes

| Purpose | Ideal Size |
|---------|------------|
| Small production | 110 m² |
| Medium production | 163 m² |
| Large production | Up to 400 tiles |
| Break room | Compact (fits equipment) |

### 8.4 Zone Types

| Zone Type | Color | Function |
|-----------|-------|----------|
| General Storage | Default | Store any items |
| Workstation Input | Blue | Store input materials for linked workstations |
| Export Zone | - | Finished products awaiting delivery |

### 8.5 Zone Strategy

**Recommended Setup Per Production Room**:
1. At least one General Storage zone
2. At least one Workstation Input zone
3. Link Input zones to all machines/billboards in room

**Loading Dock Area**:
- General Storage directly at loading dock (incoming materials)
- Large Export Zone near loading dock (outgoing products)
- Export zone should be large for bigger products later

### 8.6 Storage Optimization

**Storage Rules**:
- Workers store items in nearest legal zone
- Set up zones to guide item flow
- Use shelves in storage zones
- Blue zones (input) clearly define legal storage

**Flow Pattern**:
```
Loading Dock → General Storage → Input Zones → Workstations → General Storage → Export Zone → Loading Dock
```

### 8.7 Layout Best Practices

**Machine Placement**:
- Position machines against walls
- Leave pathway in front of machines
- Storage zones in room center
- Minimize worker walking distance

**Three-Room Strategy**:
1. Wood production room
2. Metal/Plastic production room
3. Assembly room
- Centralized storage linked to all billboards

**Pathfinding Optimization**:
- Avoid obstacles in pathways
- Keep clear corridors between zones
- Link storage to all relevant workstations

### 8.8 Expansion

**Unlocking New Plots**:
- Complete Bronze Mastery Challenges
- Research "Plots" perk in R&D
- Purchase additional factory space

**Construction Tools**:
- Required R&D perk for building new rooms
- Allows remodeling existing workshop

---

## 9. Resource Procurement

### 9.1 Material Types

| Category | Materials |
|----------|-----------|
| Wood | Planks, boards, specialty woods |
| Metal | Sheets, bars, specialty metals |
| Plastic | Pellets, specialty plastics |
| Leather | Hides, treated leather |
| Other | Foam, paint, components |

### 9.2 Material Quality Tiers

| Tier | Availability | Cost | Attributes |
|------|--------------|------|------------|
| Basic | Starting | Cheapest | Lowest stats |
| Standard | Early R&D | Medium | Medium stats |
| Premium | R&D unlock | Expensive | Highest stats |

### 9.3 Purchasing System

**Order Process**:
1. Open material shop
2. Select material type and quantity
3. Confirm purchase
4. Materials delivered to loading dock
5. Haulers/operators move to storage

**Material Delivery**:
- Delivery truck arrives at loading dock
- Workers unload to nearest storage
- Set up storage near dock for efficiency

### 9.4 Alternate Crafting Options

Some blueprints allow:
- Making parts from different materials
- Example: Metal part instead of plastic
- Example: Wood knobs vs metal handles
- Each choice affects final attributes and cost

### 9.5 Component vs Raw Material

**Buy vs Make Decision**:
- Some components can be purchased pre-made
- Alternatively, manufacture from raw materials
- Cost vs time trade-off
- Factory capacity consideration

---

## 10. Research and Tech Tree

### 10.1 R&D System Overview

Research and Development unlocks:
- New workstations
- Better materials
- Worker abilities
- Factory upgrades
- Specialist options

### 10.2 Research Points

**Earning Points**:
- Factory level-ups (XP from deliveries)
- Milestone completions
- Points per level increase as you progress

**Spending Points**:
- Each perk has a point cost
- Some perks have prerequisites
- Cannot respec spent points

### 10.3 Research Tree Structure

**Main R&D Tree**:
- Row 1-2: Tier 1 machines (100% efficiency)
- Row 3: Additional upgrades
- Row 4: Tier 2 machines (200% efficiency)
- Row 5+: Advanced perks

**Factory Focus** (Unlocks after Silver Milestone):
- Separate upgrade tree
- Both trees can be researched simultaneously
- Final row: Tier 3 machines (250% efficiency)
- Not locked to initial choice - both eventually unlockable

### 10.4 Key Research Categories

| Category | Unlocks |
|----------|---------|
| Wood Production | Wood machines, Wood Specialist |
| Metal Production | Metal machines, Metal Specialist |
| Plastic Production | Plastic machines, Plastic Specialist |
| Assembly | Assembly upgrades, Assembly Specialist |
| Construction Tools | Room building, remodeling |
| Plots | Additional factory space |
| Premium Materials | Higher quality materials |

### 10.5 Specialist Research Costs

| Specialist | Research Perk | Point Cost |
|------------|---------------|------------|
| Wood Specialist | Wood Production #2 | 2 Points |
| Metal Specialist | Metal Production #2 | 3 Points |
| Plastic Specialist | Plastic Production #2 | 3 Points |
| Assembly Specialist | Assembly Specialist | 2 Points |

### 10.6 Research Progression

| Milestone | Unlocks |
|-----------|---------|
| Game Start | Basic R&D tree |
| Bronze Milestone | New R&D perks, Medium products |
| Silver Milestone | Factory Focus tree, Advanced products, Wood Specialist |
| Gold Milestone | Additional R&D perks, More specialists |

---

## 11. Progression Systems

### 11.1 Factory Level and XP

**Earning XP**:
- Completing market deliveries
- Finishing contracts
- Completing challenges
- All market activities generate XP

**XP Value by Product**:
| Tier | XP per Unit |
|------|-------------|
| Basic | Lowest |
| Medium | Medium |
| Advanced | Highest |

**Level Requirements**:
- Level 40 (Tycoon achievement): ~6 million XP
- Each level requires progressively more XP

### 11.2 Factory Points

**Earning Factory Points**:
- Each new factory level
- Completing milestones
- Points per level increase with progression

**Spending Factory Points**:
- R&D perks
- Factory Focus perks
- Cannot be reset

### 11.3 Milestone System

#### Bronze Milestone

| Challenge | Requirement |
|-----------|-------------|
| Deliveries | Deliver 100+ products |
| Client Challenges | Complete 3+ client challenges |
| Billboard | Execute plan with linked billboard |
| Factory Level | Reach Level 5 |

**Rewards**:
- Unlocks Medium products tab
- New R&D perks
- Plot expansion option

#### Silver Milestone

| Challenge | Requirement |
|-----------|-------------|
| Medium Products | Deliver 50 medium products |
| Earnings | Earn $75,000 from deliveries |
| Client Reputation | Reach max level with one client |
| Net Worth | Achieve $150,000 net worth |

**Rewards**:
- Unlocks Advanced products tab
- Factory Focus tree
- Additional R&D perks
- Wood Specialist available

#### Gold Milestone

| Challenge | Requirement |
|-----------|-------------|
| Advanced Products | Deliver 50 advanced products |
| Specialists | Have 10 specialists |
| Max Reputation | Max rep with one client |
| Market Saturation | Simultaneously saturate two medium/advanced products |

**Rewards**:
- New R&D perks
- Additional specialists
- End-game content

#### Champion Milestone

| Challenge | Requirement |
|-----------|-------------|
| Total Deliveries | 800 products of any type |
| Ultimate Challenges | Complete ultimate challenge for 2+ clients |
| Nemesis Victory | Higher net worth and revenue than Nemesis Inc. |

### 11.4 Client Progression

Each client has independent progression:

1. Initial contracts available
2. Complete contracts → gain reputation
3. Higher reputation → better contracts
4. Level 3-4 → Special challenges unlock
5. Level 5 → Ultimate challenge available
6. Post-ultimate → Cash/XP only (no more reputation)

---

## 12. Unique Mechanics

### 12.1 Nemesis System

**Nemesis Inc.** is a rival corporation:

**Competition Aspects**:
- Tracks your net worth vs theirs
- Revenue comparison
- Market competition
- Milestone requirement (Champion)

**Nemesis Disruptions** (Base Game):
- Market manipulation
- Spy dispatches
- Sabotage attempts
- Find and remove enemy scouts

### 12.2 Factory Events

**Positive Events**:
- Market opportunities
- Client bonuses
- Special orders

**Negative Events**:
- Machine breakdowns
- Fungal/mold infections (mini-game to find and remove)
- Nemesis sabotage
- Market fluctuations

### 12.3 The Evil DLC

Expansion that reverses the narrative:

**Features**:
- Play as villain apprentice to Bladh
- Sabotage competitors
- Espionage systems
- Find competitor weaknesses
- New products and companies
- Additional skills and tricks

### 12.4 Day/Night Cycle

**Environmental Feature**:
- Full day/night simulation
- Lighting changes on diorama
- Aesthetic only (no gameplay impact on production)

### 12.5 Diorama Aesthetic Details

**Visual Elements**:
- Workshop table setting
- Coffee mugs around edges
- Blueprints scattered on table
- Drafting board atmosphere
- Miniature world feel

**Worker Personality**:
- Cute, gnome-like appearance
- Rounded cartoon features
- Individual visual variations
- Collapse animation when exhausted

### 12.6 Priority System

**Production Priorities**:
- Contracts can be prioritized
- Higher priority = worked on first
- Affects queue ordering
- Critical for meeting deadlines

**Zone Priorities** (reported issues):
- Zone priority settings available
- Some pathfinding quirks with priority
- May need manual intervention

---

## Appendix: Quick Reference Tables

### Workstation Cost Summary

| Workstation | Cost | Tier | Size |
|-------------|------|------|------|
| Metalworking Station | $2,060 | 0 | 4x4 |
| Woodworking Station | $2,475 | 0 | 6x4 |
| Assembly Station | $2,805 | 0 | 7x4 |
| Milling Machine | $2,895 | 1 | - |
| Glue Station | $3,120 | 0 | - |
| Plasma Cutter | $3,900 | 1 | 4x4 |
| Sheet Machine | $3,990 | 1 | 5x4 |
| Metal Bender | $4,740 | 1 | - |
| Foam Injection | $4,900 | 1 | - |
| Form Press | $5,120 | 1 | 4x5 |
| Plastic Extrusion | $7,130 | 2 | 6x4 |

### Room Size Reference

| Size Category | Tiles | Mood Penalty |
|---------------|-------|--------------|
| Small/Medium | 0-400 | None |
| Large | 401-600 | -25 |
| Huge | 600+ | -50 |

### Worker Costs

| Type | Relative Salary |
|------|-----------------|
| Operator | Base |
| Hauler | Higher |
| Technician | Higher |
| Specialist | Highest |

### Machine Efficiency by Tier

| Tier | Efficiency | Specialist Required |
|------|------------|---------------------|
| 0 | 100% (base workbench) | No |
| 1 | 100% (specialized) | No |
| 2 | 200% | Some machines |
| 3 | 250% | Yes |

### Market Demand Classification

| Demand Range | Classification | Saturation Speed |
|--------------|----------------|------------------|
| 15-25 | Low | Fast |
| 26-35 | Moderate | Medium |
| 36+ | High | Slow |

### Milestone Unlock Summary

| Milestone | Products Unlocked | Major Unlocks |
|-----------|-------------------|---------------|
| Start | Basic | Core R&D |
| Bronze | Medium | Plots, Medium R&D |
| Silver | Advanced | Factory Focus, Specialists |
| Gold | - | End-game R&D |
| Champion | - | Victory condition |

---

## Sources

Research compiled from:
- [Little Big Workshop Wiki (Fandom)](https://littlebigworkshop.fandom.com/wiki/Little_Big_Workshop)
- [Steam Community Guide: Tips and Game Mechanics](https://steamcommunity.com/sharedfiles/filedetails/?id=2199711182)
- [Steam Community Guide: Workshop 101 - Path to Profit](https://steamcommunity.com/sharedfiles/filedetails/?id=2210267082)
- [The Lost Noob - Little Big Workshop Guides](https://www.lostnoob.com/little-big-workshop/)
- [PSN Profiles Trophy Guide](https://psnprofiles.com/guide/14089-little-big-workshop-trophy-guide)
- [PlayStation Trophies Guide](https://www.playstationtrophies.org/game/little-big-workshop/guide/)
- [Steam Community Worker Energy Guide](https://steamcommunity.com/sharedfiles/filedetails/?id=1893251174)
- [HandyGames Official Page](https://www.handy-games.com/en/games/little-big-workshop/)
- [Wikipedia - Little Big Workshop](https://en.wikipedia.org/wiki/Little_Big_Workshop)

---

*Document compiled for game design research purposes. Data sourced from official wiki, community guides, and player resources as of 2024.*
