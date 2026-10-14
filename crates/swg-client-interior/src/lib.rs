// crates/swg-client-interior/src/lib.rs

pub mod cell;
pub mod occlusion;
pub mod portal;
pub mod transition;

pub use cell::{BuildingInterior, CellObject};
pub use occlusion::{PortalOcclusionCuller, PortalVisibilityResult};
pub use portal::{DoorState, Portal};
pub use transition::CellTransitionManager;
