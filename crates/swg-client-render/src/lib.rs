// crates/swg-client-render/src/lib.rs

pub mod character_pipeline;
pub mod clutter;
pub mod dds;
pub mod geomorph;
pub mod gpu_culling;
pub mod pipeline;
pub mod platform;
pub mod splat;

pub use character_pipeline::{
    CHARACTER_WGSL, CharacterPipeline, CustomizationTintsRaw,
};
pub use clutter::{ClutterGenerator, ClutterInstance, ClutterInstanceRaw, ClutterType};
pub use dds::{DdsBuilder, DdsError, DdsFormat, DdsImage};
pub use geomorph::{GEOMORPH_WGSL, LodGeomorphConfig, LodGeomorphEvaluator};
pub use gpu_culling::{
    CULLING_COMPUTE_WGSL, CpuCullingEvaluator, CullableInstanceRaw, DrawIndirectArgs,
    FrustumUniforms, GpuCullingPipeline,
};
pub use pipeline::{TERRAIN_WGSL, TerrainPipeline};
pub use platform::{GraphicsApi, OperatingSystem, PlatformManager, PlatformPackageConfig};
pub use splat::{SplatWeights, TerrainMaterialPalette, TerrainPbrMaterial};
