# Builderment: Comprehensive Game Design Research Document

## Table of Contents
1. [Game Overview](#1-game-overview)
2. [Initial Conditions](#2-initial-conditions)
3. [Core Mechanics](#3-core-mechanics)
4. [Resource Types and Production Chains](#4-resource-types-and-production-chains)
5. [Buildings and Machines](#5-buildings-and-machines)
6. [Technology/Research Tree](#6-technologyresearch-tree)
7. [World and Map](#7-world-and-map)
8. [Logistics Systems](#8-logistics-systems)
9. [Power System](#9-power-system)
10. [Endgame](#10-endgame)
11. [Unique Mechanics](#11-unique-mechanics)
12. [Monetization](#12-monetization)

---

## 1. Game Overview

### Basic Information

| Attribute | Value |
|-----------|-------|
| **Developer** | Builderment LLC |
| **Platforms** | iOS, Android (2021), Steam/PC (2023) |
| **Genre** | Relaxing factory building/automation |
| **Perspective** | 2D top-down |
| **Combat** | None |

### Premise

The player extracts resources from a distant planet and builds automated factories to process materials, ultimately sending items back to Earth. Unlike Factorio or Satisfactory, Builderment has no combat or survival elements - it's purely focused on factory optimization and logistics.

### Design Philosophy

- **Mobile-first**: Originally designed for touch interfaces with shorter play sessions
- **Relaxing**: No enemies, no time pressure, no failure states
- **Accessible**: Simplified mechanics compared to complex factory games
- **Infinite**: Procedurally generated world with unlimited expansion

---

## 2. Initial Conditions

### Starting State

- Player begins with a small explored area of the procedural map
- Basic resources (Wood, Iron Ore, Copper Ore, Stone, Coal) visible nearby
- Tutorial guides through first extractor and conveyor belt placement
- 100 gems (premium currency) provided at start

### First Objectives

1. Place first Extractor on a resource node
2. Connect Extractor to first production building via conveyor belt
3. Learn item routing and belt mechanics
4. Build first Research Lab to begin technology unlocks

### Tutorial Progression

The tutorial introduces mechanics incrementally:
1. Resource extraction basics
2. Conveyor belt placement and routing
3. Production building inputs/outputs
4. Research Lab and technology unlocking
5. Multi-input recipes
6. Splitters and mergers

---

## 3. Core Mechanics

### 3.1 Resource Extraction

Resources exist as nodes on the map that can be harvested infinitely by Extractors.

**Extractor Tiers**:

| Tier | Speed Multiplier | Unlock Method |
|------|------------------|---------------|
| T1 Extractor | 1.0x | Starting |
| T2 Extractor | 1.5x | Research |
| T3 Extractor | 2.0x | Research |
| T4 Extractor | 3.0x | Research |

### 3.2 Conveyor Belts

Belts transport items between buildings. Items flow in one direction along the belt.

**Belt Tiers**:

| Tier | Speed | Notes |
|------|-------|-------|
| T1 Belt | Base speed | Starting |
| T2 Belt | Faster | Research unlock |
| T3 Belt | Fastest | Late-game research |

**Belt Mechanics**:
- Items occupy discrete positions on belts
- Belts can be placed in straight lines or with turns
- Underground belts allow crossing other belt lines
- Items back up if destination is full (no loss)

### 3.3 Production Flow

```
Extractor → Belt → Production Building → Belt → Next Building/Research Lab
```

Each production building has:
- Input slots (1-4 depending on building tier)
- Output slot (1 item type)
- Processing time per item
- Recipe selection (some buildings support multiple recipes)

---

## 4. Resource Types and Production Chains

### 4.1 Raw Resources (7 Types)

| Resource | Color | Abundance | Notes |
|----------|-------|-----------|-------|
| Wood | Brown | Common | Organic resource |
| Copper Ore | Orange | Common | Basic metal |
| Iron Ore | Gray | Common | Basic metal |
| Stone | Beige | Common | Construction material |
| Coal | Black | Common | Fuel and graphite |
| Tungsten Ore (Wolframite) | Dark gray | Rare | Advanced metal |
| Uranium | Green | Very Rare | Nuclear fuel |

### 4.2 Basic Processing Recipes

**Furnace Recipes** (1 input → 1 output):

| Input | Output | Ratio |
|-------|--------|-------|
| Copper Ore | Copper Ingot | 1:1 |
| Iron Ore | Iron Ingot | 1:1 |
| Stone | Sand | 1:1 |
| Sand | Glass | 1:1 |
| Tungsten Ore | Tungsten Ingot | 1:1 |

**Workshop Recipes** (1 input → 1 output):

| Input | Output | Ratio |
|-------|--------|-------|
| Wood | Wood Plank | 1:1 |
| Iron Ingot | Iron Gear | 1:1 |
| Copper Ingot | Copper Wire | 3:1 (3 ingots → 1 wire) |

### 4.3 Intermediate Products

**Machine Shop Recipes** (2 inputs → 1 output):

| Inputs | Output |
|--------|--------|
| Iron Gear + Copper Wire | Motor |
| Wood Plank + Iron Ingot | Wood Frame |
| Glass + Copper Wire | Light Bulb |
| Sand + Coal | Graphite |

**Forge Recipes** (2 inputs → 1 output):

| Inputs | Output |
|--------|--------|
| Tungsten Ore + Graphite | Tungsten Carbide (10:1 ratio) |
| Iron Ingot + Coal | Steel |
| Copper Ingot + Iron Ingot | Bronze |

### 4.4 Advanced Products

**Industrial Factory Recipes** (3 inputs → 1 output):

| Inputs | Output |
|--------|--------|
| Motor + Steel + Copper Wire | Electric Motor |
| Glass + Copper Wire + Steel | Circuit Board |
| Wood Frame + Motor + Light Bulb | Basic Robot |

**Manufacturer Recipes** (4 inputs → 1 output):

| Inputs | Output |
|--------|--------|
| Circuit Board + Electric Motor + Steel + Glass | Computer |
| Computer + Tungsten Carbide + Electric Motor + Circuit Board | Super Computer |
| Computer + Uranium + Tungsten Carbide + Electric Motor | Matter Duplicator |

### 4.5 Key Production Ratios

For balanced production lines:

| Product | Copper Ore | Iron Ore | Wood | Stone | Coal |
|---------|------------|----------|------|-------|------|
| Copper Wire | 3 | - | - | - | - |
| Motor | 3 | 1 | - | - | - |
| Circuit Board | 3 | 1 | - | 1 | 1 |

### 4.6 Alternative Recipes

Builderment features 12 alternative recipes that allow different production paths:
- Provides flexibility when certain resources are scarce
- Allows optimization for specific map layouts
- Unlocked through research

---

## 5. Buildings and Machines

### 5.1 Production Buildings

| Building | Inputs | Outputs | Tier | Notes |
|----------|--------|---------|------|-------|
| Furnace | 1 | 1 | Basic | Smelting ores |
| Workshop | 1 | 1 | Basic | Basic crafting |
| Machine Shop | 2 | 1 | Intermediate | Component assembly |
| Forge | 2 | 1 | Intermediate | Metal processing |
| Industrial Factory | 3 | 1 | Advanced | Complex items |
| Manufacturer | 4 | 1 | End-game | Final products |

### 5.2 Logistics Buildings

| Building | Function |
|----------|----------|
| Splitter | Divides belt into 2 outputs (alternating items) |
| Item Splitter | Filters specific item types to different outputs |
| Merger | Combines 2 belt inputs into 1 output |
| Underground Belt | Allows belts to cross under other belts |
| Storage | Buffers items (limited capacity) |

### 5.3 Special Buildings

| Building | Function |
|----------|----------|
| Research Lab | Receives items to unlock technologies (max 3 labs) |
| Power Plant (Coal) | Boosts speed of nearby factories |
| Power Plant (Nuclear) | Greater speed boost, requires Uranium |
| Robotic Arm | Pick up, place, and filter items (v1.3.14+) |
| Earth Teleporter | Endgame goal, sends items to Earth |

---

## 6. Technology/Research Tree

### 6.1 Research Mechanics

- Items are sent to Research Labs to unlock technologies
- Up to 3 Research Labs can operate simultaneously
- Each technology requires specific items and quantities
- Technologies can alternatively be unlocked with gems

### 6.2 Research Categories

| Category | Unlocks |
|----------|---------|
| Buildings | New production building types |
| Extractors | Higher tier extractors (T2, T3, T4) |
| Power | Coal and Nuclear power plants |
| Logistics | Splitters, mergers, underground belts, robotic arms |
| Recipes | Alternative recipes for production flexibility |
| Decorations | Aesthetic items (no gameplay effect) |

### 6.3 Key Research Milestones

**Early Game**:
- T2 Extractor (1.5x speed)
- Machine Shop (2-input recipes)
- Basic splitters and mergers

**Mid Game**:
- T3 Extractor (2.0x speed)
- Forge and Industrial Factory
- Coal Power Plant
- Item Splitter (filtering)

**Late Game**:
- T4 Extractor (3.0x speed)
- Manufacturer
- Nuclear Power Plant
- Robotic Arms
- Alternative Recipes

---

## 7. World and Map

### 7.1 Map Generation

- **Infinite procedural generation**: Map expands as player explores
- **Resource distribution**: Random placement with clustering
- **Biomes**: Visual variety but no gameplay impact
- **No terrain obstacles**: Flat building surface everywhere

### 7.2 Exploration

- Fog of war covers unexplored areas
- New resource nodes discovered through exploration
- Rare resources (Tungsten, Uranium) found further from start
- No exploration costs or requirements

### 7.3 Map Seed System

- Each world has a unique seed
- Seeds can be shared for identical map layouts
- Community shares seeds with good resource distributions

---

## 8. Logistics Systems

### 8.1 Belt Mechanics

**Throughput Calculation**:
```
Items per minute = Belt speed × Items per tile × 60
```

**Compression**:
- Belts can be fully compressed (no gaps between items)
- Mergers help achieve full compression
- Splitters maintain compression on both outputs

### 8.2 Splitter Behavior

**Standard Splitter**:
- Alternates items between left and right outputs
- Even distribution when both paths are clear
- Items prioritize clear path if one side is backed up

**Item Splitter** (Filtered):
- Sends specified item type to one output
- All other items go to second output
- Essential for mixed-item belt management

### 8.3 Robotic Arms (v1.3.14+)

- Pick up items from belts or buildings
- Place items onto belts or into building inputs
- Can filter for specific item types
- Enable more complex factory layouts

### 8.4 Blueprint System

- Save factory sections as blueprints
- Share blueprints via unique ID codes
- Import community blueprints
- Blueprints preserve all belt and building configurations

---

## 9. Power System

### 9.1 Power Plants

| Type | Fuel | Effect Radius | Speed Boost | Notes |
|------|------|---------------|-------------|-------|
| Coal Power Plant | Coal | Medium | Moderate | Early-mid game |
| Nuclear Power Plant | Uranium | Large | High | Late game |

### 9.2 Power Mechanics

- Power plants boost processing speed of nearby buildings
- Effect stacks with multiple power plants
- No negative effects (no pollution, no danger)
- Power plants require continuous fuel supply

### 9.3 Optimization

- Place power plants centrally among production clusters
- Nuclear plants more efficient for dense factory areas
- Calculate fuel consumption vs. production speed gains

---

## 10. Endgame

### 10.1 Victory Condition

The primary goal is building the **Earth Teleporter** and maximizing **Earth Token** production.

### 10.2 Top-Tier Items

| Item | Complexity | Purpose |
|------|------------|---------|
| Super Computer | Very High | Research, Earth Tokens |
| Matter Duplicator | Highest | Earth Tokens |
| Earth Token | Final | Victory metric |

### 10.3 Infinite Progression

- No true "end" - players optimize for higher production rates
- Community benchmarks for items per minute
- Megabase building for maximum throughput

### 10.4 Optimization Goals

- Maximize Earth Token production rate
- Achieve full belt compression throughout factory
- Minimize wasted space and belt length
- Balance resource extraction with consumption

---

## 11. Unique Mechanics

### 11.1 No Combat

Unlike Factorio, Satisfactory, and most factory games:
- No enemies to defend against
- No pollution or environmental mechanics
- No survival elements (food, health, etc.)
- Pure focus on logistics optimization

### 11.2 Mobile-First Design

- Touch-optimized interface
- Simpler production chains than PC-first games
- Shorter session-friendly progression
- Portrait and landscape mode support

### 11.3 Seed Splitters

A unique optimization mechanic:
- Split single resource nodes to multiple extractors
- Increases effective extraction rate per node
- Community-discovered advanced technique

### 11.4 Gem Tree

- Special building purchasable with gems (250 gems)
- Generates gems over time
- Premium currency investment mechanic

---

## 12. Monetization

### 12.1 Free-to-Play Model

- Core game fully playable without payment
- 100 gems provided at start
- Gems can be earned by watching ads
- No pay-to-win mechanics

### 12.2 Gem Uses

| Use | Cost | Notes |
|-----|------|-------|
| Unlock technologies | Variable | Skip research |
| Gem Tree | 250 gems | Generates gems over time |
| Cosmetics | Variable | Visual customization |

### 12.3 Premium Purchase Options

- Gem packs for real money
- One-time ad removal option (PC version is premium)
- No subscription model

---

## Appendix: Quick Reference

### Production Building Summary

| Building | Inputs | Tier | Example Recipe |
|----------|--------|------|----------------|
| Furnace | 1 | 1 | Ore → Ingot |
| Workshop | 1 | 1 | Plank → Frame |
| Machine Shop | 2 | 2 | Gear + Wire → Motor |
| Forge | 2 | 2 | Ore + Graphite → Carbide |
| Industrial Factory | 3 | 3 | Motor + Steel + Wire → E-Motor |
| Manufacturer | 4 | 4 | 4 components → Computer |

### Extractor Speed Summary

| Tier | Multiplier | Relative Output |
|------|------------|-----------------|
| T1 | 1.0x | 100% |
| T2 | 1.5x | 150% |
| T3 | 2.0x | 200% |
| T4 | 3.0x | 300% |

---

*Document compiled for game design research purposes. Data sourced from Builderment Wiki, Steam Community guides, and community resources.*
