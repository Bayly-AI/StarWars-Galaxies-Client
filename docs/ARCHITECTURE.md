# SWG Modern Client Architecture Specification (Strategy B)

> **Project:** StarWars-Galaxies-Client  
> **Target Version:** Pre-Combat Upgrade (Publish 14.1)  
> **Language:** Rust (2024 Edition / 1.80+)  
> **Graphics Backend:** `wgpu` (Vulkan on Linux/Windows, Metal on macOS, DX12 on Windows)  
> **Key Crates:** `winit`, `wgpu`, `bevy_ecs`, `kira`, `egui`, `memmap2`, `glam`

---

## 1. High-Level Engine Pipeline

The modern client replaces the legacy 32-bit DirectX 9 pipeline with a modular, data-driven architecture:

```
[OS Window & Events (winit)] ---> [Local Simulation (bevy_ecs)]
                                              |
                   +--------------------------+--------------------------+
                   |                          |                          |
                   v                          v                          v
       [Network Client (tokio)]     [Terrain & Scene Graph]     [UI Engine (egui)]
       - SOE UDP Socket             - Procedural .trn Heights   - Pre-CU Radar & HAM
       - Prediction Buffer          - Skeletal Mesh Blending    - Radial Menus
       - Remote Interpolation       - Clustered Forward+ PBR    - Inventory Windows
                   \                          |                          /
                    \                         v                         /
                     +-------------> [Renderer (wgpu)] <---------------+
                                              |
                                      (Vulkan / Metal / DX12)
                                              |
                                              v
                                         [Display]
```

---

## 2. Virtual File System (VFS) & `.tre` Archives

The original game assets are distributed inside compressed Tree Archive (`.tre`) files.

### Priority Cascading:
The client mounts archives in strict reverse-patch order:
1. `patch_14_00.tre` (Highest priority)
2. `patch_13_00.tre` ... `patch_00_00.tre`
3. `data_sku0_07.tre` ... `data_sku0_00.tre` (Base assets)

### Zero-Copy Memory Mapping:
* Archives are mapped into 64-bit virtual memory using `memmap2`.
* Reading a texture or model reads directly from memory-mapped pages without intermediate file buffer copying.
* Decompression (`zlib`) is performed on-demand in multi-threaded worker pools.

---

## 3. Procedural `.trn` Terrain Generation

Rather than loading pre-computed heightmaps, SWG planets are mathematically generated at runtime:

### Mathematical Height Evaluation:
$$Y(X, Z) = Y_{\text{base}} + \sum_{k} \left( \text{Fractal}_k(X, Z) \times \text{Weight}_k(X, Z) \right) + \text{HeightOffset}(X, Z)$$

* **Fractals:** Multi-octave Perlin / Simplex noise generators scaled by planetary frequency.
* **Boundaries:** Radial, rectangular, and polygonal boundary rules define craters, mountains, and plains.
* **Filters:** Slope and elevation filters dictate which terrain splat textures (dune sand, red rock, grass) and clutter flora (grass tufts, desert pebbles) are applied.

### Dynamic Mesh Tessellation:
* The world is segmented into $64\text{m} \times 64\text{m}$ terrain chunks.
* As the camera moves, nearby chunks tessellate at high density ($65 \times 65$ vertices), while distant chunks transition through Level of Detail (LOD) levels with edge geomorphing to prevent crack artifacts.

---

## 4. 3D Meshes, Skeletal Rigging & Animation

1. **Rigid Meshes (`.msh`):**
   * Loaded into GPU vertex buffers with positions, normals, UVs, and material indices.
   * Shaders support physical properties: diffuse, specular/roughness maps, and normal maps.
2. **Skeletal Meshes (`.skm`) & Skeleton Attachment Tables (`.sat`):**
   * Characters are assembled modularly: Head, Chest, Arms, Hands, Legs, Feet.
   * Bone weights (up to 4 bones per vertex) drive smooth mesh skinning in vertex/compute shaders.
3. **Animation Engine (`.ans`):**
   * Animation State Machine handles locomotion blending (Walk, Run, Sprint), posture transitions (Stand, Kneel, Prone), and combat ability executions.
   * Quaternions and root motion are evaluated per frame.

---

## 5. Pre-CU User Interface Engine

The UI engine faithfully recreates the classic Pre-CU interface using modern vector graphics:
* **The HAM Monitor:** Displays Health, Action, and Mind bars with current values, max capacity, and wound notches.
* **Planetary Radar:** Dynamic minimap indicating cardinal orientation, planetary coordinates $(X, Z)$, waypoint pins, and nearby player/creature blips.
* **Contextual Radial Menus:** Right-click context menus with dynamic radial slices (`Examine`, `Loot`, `Open`, `Trade`, `Sample`).
* **Container Windows:** High-DPI scrollable inventories with full drag-and-drop item movement.

---

## 6. Client-Side Prediction & Interpolation

* **Local Player Prediction:** Movement keystrokes apply immediately to the local player entity. The client sends movement timestamps to the server; if a server reconciliation packet disagrees, the camera smoothly lerps to the corrected location.
* **Remote Player Interpolation:** Remote entities are rendered from a 100ms jitter buffer using Hermite spline curves, guaranteeing smooth motion even under packet jitter.
