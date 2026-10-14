// crates/swg-client-mesh/src/lib.rs

pub mod lighting;
pub mod material;
pub mod msh;
pub mod shader_template;

pub use lighting::{MeshLightingEvaluator, SceneLighting};
pub use material::PbrMaterial;
pub use msh::{MeshError, MshWriter, RigidMesh, SubMesh};
pub use shader_template::{
    BlendMode, PaletteBinding, ShaderError, ShaderTemplate, ShaderTemplateWriter, ShaderType,
};
