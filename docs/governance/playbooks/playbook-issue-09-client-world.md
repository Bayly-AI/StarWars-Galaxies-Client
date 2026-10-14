# Playbook: Issue 09 — World Integration: World Snapshot (.ws) Static City Placements & Scene Graph Rendering (`swg-client-world`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#9  
> **Branch:** `enhancement/9-world-snapshot-city-placement`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement planetary static structure placement, world snapshots, and spatial culling in `crates/swg-client-world`:

1. **World Snapshot .ws Parser & Builder (`ws.rs`):**
   - Parse authentic SWG `FORM WSNP` hierarchies.
   - Extract static object nodes with 64-bit Network ID, template identifier / path, world-space transform (`Vec3` position, `Quat` rotation, scale), cell parent ID, and bounding sphere radius.
   - Fast zero-allocation streaming of thousands of planetary static objects.

2. **Canon City Presets (`canon_cities.rs`):**
   - Presets for major planetary hubs with authentic retail coordinates:
     - **Mos Eisley (Tatooine)**: Chalmun's Cantina, Starport, Lucky Despot, Bazaar.
     - **Theed (Naboo)**: Royal Palace Complex, Theed Starport, Cantina, Bridges.
     - **Coronet (Corellia)**: Coronet Capitol, Starport, Government district.

3. **Camera Frustum & Distance Culling (`culling.rs`):**
   - Extract 6 frustum planes (Left, Right, Bottom, Top, Near, Far) from View-Projection matrix.
   - Sphere and AABB frustum intersection tests preventing rendering behind camera.
   - Configurable distance falloff culling based on object size category.

4. **Spatial Partitioning & Scene Graph Streaming (`scene_graph.rs`):**
   - 2D/3D spatial grid / Quadtree indexing planetary objects across $16 \times 16\text{ km}$ terrain maps.
   - Fast spatial range queries extracting visible objects in camera viewport for instanced rendering.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Accurately parses `.ws` world snapshot files containing multi-thousand static node hierarchies.
- [x] Positions Mos Eisley, Theed, and Coronet landmarks at authentic retail coordinates.
- [x] Frustum culler rigorously rejects geometry outside and behind the camera frustum.
- [x] Spatial scene graph queries sustain sub-millisecond query times for streaming budgets.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all quality gates.
