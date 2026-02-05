# Shapez 2: Comprehensive Game Design Research Document

## Overview

**Shapez 2** is a 3D factory-building and automation game developed by **tobspr Games**, released in Early Access on **August 15, 2024** for Windows, macOS, and Linux via Steam. It is the sequel to the open-source browser/desktop game Shapez.io.

The core gameplay loop involves extracting raw geometric shapes from asteroids, processing them through increasingly complex factory systems (cutting, rotating, stacking, painting, crystallizing), and delivering completed shapes to a central Vortex to progress through milestones and unlock new technologies.

Unlike its predecessor, Shapez 2 features full 3D graphics, multi-layer building, space platforms connected by trains, and significantly improved performance supporting factories with 100,000+ buildings.

---

## Initial Conditions and Tutorial Progression

### Starting State

- The player begins on a single asteroid platform near the central **Vortex**
- The Vortex is the massive swirling feature at the center of the map where all shapes must be delivered
- Initial access: 16 of the Vortex's 144 input ports (12 three-floor ports on each of 4 sides)
- Starting buildings: Basic extractors, belts, and simple processing machines
- No resource scarcity, time limits, or combat elements
- All buildings are free to place (no material costs)

### First Shapes

The game begins with simple single-layer shapes:
- **Circles** (C)
- **Squares/Rectangles** (R)
- **Stars** (S)
- **Diamonds** (W) - formerly "Windmill" in early development

Early milestones require delivering basic uncolored shapes, then gradually introduce:
1. Shape cutting (half shapes)
2. Shape rotation
3. Color painting
4. Shape stacking (multi-layer)
5. Advanced operations (swapping, crystals, pins)

### Tutorial Progression

The game features an integrated tutorial system that introduces mechanics progressively:
1. **Milestone 1-3**: Basic extraction and delivery of simple shapes
2. **Milestone 4-6**: Introduction to cutting and rotating
3. **Milestone 7-10**: Painting and color mixing
4. **Milestone 11+**: Stacking, multi-layer shapes, and advanced operations

Key tip: Don't rush milestones. Master current tools and shapes before advancing.

---

## Core Mechanics

### Shape Processing Operations

#### Extraction
- **Shape Miners** extract shapes from Shape Asteroids
- Base miner platform has 4 extractor locations
- Up to 3 extension platforms can be chained (4 extractors each)
- Maximum: 16 extractors per mining setup
- 12 units of extractors fill a space belt completely

#### Cutting
- **Cutter**: Slices shapes vertically into two halves (east/west)
- Both halves are preserved and output separately
- Right output always receives the right half regardless of orientation
- **Half Destroyer**: Destroys west half, outputs only east half
- Quarter pieces require: Half Destroyer -> Rotate 90 degrees -> Half Destroyer
- Crystal pieces shatter if cut through connected crystals

#### Rotating
- **Rotator**: Adjusts shape orientation
- Variants: 90 degrees clockwise, 90 degrees counter-clockwise, 180 degrees
- In regular scenarios: 90 degrees per rotation
- In hexagonal scenario: 60 degrees per rotation

#### Stacking
- **Stacker**: Places top input shape on top of bottom input shape
- One empty layer gap, then gravity rules applied
- Non-overlapping halves merge into single layer (two opposite halves become one full layer)
- Layer limits: 4 layers (normal), 5 layers (insane scenario)
- Layers above limit are discarded
- Crystals falling through stacker shatter

#### Painting
- **Painter**: Paints the top layer of incoming shapes
- Fluid consumption: 450 L/min per painter
- Buffer capacity: 20 liters
- Painting one layer uses 10 liters
- Inputs on three sides for flexibility
- Only paints top layer; stacking is needed for multi-color shapes

#### Swapping
- **Swapper**: Exchanges west halves between two input shapes
- Rule: "Only the West Side Moves"
- East sides pass through unchanged
- Equivalent to two cutters and two stackers combined
- Gentle exchange preserves crystals (unlike stacking)

#### Unstacking
- **Unstacker**: Separates top layer from remaining layers
- Default: Top layer outputs left, remaining layers output right
- Mirrored: Behavior reverses

---

## Shape Language and Notation System

### Shape Code Structure

Each shape is represented by a **shape code** string:

```
[Layer1]:[Layer2]:[Layer3]:[Layer4]
```

- Layers listed bottom to top, separated by colons (`:`)
- Each layer has 4 quadrants (or 6 in hexagonal mode)
- Quadrants listed clockwise starting from top-right (NE, SE, SW, NW)
- Each quadrant: 2 characters (shape type + color)
- Empty quadrant: `--`

