# Star Wars Galaxies: Pre-CU Publish 14.1 Mechanics Specification

This document details the exact simulation formulas, rulesets, and design mechanics of Star Wars Galaxies in the **Pre-Combat Upgrade (Publish 14.1 / Spring 2005)** era.

---

## 1. The Tri-Stat HAM Pool & Wound System

Characters have 9 vital attributes grouped into three core pools: **Health**, **Action**, and **Mind** (**HAM**):

| Primary Stat | Secondary Attribute (Drain Cost Reducer) | Tertiary Attribute (Regeneration Rate) |
| :--- | :--- | :--- |
| **Health** | **Strength** (reduces Health cost on specials) | **Constitution** (accelerates Health regen) |
| **Action** | **Quickness** (reduces Action cost on specials) | **Stamina** (accelerates Action regen) |
| **Mind** | **Focus** (reduces Mind cost on specials) | **Willpower** (accelerates Mind regen) |

### Pool Mechanics:
1. **Current Pool:** Drained when taking damage or executing special combat moves.
2. **Wounds:** Damage can permanently reduce the maximum pool limit until cured by player Doctors/Medics (Health/Action) or Entertainers (Mind).
3. **Incapacitation:** Occurs when any pool hits $0$. The player collapses for 30–60 seconds. A player attacked while incapacitated is killed (Cloning Death).
4. **Battle Fatigue:** Accrues during combat and death. Reduces the efficiency of all incoming healing. Cleansed exclusively by player Entertainers in cantinas or wilderness camps.

---

## 2. Professions & The 250 Skill Point System

There are **32 distinct professions** governed by a flexible 250 skill point budget:

### Basic Starting Professions (6):
* **Artisan:** Surveying, resource extraction, basic weapon/armor/item manufacturing.
* **Brawler:** Melee combat fundamentals (unarmed, 1-handed, 2-handed, polearms).
* **Marksman:** Ranged combat fundamentals (rifle, pistol, carbine).
* **Medic:** Field medicine, organic chemistry, wound healing, stimpacks.
* **Entertainer:** Dancing, musicianship, image design, camp performance.
* **Scout:** Wilderness survival, foraging, creature harvesting, terrain traversal, camping.

### Elite & Hybrid Professions:
* **Combat:** Rifleman, Pistolier, Carbineer, Swordsman, Fencer, Pikeman, Teras Kasi Artist (TKA), Commando, Bounty Hunter, Squad Leader.
* **Crafting:** Armorsmith, Weaponsmith, Architect, Droid Engineer, Tailor, Chef, Bio-Engineer.
* **Medical / Social:** Doctor, Combat Medic, Musician, Dancer, Image Designer.
* **Wilderness:** Ranger, Creature Handler.
* **Civic:** Politician (uses no skill points, progresses via citizen votes).
* **Force Sensitive / Jedi:** Holocron masteries -> Village of Aurilia unlock -> Padawan / Jedi Knight.

---

## 3. Combat Formulas & State Machine

Combat resolves on a 1-second tick loop:

### Hit Probability Formula:
$$\text{Hit Probability} = \frac{\text{Attacker Accuracy}}{\text{Attacker Accuracy} + \text{Defender Defense}} \times \text{Posture Mod} \times \text{State Mod}$$

### Postures:
* **Upright:** Baseline movement speed and defense.
* **Kneeling:** $+25\%$ ranged weapon accuracy, $-25\%$ melee defense.
* **Prone:** $+50\%$ ranged defense at distance, extreme vulnerability to melee attacks, immobilized.

### Combat States & "Dizzy-KD" Synergy:
* **Knockdown (KD) & Posture Down:** Forces target into Kneeling or Prone posture.
* **Dizzy:** If a dizzy player attempts to stand up or recover from knockdown, they immediately fall back down and suffer massive self-inflicted damage.
* **Stun:** Temporarily strips armor resistances and drops damage output.
* **Blind:** Reduces weapon accuracy by $50\%$.
* **Intimidate:** Halves damage inflicted by the target for 30 seconds.
* **Damage Over Time (DOTs):** Bleeds (Health), Fires (Action), Poisons (Health/Action), and Diseases (Mind/Health).

---

## 4. The Dynamic Economy & Crafting Engine

### Resource Shifts & Attribute Spectrum:
Resources spawn randomly across planets for 7 to 21 days with 11 randomized attributes ($1\text{–}1000$):
* **OQ:** Overall Quality
* **PE:** Potential Energy
* **DR:** Decay Resistance
* **CR:** Cold Resistance
* **CD:** Conductivity
* **FL:** Flammability
* **HR:** Heat Resistance
* **MA:** Malleability
* **SR:** Shock Resistance
* **UT:** Unit Toughness

### Drafting Schematics & Experimentation:
* Every item blueprint defines weighted attribute formulas (e.g., $65\%\text{ OQ} + 35\%\text{ SR}$).
* Skilled crafters use **Experimentation Points** to roll for stat increases. Critical failure risks damaging the item or losing raw materials.
* **Factories:** Mass-produce identical copies using manufacturing schematics and raw material hoppers.
* **Decay:** Items suffer permanent loss of maximum durability on every repair. Items eventually break permanently, ensuring a perpetual economic loop.

---

## 5. Player Housing, Cities & Politics

* Structures are placed freely on planetary terrain outside municipal exclusion zones.
* **City Tiers (Politician Profession):**
  * Tier 1: Outpost (75 citizens)
  * Tier 2: Village (150 citizens)
  * Tier 3: Township (300 citizens)
  * Tier 4: City (450 citizens)
  * Tier 5: Metropolis (600 citizens)
* **Civic Structures:** Mayors can build planetary **Shuttleports**, cloning facilities, hospitals, cantinas, theaters, and banks.
* **Elections:** Held every 3 weeks. Mayors set property and sales taxes, fund treasury maintenance, and manage zoning permits.
