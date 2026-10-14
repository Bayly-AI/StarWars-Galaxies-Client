// crates/swg-client-core/src/lib.rs

pub mod cluster;
pub mod ecs;
pub mod render;
pub mod window;

pub use cluster::{
    ClusterAabb, ClusterGridDimensions, ClusterTile, ClusteredLightGrid, ClusteredPointLight,
};
pub use ecs::{
    Camera, ClientWorld, DirectionalLight, GlobalTransform, LightGridResource, LocalPlayer,
    PointLight, TimeStep, Transform,
};
pub use render::{GraphicsBackend, RenderConfig, RenderContext, RenderError};
pub use window::{DisplayMetrics, PresentModeSetting, WindowConfig, WindowMode};

/// Version information and initialization check for the SWG Modern Client.
#[must_use]
pub fn client_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_version() {
        assert_eq!(client_version(), "0.1.0");
    }
}
