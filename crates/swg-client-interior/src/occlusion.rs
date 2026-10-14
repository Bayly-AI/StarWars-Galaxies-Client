// crates/swg-client-interior/src/occlusion.rs

use crate::cell::BuildingInterior;
use glam::Vec3;
use std::collections::{HashSet, VecDeque};
use swg_client_world::CameraFrustum;

/// Visibility results determining which interior cells and outdoor terrain to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortalVisibilityResult {
    pub visible_cells: HashSet<u64>,
    pub outdoor_terrain_visible: bool,
    pub traversed_portals: Vec<u32>,
}

pub struct PortalOcclusionCuller;

impl PortalOcclusionCuller {
    /// Evaluates portal visibility and indoor/outdoor occlusion culling.
    #[must_use]
    pub fn evaluate_visibility(
        interior: &BuildingInterior,
        camera_cell_id: u64,
        camera_world_pos: Vec3,
        _camera_world_frustum: &CameraFrustum,
    ) -> PortalVisibilityResult {
        let mut visible_cells = HashSet::new();
        let mut outdoor_terrain_visible = false;
        let mut traversed_portals = Vec::new();

        if camera_cell_id == 0 {
            // Camera is outdoors: outdoor terrain is always rendered
            outdoor_terrain_visible = true;

            // Check entrance portals leading from 0 into building interior
            for portal in &interior.portals {
                if portal.is_entrance() && portal.is_open() {
                    let target = if portal.source_cell_id == 0 {
                        portal.target_cell_id
                    } else {
                        portal.source_cell_id
                    };

                    if target != 0 {
                        visible_cells.insert(target);
                        traversed_portals.push(portal.id);

                        // Traverse deeper into adjoined rooms from the entrance
                        let mut queue = VecDeque::new();
                        queue.push_back(target);

                        while let Some(current_cell) = queue.pop_front() {
                            for p in interior.portals_for_cell(current_cell) {
                                if p.is_open() && !traversed_portals.contains(&p.id) {
                                    let next_cell = if p.source_cell_id == current_cell {
                                        p.target_cell_id
                                    } else {
                                        p.source_cell_id
                                    };

                                    if next_cell != 0 && visible_cells.insert(next_cell) {
                                        traversed_portals.push(p.id);
                                        queue.push_back(next_cell);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Camera is indoors in a specific cell
            let local_cam_pos = interior.world_to_cell_point(camera_world_pos);

            let mut queue = VecDeque::new();
            visible_cells.insert(camera_cell_id);
            queue.push_back(camera_cell_id);

            while let Some(current_cell) = queue.pop_front() {
                for portal in interior.portals_for_cell(current_cell) {
                    if !portal.is_open() {
                        continue;
                    }

                    // Check portal orientation relative to local camera
                    let is_facing = portal.is_in_front(local_cam_pos);
                    if !is_facing && current_cell == camera_cell_id {
                        // Portal is behind the camera in the starting cell
                        continue;
                    }

                    let next_cell = if portal.source_cell_id == current_cell {
                        portal.target_cell_id
                    } else {
                        portal.source_cell_id
                    };

                    if next_cell == 0 {
                        // Entrance doorway leads to outdoors: outdoor terrain becomes visible
                        outdoor_terrain_visible = true;
                        traversed_portals.push(portal.id);
                    } else if visible_cells.insert(next_cell) {
                        traversed_portals.push(portal.id);
                        queue.push_back(next_cell);
                    }
                }
            }
        }

        PortalVisibilityResult {
            visible_cells,
            outdoor_terrain_visible,
            traversed_portals,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellObject;
    use crate::portal::{DoorState, Portal};
    use glam::Mat4;

    #[test]
    fn test_indoor_occlusion_culls_outdoor_terrain_when_closed() {
        let mut interior = BuildingInterior::new(1001, Mat4::IDENTITY);

        // Room 1: Entrance foyer (cell 1)
        interior.add_cell(CellObject::new(
            1,
            1001,
            "foyer",
            Vec3::new(-5.0, 0.0, 0.0),
            Vec3::new(5.0, 4.0, 10.0),
        ));

        // Room 2: Deep inner vault (cell 2)
        interior.add_cell(CellObject::new(
            2,
            1001,
            "vault",
            Vec3::new(-5.0, 0.0, 10.0),
            Vec3::new(5.0, 4.0, 20.0),
        ));

        // Portal 1: Entrance from outdoors (0) to Foyer (1) - CLOSED
        interior.add_portal(Portal::new(
            1,
            0,
            1,
            vec![
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 3.0, 0.0),
                Vec3::new(-1.0, 3.0, 0.0),
            ],
            DoorState::Closed,
        ));

        // Portal 2: Connecting Foyer (1) to Vault (2) - OPEN
        interior.add_portal(Portal::new(
            2,
            1,
            2,
            vec![
                Vec3::new(-1.0, 0.0, 10.0),
                Vec3::new(1.0, 0.0, 10.0),
                Vec3::new(1.0, 3.0, 10.0),
                Vec3::new(-1.0, 3.0, 10.0),
            ],
            DoorState::Open,
        ));

        let frustum = CameraFrustum::from_view_projection(Mat4::IDENTITY);

        // Camera inside vault (cell 2) with closed entrance: outdoor terrain must NOT render
        let vis = PortalOcclusionCuller::evaluate_visibility(
            &interior,
            2,
            Vec3::new(0.0, 1.8, 15.0),
            &frustum,
        );

        assert!(vis.visible_cells.contains(&2));
        assert!(vis.visible_cells.contains(&1));
        assert!(
            !vis.outdoor_terrain_visible,
            "Outdoor terrain must be completely occluded when entrance door is closed"
        );

        // Now open the entrance door
        interior.portals[0].door_state = DoorState::Open;

        let vis_open = PortalOcclusionCuller::evaluate_visibility(
            &interior,
            2,
            Vec3::new(0.0, 1.8, 15.0),
            &frustum,
        );

        assert!(
            vis_open.outdoor_terrain_visible,
            "Outdoor terrain becomes visible through open entrance door"
        );
    }
}
