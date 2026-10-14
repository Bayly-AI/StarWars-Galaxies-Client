// crates/swg-client-render/src/lib.rs

pub mod clutter;
pub mod pipeline;
pub mod splat;

pub use clutter::{ClutterGenerator, ClutterInstance, ClutterInstanceRaw, ClutterType};
pub use pipeline::{TERRAIN_WGSL, TerrainPipeline};
pub use splat::{SplatWeights, TerrainMaterialPalette, TerrainPbrMaterial};
