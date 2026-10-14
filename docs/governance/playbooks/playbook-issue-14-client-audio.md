# Playbook: Issue 14 — Audio: 3D Positional Spatial Audio & Dynamic Planetary Music (`swg-client-audio`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#14  
> **Branch:** `enhancement/14-spatial-audio-dynamic-music`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement 3D spatial audio positioning and interactive dynamic music in `crates/swg-client-audio`:

1. **3D Audio Listener & Spatial Panning (`listener.rs`, `spatial.rs`):**
   - 3D listener tracking player camera position, orientation vectors (Forward, Up, Right), and velocity.
   - Distance attenuation models (Inverse-Square, Linear, Exponential rolloff) with near-distance clamping and maximum audible distance limits.
   - Vector panning across stereo headphones and modern 7.1 surround sound multi-channel configurations (Left, Right, Center, LFE, Side Surrounds, Rear Surrounds).

2. **Pre-CU Sound Effect Catalog (`sound_bank.rs`):**
   - Sound categories: Blaster rifle fire, Lightsaber ignition and swings, footsteps by terrain surface type (Sand, Stone, Metal, Grass), starship engines, and ambient planetary wind.

3. **Dynamic Interactive Music Director (`music.rs`):**
   - Dynamic cross-fading soundtrack engine evaluating combat excitement vs planetary ambience.
   - Smooth seamless transitions between ambient planetary themes, cantina band performances, combat skirmish cues, and victory/defeat stingers.
   - Volume curve cross-fading preventing abrupt audio cuts.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Audio sources attenuate and pan accurately in 3D space relative to listener position and orientation.
- [x] Supports multi-channel 7.1 surround spatial gains.
- [x] Dynamic music director transitions seamlessly between planetary ambience and combat excitement.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all quality gates.
