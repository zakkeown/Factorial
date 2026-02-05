# Oxygen Not Included - Game Design Research Document

## Table of Contents

1. [Game Overview](#game-overview)
2. [Initial Conditions](#initial-conditions)
3. [Core Mechanics](#core-mechanics)
4. [Physics Simulation](#physics-simulation)
5. [Duplicant Management](#duplicant-management)
6. [Technology and Research Tree](#technology-and-research-tree)
7. [Production Chains](#production-chains)
8. [World and Map](#world-and-map)
9. [Colony Survival](#colony-survival)
10. [Automation and Logic Systems](#automation-and-logic-systems)
11. [Space Exploration](#space-exploration)
12. [Endgame](#endgame)

---

## Game Overview

Oxygen Not Included is a 2D colony simulation game developed by Klei Entertainment. Players manage a space colony of "Duplicants" inside an asteroid, dealing with complex physics simulations including gas/liquid dynamics, temperature management, and resource processing. The game emphasizes survival through careful management of oxygen, food, power, and colony morale.

---

## Initial Conditions

### Starting Biome

**Terra (Default Starting Asteroid)**
- Temperature: Temperate (~20-25C)
- Atmosphere: Oxygen-rich starting area
- Initial Resources:
  - Water pools (typically 3 pools around starting area)
  - Algae deposits
  - Dirt
  - Sandstone
  - Copper ore
- Best for first-time players; well-balanced with no additional difficulty

**Starting Resources in Pod Area:**
- Printing Pod (colony centerpiece)
- Small oxygen pocket
- Access to diggable terrain containing:
  - Sandstone tiles
  - Algae
  - Dirt
  - Basic metal ores

### Duplicant Selection

At game start, players choose 3 Duplicants from a selection. Each Duplicant has:

**Attributes (11 total):**
| Attribute | Effect per Point |
|-----------|------------------|
| Athletics | +10% movement speed |
| Construction | +25% building speed |
| Excavation/Digging | +25% digging speed |
| Strength | +15% carry capacity |
| Science | Faster research |
| Cuisine | Cooking efficiency |
| Creativity | Art/decor creation |
| Medicine | Medical tasks |
| Agriculture | Farming efficiency |
| Husbandry | Ranching efficiency |
| Machinery | Operating machines |

**Interests:**
- 1-3 random interests per Duplicant
- Interest bonuses to starting attributes:
  - 1 interest: +7 to related attribute
  - 2 interests: +3 each
  - 3 interests: +1 each
- Interests reduce morale cost of related skills by 1

### Printing Pod Mechanics

**Cycle Timing:**
- Offers new selection every 3 cycles
- Provides choice between:
  - 2-3 randomly generated Duplicants
  - 1-2 Care Packages (items, food, resources)

**Selection Options:**
- Accept one Duplicant or Care Package
- "Reject All" to skip and wait another 3 cycles
- Exiting dialogue does NOT reject (can return later)
- Cooldown only starts after selection or rejection

**Care Packages:**
- Unlock based on discovery conditions
- Contains materials, food, critters, or seeds
- Useful for obtaining resources not available on current asteroid

---

## Core Mechanics

### Resource Types

**Element Categories:**

| Category | Examples |
|----------|----------|
| Gases | Oxygen, Carbon Dioxide, Hydrogen, Chlorine, Natural Gas, Polluted Oxygen |
| Liquids | Water, Polluted Water, Crude Oil, Petroleum, Magma, Super Coolant |
| Solite | Sandstone, Granite, Obsidian, Igneous Rock, Abyssalite |
| Metals | Copper Ore, Iron Ore, Gold Amalgam, Aluminum Ore |
| Refined Metals | Copper, Iron, Gold, Steel, Aluminum, Thermium, Niobium |
| Organics | Algae, Slime, Dirt, Fertilizer, Coal |
| Consumables | Food items, Seeds |
| Special | Fullerene, Isoresin, Super Coolant |

**Key Material Properties:**

| Material | Thermal Conductivity | Special Property |
|----------|---------------------|------------------|
| Abyssalite | 0.00001 DTU/m/s/C | Best natural insulator |
| Copper (Refined) | High | Excellent conductor |
| Diamond | Very High | Best thermal conductor |
| Thermium | High | +900C overheat bonus for buildings |
| Super Coolant | 9.46 DTU/m/s/C | Lowest freezing point (-271.15C) |

### Digging

- Duplicants dig tiles to gather resources and expand colony
- Dig speed affected by Excavation attribute (+25% per point)
- Different materials have different hardness levels
- Abyssalite cannot be dug without Hard Digging skill
- Obsidian requires Super-Hard Digging skill

### Building

**Building Materials:**
- Most buildings allow material selection
- Material affects:
  - Overheat temperature
  - Decor value
  - Thermal properties

**Construction Speed:**
- Base speed modified by Construction attribute (+25% per point)

### Power Generation

| Generator | Power Output | Fuel Input | Byproducts |
|-----------|-------------|------------|------------|
| Manual Generator | 400W | Duplicant labor | None |
| Coal Generator | 600W | Coal | CO2, Heat |
| Natural Gas Generator | 800W | 90g/s Natural Gas | CO2, Polluted Water |
| Hydrogen Generator | 800W | 100g/s Hydrogen | None |
| Petroleum Generator | 2000W | 2kg/s Petroleum | 500g/s CO2, 750g/s Polluted Water (40C min) |
| Solar Panel | 0-380W | Sunlight (51,213+ lux for max) | None |
| Steam Turbine | 242-850W | Steam (125C+ minimum) | Water (95C output) |

**Power Circuits and Wires:**

| Wire Type | Capacity | Decor Penalty |
|-----------|----------|---------------|
| Wire | 1,000W (1kW) | -5 |
| Conductive Wire | 2,000W (2kW) | -5 |
| Heavi-Watt Wire | 20,000W (20kW) | -25 (6 tile radius) |
| Heavi-Watt Conductive Wire | 50,000W (50kW) | -20 (4 tile radius) |

**Batteries:**

| Battery Type | Capacity | Power Loss/Cycle | Heat Output |
|--------------|----------|------------------|-------------|
| Battery | 10 kJ | Variable | Low |
| Jumbo Battery | 40 kJ | 2 kJ/cycle | 1.25 kDTU |
| Smart Battery | 20 kJ | 0.4 kJ/cycle | 0.5 kDTU |

### Conveyor and Shipping Systems

**Throughput:** 20 kg/s

**Components:**

| Component | Capacity | Function |
|-----------|----------|----------|
| Conveyor Loader | 1000 kg storage | Loads items onto rails |
| Conveyor Rail | N/A | Transports items |
| Conveyor Receptacle | 100 kg storage | Unloads items from rails |
| Conveyor Chute | N/A | Drops items |
| Conveyor Meter | N/A | Controls/measures flow |

### Piping Systems

**Liquid Pipes:**
- Standard throughput: 10 kg/s
- Mini Liquid Pump: 1 kg/s at 60W
- Liquid Pump: 10 kg/s at 240W

**Gas Pipes:**
- Standard throughput: 1 kg/s
- Mini Gas Pump: 0.1 kg/s at 60W
- Gas Pump: 0.5 kg/s at 240W

**Pipe Types:**

| Type | Thermal Property |
|------|------------------|
| Standard | Normal heat exchange |
| Insulated | 3.125% thermal conductivity of material |
| Radiant | 200% thermal conductivity (2x normal) |

---

## Physics Simulation

### Gas, Liquid, and Solid States

**State Transitions:**
- Materials change state at specific temperatures
- Melting: Solid to Liquid
- Boiling/Evaporation: Liquid to Gas
- Condensation: Gas to Liquid
- Freezing: Liquid to Solid

**Gas Behavior:**
- Gases layer by density (CO2 sinks, Hydrogen rises)
- Pressure measured in g/tile
- Overpressure mechanics for some buildings (stop at 1800-2000g)

**Liquid Behavior:**
- Flows and pools based on gravity
- Can be pumped through pipes
- Different densities create layering

### Temperature System

**Heat Transfer Mechanics:**
- Thermal Conductivity: DTU/m/s/C (energy transfer rate)
- Specific Heat Capacity: DTU/g/C (energy storage)
- Heat exchange between adjacent tiles
- Gas-to-solid exchange multiplied by 25x

**Temperature Ranges:**

| Material | Freezing Point | Boiling Point |
|----------|---------------|---------------|
| Water | 0C / 273.15K | 100C / 373.15K |
| Polluted Water | -20C | 120C |
| Crude Oil | -40C | 400C |
| Petroleum | -57C | 539C |
| Super Coolant | -271.15C | 436.85C |

**Insulation:**
- Abyssalite: Natural insulator between biomes
- Insulated Tiles: 3.125% conductivity
- Minimum 2 tiles thick for effective insulation

### Pressure System

**Overpressure Limits:**
- Electrolyzer: 1800g gas pressure
- Oxygen Diffuser: 1800g gas pressure
- Most gas-producing buildings stop at ~2000g

**Vacuum:**
- 0g pressure in space
- Used for insulation (no heat transfer)
- Required for some space operations

### Heat Transfer and Deletion

**Heat Deletion Methods:**

1. **Steam Turbine:**
   - Converts steam (125C+) to power and 95C water
   - Deletes 90% of heat taken from steam
   - 10% applied to turbine itself
   - Max output: 850W at 200C+ steam

2. **Thermo Aquatuner + Steam Turbine:**
   - Aquatuner: 1200W, cools liquid by 14C
   - Ratio: 2 Steam Turbines per 3 Aquatuners (water/polluted water)
   - System stabilizes at ~200C
   - Super Coolant enables self-powering loop

3. **Anti-Entropy Thermo-Nullifier (AETN):**
   - Consumes hydrogen, deletes heat
   - Found as POI, not craftable

**Heat Sources:**
- Duplicants generate body heat
- Machines produce heat during operation
- Hot outputs from buildings (Electrolyzer: 70C minimum)
- Geothermal sources

### Germ Mechanics

**Germ Types:**

| Germ | Habitat | Spread | Danger |
|------|---------|--------|--------|
| Food Poisoning | Polluted Water, Edibles | Contact only (not airborne) | Digestive issues |
| Slimelung | Polluted Oxygen, Slime | Airborne | Reduces productivity, not lethal |
| Zombie Spores | Sporechids | Contact | Makes Duplicants aggressive |
| Floral Scent | Some plants | Airborne | None (decorative) |

**Infection Mechanics:**
- Exposure threshold: >100 germs
- Exposure progression:
  - 10 seconds: Mild Exposure
  - +10 seconds: Medium Exposure
  - +15 seconds: Full Exposure/Infection

**Germ Elimination:**
- Chlorine kills germs in storage tanks
- Radiation: 12.5 germs/s per rad/cycle
- High temperature kills most germs
- Deodorizers convert Polluted Oxygen to Oxygen

---

## Duplicant Management

### Attributes

| Attribute | Function | Bonus per Point |
|-----------|----------|-----------------|
| Athletics | Movement speed | +10% |
| Construction | Building speed | +25% |
| Excavation | Digging speed | +25% |
| Strength | Carry capacity | +15% |
| Science | Research speed | Variable |
| Cuisine | Cooking quality | Variable |
| Creativity | Art creation | Variable |
| Medicine | Medical tasks | Variable |
| Agriculture | Farming | Variable |
| Husbandry | Ranching | Variable |
| Machinery | Machine operation | Variable |

### Traits

**Positive Traits:**

| Trait | Effect |
|-------|--------|
| Quick Learner | +5 Learning |
| Twinkletoes | +5 Athletics |
| Buff | +5 Strength |
| Night Owl | +3 all attributes at night |
| Germ Resistant | +8% immunity regen/cycle |
| Simple Tastes | -1 food quality expectation |

**Negative Traits:**

| Trait | Effect |
|-------|--------|
| Mouth Breather | +100g/s oxygen consumption |
| Flatulence | Produces 5g polluted oxygen periodically |
| Irritable Bowel | 2x bathroom time |
| Loud Sleeper | Disturbs nearby sleepers (3 tile radius) |
| Slow Learner | Reduced learning speed |

### Skills System

**Skill Structure:**
- 12 skill categories
- 8 categories have 3 tiers (I, II, III)
- 4 categories have 2 tiers
- Total: 32 skills available

**Morale Cost:**
- Each skill tier adds its number to morale requirement
- Tier 1 skill: +1 morale needed
- Tier 2 skill: +2 morale needed
- Tier 3 skill: +3 morale needed
- Interest in skill category: -1 morale cost

**Key Skills:**

| Skill Tree | Tier 1 | Tier 2 | Tier 3 |
|------------|--------|--------|--------|
| Digging | Basic digging | Hard digging (Abyssalite) | Super-hard digging (Obsidian) |
| Building | Basic building | Advanced building | - |
| Research | Basic research | Advanced research | Astronomy |
| Ranching | Basic ranching | Critter handling | Critter wrangling |
| Operating | Machine operation | Power control | - |

### Morale System

**Morale Sources:**

| Category | Source | Bonus |
|----------|--------|-------|
| Food Quality | Grisly (-1) to Ambrosial (+6) | -1 to +16 |
| Rooms | Barracks | +1 |
| Rooms | Bedroom | +2 |
| Rooms | Private Bedroom | +3 |
| Rooms | Mess Hall | +3 |
| Rooms | Great Hall | +6 |
| Rooms | Latrine | +1 |
| Rooms | Washroom | +2 |
| Rooms | Nature Reserve (passing through) | +6 |
| Rooms | Park (passing through) | +3 |
| Recreation | Water Cooler | +1 |
| Recreation | Arcade Machine | Variable |

**Morale Requirement:**
- Base: 0
- Per skill tier: +tier number
- Skills with matching interest: -1

### Stress System

**Stress Sources:**
- Low oxygen
- Bad decor
- Poor food quality
- Soiled suit
- Unmet expectations
- Bladder issues
- Sleep interruption

**Stress Threshold:** 100%

**Stress Responses (at 100% stress):**

| Response | Effect |
|----------|--------|
| Ugly Crier | Cries, reduces nearby decor/morale |
| Vomiter | Produces polluted water, loses calories |
| Destructive | Breaks buildings and tiles |
| Binge Eater | Consumes large amounts of food |

**Stress Recovery:**
- Drops to 60% after venting
- Schedule downtime
- High morale
- Massage tables

### Overjoyed Responses

**Trigger:** Morale exceeds requirement by 8+ points

**Chance:** 0.08333% base, up to 0.20833% at 20+ excess morale

| Response | Effect |
|----------|--------|
| Balloon Artist | Gives balloons that boost stats by ~8 |
| Sparkle Streaker | +8 Athletics, +5 to nearby Duplicants |
| Super Productive | 10% chance to trigger production bonus |
| Sticker Bomber | Places decorative stickers (+decor) |

### Food Quality

| Quality Tier | Morale Bonus |
|--------------|--------------|
| Ambrosial (+6) | +16 |
| Superb (+5) | +16 |
| Great (+4) | +12 |
| Good (+3) | +8 |
| Standard (+2) | +4 |
| Poor (+1) | +1 |
| Terrible (0) | 0 |
| Grisly (-1) | -1 |

### Decor

**Effect:** Impacts stress and morale
**Range:** Each item has decor radius
**Stacking:** Highest/lowest values in range apply

**Decor Sources:**

| Source | Decor Value |
|--------|-------------|
| Paintings | +10 to +40 |
| Sculptures | +10 to +40 |
| Plants | +5 to +30 |
| Industrial machinery | -10 to -25 |
| Heavi-Watt Wire | -25 (6 tile radius) |

---

## Technology and Research Tree

### Research Stations

| Station | Unlock | Points Type | Resource Cost |
|---------|--------|-------------|---------------|
| Research Station | Start | Novice | 50 kg Dirt per point |
| Super Computer | Advanced Research tech | Advanced | 50 kg Water per point |
| Virtual Planetarium | Computing tech | Interstellar | Data Banks |

**Total Research Points (Base Game):**
- Novice: 3,015 points
- Advanced: 2,900 points
- Interstellar: 2,800 points

**Total Research Points (Spaced Out!):**
- Novice: 3,490 points
- Advanced: 3,360 points
- Applied Sciences: 2,735 points
- Data Analysis: 1,730 points

### Research Tiers Overview

**Tier 1 (Basic):**
- Basic Farming
- Meal Preparation
- Plumbing
- Power Regulation
- Interior Decor
- Basic research only

**Tier 2:**
- Agriculture
- Ranching
- Sanitation
- Advanced Power Regulation
- Artistic Expression
- Requires Novice research

**Tier 3+:**
- Requires Advanced Research (Super Computer)
- Examples:
  - Filtration
  - Distillation
  - Brute-Force Refinement
  - Temperature Management
  - HVAC
  - Plastics
  - Smart Storage

**Tier 8+:**
- Requires Interstellar Research
- Space-related technologies
- Rocket modules
- Advanced materials

---

## Production Chains

### Food Production

**Plant Growth:**

| Plant | Growth Cycle | Yield | Cal/Harvest | Cal/Cycle | Requirements |
|-------|--------------|-------|-------------|-----------|--------------|
| Mealwood | 3 cycles | Meal Lice | 600 kcal | 200 kcal | 10-30C, 150g+ pressure |
| Bristle Blossom | 6 cycles | Bristle Berry | 1600 kcal | 266 kcal | 5-30C, light, water |
| Sleet Wheat | 18 cycles | Sleet Wheat Grain | 1800 kcal | 100 kcal | -55 to 5C, water |
| Nosh Sprout | 21 cycles | Nosh Bean | Variable | Variable | -25 to 0C, ethanol |

**Food per Duplicant:**
- Consumption: ~1000 kcal/cycle
- Mealwood: 5 plants per Duplicant
- Bristle Blossom: ~4 plants per Duplicant

**Cooked Foods:**

| Food | Ingredients | Calories | Quality |
|------|-------------|----------|---------|
| Mush Bar | 75kg Dirt + 75kg Water | 800 kcal | -1 |
| Liceloaf | Meal Lice + Water | 1700 kcal | 0 |
| Pickled Meal | Meal Lice | 1800 kcal | -1 |
| Gristle Berry | Bristle Berry | 2000 kcal | +1 |
| Stuffed Berry | Gristle Berry + Pincha Pepper | 4400 kcal | +4 |
| Pepper Bread | Sleet Wheat + Pincha Pepper | 4000 kcal | +5 |
| Frost Burger | Multiple | 6000 kcal | +6 |

### Oxygen Generation

**Methods:**

| Method | Input | Output | Power |
|--------|-------|--------|-------|
| Oxygen Diffuser | 550g/s Algae | 500g/s Oxygen | 120W |
| Algae Terrarium | 30g/s Algae + Water | 40g/s Oxygen | 0W |
| Electrolyzer | 1kg/s Water | 888g/s O2 + 112g/s H2 | 120W |
| Rust Deoxidizer | Salt + Rust | Oxygen + Chlorine | 60W |

**Electrolyzer Details:**
- Output temperature: 70C minimum
- Overpressure: 1800g
- Supports ~8-9 Duplicants (at 100g/s consumption each)

**SPOM (Self-Powering Oxygen Machine):**
- 1 Electrolyzer + 2 Gas Pumps + 1 Hydrogen Generator
- Net output: ~600-650 kg oxygen/cycle
- Hydrogen powers the system with surplus

### Water Purification

**Water Sieve:**
- Input: 5 kg/s Polluted Water + 1 kg/s Sand
- Output: 5 kg/s Water + 200g/s Polluted Dirt
- Power: 120W
- Does NOT remove germs

**Alternative Methods:**
- Boiling (kills germs, separates)
- Chlorine exposure (kills germs in tanks)
- Gulp Fish (converts P.Water to Water)

### Power Systems

**Early Game Power Chain:**
1. Manual Generator (400W) - Duplicant labor
2. Coal Generator (600W) - Coal from Hatches or mining

**Mid-Game Power Chain:**
1. Natural Gas Generator (800W) - Natural Gas geysers or Oil Refinery byproduct
2. Petroleum Generator (2000W) - Processed oil

**Late-Game Power Chain:**
1. Steam Turbine (850W max) - Heat deletion + power
2. Solar Panels (380W max each) - Renewable, space access needed
3. Hydrogen Generator (800W) - SPOM byproduct

**Oil Processing Chain:**
```
Crude Oil (Oil Reservoir/Oil Well)
    |
    v
Oil Refinery (5 kg/s Crude -> 2.5 kg/s Petroleum + 90g/s Nat Gas)
    |
    v
Petroleum Generator (2000W)
    |
    v
Byproducts: CO2 (500g/s) + P.Water (750g/s)
```

### Metal Refining

**Metal Refinery:**
- Power: 1200W
- Coolant: 400 kg liquid (stores 800 kg)
- Produces refined metals from ore
- Generates significant heat into coolant

**Coolant Options:**
- Water/Polluted Water (for low-temp metals)
- Crude Oil (up to ~400C)
- Petroleum (recommended, up to ~539C)
- Super Coolant (for extreme cases)

**Steel Production:**
- Ingredients: Iron + Refined Carbon + Lime
- Temperature: Requires high heat
- Essential for: Aquatuners, late-game builds

### Plastic Production

**Polymer Press:**
- Input: 100 kg Petroleum (or Naphtha/Ethanol)
- Output: 100 kg Plastic
- Byproducts: Steam, CO2
- Heat: 32.5 kDTU when active
- Rate: ~300 kg Plastic/cycle (continuous operation)

**Alternative:** Glossy Dreckos (produce plastic from scales)

---

## World and Map

### Biome Types

| Biome | Temperature | Atmosphere | Key Resources | Hazards |
|-------|-------------|------------|---------------|---------|
| Temperate (Terra) | 20-25C | Oxygen | Dirt, Algae, Sandstone | None |
| Caustic | ~40C | Chlorine, Hydrogen | Gold Amalgam, Chlorine | Temperature |
| Swamp/Slime | 25-35C | Polluted Oxygen | Slime, Algae, Gold | Slimelung |
| Frozen | -20 to 0C | Oxygen | Rust, Iron, Ice, Snow | Cold damage |
| Forest | 25-35C | Oxygen | Wood, Ethanol plants | None |
| Oil | 60-90C | CO2 | Crude Oil, Lead, Fossil | High temp, CO2 |
| Space | Vacuum, extreme temps | None | Solar, Space materials | Vacuum, radiation |
| Magma | 1500C+ | None | Magma, Obsidian | Extreme heat |

### Asteroid Types (Base Game)

| Asteroid | Start Biome | Difficulty | Notable Features |
|----------|-------------|------------|------------------|
| Terra | Temperate | Easy | Balanced, beginner-friendly |
| Oceania | Temperate + Water | Easy | Abundant water |
| Rime | Cold | Hard | Freezing temperatures |
| Verdante | Forest | Medium | Wood, Ethanol |
| Arboria | Forest | Medium | Limited metal |
| Volcanea | Volcanic | Hard | Extreme heat challenges |
| The Badlands | Barren | Hard | Limited resources |
| Aridio | Hot | Hard | Water scarcity |
| Oasisse | Desert | Hard | Central water source |

### Planetoid Clusters (Spaced Out! DLC)

**Cluster Types:**

1. **Classic Style:**
   - One main starting asteroid
   - One smaller teleporter-linked asteroid
   - Similar to base game

2. **Spaced Out Style:**
   - Medium starting asteroid (no oil access)
   - Two nearby asteroids
   - Teleporter to oil asteroid
   - Resources spread across multiple planetoids

3. **Moonlet Clusters:**
   - Five small moonlets
   - Choose starting moonlet
   - Teleporter + rocket travel between them
   - Resources highly distributed

**Starting Planetoids:**
- Terra
- Oceania
- Squelchy (Swamp start)
- Metallic Swampy
- Frozen Forest
- Flipped
- And more...

### Geysers and Vents

| Type | Output | Temperature | Cycle Pattern |
|------|--------|-------------|---------------|
| Cool Steam Vent | Steam | 110C | 40-80% active, 25-225 cycle dormancy |
| Steam Vent | Steam | 500C | 40-80% active |
| Hot Water Geyser | Water | 95C | Variable |
| Polluted Water Vent | P.Water | 30C | Variable |
| Natural Gas Geyser | Nat Gas | 150C+ | Variable |
| Hydrogen Vent | Hydrogen | 500C | Variable |
| Chlorine Vent | Chlorine | 60C | Variable |
| Iron Volcano | Liquid Iron | 2526C | Rare eruptions |
| Gold Volcano | Liquid Gold | 2626C | Rare eruptions |
| Copper Volcano | Liquid Copper | Variable | Rare eruptions |
| Minor Volcano | Magma | 1726C | Rare eruptions |
| Oil Reservoir | Crude Oil | 90C | Requires Oil Well |
| Leaky Oil Fissure | Crude Oil | 326C | Continuous (no dormancy) |

**Geyser Mechanics:**
- Each has active period (40-80% of eruption cycle)
- Dormancy: 25-225 cycles
- Output varies per instance (random within range)
- Can be analyzed for exact stats

### Resource Distribution

**Common (Most asteroids):**
- Sandstone, Dirt, Algae
- Water, Polluted Water
- Copper Ore, Iron Ore
- Coal, Gold Amalgam

**Uncommon:**
- Aluminum Ore (specific biomes)
- Wolframite (specific biomes)
- Oil (Oil biome only)

**Rare:**
- Diamond (Space, specific asteroids)
- Niobium (Space missions)
- Isoresin (Space missions)
- Fullerene (Space missions)
- Super Coolant (crafted from Fullerene)
- Thermium (crafted from Tungsten + Niobium)

---

## Colony Survival

### Oxygen Systems

**Early Game:**
1. Existing oxygen pockets
2. Algae Terrarium (no power, low output)
3. Oxygen Diffuser (550g/s Algae = 500g/s O2)

**Mid Game:**
1. Electrolyzer setups
2. SPOM (Self-Powering Oxygen Machine)
3. Rust Deoxidizer (if salt/rust available)

**Oxygen Distribution:**
- Gas flows naturally (diffusion)
- Gas pumps + vents for controlled distribution
- Atmo suits for hazardous areas

**Consumption:** 100g/s per Duplicant (base)

### Temperature Management

**Cooling Methods:**

1. **Passive:**
   - Ice-E Fans (Duplicant operated)
   - Ice Makers (creates ice from water)
   - Wheezeworts (plant, consumes phosphorite)

2. **Active:**
   - Thermo Aquatuner (cools liquid by 14C, 1200W)
   - Steam Turbine (deletes heat, produces power)
   - Thermo Regulator (cools gas, less efficient)

3. **Insulation:**
   - Insulated Tiles (3.125% conductivity)
   - Vacuum gaps (no heat transfer)
   - Abyssalite (natural barriers)

**Heating Methods:**
- Space Heater
- Tepidizer (heats liquid, 960W)
- Industrial machines (byproduct heat)

**Temperature Danger Zones:**

| Hazard | Effect |
|--------|--------|
| < -40C | Duplicant hypothermia |
| > 75C | Duplicant scalding/heat stroke |
| Varies | Crop death (per plant tolerance) |
| Varies | Building overheat damage |

### Food Spoilage

**Spoilage Mechanics:**
- All food has freshness timer
- Spoiled food becomes Rot (inedible)
- Temperature affects rate:
  - Below 0C: Frozen (no spoilage)
  - 0-4C: Refrigerated (~4x slower)
  - 4-75C: Normal spoilage
  - Above 75C: Rapid spoilage (cooking)

**Preservation:**
- Refrigerator (4C, powered)
- Sterile atmosphere (CO2 or Chlorine)
- Vacuum storage
- Freezing (below 0C)

### Disease Management

**Food Poisoning Prevention:**
- Wash stations after bathroom use
- Sink: 5kg water
- Hand Sanitizer: Chlorine
- Don't use contaminated water for food/cooking

**Slimelung Prevention:**
- Deodorizers at Slime biome entrances
- Store Slime in water/chlorine
- Atmo suits in contaminated areas
- Medical treatment if infected

**Treatment:**
- Sick Bay (med packs for treatment)
- Immunity boosted by:
  - Vitamins (Vitamin Chews)
  - Good food
  - Low stress

---

## Automation and Logic Systems

### Signal Types
- **Green:** Active/On/True/1
- **Red:** Inactive/Off/False/0

### Wiring

| Wire Type | Function |
|-----------|----------|
| Automation Wire | Single signal transmission |
| Automation Wire Bridge | Crosses wires without connecting |
| Automation Ribbon | 4-bit signal (requires Ribbon Reader/Writer) |

### Sensors

| Sensor | Detects | Output |
|--------|---------|--------|
| Atmo Sensor | Gas pressure (g) | Green when threshold met |
| Thermo Sensor | Temperature | Green when threshold met |
| Hydro Sensor | Liquid presence | Green when liquid detected |
| Motion Sensor | Duplicant movement | Green when motion detected |
| Duplicant Checkpoint | Duplicant passage | Green when passing |
| Weight Plate | Mass on tile | Green when threshold met |
| Clock Sensor | Time of day | Green during set hours |
| Cycle Sensor | Cycle count | Green at set intervals |
| Critter Sensor | Critter count | Green when threshold met |
| Element Sensor | Specific element | Green when detected |

### Logic Gates

| Gate | Inputs | Output Logic |
|------|--------|--------------|
| NOT Gate | 1 | Inverts signal |
| AND Gate | 2 | Green only if BOTH inputs green |
| OR Gate | 2 | Green if EITHER input green |
| XOR Gate | 2 | Green if inputs DIFFERENT |
| BUFFER Gate | 1 | Delays signal (0.1-200 seconds) |
| FILTER Gate | 1 | Filters short pulses (0.1-200 seconds) |
| Memory Toggle | 2 (Set/Reset) | Maintains state until reset |

### Common Automation Setups

**Smart Battery Control:**
- Smart Battery automation port
- Connects to generators
- Turns generators on at low charge
- Turns off at high charge

**Reservoir Management:**
- Liquid/Gas Reservoir automation port
- Pump control based on fill level
- Prevents overflow

**Temperature Control:**
- Thermo Sensor monitors temperature
- Controls Aquatuner/Heater
- Maintains target temperature range

**Critter Ranch Automation:**
- Critter Sensor counts population
- Auto-wrangle when over limit
- Controls incubators

---

## Space Exploration

### Base Game Rocketry

**Rocket Components:**

| Component | Function |
|-----------|----------|
| Command Module | Required, holds 1 Duplicant |
| Nose Cone | Aerodynamics |
| Engines | Propulsion (various types) |
| Oxidizer Tank | Holds oxidizer (solid or liquid) |
| Fuel Tank | Holds fuel |
| Cargo Bay | Transports materials (1000 kg) |
| Research Module | Analyzes destinations |
| Sight-Seeing Module | Tourist capacity |

**Engine Types:**

| Engine | Fuel | Range | Notes |
|--------|------|-------|-------|
| Steam Engine | Steam | Short | Early game |
| Petroleum Engine | Petroleum | Medium | Requires oxidizer |
| Hydrogen Engine | Liquid Hydrogen | Far | Can reach Temporal Tear |

**Fuel Ratios:**
- Fertilizer oxidizer: 1:1 fuel
- Oxylite oxidizer: 2:1 fuel
- Liquid Oxygen (LOX): 4:1 fuel

### Spaced Out! DLC Rocketry

**Modular System:**
- Rockets built from modules
- Can be reordered without deconstruction
- Multiple engine options
- Interior modules for Duplicant living space

**New Engines:**
- Sugar Engine (CO2 + Sucrose)
- Radbolt Engine (Radiation-powered)
- Petroleum Engine
- Hydrogen Engine

**Planetoid Travel:**
- Rockets travel between asteroid hexes
- Fuel consumption based on distance
- Multiple asteroids per cluster
- Each asteroid has unique resources

**Drillcone:**
- Mines resources from space POIs
- Extracts materials without landing
- Resource type based on POI composition

### Space Destinations (Base Game)

| Destination | Distance | Resources |
|-------------|----------|-----------|
| Terrestrial Planet | 10,000-40,000 km | Various |
| Carbon Asteroid | Variable | Carbon |
| Metallic Asteroid | Variable | Iron, Copper |
| Ice Asteroid | Variable | Ice, Polluted Ice |
| Rocky Asteroid | Variable | Rock, Minerals |
| Satellite | Variable | Various |
| Temporal Tear | Farthest | Endgame destination |

### Space POIs (Spaced Out!)

**POI Types:**
- Debris fields (harvestable resources)
- Orbital research stations
- Derelict satellites
- Resource-rich regions

**Rare Materials from Space:**
- Niobium (weight: 10-20 per trip)
- Fullerene (weight: 0.5-1 per trip)
- Isoresin (weight: 30-60 per trip)

---

## Endgame

### Victory Conditions

**The Great Escape Achievement:**
- Reach the Temporal Tear
- Send a rocket with Duplicant through it
- Rocket and Duplicant do not return

**Requirements (Base Game):**
- Hydrogen Engine rocket
- 2450 kg combined Fuel + Oxidizer
- Complete journey to farthest destination

### Late-Game Goals

1. **Self-Sustaining Colony:**
   - Renewable oxygen (Electrolyzer + water source)
   - Renewable food (farming/ranching)
   - Renewable power (Solar, Steam, Geothermal)
   - Temperature stability

2. **All Research Completed:**
   - 3000+ Novice points
   - 2900+ Advanced points
   - 2800+ Interstellar points (base game)

3. **Space Program:**
   - Functional rockets
   - Resource extraction from space
   - Multiple asteroid colonization (DLC)

4. **Rare Material Acquisition:**
   - Super Coolant (from Fullerene)
   - Thermium (from Tungsten + Niobium)
   - Visco-Gel (from Isoresin)

### Spaced Out! DLC Endgame

**Temporal Tear Opener:**
- Craftable device
- Activates portal to realm beyond space-time
- Requires significant resources and research

**Multi-Asteroid Empire:**
- Colonize multiple planetoids
- Establish trade routes via rockets
- Resource specialization per asteroid

**Achievement Hunting:**
- Various colony milestones
- Challenge scenarios
- Specialty achievements

---

## Appendix: Quick Reference Tables

### Building Power Consumption

| Building | Power (W) |
|----------|-----------|
| Ceiling Light | 10 |
| Oxygen Diffuser | 120 |
| Electrolyzer | 120 |
| Water Sieve | 120 |
| Gas Pump | 240 |
| Liquid Pump | 240 |
| Aquatuner | 1200 |
| Metal Refinery | 1200 |
| Carbon Skimmer | 740 |
| Research Station | 60 |
| Super Computer | 120 |

### Pipe Throughput Summary

| Pipe Type | Liquid | Gas |
|-----------|--------|-----|
| Standard | 10 kg/s | 1 kg/s |
| Mini Pump | 1 kg/s | 0.1 kg/s |

### Duplicant Needs

| Need | Rate/Cycle |
|------|------------|
| Oxygen | 100 g/s (60 kg/cycle) |
| Calories | 1000 kcal/cycle |
| Sleep | ~6 hours schedule |
| Bathroom | ~1 visit/cycle |

### Key Conversion Ratios

| Process | Input | Output |
|---------|-------|--------|
| Electrolyzer | 1 kg/s Water | 888 g/s O2 + 112 g/s H2 |
| Oil Refinery | 10 kg/s Crude | 5 kg/s Petroleum + 90 g/s Nat Gas |
| Water Sieve | 5 kg/s P.Water + 1 kg/s Sand | 5 kg/s Water + 200 g/s P.Dirt |
| Algae Distiller | 600 g/s Slime | 200 g/s Algae + 400 g/s P.Water |
| Polymer Press | 100 kg Petroleum | 100 kg Plastic |

---

## Sources

This document was compiled from research across multiple community resources:

- [Oxygen Not Included Wiki (wiki.gg)](https://oxygennotincluded.wiki.gg/)
- [ONI Database](https://oni-db.com/)
- [Oxygen Not Included Fandom Wiki](https://oxygennotincluded.fandom.com/)
- [Steam Community Guides](https://steamcommunity.com/app/457140/guides/)
- [Klei Entertainment Forums](https://forums.kleientertainment.com/)
- [Game Pressure Guides](https://guides.gamepressure.com/oxygen_not_included/)
- [Professor Oakshell's Calculators](https://www.professoroakshell.com/)

---

*Document generated for game design research purposes. All values and mechanics subject to game updates and patches.*