### Shape Type Codes

| Code | Shape |
|------|-------|
| `C` | Circle |
| `R` | Square/Rectangle |
| `S` | Star |
| `W` | Diamond (formerly Windmill) |
| `P` | Pin |
| `c` | Crystal (context-dependent) |

### Color Codes

| Code | Color |
|------|-------|
| `u` | Uncolored |
| `r` | Red |
| `g` | Green |
| `b` | Blue |
| `y` | Yellow (Red + Green) |
| `c` | Cyan (Green + Blue) |
| `m` | Magenta (Red + Blue) |
| `w` | White (all three primaries) |

### Example Shape Codes

| Code | Description |
|------|-------------|
| `CuCuCuCu` | Single layer: 4 uncolored circles |
| `RrRrRrRr` | Single layer: 4 red squares |
| `CuCu----` | Half circle (east side only) |
| `CuCuCuCu:RrRrRrRr` | 2 layers: uncolored circles on bottom, red squares on top |
| `Cu------:--Cu----` | Complex layered shape with gaps |

### Layer Limits by Scenario

| Scenario | Max Layers | Quadrants per Layer |
|----------|------------|---------------------|
| Normal | 4 | 4 |
| Hard | 4 | 4 |
| Insane | 5 | 4 |
| Hexagonal | 4 | 6 |

---

## Shape Physics and Gravity Rules

### No Floating Pieces

Unlike Shapez 1, **floating layers cannot exist** in Shapez 2. The game uses physics-based simulation:

1. After any operation, gravity rules are applied
2. Layers split into groups of horizontally connected parts
3. Parts connected horizontally (not diagonally) form groups
4. Unsupported groups fall until supported
5. Falling groups with crystals: crystals shatter, potentially splitting the group

### Pins as Support

- **Pins** act as supports in place of gaps
- Allow shapes that would otherwise float
- Pins are never "supported" themselves for gravity purposes
- If stacking creates an overhang, pins fall to lower layers
- Shape parts connected through pins are NOT considered horizontally connected

### Crystal Fragility

- Crystals connecting regular parts can shatter if cut or dropped
- Shattered crystals may split shapes into multiple groups
- Crystal-connected pieces must be handled carefully
- Swapper gently exchanges halves without breaking crystals

---

## Fluid and Color System

### Primary Colors

- Red (r)
- Green (g)
- Blue (b)

### Color Mixing

| Input 1 | Input 2 | Output |
|---------|---------|--------|
| Red | Green | Yellow |
| Green | Blue | Cyan |
| Red | Blue | Magenta |
| Secondary | Missing Primary | White |

### Fluid System Specifications

| Component | Specification |
|-----------|--------------|
| Mixer ratio | 1:1 (25L each = 50L output) |
| Mixer max production | 900 L/min |
| Fluid launcher/catcher | 1800 L/min (2 mixers worth) |
| Painter consumption | 450 L/min |
| Painter buffer | 20 L |
| Paint per layer | 10 L |
| Fluid tank capacity | 1,800 L |
| Pump extraction | 450 L/min each |

### Fluid Extraction

- **Fluid Miners** extract paint from Fluid Asteroids
- Base platform: 4 pump locations
- Up to 3 extensions (4 pumps each)
- Maximum: 16 pumps per setup
- 72 pump units fill a space pipe completely

---

## Crystal System

### Crystal Generator / Crystallizer

- Generates crystals in all gaps and pin positions
- Only fills up to the highest used layer in the input shape
- Existing crystals pass through unchanged
- Liquid "drips" down to lowest empty/pinned slot in that quadrant

### Crystal Properties

- Crystals are fragile
- Shatter when falling through stacker
- Shatter when cut through connected pieces
- Shatter when gravity causes them to fall
- Use pins to hold floating quarters before crystallization
- Crystals replace pins during crystallization

---

## Buildings and Machines

### Extraction Buildings

| Building | Function |
|----------|----------|
| Shape Miner | Extracts shapes from asteroids |
| Shape Miner Extension | Adds 4 more extraction points |
| Fluid Miner (Pump) | Extracts paint from fluid asteroids |
| Fluid Miner Extension | Adds 4 more pump points |

### Transport Buildings

