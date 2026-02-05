# Production Line: Comprehensive Game Design Research Document

## Table of Contents
1. [Game Overview](#1-game-overview)
2. [Initial Conditions](#2-initial-conditions)
3. [Core Mechanics](#3-core-mechanics)
4. [Production Slots](#4-production-slots)
5. [Research/Tech Tree](#5-researchtech-tree)
6. [Resource System](#6-resource-system)
7. [Car Customization](#7-car-customization)
8. [Business Simulation](#8-business-simulation)
9. [Factory Layout](#9-factory-layout)
10. [DLC Content](#10-dlc-content)
11. [Unique Mechanics](#11-unique-mechanics)

---

## 1. Game Overview

### Basic Information

| Attribute | Value |
|-----------|-------|
| **Developer** | Positech Games (Cliff Harris) |
| **Platform** | PC (Steam) |
| **Genre** | Factory simulation / Tycoon |
| **Perspective** | 2D top-down |
| **Theme** | Car manufacturing |

### Premise

Players design and manage a car factory, optimizing production lines to manufacture vehicles efficiently while competing against AI rivals in a dynamic market. The core gameplay loop involves progressive subdivision of labor - starting with simple combined production slots and unlocking increasingly specialized stations through research.

### Design Philosophy

- **Incremental complexity**: Start simple, unlock specialization
- **Economic simulation**: Dynamic market with competitor AI
- **Spatial optimization**: Factory layout directly impacts efficiency
- **No combat/survival**: Pure business and logistics simulation

---

## 2. Initial Conditions

### Starting State

| Element | Starting Value |
|---------|----------------|
| Capital | Large sum (varies by scenario/map) |
| Body Styles | Sedan only |
| Features | Minimal (no AC, power steering, airbags, etc.) |
| Slots | Basic combined slots only |
| Competitors | AI rivals active from start |

### Game Modes

| Mode | Description |
|------|-------------|
| Sandbox | Unlimited funds, full experimentation |
| Freeplay | Standard economic simulation |
| Scenario | Mission-driven with specific objectives |

### Failure Condition

- **Bankruptcy**: Game over when funds deplete
- No autosave protection from poor decisions

### Tutorial Progression

1. Place basic chassis assembly slot
2. Connect to fit body slot
3. Add paint station
4. Connect to export
5. Learn resource importing
6. Introduction to research

---

## 3. Core Mechanics

### 3.1 Production Flow

The fundamental production chain:

```
Chassis Assembly → Fit Body → Paint → Fit Engine → Fit Accessories → Fit Electronics → QA → Export
```

### 3.2 Slot-Based Production

Each car passes through production "slots" on conveyor belts:
- Slots perform specific assembly tasks
- Each slot has processing time
- Cars queue when slots are busy
- Bottlenecks form at slowest slots

### 3.3 Progressive Subdivision

The core progression mechanic:

**Example - Body Assembly**:
- Start: Single "Fit Body" slot (handles everything)
- Research unlocks:
  - "Fit Body Frame"
  - "Fit Roof"
  - "Fit Doors"
- Specialized slots are faster per task
- Total throughput increases with subdivision

### 3.4 Conveyor System

| Type | Purpose |
|------|---------|
| Vehicle Conveyor | Moves cars between slots |
| Resource Conveyor | Overhead system for components |
| Smart Junction | Routes cars by design/features |

---

## 4. Production Slots

### 4.1 Body Slots

| Slot | Function | Subdivision From |
|------|----------|------------------|
| Fit Body | Complete body assembly | Starting slot |
| Fit Body Frame | Attach frame only | Fit Body |
| Fit Roof | Attach roof only | Fit Body |
| Fit Doors | Attach doors only | Fit Body |

### 4.2 Paint Slots

| Slot | Processing Time | Notes |
|------|-----------------|-------|
| Paint Undercoat | ~4 minutes | Base coat |
| Dry Undercoat | ~12 minutes | Longest slot |
| Paint Finish | ~4 minutes | Top coat |
| Dry Finish | ~8 minutes | Curing time |
| Polish | ~3 minutes | Final shine |

**Paint is the primary bottleneck** - Dry Undercoat's 12-minute time requires parallel lines.

### 4.3 Engine Slots

| Slot | Components Required |
|------|---------------------|
| Fit Engine Assembly | Engine block |
| Fit Radiator | Radiator |
| Fit Exhaust | Exhaust system |
| Fit Wheels | Wheels, tires |
| Fit Steering | Steering column |
| Fit Brakes | Brake assembly |
| Fit Fuel Tank | Fuel tank (ICE/Hybrid) |

### 4.4 Accessory Slots

| Slot | Components |
|------|------------|
| Fit Seats | Front seats, rear seats |
| Fit Interior | Interior trim |
| Fit Dashboard | Dashboard assembly |
| Fit Windows | Windows, windshield |
| Fit Door Panels | Interior door panels |
| Fit Lights | Headlights, taillights |

### 4.5 Electronics Slots

| Slot | Features Enabled |
|------|------------------|
| Fit GPS | Navigation system |
| Fit Bluetooth | Connectivity |
| Fit Alarm | Security system |
| Fit Infotainment | Entertainment system |

### 4.6 Quality Slots

| Slot | Function |
|------|----------|
| Quality Assurance | Inspects completed cars |
| Rework Station | Fixes detected defects |

### 4.7 Manufacturing Slots ("Make" Slots)

Local production vs. importing:

| Make Slot | Input | Output | Ratio |
|-----------|-------|--------|-------|
| Make Doors | Steel | Doors | 1:2 |
| Make Tires | Rubber | Tires | 1:2 |
| Make Seats | Fabric | Seats | 1:1 |
| Make Wheels | Aluminum | Wheels | 1:2 |
| Make Windows | Glass | Windows | 1:2 |
| Make Brakes | Steel | Brakes | 1:1 |
| Make Lights | Plastic + Electronics | Lights | 1:1 |
| Make Electric Motors | Steel + Electronics | Motors | 1:1 |

---

## 5. Research/Tech Tree

### 5.1 Research Categories

| Category | Unlocks |
|----------|---------|
| **Specialization** | New subdivided Fit slots |
| **Manufacture** | Local Make slots |
| **Efficiency** | Speed upgrades, smart routing |
| **Administration** | Support buildings |

### 5.2 Specialization Research

Progression through slot subdivision:

```
Fit Body (start)
├── Fit Body Frame
├── Fit Roof
└── Fit Doors
    ├── Fit Door Handles
    └── Fit Door Seals

Fit Engine (start)
├── Fit Engine Block
├── Fit Radiator
├── Fit Exhaust
└── Fit Transmission
```

### 5.3 Efficiency Research

| Technology | Effect |
|------------|--------|
| Fast Conveyors | +X% conveyor speed |
| Fast Importers | +X% import rate |
| Smart Junctions | Route by car design |
| Robot Arms | Automated component delivery |
| Parallel Processing | Multiple cars in one slot |

### 5.4 Administration Research

| Building | Function |
|----------|----------|
| Research Center | Generates research points |
| Design Studio | Creates new car designs |
| Marketing Office | Generates marketing "ideas" |
| HR Office | Worker management |

### 5.5 Research Point Generation

- Research Centers generate points over time
- More centers = faster research
- Placement in admin zone required

---

## 6. Resource System

### 6.1 Resource Importers

- Placed on factory edges
- Import specific component types
- Limited throughput per importer
- Can become bottleneck

### 6.2 Stockpiles

| Attribute | Value |
|-----------|-------|
| Capacity | 36 resources |
| Function | Buffer between import and production |
| Placement | Adjacent to production slots |

### 6.3 Overhead Conveyor Network

- Separate from vehicle conveyors
- Carries components to slots
- Three-dimensional (passes over vehicle lines)
- Must connect stockpiles to slots

### 6.4 Local Manufacturing

Manufacturing advantages:
- Eliminates import dependency
- Multiplies raw materials (1 rubber → 2 tires)
- Requires additional factory space
- Needs raw material imports instead

### 6.5 Resource List

**Raw Materials**:
- Steel
- Rubber
- Aluminum
- Glass
- Plastic
- Fabric
- Electronics

**Manufactured Components**:
- Doors, Tires, Wheels, Windows
- Brakes, Lights, Seats
- Electric Motors
- Engine blocks, Transmissions

---

## 7. Car Customization

### 7.1 Body Styles

| Body Style | Unlock | Notes |
|------------|--------|-------|
| Sedan | Starting | Standard 4-door |
| SUV | Research | Higher margin |
| Sports | Research | Performance focus |
| Compact | Research | Budget segment |
| Offroad | Research | Specialized market |
| Pickup | Research | Utility vehicle |
| Van | Research | Commercial use |

### 7.2 Powertrains

| Powertrain | Components | Market Trend |
|------------|------------|--------------|
| ICE (Internal Combustion) | Engine, Fuel Tank, Exhaust | Declining |
| Hybrid | Engine + Electric Motor + Battery | Growing |
| Electric | Electric Motor + Battery | Growing |

### 7.3 Market Segments

| Segment | Features Expected | Price Range |
|---------|-------------------|-------------|
| Budget | Minimal | Low |
| Economy | Basic comfort | Medium-Low |
| Premium | Full features | Medium-High |
| Luxury | Everything + extras | High |

### 7.4 Features

Features add value but require additional slots/time:

| Feature Category | Examples |
|------------------|----------|
| Comfort | AC, Power Windows, Heated Seats |
| Safety | Airbags, ABS, Parking Sensors |
| Technology | GPS, Bluetooth, Infotainment |
| Performance | Turbo, Sports Suspension |

---

## 8. Business Simulation

### 8.1 Dynamic Pricing

- Feature values decrease as competitors adopt them
- Early adopters get premium pricing
- Market saturation reduces margins
- Must continuously innovate

### 8.2 AI Competitors

- Research technology independently
- Release competing vehicles
- Affect market feature values
- Cannot be directly interacted with

### 8.3 Marketing System

| Building | Output | Effect |
|----------|--------|--------|
| Marketing Office | "Ideas" | Generates campaigns |

**Campaign Types**:
- Brand awareness
- Feature promotion
- Price perception
- Market segment targeting

### 8.4 Showroom Indicator

| Showroom State | Meaning |
|----------------|---------|
| Empty | Cars underpriced (selling too fast) |
| Full | Cars overpriced (not selling) |
| Moderate | Pricing balanced |

### 8.5 Quality and Reputation

- QA slot catches defects
- Defects shipped = reputation damage
- Reputation is lagging indicator (slow to recover)
- High reputation = price premium

---

## 9. Factory Layout

### 9.1 Zone Types

| Zone | Contents |
|------|----------|
| Production | All manufacturing slots |
| Admin | Research, Marketing, Design offices |
| Import | Resource importers (edges only) |
| Export | Vehicle export points |

**Strict Separation**: Admin buildings cannot be placed in production areas.

### 9.2 Layout Strategies

**Serial Layout**:
```
[Chassis] → [Body] → [Paint] → [Engine] → [Accessories] → [QA] → [Export]
```
- Simple to understand
- Long factory required
- Single point of failure

**Parallel Layout**:
```
[Chassis] → [Body] → [Paint Line 1] ↘
                     [Paint Line 2] → [Merge] → [Engine] → ...
                     [Paint Line 3] ↗
```
- Higher throughput
- Handles bottlenecks
- More complex routing

### 9.3 Conveyor Routing

- Minimize conveyor length (reduces transit time)
- Avoid crossings where possible
- Use smart junctions for mixed production
- Plan for expansion

---

## 10. DLC Content

### 10.1 Design Variety Pack

| Content | Effect |
|---------|--------|
| Additional body visuals | Doubles visual options per body style |
| Price | Cosmetic only |

### 10.2 The Doors That Go Like This

| Content | Effect |
|---------|--------|
| Scissor Doors | Premium feature option |
| Butterfly Doors | Premium feature option |
| Gull-wing Doors | Premium feature option |
| Supercar Body Style | New high-end body type |
| Price | $4.99 |

---

## 11. Unique Mechanics

### 11.1 Retooling Penalties

- Switching body styles mid-production incurs time penalty
- Production line must "retool" for different bodies
- Encourages batch production or dedicated lines
- Smart junctions help manage mixed production

### 11.2 Slot Specialization Trade-off

- Combined slots: Simple but slow
- Specialized slots: Fast but requires more space
- Optimal strategy: Specialize bottleneck slots first
- Paint drying is always the critical path

### 11.3 Smart Junction Routing

Late-game technology enabling:
- Route by body style
- Route by feature set
- Route by powertrain
- Enables single factory, multiple products

### 11.4 No Random Events

Unlike many tycoons:
- No disasters
- No random breakdowns
- No supply chain disruptions
- Pure optimization puzzle

### 11.5 Time Acceleration

| Speed | Use Case |
|-------|----------|
| Pause | Planning layouts |
| 1x | Monitoring production |
| 2x-4x | Waiting for research |
| Max | Accumulating funds |

---

## Appendix: Quick Reference

### Bottleneck Analysis

| Slot | Typical Time | Bottleneck Risk |
|------|--------------|-----------------|
| Dry Undercoat | ~12 min | CRITICAL |
| Dry Finish | ~8 min | High |
| QA | ~5 min | Medium |
| Other slots | ~3-4 min | Low |

### Parallel Line Requirements

To maintain throughput with 12-minute Dry Undercoat:
- 3 parallel paint lines for every 1 chassis line
- Or research efficiency upgrades

### Economic Formulas

```
Profit = Sale Price - (Import Costs + Manufacturing Costs + Overhead)
Sale Price = Base Value + Feature Value - Market Saturation Penalty
Throughput = 1 / Slowest Slot Time
```

---

*Document compiled for game design research purposes. Data sourced from Steam community guides, official documentation, and gameplay analysis.*
