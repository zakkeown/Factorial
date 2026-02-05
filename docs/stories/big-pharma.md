# Big Pharma: Comprehensive Game Design Research Document

## Table of Contents
1. [Game Overview](#1-game-overview)
2. [Initial Conditions](#2-initial-conditions)
3. [Core Mechanics](#3-core-mechanics)
4. [The Ingredient Line System](#4-the-ingredient-line-system)
5. [Machines](#5-machines)
6. [Research and Tech Tree](#6-research-and-tech-tree)
7. [Explorer System](#7-explorer-system)
8. [Production Mechanics](#8-production-mechanics)
9. [Drug Packaging and Delivery](#9-drug-packaging-and-delivery)
10. [Business Simulation](#10-business-simulation)
11. [Map and Building Layout](#11-map-and-building-layout)
12. [Campaign vs Sandbox Modes](#12-campaign-vs-sandbox-modes)
13. [DLC Content](#13-dlc-content)

---

## 1. Game Overview

### Basic Information

| Attribute | Value |
|-----------|-------|
| **Developer** | Twice Circled |
| **Publisher** | Positech Games |
| **Release Date** | August 27, 2015 |
| **Platforms** | PC, Mac, Linux, Nintendo Switch |
| **Genre** | Puzzle/Simulation/Tycoon, Factory-Building |

### Premise

Players take on the role of CEO of a pharmaceutical company, producing cures for over 40 different maladies using an assembly line approach. The core gameplay revolves around the unique "concentration" mechanic - processing ingredients through machines to activate beneficial effects while removing side effects.

### Key Features

- Assembly line drug production with concentration-based mechanics
- Over 40 curable diseases across 11 ingredient categories
- Research tree with 40 technology nodes
- Competitor AI companies
- Dynamic market with supply/demand economics

### Design Philosophy

- **Puzzle-like optimization**: Production lines require spatial and logical problem-solving
- **Economic simulation**: Dynamic markets with competition
- **Unique core mechanic**: Concentration system differentiates from traditional factory games

---

## 2. Initial Conditions

### Starting Resources

| Scenario Type | Starting Cash | Loan Amount | Goal |
|---------------|---------------|-------------|------|
| Beginner | Higher than average | $250,000 loan to repay | Become debt-free within 6 years |
| Standard | Varies by scenario | Varies | Scenario-specific objectives |
| Custom | Configurable | Configurable | Player-defined |

### Initial Equipment Available

| Machine | Function | Research Required |
|---------|----------|-------------------|
| Belts | Basic transport | None (free) |
| Evaporator | +1 Concentration | Basic Pharma (free) |
| Dissolver | -1 Concentration | Basic Pharma (free) |
| Pill Printer | Creates pills for sale | Basic Pharma (free) |
| Analyzer | Discovers max strength concentration | Basic Pharma (free) |

### Early Game Strategy

- Start with 1-2 ingredients until more capital is available
- Prioritize ingredients with profitable upgrade paths
- Consider restarting if initial ingredient cures lack lucrative upgrades
- Take loans with the lowest daily payment for easier early repayment

---

## 3. Core Mechanics

### 3.1 Concentration System

The concentration level is the fundamental mechanic that governs all drug processing:

| Property | Value |
|----------|-------|
| Range | 0-20 |
| Display | Blue box indicator |
| Max Strength | Random per game; discovered via Analyzer |

**How It Works**:
- Each effect (cure, side effect, booster) has a specific concentration range where it becomes active
- Machines modify concentration up or down to activate/deactivate effects
- Goal: Reach concentration that activates cures while deactivating side effects

### 3.2 Effect Types

| Effect Type | Color | Description |
|-------------|-------|-------------|
| Cures | Green bars | Positive effects that treat diseases |
| Side Effects | Red bars | Negative effects that reduce drug value |
| Boosters | Yellow bars | Sales bonuses; active at ALL concentrations (0-20) |

### 3.3 Effect Slots

- Each ingredient has **up to 4 effect slots**
- Ingredients start with minimum 2 effects (at least one cure, at least one side effect)
- Empty slots can be filled via Multimixer
- Effects can be repositioned via Shaker
- Effects can be swapped between ingredients via Centrifuge

### 3.4 Catalysts

Catalysts are special properties required for upgrading cures to higher levels:

- Displayed as stylized color-coded "molecule" symbols (2-6 dots)
- Any effect type can also be a catalyst
- **Critical**: Catalysts have their own "active ranges" and can be accidentally removed during concentration processing
- If an ingredient lacks a required catalyst, it must be added via Multimixer

### 3.5 Removing Side Effects

| Method | How It Works |
|--------|--------------|
| Machine Processing | Hover over effect to see required concentration and machine |
| Centrifuge | Swap side effects to an empty slot on another ingredient |
| Booster Replacement | Use Booster Mixer to replace side effects with boosters |
| Shaker Repositioning | Move effects to different slots for easier processing |

---

## 4. The Ingredient Line System

This is the unique core mechanic that differentiates Big Pharma from other factory games.

### Production Flow

```
Import → Processing Chain → Effect Activation → Upgrade Path → Packaging → Export
```

1. **Import**: Raw ingredients enter from wall ports at a set concentration level
2. **Processing Chain**: Ingredients pass through machines that modify concentration
3. **Effect Activation**: As concentration changes, different effects activate/deactivate
4. **Upgrade Path**: Cures can be upgraded by reaching specific concentrations with catalysts
5. **Packaging**: Final product is packaged via Pill Printer, Creamer, Syringe Injector, or Sachet Fabricator
6. **Export**: Finished drugs exit through wall ports to be sold

### Timing and Throughput

| Metric | Value |
|--------|-------|
| Processing unit | "Ticks" (~2-3 seconds at normal speed) |
| Belt crossing time | 2 ticks |
| Import rate | 1 ingredient per clock beat |
| Export rate (no Packer) | 1 product/second |
| Export rate (with Packer) | 7 products/second |

---

## 5. Machines

### 5.1 Concentration Modifying Machines

| Machine | Effect | Research Required | Process Cost | Notes |
|---------|--------|-------------------|--------------|-------|
| Evaporator | +1 Concentration | Basic Pharma (free) | $10 | Entry-level increase |
| Dissolver | -1 Concentration | Basic Pharma (free) | $10 | Entry-level decrease |
| Agglomerator | +3 Concentration | 1 scientist, 3 months | $35→$20 | Efficient increase |
| Ioniser | -3 Concentration | 1 scientist, 3 months | $35 | Efficient decrease |
| Autoclave | Halves concentration | 4 scientists, 8 months | $45 | Tier 3; divides by 2 |
| Cryogenic Condenser | Doubles concentration | Research required | High | Tier 3; multiplies by 2 |
| Chromatograph | +10 if <11, else -10 | 7 scientists, 9 months | $50 | Dramatic swings |
| Sequencer | Sets exact concentration | Late-game research | Very high | Choose any concentration |
| Hadron Collider | All effects active | End of research tree | Very high | Game-breaking; ignores concentration |

### 5.2 Effect Manipulation Machines

| Machine | Function | Research Required | Process Cost |
|---------|----------|-------------------|--------------|
| Shaker | Moves effect down 1-3 slots | Research required | Varies |
| Multimixer | Combines 2 ingredients; keeps base + adds from other | 2 scientists, 3 months | $30 |
| Centrifuge | Swaps specified effect slots between 2 ingredients | 5 scientists, 7 months | $40→$20 |
| Booster Mixer | Transfers boosters; replaces effects in matching slots | DLC/Research | Varies |

### 5.3 Multimixer Mechanics

- Takes two inputs (A and B)
- **Base ingredient** (Input A by default) keeps all its effects
- Effects from non-base ingredient fill empty slots only
- Non-base effects in occupied slots are **permanently lost**
- Output concentration matches base ingredient's concentration
- "Toggle Base" button swaps which input is the base
- If all 4 slots full, must remove an effect first using Shaker

### 5.4 Centrifuge Mechanics

- Takes two inputs
- Swaps specified effect slots between ingredients (A→B and B→A)
- "Green flagged" components are swapped, even if swapping with empty slots
- Useful for removing "Cannot be removed" side effects

### 5.5 Analysis and Utility Machines

| Machine | Function | Notes |
|---------|----------|-------|
| Analyzer | Discovers max strength concentration | 50% success at L0, 100% at L3 |
| Stock Gate | Limits production of specific drug | Prevents oversaturation |
| Packer | Combines products for efficient export | 7 products/second vs 1/second |

---

## 6. Research and Tech Tree

### 6.1 Overview

| Property | Value |
|----------|-------|
| Total research nodes | 40 |
| Structure | Hierarchical with dependencies |
| Requirements | Scientists (staff) + Time (in-game months) |
| Idle bonus | Scientists generate Research Upgrade Points |

### 6.2 Scientist Economics

| Cost Type | Amount |
|-----------|--------|
| Hiring Fee | $3,000 |
| Daily Salary | $30/day |
| Monthly Salary | ~$900/month |

### 6.3 Key Research Nodes

| Research | Scientists | Months | Effect |
|----------|------------|--------|--------|
| Advanced Construction Techniques | 2 | 3 | Machine price: 90%→60% |
| Agglomerator | 1 | 3 | +3 concentration machine |
| Ioniser | 1 | 3 | -3 concentration machine |
| Multimixer | 2 | 3 | Combine ingredients |
| Autoclave | 4 | 8 | Halve concentration |
| Centrifuge | 5 | 7 | Swap effects |
| Chromatograph | 7 | 9 | +10/-10 concentration |
| Creamer | 2 | 3 | Side effect reduction packaging |

---

## 7. Explorer System

### Explorer Economics

| Cost Type | Amount |
|-----------|--------|
| Daily Salary | $50/day |
| Discovery Time | ~3 months per ingredient |

### Ingredient Categories (11 Total)

1. Blood
2. Body Response
3. Digestion
4. Infection
5. Liver
6. Lungs
7. Pain
8. Psychological
9. Relaxants
10. Sexual Health
11. Skin

---

## 8. Production Mechanics

### 8.1 Cure Rating System

Ratings affect drug value and market perception:

| Rating | Threshold | Price Modifier |
|--------|-----------|----------------|
| S++ | >= 160 | Highest |
| S+ | >= 130 | Very High |
| S | >= 114 | High |
| A+ | >= 99 | +20% |
| A | >= 89 | +20% |
| B+ | >= 79 | Moderate |
| B | >= 69 | Moderate |
| C+ | >= 59 | Slight |
| C | >= 51 | Baseline |
| D | >= 44 | Slight penalty |
| E | >= 30 | -20% |
| F | < 20 | Lowest |

### 8.2 Rating Calculation Formula

```
p = 1 - (300 / (x + 300))

Where:
- p = percentage progress from 55 to final rating
- x = number of products sold
- Rating starts at 55 and approaches final rating asymptotically
```

---

## 9. Drug Packaging and Delivery

### Packaging Machine Comparison

| Machine | Process Cost | Value Bonus | Special Effect |
|---------|--------------|-------------|----------------|
| Pill Printer | $20 | Up to $35 | None |
| Creamer | $30 | $30 flat | Reduces side effect severity 50-75% |
| Syringe Injector | $100 (L5) | Value x 1.4 - $100 | Best for high-value drugs >$287.50 |
| Sachet Fabricator | Varies | $30-$55 per cure | Bonus scales with number of cures |

### When to Use Each

- **Pill Printer**: Default, low-cost option
- **Creamer**: When you can't remove all side effects
- **Syringe Injector**: High-value premium drugs only
- **Sachet Fabricator**: Multi-cure drugs

---

## 10. Business Simulation

### 10.1 Market Dynamics

| Metric | Value |
|--------|-------|
| 100% Saturation | Monthly supply = 10% of total sufferers |
| Oversaturation Warning | >500% saturation can drop prices to $0 |

### 10.2 Patent System

| Property | Details |
|----------|---------|
| Duration | 1-10 years |
| Scope | Covers ACTIVE effects only, not final form |
| Workarounds | Different packaging or modified effects = different product |

### 10.3 Competition

- AI competitor companies research independently
- Competitors can saturate markets
- First-mover advantage on new cures
- Patent blocking strategies possible

---

## 11. Map and Building Layout

### Layout Constraints

| Property | Value |
|----------|-------|
| Grid type | Square tile-based |
| Maximum size | Up to 4x4 building grid configuration |
| Port throughput (no Packer) | 1 product/second |
| Port throughput (with Packer) | 7 products/second |

### Strategy

- Prioritize more ports over more floor space early game
- Plan production lines to minimize belt crossings
- Leave expansion room for late-game complexity

---

## 12. Campaign vs Sandbox Modes

### Campaign Mode

| Property | Value |
|----------|-------|
| Total levels | 35 |
| Scenarios | 7 unique scenarios |
| Difficulty progression | Tutorial → Beginner → Advanced → Specialty |
| Mastery tiers | 3 per scenario |

### Sandbox Mode

- Custom game generator available
- Configurable: difficulty, resources, research, time limits, competition
- Full freedom to experiment

---

## 13. DLC Content

### Marketing and Malpractice DLC

**Release Date**: April 26, 2016

| Feature | Description |
|---------|-------------|
| Disease Awareness Campaigns | Increase demand for your cures |
| Doctor Gifts/Bribery | Boost sales with publicity risk |
| Clinical Trial Manipulation | Permanent bonuses but risk rating damage |
| Manual Price Setting | Dynamic pricing strategy |
| Executives | Hire for marketing tasks (social media to suppressing results) |

### Executive Perks System

Choose 2 perks before each game:

| Perk | Effect |
|------|--------|
| Executive | Start with 3 bonus executives |
| Genius | +20% Research Speed |
| Local Networks | Ingredients start with 50% discount |
| Mixologist | Multimixer and Centrifuge process time = 1 |

---

## Appendix: Quick Reference

### Concentration Change Summary

| Machine | Change | Tier |
|---------|--------|------|
| Evaporator | +1 | Basic |
| Dissolver | -1 | Basic |
| Agglomerator | +3 | Tier 2 |
| Ioniser | -3 | Tier 2 |
| Autoclave | ÷2 | Tier 3 |
| Cryogenic Condenser | x2 | Tier 3 |
| Chromatograph | +10/-10 | Tier 3 |
| Sequencer | Set exact | Late |
| Hadron Collider | All active | End |

### Cost Breakdown Example (Per Drug)

```
Raw Ingredient Import: $X
+ Processing Costs (machines): $Y
+ Packaging Cost: $Z
= Total Production Cost

Profit = Sale Price - Total Production Cost
```

---

*Document compiled for game design research purposes. Data sourced from Big Pharma Wiki, Steam Community guides, and official documentation.*