| Building | Function |
|----------|----------|
| Conveyor Belt | Transports shapes between machines |
| Splitter | 1 input to 2-3 outputs |
| Merger | 2-3 inputs to 1 output |
| Balancer (Distributor) | Evenly distributes between 2 belts |
| Space Belt | Connects platforms across space |
| Belt Launcher | Sends shapes to distant catchers |
| Belt Catcher | Receives shapes from launchers |
| Pipes | Transport fluids |
| Space Pipe | Connects platforms for fluids |
| Fluid Launcher/Catcher | Long-distance fluid transport |

### Shape Processing Buildings

| Building | Function |
|----------|----------|
| Cutter | Cuts shapes into halves (both preserved) |
| Half Destroyer | Destroys west half |
| Rotator | Rotates shapes (90 CW, 90 CCW, 180) |
| Stacker | Stacks shapes vertically |
| Unstacker | Separates top layer from rest |
| Swapper | Exchanges west halves between two shapes |
| Pin Pusher | Adds pins under shape |
| Crystal Generator | Fills gaps with crystals |

### Painting Buildings

| Building | Function |
|----------|----------|
| Painter | Paints top layer of shapes |
| Color Mixer | Combines two fluid colors |
| Fluid Tank | Buffers 1,800L of fluid |

### Flow Control Buildings

| Building | Function |
|----------|----------|
| Trash | Destroys shapes |
| Belt Reader | Displays throughput, outputs shape signal |
| Belt Filter | Routes shapes based on wire signal |
| Storage | Stores shapes (up to 5,000 units per type) |

### Train System Buildings

| Building | Function |
|----------|----------|
| Train Rail | Placed in space mode, free cost |
| Train Stop (Station) | Tells locomotive where to stop |
| Train Loader (Shapes) | Loads shapes onto wagons |
| Train Unloader (Shapes) | Unloads shapes from wagons |
| Fluid Loader | Loads fluids onto wagons |
| Fluid Unloader | Unloads fluids from wagons |
| Quick Stop | Fast package transfer |

### Logic/Wiring Buildings

| Building | Function |
|----------|----------|
| Wire | Carries signals |
| AND Gate | Output 1 if both inputs truthy |
| OR Gate | Output 1 if any input truthy |
| NOT Gate | Output 1 if input not truthy |
| XOR Gate | Output 1 if exactly one input truthy |
| Gate (Transistor) | Passes any signal type when enabled |
| Comparison | Compares signals (==, >, >=, <, <=, !=) |
| Signal Producer | Generates constant signals |
| Shape Analyzer | Outputs shape code and NE quadrant color |

### Simulated Buildings (Virtual Processors)

All physical processing buildings have simulated equivalents that operate on signals:
- Simulated Rotator
- Simulated Cutter
- Simulated Stacker
- Simulated Unstacker
- Simulated Painter
- Simulated Pin Pusher
- Simulated Swapper

---

## Train System

### Specifications

| Component | Specification |
|-----------|--------------|
| Wagon capacity per layer | 960 shapes OR 9,600 L fluid |
| Wagon total capacity (3 layers) | 2,880 shapes OR 28,800 L fluid |
| Minimum load | 960 shapes / 9,600 L (full package) |
| Loader/Unloader buffer | 2 full packages per layer |
| Unloader pending packages | Up to 39 (12 per floor + progress) |

### Train Behavior

- Trains only pick up completely filled packages
- Rails placed in Space mode, cost 0 Platform Units
- Rails can split and merge like belts
- Each rail supports one or more trains (color-coded)
- Train rail jump targets center of Vortex for direct delivery

---

## Platform and Space System

### Platform Units (PU)

- Currency for expanding factory footprint
- Earned through completing Milestones and Tasks
- Cost equals number of space grid tiles occupied
- 1x1 foundation = 1 PU, 2x1 = 2 PU, etc.

### Foundations

- Provide building area for factory buildings
- Connect via "notches" (1x4 areas on edges)
- Initial foundations: 1x1, 2x1 (unlocked at Space Platforms Milestone)
- Additional sizes purchased with Research Points

### Space Connections

| Component | Throughput |
|-----------|------------|
| Space Belt | Base throughput |
| Space Pipe | 4x belt throughput |
| Belt Launcher per port | 1800/min (1/4 of space belt when using crossings) |
| Fluid Launcher per port | 1800 L/min |

---

## Wiring and Logic System

### Signal Types

| Type | Examples |
|------|----------|
| Null | Empty wire, no signal |
| Boolean | 0 (false), 1 (true) |
| Integer | Any whole number |
| Color | r, g, b, y, c, m, w, u |
| Shape | Any valid shape code |

### Signal Truthiness

