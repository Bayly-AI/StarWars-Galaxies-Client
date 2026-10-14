// crates/swg-client-mesh/src/lib.rs

pub mod lighting;
pub mod material;
pub mod msh;

pub use lighting::{MeshLightingEvaluator, SceneLighting};
pub use material::PbrMaterial;
pub use msh::{MeshError, MshWriter, RigidMesh, SubMesh};
