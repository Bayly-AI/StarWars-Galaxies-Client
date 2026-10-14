// crates/swg-client-world/src/scene_graph.rs

use crate::culling::{CameraFrustum, DistanceCuller};
use crate::ws::{WorldSnapshot, WorldSnapshotNode};
use glam::Vec3;
use std::collections::{HashMap, HashSet};

/// 2D Spatial grid acceleration structure for 16km x 16km planetary terrain object streaming.
pub struct SpatialSceneGraph {
    pub cell_size: f32,
    nodes: Vec<WorldSnapshotNode>,
    grid: HashMap<(i32, i32), Vec<usize>>,
}

impl SpatialSceneGraph {
    #[must_use]
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size: cell_size.max(64.0),
            nodes: Vec::new(),
            grid: HashMap::new(),
        }
    }

    /// Builds a spatial scene graph from a World Snapshot.
    #[must_use]
    pub fn from_snapshot(snapshot: WorldSnapshot, cell_size: f32) -> Self {
        let mut graph = Self::new(cell_size);
        for node in snapshot.nodes {
            graph.insert(node);
        }
        graph
    }

    /// Inserts a static world node into the spatial acceleration structure.
    pub fn insert(&mut self, node: WorldSnapshotNode) {
        let node_idx = self.nodes.len();
        let r = node.radius;

        let min_cx = ((node.position.x - r) / self.cell_size).floor() as i32;
        let max_cx = ((node.position.x + r) / self.cell_size).floor() as i32;
        let min_cz = ((node.position.z - r) / self.cell_size).floor() as i32;
        let max_cz = ((node.position.z + r) / self.cell_size).floor() as i32;

        for cx in min_cx..=max_cx {
            for cz in min_cz..=max_cz {
                self.grid.entry((cx, cz)).or_default().push(node_idx);
            }
        }

        self.nodes.push(node);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Queries all visible objects within the camera frustum and maximum draw distance.
    #[must_use]
    pub fn query_visible(
        &self,
        camera_pos: Vec3,
        frustum: &CameraFrustum,
        max_distance: f32,
    ) -> Vec<&WorldSnapshotNode> {
        let culler = DistanceCuller::new(max_distance);

        let min_cx = ((camera_pos.x - max_distance) / self.cell_size).floor() as i32;
        let max_cx = ((camera_pos.x + max_distance) / self.cell_size).floor() as i32;
        let min_cz = ((camera_pos.z - max_distance) / self.cell_size).floor() as i32;
        let max_cz = ((camera_pos.z + max_distance) / self.cell_size).floor() as i32;

        let mut visited_indices = HashSet::new();
        let mut visible_nodes = Vec::new();

        for cx in min_cx..=max_cx {
            for cz in min_cz..=max_cz {
                if let Some(indices) = self.grid.get(&(cx, cz)) {
                    for &idx in indices {
                        if visited_indices.insert(idx) {
                            let node = &self.nodes[idx];

                            // 1. Distance culling
                            if !culler.is_visible(camera_pos, node.position, node.radius) {
                                continue;
                            }

                            // 2. Frustum culling
                            if !frustum.intersects_sphere(node.position, node.radius) {
                                continue;
                            }

                            visible_nodes.push(node);
                        }
                    }
                }
            }
        }

        visible_nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Mat4, Quat};

    #[test]
    fn test_spatial_query_culling() {
        let mut graph = SpatialSceneGraph::new(256.0);

        // Target building in front of camera
        graph.insert(WorldSnapshotNode::new(
            1,
            "shared_cantina.iff",
            0,
            Vec3::new(0.0, 0.0, -100.0),
            Quat::IDENTITY,
            30.0,
        ));

        // Building behind camera
        graph.insert(WorldSnapshotNode::new(
            2,
            "shared_starport.iff",
            0,
            Vec3::new(0.0, 0.0, 100.0),
            Quat::IDENTITY,
            50.0,
        ));

        // Building very far away
        graph.insert(WorldSnapshotNode::new(
            3,
            "shared_palace.iff",
            0,
            Vec3::new(0.0, 0.0, -5000.0),
            Quat::IDENTITY,
            100.0,
        ));

        let proj = Mat4::perspective_rh(1.0, 1.333, 0.1, 1000.0);
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0), Vec3::Y);
        let frustum = CameraFrustum::from_view_projection(proj * view);

        let visible = graph.query_visible(Vec3::ZERO, &frustum, 1000.0);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].network_id, 1);
    }
}