- **Truthy**: All signals except null and false (0)
- **Falsy**: Null signal and 0

### Logic Gates

| Gate | Behavior |
|------|----------|
| AND | 1 if both inputs truthy, else 0 |
| OR | 1 if any input truthy, else 0 |
| NOT | 1 if input falsy, else 0 |
| XOR | 1 if exactly one input truthy, else 0 |
| Gate | Passes input signal when side input truthy; null otherwise |
| Comparison | Compares signals, outputs 0 or 1 |

### Shape Analyzer

- Input: Physical shape
- Output 1: Uncolored shape signal
- Output 2: Color of NE (corner 1) segment
- Used for detecting and routing specific shapes

---

## Progression Systems

### Milestones

Primary progression goals requiring large shape deliveries:

- Complete Milestones to unlock buildings, upgrades, and tasks
- Each scenario has unique milestones
- Shape requirements scale with difficulty settings
- Milestones build on each other (indicated by arrows between shapes)
- Shape delivery indicators at Vortex show progress

#### Vortex Delivery Indicators

| Background | Meaning |
|------------|---------|
| Gray | Shape not needed for any goal |
| Green | Counting toward current Milestone/Task |
| Flag | Requirement met, reward available |
| Orange Lock | Correct but for future locked Milestone/Task |
| Purple Star | Operator Level shape |
| Blue B | Blueprint shape |

### Tasks

Side objectives parallel to Milestones:
- Complete Tasks to earn Research Points
- Each scenario has unique Tasks
- Rewards vary by difficulty settings
- At higher difficulties, completing Tasks first provides crucial PU and Research Points

### Upgrades

Two categories purchased with Research Points:

#### Linear Upgrades (Speed)
- Belt speed increases
- Cutter/Stacker processing speed
- Painter efficiency
- Train speed and storage

#### Unlockable Upgrades
- New foundation types and sizes
- Third conveyor level
- Advanced buildings
- Train components

### Operator Level (Infinite Progression)

After completing all Milestones and Tasks:
- Operator research tab becomes main focus
- Infinite goals with exponentially scaling requirements
- Random Operator Shapes require "Make Anything Machine" (MAM) factories
- Shape Multiplier upgrade makes each shape count as multiple deliveries
- Encourages maintaining and improving milestone factories

---

## Scenarios and Difficulty

### Available Scenarios

| Scenario | Description | Max Layers | Quadrants |
|----------|-------------|------------|-----------|
| Normal | Recommended for new players | 4 | 4 |
| Hard | For experienced players seeking challenge | 4 | 4 |
| Insane | Maximum difficulty, requires expert knowledge | 5 | 4 |
| Hexagonal | Experimental mode | 4 | 6 |

### Difficulty Settings

Adjustable parameters:
- **Goal Multiplier**: Shapes required per milestone
- **Copy/Paste Cost**: Blueprint placement costs
- **Platform Limit**: Maximum platform expansion

Scenarios are permanent and cannot be changed after savegame creation.

---

## Blueprint System

### Creating Blueprints

1. Hold 'Select Area' (default: Shift)
2. Drag with 'Base Selection' (default: Left Click)
3. Release to create selection box
4. Selection becomes blueprint hologram

### Blueprint Manipulation

| Action | Default Key |
|--------|-------------|
| Place blueprint | Left Click |
| Cancel placement | Escape |
| Rotate | R |
| Rotate inverse | Shift+R |
| Mirror | F |
| Mirror inverse | Shift+F |
| Paste previous | Ctrl+V |

### Blueprint Sharing

- Blueprints can be saved as codes or files
- Shareable between players
- Community blueprint collections available
- Platform blueprints for standardized factory modules

---

## Performance Optimizations

### Technical Achievements

| Metric | Shapez 1 | Shapez 2 |
|--------|----------|----------|
| Smooth performance | ~5,000 buildings | ~100,000 buildings |
| Playable limit | ~10,000 buildings | ~500,000 buildings |

### Optimization Strategies

Built on Unity with deliberate performance-first design:

1. **Simulation Optimization**
   - Physics simulation caching for shapes
   - Efficient gravity rule calculations
   - Batched processing updates

2. **Rendering Optimization**
   - Limited animations to support scale
   - Bypassed standard Unity systems
   - Custom rendering pipeline

3. **Platform/Chunk System**
   - Increased platform and chunk limits
   - Efficient spatial partitioning
   - Progressive loading

### Three Optimization Categories

