# Playbook: Issue 06 — Terrain Rendering: Multi-Layer Ground Splatting Shader & Flora Clutter (`swg-client-render`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#6  
> **Branch:** `enhancement/6-terrain-rendering-ground-splatting`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement modern terrain surface rendering and instanced planetary clutter in `crates/swg-client-render`:

1. **Multi-Layer Ground Splatting Pipeline (`splat.rs`, `terrain.wgsl`):**
   - Multi-texture layer weights computed from terrain surface normal slope angle ($\cos \theta = \vec{N} \cdot \vec{U}$) and elevation altitude.
   - PBR material attribute evaluation: blended diffuse albedo, roughness, normal perturbations, and ambient occlusion.
   - WGSL forward rendering shader pipeline with bind group layouts for camera uniforms and terrain material textures.

2. **Procedural Flora Clutter Instancing (`clutter.rs`):**
   - Procedural placement of ground detail instances (desert grass tufts, rocks, shrubs) within camera proximity.
   - GPU instancing buffer containing transforms and instance IDs.
   - Distance-attenuated density fading and view frustum culling.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Multi-layer ground splatting blends sand, rock, and mesa layers according to slope and elevation.
- [x] WGSL shader compiles cleanly and produces valid render pipeline descriptors.
- [x] Instanced clutter generator places thousands of detail instances with distance culling.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all gates.
