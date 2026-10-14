# Playbook: Issue 05 — Terrain: Procedural .trn Height Evaluation & Real-Time GPU Mesh Chunk Tessellation (`swg-client-terrain`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#5  
> **Branch:** `enhancement/5-terrain-trn-height-evaluation`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement procedural planetary terrain generation and LOD mesh tessellation in `crates/swg-client-terrain`:

1. **Procedural .trn Height Generator & Surface Normal Evaluation (`height.rs`):**
   - Multi-fractal procedural height evaluation $H(x, z)$ with configurable octaves, frequency, lacunarity, and persistence.
   - Boundary shapes (circles, rectangles) and height blend filters (flats, canyons, hills).
   - Analytic / central-difference surface normal calculation yielding smooth PBR shading.

2. **Dynamic 64m x 64m Chunk Tessellation (`chunk.rs`):**
   - Grid chunk generation centered dynamically around camera position.
   - Vertex buffer generation: positions `Vec3`, normals `Vec3`, texture coordinates `Vec2`.
   - Continuous Level of Detail (LOD 0 to LOD 3) index buffer generation.

3. **Crack-Free Edge Stitching & Geomorphing (`lod.rs`):**
   - T-junction elimination through neighbor-aware boundary stitching (connecting finer LOD edge vertices to coarser LOD edge midpoints).
   - 0 visible seams or cracks across any LOD transition boundary.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Procedural height evaluation computes elevations and smooth surface normals.
- [x] Generates $64\text{m} \times 64\text{m}$ chunk meshes around camera.
- [x] Edge stitching completely eliminates T-junction seams between neighboring chunks of different LOD levels.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all gates.
