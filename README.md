# StarWars-Galaxies-Client (Client)

**Ground-Up 64-Bit Modern Client for Star Wars Galaxies (Strategy B) & Enterprise Agentic Application**

> Technical Reference: See [TECH_README.md](TECH_README.md) for architecture, subsystem maps, and specifications.

---

## 🌟 Sales & Feature Overview

`StarWars-Galaxies-Client` is a clean-room, native 64-bit cross-platform client designed to replace the legacy 32-bit DirectX 9 client (`SwgClient_r.exe`). Powered by Rust and governed by the HATHOR agentic framework:

- 🎮 **Strategy B Modern Engine**: Native 64-bit engine mounting authentic `.tre` archives via zero-copy memory mapping and procedural `.trn` terrain tessellation.
- 🎨 **Modern Graphics (`wgpu`)**: Cross-platform Vulkan, Metal, and DirectX 12 rendering with clustered forward lighting and high refresh rate support (144Hz+).
- 🔊 **3D Positional Audio (`kira`)**: Dynamic cantina performances, ambient planet soundscapes, and combat audio.
- 🛡️ **Zero-Trust Governance & Execution**: Hardened boundaries against unauthorized file mutations and automated quality gates.
- 🧠 **Tri-Graph Substrate**: Integrated KnowledgeGraph, ContextGraph, and MemoryGraph for autonomous reasoning.
- ⚡ **Autonomous Workflows**: Pre-configured bot workflows for CI/CD and repository hygiene.

---

## Quick Start

```sh
hath0r doctor
hath0r agentgraph status
```

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

```text
Client/
├── Cargo.toml                  # Client workspace configuration
├── crates/
│   ├── swg-client-app/         # Client main executable & winit loop
│   ├── swg-client-render/      # wgpu graphics engine, shaders, terrain tessellator
│   ├── swg-client-mesh/        # .msh, .skm skeletal mesh and .ans animation blender
│   ├── swg-client-ui/          # Authentic Pre-CU HUD, radial menus & containers
│   ├── swg-client-net/         # Client SOE UDP socket, prediction & interpolation
│   └── swg-client-audio/       # 3D spatial audio and interactive music engine
├── docs/
│   ├── ARCHITECTURE.md         # Full client rendering pipeline and subsystems
│   ├── PRE_CU_SPECIFICATION.md # Gameplay, UI, and animation behavioral specs
│   └── governance/playbooks/   # HATH0R coding and troubleshooting playbooks
├── contracts/                  # JSON schema contracts
├── cfg/                        # Configuration definitions
└── tests/                      # Automated test harnesses
```

---

## Documentation
* See [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) for details on terrain tessellation, the asset pipeline, and the rendering architecture.
* See [`docs/PRE_CU_SPECIFICATION.md`](./docs/PRE_CU_SPECIFICATION.md) for gameplay interface and asset specifications.
* See [`TECH_README.md`](./TECH_README.md) for technical architecture maps and subsystem registries.

---

## License

Apache License 2.0
