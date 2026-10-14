# Playbook: Issue 11 — UI HUD: Pre-CU Authentic HUD: Radar, HAM Bars, Target Status & Macro Button Bar (`swg-client-ui`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#11  
> **Branch:** `enhancement/11-pre-cu-hud-radar-ham`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement the authentic Pre-CU heads-up display (HUD) in vector graphics in `crates/swg-client-ui`:

1. **HAM Status Bars (`ham.rs`):**
   - Three primary pools: Health (Red), Action (Green), Mind (Blue).
   - Real-time delta updates for current pool, max capacity, and permanent wounds (black tick notches).
   - Incapacitation & death triggers when any primary pool drops $\le 0$.

2. **Planetary Radar Minimap (`radar.rs`):**
   - Rotating compass radar with heading rotation ($0^\circ = \text{North}$).
   - Target blips (waypoints, friendly players, neutral fauna, hostile enemies, corpses).
   - Circular radar perimeter edge-clamping for off-radar objectives.
   - Dynamic zoom levels (128m, 256m, 512m, 1024m).

3. **Target Status Frame (`target_status.rs`):**
   - Selected target vitals: HAM percentages, target name, posture, and 3D Euclidean distance.
   - Hostility indicator and faction affiliation (Rebel, Imperial, Neutral).

4. **Macro Button Bar & Hotkeys (`button_bar.rs`):**
   - 12-slot action macro bar with pagination (F1–F12, 1–0 hotkeys).
   - Cooldown clock timers and execution trigger events (`/attack`, `/burstRun`, `/heal`).

5. **High-DPI Vector Layout (`hud_layout.rs`):**
   - Crisp resolution-independent scaling from 1080p to 1440p and 4K UHD.
   - Screen anchor positioning (Top-Left, Top-Right, Top-Center, Bottom-Center).

---

## 2. Acceptance Criteria & Quality Gates

- [x] UI scales crisply to 1440p and 4K resolutions without pixelation.
- [x] HAM bars update in real-time as server deltas arrive, rendering wound notches correctly.
- [x] Radar correctly rotates blips according to player heading and clamps off-screen markers to circle edge.
- [x] Macro button bar executes assigned command macros and tracks ability cooldowns.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all quality gates.
