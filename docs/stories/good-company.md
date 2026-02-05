# Good Company: Comprehensive Game Design Research Document

## Table of Contents
1. [Game Overview](#1-game-overview)
2. [Initial Conditions](#2-initial-conditions)
3. [Core Mechanics](#3-core-mechanics)
4. [Research and Technology](#4-research-and-technology)
5. [Worker/Employee System](#5-workeremployee-system)
6. [Product Design System](#6-product-design-system)
7. [Machines and Workstations](#7-machines-and-workstations)
8. [Logistics System](#8-logistics-system)
9. [Business Simulation](#9-business-simulation)
10. [Map and Building Layout](#10-map-and-building-layout)
11. [Game Modes and Progression](#11-game-modes-and-progression)
12. [Unique Mechanics](#12-unique-mechanics)

---

## 1. Game Overview

### Basic Information

| Attribute | Value |
|-----------|-------|
| **Developer** | Chasing Carrots (German indie studio) |
| **Release Date** | March 31, 2020 (Early Access), June 21, 2022 (1.0) |
| **Platforms** | PC (Steam, GOG, Epic Games Store) |
| **Genre** | Factory Management / Business Tycoon |
| **Price** | $24.99 USD |

### Premise

Players take on the role of a local entrepreneur saving their home county's economy by building a tech manufacturing empire. Starting from a humble garage, players progress through crafting simple calculators to eventually producing advanced robots and drones.

### Key Features

- Product design system with module combinations
- Market dynamics with product lifecycles
- Employee management with skills and happiness
- Transition from manual labor to automation
- Campaign with story and freeplay sandbox

---

## 2. Initial Conditions

### Starting Scenario

Players begin by taking over **Charlie's Circuits** - their father's old company.

| Element | Starting Value |
|---------|----------------|
| Workspace | Small garage |
| Zones | Pre-established Incoming and Outgoing zones |
| Capital | Varies by mission/mode |
| First Product | Calculators |
| Advisor | Pam from Johnson Invest |

### Tutorial Structure

The first four Campaign levels serve as tutorials introducing:
1. Basic crafting and production mechanics
2. Logistics and material flow
3. Employee management
4. Market and sales concepts

**Kerry Goldfield** introduces players to base business concepts including crafting, assembly, and factory logistics organization.

---

## 3. Core Mechanics

### 3.1 Production Hierarchy

Good Company uses a four-tier production hierarchy:

```
Materials → Components → Modules → Products
```

| Tier | Description | Created At | Example |
|------|-------------|------------|---------|
| Materials | Raw inputs purchased from suppliers | Incoming Zone (delivered) | Metal, Plastic, Silicon |
| Components | Basic crafted items from materials | Tinker Tables | Circuit boards, Gearwheels, Coils |
| Modules | Complex assemblies from components | Crafting Tables/Machines | Batteries, Displays, Speakers |
| Products | Final sellable goods | Assembly Tables | Calculators, Phones, Robots |

### 3.2 Zone System

| Zone Type | Function | Notes |
|-----------|----------|-------|
| Incoming Zone | Receives purchased materials via daily courier deliveries | Must set purchase orders |
| Outgoing Zone | Ships products, modules, and components for weekly sales | Bandwidth upgradeable |
| Work Zones | Player-defined areas for organizing production | Used for statistics filtering |

### 3.3 Time and Sales Cycles

| Cycle | Duration | Purpose |
|-------|----------|---------|
| Daily | Game time cycle | Material deliveries arrive on Courier Pallets |
| Weekly | 7 game days | Products are sold, revenue is collected |

### 3.4 Example Production Chain (Cassette Player)

1. **Order materials**: Metal and Plastic from Incoming Zone
2. **Craft components**: Metal + Plastic → Coils
3. **Craft modules**: Coils + other components → Speaker Module, Case Module, Battery Module, Display Module
4. **Assemble product**: All modules combined at Assembly Table → Cassette Player
5. **Sell product**: Transfer to Outgoing Zone for weekly shipment

---

## 4. Research and Technology

### 4.1 Research Point Generation

1. **Analysis**: Connect a shelf containing modules to an Analysis Desk
2. **Consumption**: Modules are consumed during analysis to generate research data
3. **Application**: Use research points at a Research Table to unlock new technologies

### 4.2 Research Categories

| Category | Focus | Example Technologies |
|----------|-------|---------------------|
| Audiovisual | Display and sound technology | Brightness sensors → High resolution cameras |
| Motion | Motors and movement | Small DC motors → High speed motors |
| Power Supply | Batteries and energy | Single cell batteries → Advanced power systems |
| Electronics | Circuit technology | Basic circuits → Advanced processors |

### 4.3 Business Development

Separate from product research, provides company-wide upgrades:

| Type | Examples |
|------|----------|
| New Equipment | Specialized crafting tables, advanced machines |
| Efficiency Upgrades | Increased machine speed, improved product handling |
| Market Expansion | Access to new markets and product categories |

### 4.4 Freeplay Milestones

- Milestone 10 requires selling 10,000 products within a year
- Unlocks two additional boards for market expansion and company-wide buffs

### 4.5 Research Acceleration

- More researchers = faster research completion
- More analysts = more research points generated
- Research priority competes with production resources

---

## 5. Worker/Employee System

### 5.1 Employee Roles

| Role | Function | Workstation |
|------|----------|-------------|
| Developer | Creates basic components from materials | Tinker Tables |
| Lead Developer | Combines components into modules | Crafting Tables |
| Assembler | Builds final products from modules | Assembly Tables |
| Logistics Worker | Transports items between stations | Courier Routes |
| Analyst | Consumes modules to generate research data | Analysis Desks |
| Researcher | Applies research points to unlock technology | Research Tables |
| Manager | Organizes and directs other employees | Management stations |

### 5.2 Skills and Happiness

The "Skills & Happiness" update introduced:

| System | Effect |
|--------|--------|
| Employee Skills | Workers can be trained to improve performance |
| Happiness System | Morale affects productivity |
| Bonuses and Promotions | Tools for employee retention |

### 5.3 Staffing Considerations

**Common Issues**:
- Too few logistics employees = production bottlenecks
- Shelves remain full when logistics workers cannot empty them fast enough

**Best Practices**:
- Maintain adequate logistics staff ratio to production workers
- Set priority correctly: processing priority > sales priority
- This ensures intermediate products flow to next production stage before being sold

### 5.4 Automation Transition

As the company grows, players can:
1. Design and manufacture robots
2. Replace human workers with automated machines
3. Shift focus from direct production to process optimization and robot design

---

## 6. Product Design System

### 6.1 Blueprint Designer

Products are created in the Blueprint Designer by combining modules:

| Element | Description |
|---------|-------------|
| Module Slots | Products have slots for different module types |
| Features | Modules add features that affect market appeal |
| Drawbacks | Some module combinations create negative effects |
| Quality | Module quality affects overall product quality |

### 6.2 Features and Drawbacks

| Concept | Impact |
|---------|--------|
| Positive Features | Increase market appeal and sale price |
| Drawbacks | Decrease market value |
| Feature Requirements | Markets demand specific features |
| Overengineering | Too many components = expensive production, longer manufacture time |

### 6.3 Market Fit

- **Outdated blueprints** = market value too low to profit
- **Overbuilt products** = production costs exceed sale price
- **Balanced design** = optimal profit margin

### 6.4 Quality System

| Factor | Effect |
|--------|--------|
| Module Quality | Higher quality modules = higher product value |
| Research Level | Advanced research unlocks higher quality variants |
| Production Speed | Quality vs. speed tradeoffs |

---

## 7. Machines and Workstations

### 7.1 Workstation Categories

| Category | Purpose | Examples |
|----------|---------|----------|
| Crafting Workplaces | Convert materials to components/modules | Tinker Tables, Workbenches |
| Assembly Workplaces | Build final products | Assembly Tables |
| Analysis Workplaces | Generate research data | Analysis Desks |
| Research Workplaces | Apply research points | Research Tables |
| Design Workplaces | Create product blueprints | Design Tables |
| Storage | Hold items between processes | Shelves, Pallets |

### 7.2 Known Workstation Types

| Workstation | Function | Notes |
|-------------|----------|-------|
| Tinker Table | Craft basic modules (batteries, circuits, cases, displays) | Starting workstation |
| Workbench | Upgraded crafting table | More efficient |
| Chemistry Table | Specialized chemical component crafting | Unlocked through research |
| Design Table | Create new product blueprints | Required for product design |
| Assembly Table | Combine modules into products | Multiple upgrade tiers |
| Analysis Desk | Consume modules for research data | Links to module storage |
| Research Table | Apply research to unlock technologies | Uses research points |

### 7.3 Automated Machines

| Machine Type | Function | Cost Example |
|--------------|----------|--------------|
| Pick and Place Machine | Automated circuit assembly | ~40,000G |
| Automated Crafting Machine | Unmanned component production | Varies |
| Conveyor Systems | Automated item transport | Per-tile cost |

### 7.4 Machine Unlocks

Machines are unlocked through:
- Business Development progression
- Research investment
- Campaign milestone completion

---

## 8. Logistics System

### 8.1 Logistics Modes

| Mode | Description | Best For |
|------|-------------|----------|
| Automatic Logistics | Workers autonomously decide item routing | Simple setups |
| Manual Logistics | Player defines exact routing rules | Complex optimization |

### 8.2 Courier System

| Setting | Function |
|---------|----------|
| Courier Routes | Defined paths for item transport |
| Pallet Percentages | Set input/output ratios per pallet |
| Priority Settings | Control which routes take precedence |

**Key Mechanic**: Each pallet can only have one input percentage and one output percentage.

### 8.3 Conveyor Belt System

| Component | Function |
|-----------|----------|
| Conveyor Belts | Transport items automatically |
| Splitters | Divide item flow to multiple destinations |
| Mergers | Combine multiple sources into single line |

**Conveyor Logic**:
- Output rules based on linked shelf/central item ruleset
- Belt checks fill amounts before sending items
- Designed to minimize employee walking distance

### 8.4 Logistics Setup Process

1. Enter **Logistics Mode**
2. Drag material icon from Incoming Zone to workstation
3. Drag workstation output to shelf or Outgoing Zone
4. **Direction matters** - connections are one-way
5. Set priorities (processing > sales to avoid premature selling)

---

## 9. Business Simulation

### 9.1 Revenue Streams

| Source | Timing | Notes |
|--------|--------|-------|
| Product Sales | Weekly | Primary income source |
| Module Sales | Weekly | Sell intermediate products for supplementary income |
| Component Sales | Weekly | Lower margin but useful for surplus |

### 9.2 Financial Management

**Cost Categories**:
- Employee wages
- Material purchases
- Building/expansion costs
- Machine purchases
- Research investment

**Best Practices**:
- Maintain positive cash flow before major investments
- Keep "tens of thousands" as financial cushion
- New production lines take time to become profitable

### 9.3 Market Phases

Products follow a lifecycle:

| Phase | Characteristics |
|-------|-----------------|
| Introduction | New product category, early adopter market |
| Growth | Expanding demand, increasing sales potential |
| Saturation | Peak market penetration, maximum competition |
| Decline | Shrinking demand, reduced profit margins |

### 9.4 Market Dynamics

- Markets advance over time independent of player
- Product expectations increase as market matures
- Cannot sell basic calculators forever - must innovate
- Research new technology and design new blueprints to match market demands

### 9.5 Pricing Factors

| Factor | Impact on Price |
|--------|-----------------|
| Product Quality | Higher quality = higher price |
| Feature Set | More demanded features = higher value |
| Market Phase | Mature markets have price pressure |
| Competition | Market saturation affects pricing |

---

## 10. Map and Building Layout

### 10.1 Building Progression

| Stage | Description |
|-------|-------------|
| Garage | Initial manufacturing headquarters |
| Small Factory | First expansion |
| Large Facilities | Late-game production complexes |
| Multi-Building | Spread operations across multiple structures |

### 10.2 Zone Management

Each worksite includes:

| Zone | Notes |
|------|-------|
| Pre-established Incoming Zone | Cannot be removed, can be upgraded |
| Pre-established Outgoing Zone | Cannot be removed, can be upgraded |
| Player-defined Work Zones | Created for organization and statistics |

### 10.3 Expansion System

**Campaign**: Buildings are provided by mission structure

**Freeplay**: Unlock Business Expansions through:
- Milestone achievement (e.g., Milestone 10 = 10,000 products/year)
- Business Development investment

### 10.4 Layout Considerations

- Workstations need connection space for logistics
- Production chains can be difficult to link when desks/shelves are too close
- Zone-based statistics help optimize each production area
- Decoration items available for aesthetic customization

---

## 11. Game Modes and Progression

### 11.1 Game Modes Overview

| Mode | Description |
|------|-------------|
| Tutorial | First 4 campaign levels teach mechanics |
| Campaign | Story-driven missions with objectives |
| Challenges | Bonus levels with specific constraints |
| Freeplay | Endless sandbox mode |
| Multiplayer | Co-op Freeplay (up to 4 players) |

### 11.2 Campaign Structure

**Story**: Local entrepreneur saves county economy by reviving Charlie's Circuits

**Level Structure**:
- 8-13 milestones per level
- 3 level goals per mission
- 1 trophy awarded per level goal achieved

**Progression**:
```
Complete Milestones → Reach Level Goal → Earn Trophy → Unlock Next Level
```

**World Map**:
- Visual progression through county locations
- Meet different characters at each location
- Mini-quests in the form of challenges
- Requires minimum 1 trophy to advance

### 11.3 Milestone and Trophy System

| Element | Description |
|---------|-------------|
| Milestones | Specific objectives within a level |
| Level Goals | Achieved after completing X milestones |
| Trophies | Awarded when level goal is reached (up to 3 per level) |
| Rating | Based on trophies earned |

### 11.4 Freeplay Mode

**Key Differences from Campaign**:
- Not a true sandbox - requires active progression
- All game concepts active from start (or unlocked by conditions)
- No storyline restrictions
- Business Expansions primarily available here
- Customizable settings

**Multiplayer Freeplay**:
- Up to 4 players in same company
- Shared resources and production
- Cooperative gameplay only (no competition)

### 11.5 Challenge Mode

- Unlocked by progressing through Campaign World Map
- Self-contained scenarios with specific objectives
- Tests mastery of specific mechanics

---

## 12. Unique Mechanics

### 12.1 What Distinguishes Good Company

| Feature | Comparison to Other Factory Games |
|---------|----------------------------------|
| Product Design | Unlike Factorio/Satisfactory, players design what they manufacture |
| Market Dynamics | Products have lifecycles; market expectations evolve |
| Business Management | Tycoon elements (finances, expansion, investment) |
| Employee Focus | Human workers central to production (can be automated later) |
| Blueprint System | Creative product design with feature/drawback tradeoffs |

### 12.2 Product Design as Core Loop

The central gameplay loop:

1. **Research** new technologies
2. **Design** products to match market demands
3. **Optimize** production chains for efficiency
4. **Sell** products to generate revenue
5. **Reinvest** profits in research and expansion
6. **Repeat** as markets evolve

### 12.3 Human to Automation Transition

Unique progression arc:
- **Early game**: Manual labor with human workers
- **Mid game**: Optimization and efficiency improvements
- **Late game**: Design and manufacture robots to replace humans
- **End game**: Focus shifts to process engineering and robot design

### 12.4 Market Evolution Pressure

Unlike static factory games:
- Markets advance independently of player
- Product expectations increase over time
- Forces constant innovation
- Cannot rely on single product indefinitely

### 12.5 Zone-Based Production Organization

Rather than free-form factory building:
- Work zones provide organizational structure
- Statistics can be filtered by zone
- Helps manage complexity as factory grows
- Different from tile-based freedom of Factorio

---

## Appendix: Product Categories

### Product Types (Progression Order)

| Product | Complexity | Notes |
|---------|------------|-------|
| Calculators | Entry Level | Tutorial product |
| Pocket Computers | Early | First progression |
| Flip Phones | Early-Mid | Mobile market |
| Audio Players | Mid | Entertainment category |
| Handheld Gaming Devices | Mid | Entertainment category |
| Computers | Advanced | Complex production chains |
| 3D Printers | Advanced | Manufacturing products |
| Cleaner Bot | Late Game | Automation products |
| Delivery Drone | Late Game | Automation products |
| Robots | End Game | Ultimate manufacturing goal |

### Module Examples

| Module Type | Basic Version | Advanced Version |
|-------------|---------------|------------------|
| Battery | Single Cell Battery | Advanced Power Cell |
| Display | LED Array | High Resolution Screen |
| Frame | Simple Frame | Reinforced Frame |
| Circuit | Basic Circuit | Advanced Processor |
| Sensor | Brightness Sensor | High Resolution Camera |
| Motor | Small DC Motor | High Speed Motor |
| Speaker | Monophone Speaker | Stereo Speaker |

---

*Document compiled for game design research purposes. Data sourced from Good Company Wiki, Steam Community guides, and official documentation.*
