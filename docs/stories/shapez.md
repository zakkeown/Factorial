# Shapez - Game Design Research Document

**Game:** Shapez (also known as shapez.io)
**Developer:** Tobias Springer (tobspr)
**Release Date:** July 10, 2023 (Steam)
**Genre:** 2D Abstract Minimalist Factory Automation
**Price:** $1.99 (base $9.99)
**Steam Rating:** Overwhelmingly Positive (96/100 from 14,601 reviews)

---

## Table of Contents

1. [Game Overview](#game-overview)
2. [Initial Conditions](#initial-conditions)
3. [Core Mechanics](#core-mechanics)
   - [Shape Types](#shape-types)
   - [Color System](#color-system)
   - [Buildings and Machines](#buildings-and-machines)
   - [Belt System](#belt-system)
4. [Upgrade System](#upgrade-system)
5. [Hub and Delivery Mechanics](#hub-and-delivery-mechanics)
6. [Level Progression](#level-progression)
7. [Production Chains](#production-chains)
8. [World and Map](#world-and-map)
9. [Wires and Logic Systems](#wires-and-logic-systems)
10. [Endgame Content](#endgame-content)
11. [Achievements](#achievements)
12. [Sources](#sources)

---

## Game Overview

Shapez is a factory-building game focused on automating the creation and processing of increasingly complex geometric shapes. Unlike traditional factory games, Shapez features:

- **Infinite procedurally generated map** with unlimited resources
- **No combat or survival elements** - pure factory optimization
- **Abstract minimalist aesthetic** with colored geometric shapes
- **Infinite progression** through randomly generated shape requirements
- **Logic/wires system** for advanced automation (Make Anything Machines)

The core gameplay loop involves extracting shapes from resource patches, processing them through various machines (cutting, rotating, stacking, painting), and delivering them to a central Hub to progress through levels and unlock upgrades.

---

## Initial Conditions

### Starting Setup

- **Hub:** The central building is placed at the map center (coordinates 0,0)
- **Starting Resources:** Multiple ore patches spawn near the Hub containing:
  - Circles (most common)
  - Rectangles/Squares
  - Stars (only naturally generated near the Hub)
  - Half Windmills (windmill corners - full windmills are never naturally generated)
- **Starting Buildings:** Conveyor Belt and Extractor are available immediately
- **Starting Colors:** The three primary colors (Red, Green, Blue) spawn in patches near the Hub

### Tutorial Structure (Levels 1-26)

The first 26 levels serve as an extended tutorial, with each level:
1. Requiring delivery of a specific shape in a specific quantity
2. Unlocking a new building, mechanic, or feature upon completion
3. Gradually increasing complexity of required shapes

**Key Tutorial Milestones:**

| Level | Unlock | Description |
|-------|--------|-------------|
| 1 | Cutter, Trash | Basic shape manipulation |
| 2 | - | Introduces cutting mechanics |
| 3 | Balancer | Belt management |
| 4 | Rotator | Shape rotation (90° CW) |
| 5 | Tunnel | Underground belt transport |
| 6 | Painter | Color application |
| 7 | CCW Rotator | Counter-clockwise rotation |
| 8 | Color Mixer | Color combination |
| 10 | Stacker | Shape layering |
| 12/13 | Blueprints | Copy/paste factory sections |
| 14 | Per-second delivery | Rate-based goals begin |
| 15 | Storage | Item buffering |
| 17 | Double Painter | Paint two shapes at once |
| 18 | Wires Layer | Logic system introduction |
| 20 | Quad Painter | Paint each quadrant separately |
| 22 | Constant Signal | Programmable signals |
| 26 | Freeplay Mode | Infinite random levels |

---

## Core Mechanics

### Shape Types

Shapez features four base shape types, each represented by a single uppercase letter in shape codes:

| Shape | Code | Description | Natural Generation |
|-------|------|-------------|-------------------|
| Circle | C | Round shape | Common near Hub |
| Rectangle | R | Square shape | Common near Hub |
| Windmill | W | Fan/propeller shape | Only as corners (never full) |
| Star | S | Four-pointed star | Only near Hub |

**Shape Structure:**
- Each shape can have up to **4 layers** (stacked vertically)
- Each layer consists of **4 quadrants** (top-right, bottom-right, bottom-left, top-left - clockwise order)
- Each quadrant can be empty (--) or contain a shape piece with a color

**Shape Code Format:**
```
[Quadrant1][Quadrant2][Quadrant3][Quadrant4]:[Layer2]:[Layer3]:[Layer4]
```

Each quadrant is two characters: Shape letter (uppercase) + Color letter (lowercase)

**Examples:**
- `CuCuCuCu` - Full uncolored circle (one layer)
- `RgRgRgRg` - Full green rectangle
- `Cu------` - Quarter circle (top-right only)
- `CuCuCuCu:RgRgRgRg` - Circle on bottom, green rectangle on top (2 layers)

### Color System

#### Primary Colors
| Color | Code | RGB Basis |
|-------|------|-----------|
| Red | r | Primary |
| Green | g | Primary |
| Blue | b | Primary |

#### Secondary Colors (Mixed)
| Color | Code | Recipe |
|-------|------|--------|
| Yellow | y | Red + Green |
| Cyan | c | Green + Blue |
| Purple/Magenta | p | Red + Blue |

#### Special Colors
| Color | Code | Recipe |
|-------|------|--------|
| White | w | Red + Green + Blue (all three) |
| Uncolored | u | Default/no paint |

**Color Mixing Rules (Additive Light Model):**
- Mixing follows additive color theory (like light, not paint)
- Any secondary + its missing primary = White
- Green + Purple = White
- Mixing identical colors = same color
- Colors can only be applied via Painter buildings

### Buildings and Machines

#### Extraction and Transport

| Building | Unlock | Function | Speed |
|----------|--------|----------|-------|
| **Extractor** | Start | Extracts shapes/colors from resource patches | Base: 2 items/sec |
| **Conveyor Belt** | Start | Transports items between buildings | 2.70 tiles/sec, 2 items/sec base |
| **Tunnel** | Level 5 | Underground transport | Tier I: 5 tiles, Tier II: 9 tiles apart |
| **Balancer** | Level 3 | Merge 2 belts, split 1 belt, or balance 2 belts | Same as belt |

#### Processing Buildings

| Building | Unlock | Function | Inputs | Outputs |
|----------|--------|----------|--------|---------|
| **Cutter** | Level 1 | Cuts shapes vertically in half | 1 shape | 2 halves (left/right) |
| **Rotator (CW)** | Level 4 | Rotates shape 90° clockwise | 1 shape | 1 rotated shape |
| **Rotator (CCW)** | Level 7 | Rotates shape 90° counter-clockwise | 1 shape | 1 rotated shape |
| **Rotator (180°)** | Level 18 | Rotates shape 180° | 1 shape | 1 rotated shape |
| **Stacker** | Level 10 | Stacks shapes vertically (up to 4 layers) | 2 shapes | 1 stacked shape |
| **Painter** | Level 6 | Paints shape with input color | 1 shape + 1 color | 1 painted shape |
| **Double Painter** | Level 17 | Paints two shapes simultaneously | 2 shapes + 1 color | 2 painted shapes |
| **Quad Painter** | Level 20 | Paints each quadrant with different colors | 1 shape + 4 colors | 1 painted shape |
| **Color Mixer** | Level 8 | Combines two colors additively | 2 colors | 1 mixed color |

#### Utility Buildings

| Building | Unlock | Function |
|----------|--------|----------|
| **Trash** | Level 1 | Destroys all incoming items (prevents clogs) |
| **Storage** | Level 15 | Buffers items, prioritizes left output (overflow gate) |
| **Compact Splitter** | Later | Splits 1 belt into 2 (alternating) |
| **Compact Merger** | Later | Merges 2 belts into 1 |

#### Machine Speed Ratios

At equivalent upgrade tiers, to achieve full belt saturation:
- **6 Cutters** = 1 full belt
- **10 Mixers** = 1 full belt
- **12 Painters** = 1 full belt
- **12 Stackers** = 1 full belt

**Note:** Many buildings operate slightly slower than their displayed speed due to tick rate precision and rounding.

### Belt System

**Base Belt Speed:**
- 2.70 tiles per second travel speed
- 2 items per second throughput (base)
- Speed increases with Belt upgrade tiers

**Belt Mechanics:**
- Items travel on belts in a queue
- Full belts are slightly slower than theoretical maximum (tick rate limitation)
- Game operates at 60 ticks per second by default
- Maximum practical throughput is approximately 15 items/second with 5 extractors

**Tunnel Mechanics:**
- Tier I tunnels: Maximum 5 tiles apart (crosses 4 tiles)
- Tier II tunnels: Maximum 9 tiles apart (crosses 8 tiles)
- Smart Tunnels feature auto-deletes belts between tunnel pairs
- No speed advantage over regular belts, but reduces rendering load

---

## Upgrade System

### Upgrade Categories

There are four upgrade categories, each affecting different building types:

| Category | Affected Buildings |
|----------|-------------------|
| **Belt** | Conveyor Belts, Tunnels |
| **Miner/Extractor** | Extractors |
| **Processors** | Cutters, Rotators, Stackers |
| **Painting** | Painters, Color Mixers |

### Upgrade Tier Structure

- **Tiers I-V:** Each tier requires progressively more shape types
  - Tier I: 1 shape type
  - Tier II: 2 shape types
  - Tier III: 3 shape types
  - Tier IV: 4 shape types
  - Tier V: 5 shape types

- **Tiers VI+:** After Tier V, all upgrades require the same 3 shapes across all categories

### Upgrade Effects

- Each tier increases the speed/throughput of affected buildings
- Speed improvements are significant until Tier VIII
- After Tier VIII, improvements decrease by an order of magnitude (1/10 effectiveness)
- Upgrades are permanent and apply globally to all buildings of that type

### Example Upgrade Requirements

| Tier | Category | Shape Required | Amount |
|------|----------|----------------|--------|
| I | Painting | Blue half-squares | 600 |
| I | Other | Red circles | 300 |
| II | All | 2 different shapes | Varies |

**Strategic Note:** Prioritize upgrades over level advancement when possible, as upgrades make everything faster. The first major opportunity to optimize old factories comes at upgrade Tier VI (around Level 19).

---

## Hub and Delivery Mechanics

### The Hub

- Central building located at map center (0,0)
- Cannot be moved or destroyed
- Accepts shape deliveries from **16 input points** around its perimeter
- Displays current level goal and progress

### Delivery System

- Shapes are delivered by connecting belts to Hub input points
- Each input can accept items at belt speed
- Maximum theoretical input rate accounts for all 16 inputs
- The Hub verifies delivered shapes match the current goal

### Goal Types

1. **Quantity Goals (Levels 1-13):** Deliver X number of the required shape
2. **Rate Goals (Level 14+):** Deliver X shapes per second continuously

**Freeplay Rate Requirements:**
| Level | Required Rate |
|-------|---------------|
| 27 | 4 shapes/sec |
| 50 | ~10 shapes/sec |
| 100 | 22 shapes/sec |

---

## Level Progression

### Predetermined Levels (1-26)

| Level | Shape Code | Amount | Unlocks |
|-------|------------|--------|---------|
| 1 | CuCuCuCu | 30 | Cutter, Trash |
| 2 | ----CuCu | 40 | - |
| 3 | RuRuRuRu | 70 | Balancer |
| 4 | RuRu---- | 70 | Rotator |
| 5 | Cu----Cu | 170 | Tunnel |
| 6 | CuCuCuCu (variation) | 270 | Painter |
| 7 | Various | 300 | CCW Rotator |
| 8 | Blue shapes | 480 | Color Mixer |
| 9 | Various | 600 | - |
| 10 | Various | 800 | Stacker |
| 11 | Various | 1,000 | - |
| 12 | Various | 1,000 | Blueprints |
| 13 | Various | 3,800 | - |
| 14 | Various | 4/sec | (Rate-based begins) |
| 15 | Various | 8,000 | Storage |
| 16 | SrSrSrSr:CyCyCyCy:SwSwSwSw | 6,000 | Cutter Variant |
| 17 | CbRbRbCb:CwCwCwCw:WbWbWbWb | 20,000 | Double Painter |
| 18 | Sg----Sg:CgCgCgCg:--CyCy-- | 20,000 | 180° Rotator, Wires |
| 19 | CpRpCp--:SwSwSwSw | 25,000 | Splitter |
| 20 | RuCw--Cw:----Ru-- | 25,000 | Quad Painter |
| 21 | CrCwCrCw:CwCrCwCr:CrCwCrCw:CwCrCwCr | 25,000 | Item Filter |
| 22 | Cg----Cr:Cw----Cw:Sy------:Cy----Cy | 25,000 | Constant Signal |
| 23 | CcSyCcSy:SyCcSyCc:CcSyCcSy | 25,000 | Display |
| 24 | CcRcCcRc:RwCwRwCw:Sr--Sw--:CyCyCyCy | 25,000 | Logic Gates |
| 25 | Rg--Rg--:CwRwCwRw:--Rg--Rg | 25,000 | Virtual Processing |
| 26 | Spaceship shape | 50,000 | Freeplay Mode |

### Special Shapes

**Level 20 - The Logo Shape:** `RuCw--Cw:----Ru--`
- Requires "floating layers" technique
- Cannot be built with standard stacking methods

**Level 26 - The Spaceship Shape:**
- Most complex predetermined shape
- Requires mastery of floating layers
- Unlocks endless freeplay mode

### Floating Layers Mechanic

Levels 20 and 26 introduce shapes that appear "physically impossible" - layers that seem to float without support. The technique works because:

1. The stacker places one shape on top of another
2. It checks if the shape can "slip down" further
3. If layers collide and prevent downward movement, they become "glued together"
4. This allows creating shapes with supported floating sections

This mechanic is so unintuitive that it generated hundreds of bug reports.

### Freeplay Levels (27+)

After Level 26, shapes are randomly generated:

| Level Range | Layer Complexity |
|-------------|------------------|
| 27-50 | 2 layers |
| 51-75 | 3 layers |
| 76+ | 4 layers |

All freeplay levels use rate-based goals (shapes per second).

---

## Production Chains

### Basic Shape Processing

```
Extractor -> [Raw Shape]
    |
    v
Cutter -> [Half Shapes]
    |
    v
Rotator -> [Rotated Shapes]
    |
    v
Stacker -> [Combined Shapes]
```

### Color Production Chain

```
Red Extractor ----\
                   v
Green Extractor -> Mixer -> Yellow
                   ^
Blue Extractor ----/
                   v
                 Mixer -> Cyan
                   ^
                   |
Red + Blue -------> Mixer -> Purple

Red + Green + Blue (or Secondary + Missing Primary) -> White
```

### Complete Shape Production

```
Shape Source -> Cutter -> Rotator -> Stacker
                                        |
Color Source -> Mixer ----------------> Painter -> Hub
```

### Windmill Production

Since full windmills never spawn naturally:

```
Windmill Corner Patch -> Extractor -> Cutter -> [Corner pieces]
                                                     |
                                                     v
                                              Stacker (x4) -> Full Windmill
```

---

## World and Map

### Map Generation

- **Infinite procedural generation** - map extends infinitely in all directions
- **Seed-based:** A seed value determines all resource patch positions and contents
- **Shareable seeds:** Players can share seeds to generate identical maps

### Resource Distribution

**Near Hub (Starting Area):**
- Primary colors (Red, Green, Blue) in separate patches
- Circles (most common)
- Rectangles
- Stars (only found near Hub)
- Windmill corners (half windmills)

**Further from Hub:**
- Resources become more randomized
- Mixed/combined shapes appear
- More complex patches
- Full windmills are NEVER naturally generated anywhere

### Extractor Placement

- Extractors must be placed directly on resource tiles
- Each extractor covers one tile of the resource patch
- Multiple extractors can work the same patch
- Resources are infinite - patches never deplete

---

## Wires and Logic Systems

### Overview

Unlocked at Level 18, the wires system adds a second layer for building logic circuits. Players can switch between the regular layer (buildings/belts) and the wires layer (logic/signals).

### Signal Types

| Signal Type | Description |
|-------------|-------------|
| Boolean | True (1) / False (0) |
| Shape | Any valid shape code |
| Color | Any valid color code |

### Logic Gate Buildings

| Building | Function |
|----------|----------|
| AND Gate | Output true if both inputs true |
| OR Gate | Output true if either input true |
| NOT Gate | Invert input signal |
| XOR Gate | Output true if inputs differ |
| Transistor | Conditional signal pass-through |

### Wire Buildings

| Building | Unlock | Function |
|----------|--------|----------|
| **Belt Reader** | Level 18 | Outputs shape/count passing through belt |
| **Display** | Level 23 | Visualizes signal on regular layer |
| **Item Filter** | Level 21 | Splits belt based on shape match |
| **Constant Signal** | Level 22 | Emits constant shape/color/boolean |
| **Shape Analyzer** | Level 25 | Returns shape and color of top-right quadrant |
| **Comparator** | Level 25 | Returns true if both inputs equal |

### Virtual Processors

These buildings process shape SIGNALS on the wires layer (not physical shapes):

| Building | Function |
|----------|----------|
| Virtual Cutter | Virtually cuts shape signal in half |
| Virtual Rotator | Virtually rotates shape signal |
| Virtual Unstacker | Extracts top layer from shape signal |
| Virtual Stacker | Combines two shape signals |
| Virtual Painter | Applies color signal to shape signal |

### Connecting Hub

The Hub emits a signal containing the current required shape, which can be read by connecting wires to it. This is essential for Make Anything Machines.

---

## Endgame Content

### Freeplay Mode

Unlocked after Level 26, freeplay provides infinite progression:

- Randomly generated shapes each level
- Increasing complexity (2 -> 3 -> 4 layers)
- Increasing rate requirements
- No theoretical maximum level

### Make Anything Machine (MAM)

The ultimate endgame goal - a factory that can automatically produce ANY shape:

**MAM Design Principles:**
1. **Decompose:** Read hub signal, unstack into layers, cut into quadrants
2. **Fetch:** Retrieve required shape pieces and colors
3. **Combine:** Assemble the required shape automatically

**MAM Requirements:**
- Full wires system mastery
- Virtual processors for signal manipulation
- Automated color mixing
- Storage buffers for all shape types
- Hub signal reading

**MAM Achievement:** Complete any freeplay level without modifying your factory (1.98% of players)

### Level Milestones

| Level | Significance |
|-------|--------------|
| 26 | Tutorial complete, freeplay unlocked |
| 50 | 2-layer shapes end, 3-layer begin |
| 75 | 3-layer shapes end, 4-layer begin |
| 100 | "Is this the end?" achievement |

### Puzzle DLC

A separate DLC mode featuring:
- Limited space and resources
- Predefined puzzles
- Level editor
- Community puzzle sharing

---

## Achievements

### Total Achievements: 45
### Average Completion Time: 40-50 hours

### Notable Achievements

| Achievement | Requirement | Rarity |
|-------------|-------------|--------|
| Cutter | Cut a shape | 99.55% |
| Rotater | Rotate a shape | 90.88% |
| Painter | Paint a shape | 81.32% |
| Freedom | Complete Level 26 | 8.28% |
| MAM | Complete freeplay level without changes | 1.98% |
| Is this the end? | Reach Level 100 | 1.58% |
| It's so slow | Complete Level 12 without belt upgrades | 1.19% |
| Speedrun Novice | Complete Level 12 in under 60 minutes | 0.79% |
| Speedrun Master | Complete Level 12 in under 30 minutes | 0.40% |
| Microsoft Logo | Deliver RgRyRbRr shape | Hidden |

### Hidden Achievements

- **Microsoft Logo:** Create the shape `RgRyRbRr` (colored quadrants matching Microsoft logo)
- **Rocket Shape:** Create `CbCuCbCu:Sr------:--CrSrCr:CwCwCwCw`

---

## Sources

### Official Resources
- [Shapez.io Official Website](https://shapez.io)
- [Shapez.io Wires Update](https://shapez.io/wires/)
- [Shapez Shape Viewer](https://viewer.shapez.io/)

### Wiki Resources
- [Shapez.io Wiki - Fandom](https://shapezio.fandom.com/wiki/)
- [Levels Page](https://shapezio.fandom.com/wiki/Levels)
- [Upgrading Page](https://shapezio.fandom.com/wiki/Upgrading)
- [Buildings Page](https://shapezio.fandom.com/wiki/Buildings)
- [Everything Machines](https://shapezio.fandom.com/wiki/Everything_Machines)
- [Strategy Guide](https://shapezio.fandom.com/wiki/Strategy_Guide)

### Steam Community Guides
- [Shapez.io 100% Achievement Guide](https://steamcommunity.com/sharedfiles/filedetails/?id=2423657811)
- [Walkthrough: Inspiration for solving every level](https://steamcommunity.com/sharedfiles/filedetails/?id=2204447577)
- [Detailed explanation on "floating layers" (level 20/26)](https://steamcommunity.com/sharedfiles/filedetails/?id=2574780962)
- [Compact Everything Machine Guide](https://steamcommunity.com/sharedfiles/filedetails/?id=2207930168)

### Technical Resources
- [GitHub Repository](https://github.com/tobspr-games/shapez.io)
- [DeepWiki - Game Mechanics](https://deepwiki.com/tobspr-games/shapez.io/3-game-mechanics)
- [Steam Charts](https://steamcharts.com/app/1318690)

### Community Discussions
- [Steam Community Forums](https://steamcommunity.com/app/1318690)
- [Archipelago Game Guide](https://archipelago.gg/games/shapez/info/en)

---

## Design Takeaways for Factorial

1. **Infinite Resources:** No resource depletion creates pure optimization focus
2. **Tutorial as Progression:** Each level teaches exactly one new concept
3. **Shape Code System:** Elegant encoding allows complex shape generation
4. **Floating Layers:** Counterintuitive mechanics can create interesting puzzles
5. **MAM as Endgame:** "Make anything" machines provide infinite replayability
6. **Rate vs Quantity Goals:** Switching to rate-based goals changes factory design philosophy
7. **Wires Layer:** Separate logic layer keeps main factory visually clean
8. **Virtual Processors:** Signal-based processing enables complex automation
9. **Seed Sharing:** Deterministic generation allows community challenges
10. **Minimal UI:** Abstract aesthetic keeps focus on factory design
