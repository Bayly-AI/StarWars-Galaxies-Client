# Playbook: Issue 10 — Interiors: CellObject Indoor Rendering, Portals, and Occlusion Culling (`swg-client-interior`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#10  
> **Branch:** `enhancement/10-interior-cell-portal-culling`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement interior building rendering and portal-based occlusion culling in `crates/swg-client-interior`:

1. **CellObject Coordinate Spaces (`cell.rs`):**
   - Support hierarchical cell coordinate systems within buildings (`building_id`, `cell_id`).
   - Bidirectional matrix transformations between local cell space and global planetary coordinates.
   - Cell bounding boxes and indoor room content containment.

2. **Doorways & Portals (`portal.rs`):**
   - Convex planar doorway polygons connecting adjoining cells or linking interior to outdoor terrain (`cell_id: 0`).
   - Door states (`Open`, `Closed`, `Locked`) controlling visibility and pass-through.
   - Portal frustum clipping: generating reduced camera clipping frustums through doorway vertices.

3. **Portal Occlusion Culler (`occlusion.rs`):**
   - Graph traversal of interconnected rooms via visible portal frustums.
   - Strict occlusion of outdoor terrain and landscape meshes when inside windowless/closed rooms.
   - Doorway viewports: rendering outdoor terrain strictly within clipped doorway frustum when looking out an open entrance.

4. **Seamless Entrance & Cell Transitions (`transition.rs`):**
   - Detect camera crossing entrance/room portal planes with hysteresis damping.
   - Seamlessly switch rendering context and active ambient lighting between interior cells and outdoors.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Smoothly transforms coordinates between interior cells and planetary world space.
- [x] Portal frustum culling traverses adjoining rooms and clips visibility through doorways.
- [x] Outdoor terrain is 100% culled when completely occluded inside windowless rooms.
- [x] Seamless camera transitions when crossing doorway thresholds without popping.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all quality gates.
