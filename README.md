# StarWars-Galaxies-Client

> **Ground-Up 64-Bit Modern Client for Star Wars Galaxies (Strategy B)**  
> Built in Rust with `wgpu`, `winit`, `bevy_ecs`, `kira`, and `egui`.

---

## Overview

`StarWars-Galaxies-Client` is a clean-room, native 64-bit cross-platform client designed to replace the legacy 32-bit DirectX 9 client (`SwgClient_r.exe`).

By mounting authentic Star Wars Galaxies `.tre` archives directly into a zero-copy Virtual File System (VFS) and re-implementing the procedural `.trn` fractal terrain pipeline, it delivers modern visual fidelity, high frame rates (144Hz+), native multi-monitor/4K scaling, and support for Windows, macOS (via Metal), and Linux (via Vulkan).

---

## Strategy B Architectural Pillars

* **Zero-Copy Virtual File System (`swg-tre`):** Mounts cascaded `.tre` archives via memory-mapped files (`memmap2`) with authentic priority resolution (e.g. `patch_14_00.tre` overriding `data_sku0_00.tre`).
* **Procedural `.trn` Terrain Pipeline (`swg-client-render`):** Direct mathematical re-implementation of the SOE procedural terrain generator. Evaluates fractal noise, affine transforms, and slope/height boundaries to tessellate 3D terrain meshes in real-time with multi-layer texture splatting.
* **Modern Cross-Platform Renderer (`wgpu`):** Uses modern Vulkan, Metal, and DirectX 12 backends. Features clustered forward lighting for dynamic blaster fire and lightsaber glows, PBR material workflows, and GPU compute-shader mesh culling.
* **Skeletal Animation & Modular Mesh System (`swg-client-mesh`):** Loads `.msh` static meshes, `.skm` skeletal hierarchies, and `.sat` attachment templates. Blends animations from `.ans` state machines and supports character morph sliders (height, weight, muscle mass) via vertex shaders.
* **Authentic Pre-CU HUD & Vector UI (`swg-client-ui`):** Recreates the beloved Pre-CU interface (HAM bars, planetary radar, target status, contextual radial menus, floating containers, and chat windows) with high-DPI font rendering and scalable layout boxes.
* **Predictive Network Subsystem (`swg-client-net`):** Implements the client-side SOE UDP protocol state machine with local client movement prediction, reconciliation, and Hermite spline interpolation for smooth remote entity movement.
* **3D Positional Audio (`swg-client-audio`):** Modern spatial audio engine (`kira`) supporting dynamic music transitions, cantina performances, ambient environmental loops, and blaster sound effects.

---

## Directory Structure

```
Client/
├── Cargo.toml                  # Client workspace configuration
├── crates/
│   ├── swg-client-app/         # Client main executable & winit loop
│   ├── swg-client-render/      # wgpu graphics engine, shaders, terrain tessellator
│   ├── swg-client-mesh/        # .msh, .skm skeletal mesh and .ans animation blender
│   ├── swg-client-ui/          # Authentic Pre-CU HUD, radial menus & containers
│   ├── swg-client-net/         # Client SOE UDP socket, prediction & interpolation
│   └── swg-client-audio/       # 3D spatial audio and interactive music engine
└── docs/
    ├── ARCHITECTURE.md         # Full client rendering pipeline and subsystems
    └── PRE_CU_SPECIFICATION.md # Gameplay, UI, and animation behavioral specs
```

---

## Documentation
* See [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) for details on terrain tessellation, the asset pipeline, and the rendering architecture.
* See [`docs/PRE_CU_SPECIFICATION.md`](./docs/PRE_CU_SPECIFICATION.md) for gameplay interface and asset specifications.
