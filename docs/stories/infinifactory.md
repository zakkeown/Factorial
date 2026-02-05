# Infinifactory - Game Design Research Document

## Table of Contents

1. [Overview](#overview)
2. [Initial Conditions](#initial-conditions)
3. [Core Mechanics](#core-mechanics)
   - [Block Types](#block-types)
   - [Movement Blocks](#movement-blocks)
   - [Modification Blocks](#modification-blocks)
   - [Logic Blocks](#logic-blocks)
   - [Basic/Utility Blocks](#basicutility-blocks)
4. [Puzzle Structure](#puzzle-structure)
5. [Technology and Progression](#technology-and-progression)
6. [Building Mechanics](#building-mechanics)
7. [Optimization Goals and Scoring](#optimization-goals-and-scoring)
8. [Level and World Structure](#level-and-world-structure)
9. [Narrative Elements](#narrative-elements)
10. [Steam Workshop and Custom Puzzles](#steam-workshop-and-custom-puzzles)
11. [Sources](#sources)

---

## Overview

**Developer:** Zachtronics
**Release Date:** June 30, 2015 (PC/Mac/Linux), December 2015 (PlayStation 4)
**Genre:** First-person 3D puzzle/factory automation
**Predecessor Influences:** SpaceChem (assembly mechanics), Infiniminer (3D block building)

Infinifactory is a first-person logic puzzle game about constructing self-sufficient, three-dimensional factory machines through the use of individual building blocks. Players build assembly lines from blocks in a three-dimensional space to transform input materials into required output products.

---

## Initial Conditions

### Starting Scenario

The player begins the game with no prior context, experiencing their character's abduction in real-time:

- The protagonist is driving through the American Midwest when blinding lights and disorientation occur
- The character awakens in an isolated alien chamber
- Initial equipment consists only of a survival suit and jetpack for navigation
- The player is immediately put to work constructing factories

### Narrative Premise: Alien Abduction

- Players assume the role of a human engineer abducted from Earth by extraterrestrial beings known as "the Overlords"
- The protagonist is forced into slave labor constructing factories to meet alien industrial needs
- The character is not the first human taken for this purpose - corpses of previous abductees are scattered throughout levels
- The tone blends dark humor with commentary on exploitation and coercive labor

### Initial Tutorial Progression

- The first zone (Proving Grounds) serves as the tutorial area
- Training Routines 1-5 introduce core mechanics progressively
- Players start with basic blocks (platforms, conveyors) and unlock additional tools as they progress
- The Test Zone sandbox unlocks after completing Skydock 19 (the second set of puzzles)

---

## Core Mechanics

### Block Types

Infinifactory's blocks are organized into several functional categories:

---

### Movement Blocks

#### Conveyor Belt

**Function:** Moves blocks in a single direction along a horizontal plane

**Mechanics:**
- Only one conveyor block per X units is needed to move an object X units long
- A single conveyor will move an object of any size as long as that object is touching the conveyor
- Objects affected by conveyors pointing in opposite directions move according to which direction has more belts
- One axis (North-South) always has higher priority than the other (East-West) - this is the "dominant" axis
- Inverted conveyors (moving blocks toward the conveyor) always take priority over regular conveyors

**Movement Priority Order:**
1. Pushers/Blockers
2. Lifters (Vertically Upward)
3. Gravity (Vertically Downward)
4. Inverted Conveyor
5. Conveyor

**Speed Considerations:**
- Faster conveyor operation requires triggering belts one cycle before blocks need to change direction
- Input rate controls how fast items spawn (adjustable with [ ] keys)
- At max input speed, blocks spawn with no gaps between them

#### Rotator

**Function:** Rotates objects 90 degrees clockwise or counterclockwise around a vertical axis

**Mechanics:**
- Rotation always takes 1 turn/cycle to complete
- Direction depends on rotator orientation (clockwise or counterclockwise variants exist)

**Activation Rules:**
- When covered by a block, the rotator attempts to rotate the object on top
- The rotator keeps trying until successful or the object is no longer covering it
- After successful rotation, the rotator will not try again until the rotated object moves off

**Space Requirements:**
- The path of rotating blocks must not intersect any other blocks
- The path must not intersect any moving blocks during the entire turn
- None of the square block faces of the rotating object can be adjacent to other block faces at start or end positions

**Special Properties:**
- A rotator acts as a platform after rotating something
- Objects can get stuck on rotators and require pushers to remove
- A series of rotators can move objects faster than a line of conveyors (for 3x3 or larger objects)
- A zig-zagging series of rotators can move 2x2+ objects diagonally

#### Lifter (Cargo Uplifter)

**Function:** Moves blocks vertically upward

**Mechanics:**
- Lifts any block positioned above it
- Continues lifting blocks to the maximum height as long as they remain influenced
- Lifters raise blocks even with other components (like conveyors) placed over them
- Can suspend blocks in the air indefinitely

**Creating Lifter Elevators:**
1. Conveyor piece onto lifter
2. Lifter raises piece into the air
3. Sensor detects piece at desired height
4. Sensor triggers pusher to push piece off lifter
5. Piece drops onto next conveyor or platform
6. Can chain multiple lifters for unlimited height

**Priority:** Lifting beats Conveying (lifters will hold blocks against conveyor force)

#### Launcher

**Function:** Propels blocks through the air across distances

**Usage:** Used in specific puzzle contexts to move blocks over gaps or obstacles

---

### Modification Blocks

#### Welder

**Function:** Permanently joins adjacent blocks together

**Mechanics:**
- Each welder emits a welding spot directly in front of it (or below for overhead welders)
- If at least two welding spots are adjacent, those faces are welded together
- Welding occurs as one of the first operations each cycle

**Welder Types:**
- Horizontal welder: Projects welding spot horizontally in front
- Overhead/Vertical welder: Projects welding spot directly below

**Welding Techniques:**
- **Linear Beam (W.A):** Two facing welders create a beam between them
- **Adjacent Beam (W.B):** Adjacent welders create beams that connect
- Multiple welders in a row connect beams to weld larger objects
- Can create grids of any size using this principle

**Vertical Welding:**
- Requires dropping pieces vertically onto one another
- Use lifter elevators to position pieces at height before dropping

**Orientation Flexibility:** Welders can be in any orientation as long as welding locations are adjacent to each other

#### Eviscerator

**Function:** Destroys/removes blocks

**Mechanics:**
- Eviscerates up to three blocks per turn
- If stationary, eviscerates the block that will be in front of it at end of cycle
- Used for milling, cutting, and removing unwanted material
- Metal blocks can be cut free and milled into products using eviscerators

**Restrictions:**
- Platform blocks (player-placed grey blocks) cannot be eviscerated
- Input blocks can be destroyed, but blocks the player placed cannot

#### Laser

**Function:** Long-range block destruction with toggle capability

**Mechanics:**
- Functions similarly to eviscerator but with much longer range
- Can be toggled on and off via sensor signals
- When receiving a sensor signal on any conduit port, fires instantly
- Destroys every block in its path when fired

**Key Difference from Eviscerator:** The laser's toggle ability makes it useful for timed destruction

#### Stamper

**Function:** Modifies blocks by stamping patterns or shapes onto them

**Usage:** Applied in specific puzzles requiring block modification

#### Painter

**Function:** Applies color to block surfaces

**Mechanics:**
- Sprays paint on the sides of blocks as they pass by
- Used in puzzles requiring color-coded or marked products

---

### Logic Blocks

#### Sensor (Proximity Sensor)

**Function:** Detects blocks passing through and triggers connected machinery

**Mechanics:**
- Only active for 1 cycle when another block passes
- Processes during the "Sensors and Counters" phase each cycle
- Downward-facing sensors act as blockers, useful with lifters for height control

**Signal Behavior:**
- Outputs a single-cycle signal when triggered
- Can be connected to pushers, blockers, lasers, and other actuators

#### Counter

**Function:** Sensor that activates after a specified number of blocks pass

**Mechanics:**
- User-configurable count threshold
- Activates connected machinery when count is reached
- Useful for batch processing and sequenced operations

#### Pusher

**Function:** Pushes blocks in a specified direction when triggered

**Mechanics:**
- Activated by sensor signals
- Waits until target area is completely empty before moving parts
- This waiting behavior can slow spawn rates for timing control

**Pusher Limitations:**
1. Cannot push blocks attached to the ground
2. Cannot push blocks that would also push the pusher itself
3. A pusher on the ground with an attached platform won't work (platform welded to ground)

**Advanced Technique:** Place pusher at level of highest block in stack, sensor at least two blocks above - sensor triggers every time but pusher only activates when stack is high enough

#### Blocker

**Function:** Stops/holds blocks until sensor trigger releases them

**Mechanics:**
- Blocks items (e.g., on conveyor belt) until sensor triggers release
- Allows complex assembly by holding items until welding is complete
- Piston position changes when triggered

**Known Issue:** Single-block elements may not have time to pass before blocker resets

#### Conduit

**Function:** Carries signals between sensors and actuators

**Mechanics:**
- Functions as implicit OR gates - multiple sensors on one conduit will demonstrate this
- Used to route signals around corners and through complex paths

**Logic Gate Construction:**
- **OR Gate:** Default when joining conduits from two switches
- **AND Gate:** Requires one switch to cause a pusher to shove a conduit piece into place to complete another conduit line

**Timing Note:** Conduits have inherent delays that must be accounted for in precise timing

#### Toggle

**Function:** Allows direct player interaction with running factory

**Mechanics:**
- Introduced in the Test Zone sandbox
- Enables manual control over factory operations
- Useful for debugging and experimental builds

#### Transceiver

**Function:** Wireless signal transmission

**Usage:** Sends signals between distant parts of the factory without physical conduit connections

---

### Basic/Utility Blocks

#### Platform Block

**Function:** Structural building block for factory construction

**Mechanics:**
- Grey cube blocks
- Do NOT count toward the Blocks score
- Can be used extensively without optimization penalty
- Cannot be eviscerated
- A single conveyor at the beginning can push items along a chain of platform blocks (items push each other)

#### Product Blocks

**Function:** Input materials that must be assembled into output products

**Types:** Various shapes and configurations depending on the puzzle

#### Teleporter

**Function:** Instantly moves blocks over great distances

**Mechanics:**
- Moves one block at a time (or the player)
- Any block entering the teleporter is separated from welded blocks
- Block immediately appears at the other end
- Objects teleported during a cycle cannot be welded that same cycle
- Can be welded on the following cycle if left sitting in the teleporter

---

## Puzzle Structure

### How Puzzles Work

Each puzzle presents:
1. **Input Spawners:** Sources that spawn raw material blocks at configurable rates
2. **Output Platform:** Destination where assembled products must be delivered
3. **Target Product:** A specific 3D shape/assembly that must be constructed
4. **Required Quantity:** Typically 10 output items to complete a puzzle

### Input/Output Requirements

**Input System:**
- Raw blocks spawn from designated input points
- Spawn rate is adjustable ([ ] keys increase/decrease)
- Input rate affects Cycles score - higher rates enable faster completion
- Different puzzles may have multiple input types

**Output System:**
- Output platforms can be joined to form different required shapes
- Product blocks must be placed correctly on output blocks
- Assembled products must match the target configuration exactly
- Victory requires delivering the specified number of correct products

### Level Completion

- Puzzles are considered complete when the required number of valid products reach the output
- Players receive scores in three categories upon completion
- Scores are displayed on histograms comparing to all players worldwide
- Players can return to improve scores at any time

---

## Technology and Progression

### Block Unlock System

- New block types unlock as players complete puzzles that introduce them
- Once a block is unlocked, it can be used in all previously completed levels
- Example: Beating the level that introduces lifters allows using lifters in earlier levels
- Blocks unlocked in a campaign can be used in every level of that campaign

### Puzzle Dependencies

- Certain zones/campaigns require completing prerequisites
- Production Zone completion is required to access later content
- The Heist and Production Zone 1 mission conclusions unlock further progression
- Not every mission in a zone must be completed - sufficient completion triggers progression

### General Progression Flow

1. **Proving Grounds:** Basic mechanics (conveyors, welders, basic assembly)
2. **Skydock 19:** Intermediate mechanics, unlocks Test Zone sandbox
3. **Resource Sites:** Advanced mechanics (sensors, pushers, logic)
4. **Production Zones:** Complex multi-step assembly challenges
5. **Atropos Station:** Processing-heavy puzzles (evisceration)
6. **The Homeward Fleet:** Massive product construction, story conclusion

---

## Building Mechanics

### 3D Grid System

- Uses an invisible cubic grid system similar to Minecraft
- No pixel-perfect alignment required - blocks snap to grid
- All blocks occupy discrete grid positions

### Block Placement Rules

**Placement Method:**
- Blocks are placed by choosing the face of an existing block to attach to
- No way to place a block except by attaching to another block face
- This constraint shapes construction approach

**Gravity During Construction:**
- Gravity is disabled during the construction/build phase
- Structures don't fall apart while building
- Gravity activates when running the factory

**Support and Falling:**
- Unsupported structures don't fall in the editor
- Once running, unsupported blocks fall according to gravity
- Blocks/groups without adjacent blocks at design time can move freely
- Can be rotated, welded, and moved like puzzle input blocks

### First-Person Navigation

- Players navigate the 3D space in first-person view
- Jetpack allows vertical movement and flying
- Can view factory from any angle during construction and operation

### Physics Interactions

**Movement Precedence:**
1. Pushing beats Falling
2. Falling beats Lifting
3. Lifting beats Conveying
4. Conveying beats Rotating

**Welded Structures:**
- Welded blocks move as a single unit
- Pushing welded structures requires ability to move the entire assembly
- Ground-attached blocks cannot be pushed

---

## Optimization Goals and Scoring

### The Three Scoring Metrics

#### 1. Cycles

**Definition:** Time taken between starting the assembly line and requirements being met

**Mechanics:**
- Measured in "ticks" or cycles
- Lower is better
- Input rate significantly impacts this score
- Parallel processing and efficient routing reduce cycles

**Optimization Tips:**
- Maximize input rate
- Use parallel assembly lines
- Minimize transportation distances
- Rotator chains can move large objects faster than conveyors

#### 2. Footprint

**Definition:** Total floor space occupied horizontally by the solution

**Mechanics:**
- Counts how many input blocks or self-placed blocks occupy unique (x,z) positions
- Measured from top-down perspective
- Vertical height does NOT affect this score
- Lower is better

**Optimization Tips:**
- Stack vertically as much as possible
- Use height instead of horizontal spread
- Compact horizontal layouts reduce footprint

#### 3. Blocks

**Definition:** Number of non-platform blocks placed

**Mechanics:**
- Platform blocks (grey cubes) do NOT count
- All functional blocks (conveyors, welders, sensors, etc.) count
- Lower is better

**Optimization Tips:**
- Replace long conveyor chains with platform blocks plus minimal conveyors
- Use single conveyors to push items along platform chains
- Minimize sensor/pusher usage through clever timing

### Histogram System

- Scores are displayed on histograms showing distribution of all players worldwide
- Removes hard numbers - shows relative standing to population
- Most levels show bell-curve distributions
- First attempts typically fall in the middle

### Optimization Strategy

**Key Insight:** Top scores in all three categories usually require separate solutions

**Recommended Approach:**
1. One build focused on minimizing Cycles
2. One build focused on minimizing Footprint
3. One build focused on minimizing Blocks

Different solutions may have conflicting requirements (speed vs. size vs. components)

---

## Level and World Structure

### Campaign Zones

#### Proving Grounds
- **Purpose:** Tutorial area
- **Levels:** Training Routines 1-5
- **Focus:** Basic mechanics introduction
- **New Blocks:** Conveyors, platforms, basic welders

#### Skydock 19
- **Purpose:** Intermediate progression
- **Significance:** Completing this unlocks the Test Zone sandbox
- **Focus:** Building on tutorial concepts

#### Resource Sites
- **Variants:** Resource Site 526.81, Resource Site 338.11, Resource Site 902.42
- **Purpose:** Various intermediate challenges
- **Focus:** Resource processing and assembly

#### Production Zones
- **Levels:** Production Zone 1, Production Zone 2
- **Purpose:** Complex assembly challenges
- **Requirements:** Mission conclusion needed for progression
- **Focus:** Multi-step manufacturing processes

#### The Heist
- **Purpose:** Story-driven campaign section
- **Requirements:** Mission conclusion needed for progression

#### Atropos Station (Mini-Campaign)
- **Levels:** 6 new puzzles
- **Focus:** Building products from inputs requiring processing (evisceration)
- **Character Introduction:** Ortis, a fuzzy four-armed engineer who built Infinifactory's tools
- **Significance:** Not all missions required to progress further

#### The Homeward Fleet (Mini-Campaign)
- **Levels:** 6 new puzzles
- **Focus:** Building massive products (epic scope)
- **Significance:** Final conclusion of the story
- **Prerequisites:** Complete The Heist and sufficient Production Zone progress

### Sandbox Modes

#### Test Zone
- **Unlock:** After beating Skydock 19
- **Features:** Open sandbox for experimentation
- **Special Block:** Toggle switch for direct factory interaction

#### Test Zone X
- **Unlock:** Late game
- **Features:** Configurable outputs of factory blocks
- **Purpose:** Advanced experimentation and testing

### Final Content

After Atropos Station:
1. Four levels about building very large objects
2. A defense mission
3. Final Building Challenge

---

## Narrative Elements

### Story Framework

The narrative uses a dark science fiction setting:

- **Theme:** Coercive alien labor and exploitation
- **Tone:** Dark humor mixed with commentary on industrialization
- **Structure:** Story unfolds across six worlds representing distinct alien locales

### Audio Logs (Failure Logs)

**Discovery Method:**
- Found on corpses of previous human engineers scattered throughout levels
- Players approach bodies and listen to recorded final messages

**Content:**
- Describe the experiences of previous prisoners
- Detail violent acts and demises
- Provide lore about the alien facility and its history
- Reveal information about other abductees and their fates

**Narrative Function:**
- Primary storytelling device
- Motivates progression through factory demands
- Blends seamlessly with gameplay
- Creates emotional investment in escaping

### Story Progression

- Narrative advances through escalating challenges
- Each world reveals new information about the aliens and their purposes
- Character of Ortis introduced in Atropos Station provides different perspective
- The Homeward Fleet provides story resolution

### Tone and Atmosphere

- Dark and oppressive alien environments
- Isolation emphasized through design
- ESRB content includes descriptions of violent acts in audio logs
- Contrast between mundane puzzle-solving and dire circumstances

---

## Steam Workshop and Custom Puzzles

### Steam Workshop Integration

- Players can create, share, and play custom puzzles
- Community-created content extends game longevity
- Puzzles must be solved by creator before upload

### Level Editor

#### Standard Editor
- Available after completing approximately half the campaign
- Basic puzzle creation tools
- Limited block and environment options

#### Advanced Editor
- Same tool used by developers to create puzzles
- Access to almost every environment and block type
- Includes options not in standard editor:
  - Default spawn rate configuration
  - Additional environment settings
  - Hundreds of additional blocks

### Custom Puzzle Blocks

- The Advanced Editor provides access to hundreds of additional block types
- Beyond standard campaign blocks
- Enables unique puzzle designs not possible in main game

### Test Zone Integration

- Test Zone serves as sandbox for experimentation
- Can prototype ideas before creating formal puzzles
- Test Zone X provides configurable outputs for advanced testing

### Quality Assurance

- Creators must solve their own puzzles before uploading
- Ensures all shared puzzles have valid solutions
- Community can rate and provide feedback on puzzles

---

## Sources

### Official Sources
- [Zachtronics Official Infinifactory Page](https://www.zachtronics.com/infinifactory/)
- [Steam Store Page](https://store.steampowered.com/app/300570/Infinifactory)

### Wiki Resources
- [Infinifactory Wiki - Fandom](https://infinifactory.fandom.com/wiki/Infinifactory_Wiki)
- [Levels | Infinifactory Wiki](https://infinifactory.fandom.com/wiki/Levels)
- [Tips and Tricks | Infinifactory Wiki](https://infinifactory.fandom.com/wiki/Tips_and_Tricks)
- [Gadgets | Infinifactory Wiki](https://infinifactory.fandom.com/wiki/Gadgets)
- [All Custom Puzzle Blocks | Infinifactory Wiki](https://infinifactory.fandom.com/wiki/All_Custom_Puzzle_blocks)

### Community Guides
- [Steam Guide: Mechanics and Optimizations](https://steamcommunity.com/sharedfiles/filedetails/?id=820679524)
- [Steam Guide: How Do Scores Work?](https://steamcommunity.com/sharedfiles/filedetails/?id=623821926)
- [Steam Guide: Complete Video Solution Guide](https://steamcommunity.com/sharedfiles/filedetails/?id=382895151)
- [Steam Guide: Advanced Level Editor Basics](https://steamcommunity.com/sharedfiles/filedetails/?id=478064618)

### Developer Updates
- [Infiniupdate #1](https://www.zachtronics.com/infiniupdate-1/)
- [Infiniupdate #2](https://www.zachtronics.com/infiniupdate-2/)
- [Infiniupdate #3](https://www.zachtronics.com/infiniupdate-3/)

### Reference Articles
- [Infinifactory - Wikipedia](https://en.wikipedia.org/wiki/Infinifactory)
- [Infinifactory - TV Tropes](https://tvtropes.org/pmwiki/pmwiki.php/VideoGame/Infinifactory)
- [Review: Infinifactory | Gold-Plated Games](https://goldplatedgames.com/2017/08/03/review-infinifactory/)

### Community Discussions
- [The Complete Infinifactory Turn Order](https://steamcommunity.com/app/300570/discussions/0/530649887197893834/)
- [How Rotators Work Discussion](https://steamcommunity.com/app/300570/discussions/0/523890681405760762/)
- [Welder Mechanics Discussion](https://steamcommunity.com/app/300570/discussions/0/613948093873030189/)
