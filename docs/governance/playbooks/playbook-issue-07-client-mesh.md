# Playbook: Issue 07 — Mesh & Models: 3D Rigid Mesh (.msh) Loading and PBR Material Pipeline (`swg-client-mesh`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#7  
> **Branch:** `enhancement/7-mesh-msh-pbr-pipeline`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement 3D model asset loading and PBR material synthesis in `crates/swg-client-mesh`:

1. **Rigid Mesh .msh Parser & Builder (`msh.rs`):**
   - Parse authentic SWG `FORM MSH_` IFF hierarchies.
   - Extract vertex buffer streams (positions `f32x3`, normals `f32x3`, tangents `f32x4`, UV coordinates `f32x2`).
   - Extract index buffers (`INDX`) and multi-material sub-mesh partitions.
   - Bounding sphere and AABB calculation for camera frustum culling.

2. **Legacy to Modern PBR Material Synthesizer (`material.rs`):**
   - Synthesize modern PBR material textures (Albedo, Roughness, Metallic, Normal, Emissive).
   - Convert legacy gloss/specular maps to roughness ($R = \sqrt{1.0 - \text{gloss}}$).
   - Emissive radiance mapping for lightsabers, blaster bolts, and engine exhausts.

3. **Clustered Forward+ PBR Mesh Shading (`lighting.rs`):**
   - Integration with `swg-client-core` clustered light grid.
   - Per-fragment lighting evaluation for multiple dynamic point lights and directional planetary suns.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Accurately parses complex multi-part .msh models (buildings, weapons, starships).
- [x] Converts legacy materials to PBR properties with roughness and metallic attributes.
- [x] Evaluates dynamic clustered lights on mesh surfaces.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all gates.