1. **Simulation**: Game logic and physics
2. **Rendering**: Visual display
3. **Interfacing**: UI and player interaction

---

## Differences from Shapez 1

### Major Changes

| Feature | Shapez 1 | Shapez 2 |
|---------|----------|----------|
| Graphics | 2D | Full 3D |
| Vertical building | Not possible | Core mechanic |
| Floating layers | Allowed | Physics-based (no floating) |
| Performance | ~5k buildings | ~100k buildings |
| Platform system | Infinite ground | Space platforms + PU cost |
| Transportation | Belts only | Belts + Space belts + Trains |
| Pricing | Free, open source | Paid (Steam) |
| Wiring | Available | Available with improvements |

### New Mechanics in Shapez 2

1. **3D Multi-Layer Building**: Build factories vertically across multiple layers
2. **Space Platforms**: Limited space requiring efficient use of Platform Units
3. **Train System**: Connect distant platforms with trains carrying bulk cargo
4. **Crystal System**: New fragile shape type with unique behavior
5. **Pin System**: Support pieces allowing otherwise impossible shapes
6. **Swapper Building**: Efficient half-exchange operation
7. **Physics-Based Shapes**: Realistic gravity simulation
8. **Fluid System**: Paint transported via pipes instead of belts

### Removed/Changed Mechanics

- **Floating layers eliminated**: Replaced with physics-based gravity
- **Windmill shape renamed**: Now called "Diamond" (code still W)
- **Hub replaced with Vortex**: Central delivery point with 144 inputs
- **Infinite ground eliminated**: Space platforms with PU costs

---

## Throughput Reference

### Belt Speeds

| Upgrade Level | Speed |
|---------------|-------|
| Level 1 | 60 shapes/min |
| Level 2 | 90 shapes/min |
| Level 3 | 120 shapes/min |
| Maximum | 180 shapes/min |

### Processing Speeds (Level 1)

| Building | Throughput |
|----------|------------|
| Painter | 30/min (4 per belt) |
| Cutter | 6 per belt |
| Stacker (straight) | 6 per belt |
| Stacker (bent) | 4 per belt (more compact) |

### Transport Comparison

| Method | Capacity |
|--------|----------|
| Belt (Level 3) | 120 shapes/min |
| Train (Level 3) | 720 shapes/min |
| Space Pipe | 4x belt throughput |

---

## Make Anything Machine (MAM)

### Concept

A factory capable of producing any random shape requested by Operator Level goals:

- Required for infinite progression beyond milestones
- Must handle all shape types, colors, layers, and special elements
- Represents ultimate factory building challenge

### MAM Requirements

1. **All 4 shape types**: Circle, Square, Star, Diamond
2. **All 8 colors**: Uncolored + 7 paint colors
3. **All layer configurations**: 1-4 layers (1-5 in Insane)
4. **Special elements**: Pins and Crystals
5. **Signal processing**: Analyze target shape, route components

### MAM Complexity

- Requires shape analyzer to decode target
- Virtual processors simulate operations
- Complex routing and flow control
- Crystal handling needs special care
- Represents hundreds of hours of optimization

---

## Unique Distinguishing Features

### vs. Other Factory Games

1. **Abstract Focus**: No narrative, combat, or survival - pure logistics
2. **Free Building**: No material costs for buildings
3. **Shape Language**: Unique encoding system for complex shapes
4. **Physics Simulation**: Realistic shape gravity behavior
5. **Infinite Scaling**: No upper limit on progression
6. **Visual Clarity**: Clean aesthetic despite complexity
7. **Accessibility**: No time pressure or failure states

### Core Innovation

Shapez 2's fundamental innovation is treating **shapes as a rich data type** that can be:
- Decomposed (cutting, unstacking)
- Transformed (rotating, painting)
- Combined (stacking, swapping)
- Analyzed (shape codes, signals)
- Validated (physics rules)

This creates emergent complexity from simple rules, similar to cellular automata or programming languages.

---

## Sources

- [Shapez 2 Official Site](https://shapez2.com/)
- [Shapez 2 Wiki](https://shapez2.wiki.gg/)
- [Shapez 2 on Steam](https://store.steampowered.com/app/2162800/shapez_2/)
- [tobspr Games](https://tobspr.io/)
- [Shapez 2 Wikipedia](https://en.wikipedia.org/wiki/Shapez_2)
- Steam Community Discussions and Guides
- Shapez 2 Suggestions and Feedback Portal

---

*Document compiled for game design research purposes. Shapez 2 is developed by tobspr Games.*
