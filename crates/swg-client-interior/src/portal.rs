// crates/swg-client-interior/src/portal.rs

use glam::Vec3;
use serde::{Deserialize, Serialize};
use swg_client_world::FrustumPlane;

/// Door interaction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DoorState {
    Open,
    Closed,
    Locked,
}

/// A 3D polygonal aperture or doorway connecting two cells or connecting an interior to the outdoors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Portal {
    pub id: u32,
    pub source_cell_id: u64,
    pub target_cell_id: u64,
    pub vertices: Vec<Vec3>,
    pub door_state: DoorState,
}

impl Portal {
    #[must_use]
    pub fn new(
        id: u32,
        source_cell_id: u64,
        target_cell_id: u64,
        vertices: Vec<Vec3>,
        door_state: DoorState,
    ) -> Self {
        Self {
            id,
            source_cell_id,
            target_cell_id,
            vertices,
            door_state,
        }
    }

    /// True if this portal leads between interior and the outdoor world (cell_id 0).
    #[must_use]
    pub fn is_entrance(&self) -> bool {
        self.source_cell_id == 0 || self.target_cell_id == 0
    }

    /// True if the portal is currently open for line-of-sight and traversal.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.door_state == DoorState::Open
    }

    /// Computes the arithmetic centroid of the portal polygon.
    #[must_use]
    pub fn center(&self) -> Vec3 {
        if self.vertices.is_empty() {
            return Vec3::ZERO;
        }
        let sum: Vec3 = self.vertices.iter().copied().sum();
        sum / (self.vertices.len() as f32)
    }

    /// Computes the outward plane normal (CCW winding of vertices 0, 1, 2).
    #[must_use]
    pub fn normal(&self) -> Vec3 {
        if self.vertices.len() >= 3 {
            let v0 = self.vertices[0];
            let v1 = self.vertices[1];
            let v2 = self.vertices[2];
            (v1 - v0).cross(v2 - v0).normalize_or_zero()
        } else {
            Vec3::Z
        }
    }

    /// Computes the geometric plane of the portal aperture.
    #[must_use]
    pub fn plane(&self) -> FrustumPlane {
        let n = self.normal();
        let c = self.center();
        let d = -n.dot(c);
        FrustumPlane::new(n.x, n.y, n.z, d)
    }

    /// Tests if a local camera position is in front of the portal polygon.
    #[must_use]
    pub fn is_in_front(&self, camera_pos: Vec3) -> bool {
        let p = self.plane();
        p.distance_to_point(camera_pos) > -0.05
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portal_plane_and_normal() {
        // Vertical doorway polygon on XY plane at Z = 5
        let vertices = vec![
            Vec3::new(-1.0, 0.0, 5.0),
            Vec3::new(1.0, 0.0, 5.0),
            Vec3::new(1.0, 2.5, 5.0),
            Vec3::new(-1.0, 2.5, 5.0),
        ];

        let portal = Portal::new(1, 101, 102, vertices, DoorState::Open);
        assert!(!portal.is_entrance());
        assert!(portal.is_open());

        let center = portal.center();
        assert!((center.x - 0.0).abs() < 1e-4);
        assert!((center.y - 1.25).abs() < 1e-4);
        assert!((center.z - 5.0).abs() < 1e-4);

        let normal = portal.normal();
        assert!((normal.z.abs() - 1.0).abs() < 1e-4);
    }
}
