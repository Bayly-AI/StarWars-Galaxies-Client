// crates/swg-client-interior/src/cell.rs

use crate::portal::Portal;
use glam::{Mat4, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An interior room or spatial cell within a building structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellObject {
    pub cell_id: u64,
    pub building_id: u64,
    pub name: String,
    pub local_bounds_min: Vec3,
    pub local_bounds_max: Vec3,
    pub object_ids: Vec<u64>,
}

impl CellObject {
    #[must_use]
    pub fn new(
        cell_id: u64,
        building_id: u64,
        name: impl Into<String>,
        local_bounds_min: Vec3,
        local_bounds_max: Vec3,
    ) -> Self {
        Self {
            cell_id,
            building_id,
            name: name.into(),
            local_bounds_min,
            local_bounds_max,
            object_ids: Vec::new(),
        }
    }

    pub fn add_object(&mut self, object_id: u64) {
        self.object_ids.push(object_id);
    }

    /// Tests if a local point is inside this cell's bounding box.
    #[must_use]
    pub fn contains_local_point(&self, p: Vec3) -> bool {
        p.x >= self.local_bounds_min.x
            && p.x <= self.local_bounds_max.x
            && p.y >= self.local_bounds_min.y
            && p.y <= self.local_bounds_max.y
            && p.z >= self.local_bounds_min.z
            && p.z <= self.local_bounds_max.z
    }
}

/// A complete building interior containing multiple interconnected cells and portals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingInterior {
    pub building_id: u64,
    pub world_transform: Mat4,
    pub inv_world_transform: Mat4,
    pub cells: HashMap<u64, CellObject>,
    pub portals: Vec<Portal>,
}

impl BuildingInterior {
    #[must_use]
    pub fn new(building_id: u64, world_transform: Mat4) -> Self {
        let inv_world_transform = world_transform.inverse();
        Self {
            building_id,
            world_transform,
            inv_world_transform,
            cells: HashMap::new(),
            portals: Vec::new(),
        }
    }

    pub fn add_cell(&mut self, cell: CellObject) {
        self.cells.insert(cell.cell_id, cell);
    }

    pub fn add_portal(&mut self, portal: Portal) {
        self.portals.push(portal);
    }

    #[must_use]
    pub fn get_cell(&self, cell_id: u64) -> Option<&CellObject> {
        self.cells.get(&cell_id)
    }

    #[must_use]
    pub fn get_cell_by_name(&self, name: &str) -> Option<&CellObject> {
        self.cells.values().find(|c| c.name == name)
    }

    /// Transforms a point from local building/cell space to global planetary coordinates.
    #[must_use]
    pub fn cell_to_world_point(&self, local_point: Vec3) -> Vec3 {
        (self.world_transform * Vec4::new(local_point.x, local_point.y, local_point.z, 1.0))
            .truncate()
    }

    /// Transforms a point from global planetary coordinates into local building/cell space.
    #[must_use]
    pub fn world_to_cell_point(&self, world_point: Vec3) -> Vec3 {
        (self.inv_world_transform * Vec4::new(world_point.x, world_point.y, world_point.z, 1.0))
            .truncate()
    }

    /// Returns portals connected to a given cell.
    pub fn portals_for_cell(&self, cell_id: u64) -> impl Iterator<Item = &Portal> {
        self.portals
            .iter()
            .filter(move |p| p.source_cell_id == cell_id || p.target_cell_id == cell_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_coordinate_transforms() {
        // Building rotated 90 degrees around Y and positioned at (100, 5, 200)
        let world_mat = Mat4::from_translation(Vec3::new(100.0, 5.0, 200.0))
            * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let mut interior = BuildingInterior::new(5001, world_mat);

        let cell = CellObject::new(
            1,
            5001,
            "cantina_foyer",
            Vec3::new(-10.0, 0.0, -10.0),
            Vec3::new(10.0, 5.0, 10.0),
        );
        interior.add_cell(cell);

        // Local origin (0, 0, 0) in cell corresponds to building world origin (100, 5, 200)
        let world_pt = interior.cell_to_world_point(Vec3::ZERO);
        assert!((world_pt.x - 100.0).abs() < 1e-4);
        assert!((world_pt.y - 5.0).abs() < 1e-4);
        assert!((world_pt.z - 200.0).abs() < 1e-4);

        // Roundtrip back to local space
        let local_pt = interior.world_to_cell_point(world_pt);
        assert!(local_pt.length() < 1e-4);
    }
}
