# Playbook: Issue 16 — Character Customization: Species, Morph Sliders & Palette Tinting (`swg-client-customization`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#16  
> **Branch:** `enhancement/16-character-customization-palettes`  
> **Status:** Implementation Playbook  
> **Related Files:**
> - [species.rs](file:///Users/raybayly/Development/BAI/GAMES/SWG/Client/crates/swg-client-customization/src/species.rs)
> - [morph.rs](file:///Users/raybayly/Development/BAI/GAMES/SWG/Client/crates/swg-client-customization/src/morph.rs)
> - [palette.rs](file:///Users/raybayly/Development/BAI/GAMES/SWG/Client/crates/swg-client-customization/src/palette.rs)
> - [customization_pipeline_tests.rs](file:///Users/raybayly/Development/BAI/GAMES/SWG/Client/crates/swg-client-customization/tests/customization_pipeline_tests.rs)

---

## 1. Objectives & Scope

Implement complete character creation and appearance customization in `crates/swg-client-customization`:

1. **Galactic Species & Gender Profiles (`species.rs`):**
   - 8 playable species: Human, Trandoshan, Wookiee, Twi'lek, Zabrak, Bothan, Mon Calamari, and Rodian.
   - Gender support (Male and Female) with species-specific anatomy constraints, scale limits, and default skeletal rigs.
   - Species feature profiles (Twi'lek lekku, Zabrak horns, Trandoshan scales, Wookiee fur).

2. **Real-Time Vertex Morph Sliders (`morph.rs`):**
   - Blend shape deformation engine applying delta offsets ($\Delta V$, $\Delta N$) to base 3D vertex meshes.
   - Sliders for Height, Weight, Muscle definition, and facial/cranial feature morph targets (jawline, brow, eye size, lekku length, horn size).
   - Real-time linear blend calculation preserving watertight mesh topology and surface normals.

3. **Color Ramp Palette Engine (`palette.rs`):**
   - SWG color ramp palette (`.pal`) binary parser and color indexing.
   - Dynamic tinting lookup for skin tones, hair colors, eye colors, and clothing dyes.
   - Full serialization and deserialization of player character customization strings.

---

## 2. Acceptance Criteria & Quality Gates

- [x] All 8 canonical species and both genders defined with accurate scale bounds and asset mappings.
- [x] Morph sliders deform 3D character meshes in real-time with zero vertex distortion or disjoint normals.
- [x] Palette indexing accurately samples and tints skin, hair, eyes, and clothing colors.
- [x] 100% test pass rate across workspace (`cargo test --workspace`).
- [x] Zero warnings across workspace (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes cleanly before PR creation.
