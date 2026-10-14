# Playbook: Issue 12 — UI Interaction: Context-Sensitive Dynamic Radial Menu and Cursor Interaction (`swg-client-ui`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#12  
> **Branch:** `enhancement/12-context-sensitive-radial-menu`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement the classic Pre-CU SWG context-sensitive radial menu and 3D world cursor picking in `crates/swg-client-ui`:

1. **3D Mouse Cursor Raycasting (`cursor_ray.rs`):**
   - Unproject screen-space mouse coordinates $[0, W] \times [0, H]$ through inverse View-Projection matrix into a normalized 3D world ray (`Ray3D`).
   - Ray-sphere and ray-AABB intersection tests for interactive targeting of terrain objects, buildings, creatures, and players.
   - Closest-hit sorting with distance evaluation.

2. **Context-Sensitive Capability Evaluator (`interaction.rs`):**
   - Dynamically populates radial options based on targeted entity class:
     - `Corpse`: Loot, Harvest (Meat, Hide, Bone), Examine.
     - `Container`: Open, Pick Up, Examine.
     - `Player`: Trade, Invite to Group, Follow, Duel, Examine.
     - `ResourceNode`: Sample, Survey, Examine.
     - `Terminal`: Use, Examine.

3. **Dynamic Radial Menu Engine (`radial.rs`):**
   - Circular slice arrangement around mouse click location: $\theta_i = \frac{2\pi \cdot i}{N}$.
   - Wedge hover detection by mouse angle and distance from center ($R_{\text{inner}} \le r \le R_{\text{outer}}$).
   - Numerical keyboard shortcut triggers (keys 1–8).
   - Action dispatching yielding authentic slash commands (`/loot`, `/openContainer`, `/trade`, `/sample`, `/examine`).

---

## 2. Acceptance Criteria & Quality Gates

- [x] Mouse unprojection accurately casts 3D rays from screen pixels into world space.
- [x] Dynamically constructs context-sensitive radial slices matching retail Pre-CU capabilities.
- [x] Mouse position over radial slices selects correct slice by angular arc and radius.
- [x] Selecting a slice returns the command and dispatches to server netcode.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all quality gates.
