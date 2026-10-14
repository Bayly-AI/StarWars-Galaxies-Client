# Technical Architecture — StarWars-Galaxies-Client

This document provides the definitive technical specification and subsystem map for `StarWars-Galaxies-Client` (Strategy B).

> See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full rendering pipeline, terrain shaders, and VFS architecture.  
> See [`docs/PRE_CU_SPECIFICATION.md`](docs/PRE_CU_SPECIFICATION.md) for gameplay interface and client-side asset specifications.

---

## Architecture Map

```text
[OS Window & Events (winit)] ───► [Local Simulation (bevy_ecs)]
                                          │
               ┌──────────────────────────┼──────────────────────────┐
               ▼                          ▼                          ▼
   [Network Client (tokio)]     [Terrain & Scene Graph]     [UI Engine (egui)]
   • SOE UDP Socket             • Procedural .trn Heights   • Pre-CU Radar & HAM
   • Prediction Buffer          • Skeletal Mesh Blending    • Radial Menus
   • Remote Interpolation       • Clustered Forward+ PBR    • Inventory Windows
               │                          │                          │
               └──────────────────────────┼──────────────────────────┘
                                          ▼
                                  [Renderer (wgpu)]
                              (Metal / Vulkan / DX12)
                                          │
                                          ▼
                            [HATH0R AgentGraph & Tri-Graph]
                               (KG / CG / MG Substrates)
```

---

## Subsystem Registry

- `.hath0r/`: Framework metadata root (configs, memory, cache, AgentGraph)
- `contracts/`: JSON schema contracts and validation specifications
- `cfg/`: Engine and client configuration schemas
- `docs/`: Technical specifications, architecture docs, and HATH0R governance playbooks (`docs/governance/playbooks/`)
- `tests/`: Automated regression tests, smoke tests, and stress harnesses
- `crates/`: Rust modular crates (`swg-client-app`, `swg-client-render`, `swg-client-mesh`, `swg-client-ui`, `swg-client-net`, `swg-client-audio`)

---

## Performance Targets

- **Frame Rate**: Minimum 60 FPS at 4K; 144Hz+ high-refresh display support
- **Terrain Tessellation**: Dynamic LOD evaluation under $2\text{ms}$ per frame
- **Asset Loading**: Memory-mapped zero-copy `.tre` reads with asynchronous zlib decompression
- **Network Smoothness**: Hermite spline interpolation with client-side movement prediction and reconciliation
