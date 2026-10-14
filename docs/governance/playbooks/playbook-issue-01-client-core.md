# Playbook: Issue 01 — Client Core: Initialize Client Workspace, winit + wgpu / Bevy Rendering Pipeline (`swg-client-core`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#1  
> **Branch:** `enhancement/1-client-core-workspace-wgpu`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Setup the Strategy B 64-bit modern client workspace and core substrate in `crates/swg-client-core`:

1. **Workspace & Toolchain Initialization:**
   - Define root `Cargo.toml` with workspace members and pinned shared dependencies (`glam`, `bevy_ecs`, `wgpu`, `winit`, `thiserror`, `tokio`, `serde`).
   - Configure SonarCloud properties and quality gates.

2. **Windowing, High-DPI & Display Scaling (`window.rs`):**
   - High-DPI resolution scaling calculation mapping logical coordinates to physical framebuffers without blurring.
   - Support for high-refresh display presentation modes (`Mailbox` / `Immediate` for 144Hz+ unthrottled rendering, `Fifo` for V-Sync).
   - Configurable window modes (Windowed, Borderless Fullscreen, Exclusive Fullscreen).

3. **Graphics Device Abstraction & Clustered Forward+ Pipeline (`render.rs`, `cluster.rs`):**
   - Headless and hardware-accelerated `wgpu` initialization supporting Vulkan, Metal, and DX12 backends.
   - Clustered Forward+ rendering pipeline setup:
     - 3D frustum slicing into cluster tiles ($16 \times 9 \times 24$ depth slices).
     - Point light and spotlight spatial clustering and assignment.
     - Unified camera projection matrix computation.

4. **ECS World & System Scheduler (`ecs.rs`):**
   - Integration with `bevy_ecs`: World initialization, component definitions (`Transform`, `Camera`, `Light`, `LocalPlayer`).
   - Deterministic client simulation schedule.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Renders clear window at 144Hz+ across macOS (Metal) and Windows/Linux (Vulkan).
- [x] High-DPI display scaling support without blurring.
- [x] Clustered Forward+ light grid slices frustum and assigns light indices.
- [x] ECS world schedule updates entity transforms and camera matrices.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all gates.
