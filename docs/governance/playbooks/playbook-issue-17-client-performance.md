# Playbook: Issue 17 — Performance & Cross-Platform: GPU Compute Occlusion Culling, LOD Geomorphing & Packaging (`swg-client-render`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#17  
> **Branch:** `enhancement/17-gpu-culling-cross-platform`  
> **Status:** Implementation Playbook  
> **Related Files:**
> - [gpu_culling.rs](file:///Users/raybayly/Development/BAI/GAMES/SWG/Client/crates/swg-client-render/src/gpu_culling.rs)
> - [geomorph.rs](file:///Users/raybayly/Development/BAI/GAMES/SWG/Client/crates/swg-client-render/src/geomorph.rs)
> - [platform.rs](file:///Users/raybayly/Development/BAI/GAMES/SWG/Client/crates/swg-client-render/src/platform.rs)
> - [gpu_culling_tests.rs](file:///Users/raybayly/Development/BAI/GAMES/SWG/Client/crates/swg-client-render/tests/gpu_culling_tests.rs)

---

## 1. Objectives & Scope

Optimize and package the client across operating systems in `crates/swg-client-render`:

1. **GPU Compute Shader Occlusion & Frustum Culling (`gpu_culling.rs`):**
   - High-throughput parallel WGSL compute shader (`CULLING_COMPUTE_WGSL`) evaluating frustum plane tests and Hi-Z depth occlusion.
   - Indirect draw buffer compaction (`wgpu::util::DrawIndirectArgs`) eliminating CPU-to-GPU synchronization bottlenecks.
   - Replaces single-threaded legacy CPU DPVS culling with massive parallel GPU compute execution.

2. **LOD Geomorphing (`geomorph.rs`):**
   - Continuous distance-based transition blend shapes ($t \in [0.0, 1.0]$) eliminating visual mesh popping across LOD transitions.
   - Hermite/linear morph interpolation between adjacent LOD vertex topologies.
   - Real-time CPU and WGSL shader morph evaluation.

3. **Cross-Platform Target Architecture & Packaging (`platform.rs`):**
   - Native support for Windows (DirectX 12 / Vulkan), macOS (Metal), and Linux (Vulkan).
   - Platform backend capability detection and automated packaging manifests (`.app` bundle for macOS, Windows executable manifest, Linux AppImage descriptor).

4. **100+ FPS Dense City Performance Benchmark:**
   - Benchmark simulation in dense urban environments (50+ active player characters, 10,000+ clutter and building instances).
   - Verification of 100+ FPS (targeting 120+ FPS) with frame processing times under 8.33ms.

---

## 2. Acceptance Criteria & Quality Gates

- [x] GPU compute culling compiles cleanly in WGSL and compacts visible instances into indirect draw buffers.
- [x] LOD geomorphing smoothly transitions mesh vertices between LOD levels with zero visual popping.
- [x] Cross-platform configuration validates Windows (DX12/Vulkan), macOS (Metal), and Linux (Vulkan).
- [x] Sustains 100+ FPS in dense player city simulation with 50+ concurrent characters.
- [x] 100% test pass rate across workspace (`cargo test --workspace`).
- [x] Zero warnings across workspace (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes cleanly before PR creation.
