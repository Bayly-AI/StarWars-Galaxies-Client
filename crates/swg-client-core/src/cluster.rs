// crates/swg-client-core/src/cluster.rs

use glam::{Mat4, Vec3, Vec4};
use serde::{Deserialize, Serialize};

/// Clustered Forward+ Grid dimensions.
/// Default: 16 x 9 tile columns/rows x 24 exponential depth slices = 3456 clusters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterGridDimensions {
    pub x_tiles: usize,
    pub y_tiles: usize,
    pub z_slices: usize,
}

impl Default for ClusterGridDimensions {
    fn default() -> Self {
        Self {
            x_tiles: 16,
            y_tiles: 9,
            z_slices: 24,
        }
    }
}

impl ClusterGridDimensions {
    #[must_use]
    pub fn total_clusters(&self) -> usize {
        self.x_tiles * self.y_tiles * self.z_slices
    }

    #[must_use]
    pub fn linear_index(&self, x: usize, y: usize, z: usize) -> usize {
        (z * self.y_tiles + y) * self.x_tiles + x
    }
}

/// Axis-aligned bounding box in view space for a cluster frustum slice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClusterAabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl ClusterAabb {
    #[must_use]
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        // Find closest point on AABB to sphere center
        let closest = Vec3::new(
            center.x.clamp(self.min.x, self.max.x),
            center.y.clamp(self.min.y, self.max.y),
            center.z.clamp(self.min.z, self.max.z),
        );
        let dist_sq = (center - closest).length_squared();
        dist_sq <= radius * radius
    }
}

/// Point light representation for clustered forward shading.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClusteredPointLight {
    pub position: Vec3,
    pub radius: f32,
    pub color: Vec3,
    pub intensity: f32,
}

/// Represents a single cluster tile in the GPU-bound light grid.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterTile {
    pub offset: u32,
    pub count: u32,
}

/// Clustered Forward+ light grid managing frustum slicing and dynamic light assignment.
#[derive(Debug, Clone)]
pub struct ClusteredLightGrid {
    pub dims: ClusterGridDimensions,
    pub near_plane: f32,
    pub far_plane: f32,
    pub depth_slices: Vec<f32>,
    pub cluster_aabbs: Vec<ClusterAabb>,
    pub tiles: Vec<ClusterTile>,
    pub light_indices: Vec<u32>,
    pub max_lights_per_cluster: usize,
}

impl ClusteredLightGrid {
    /// Creates a clustered light grid with the specified dimensions, near plane, and far plane.
    #[must_use]
    pub fn new(dims: ClusterGridDimensions, near_plane: f32, far_plane: f32) -> Self {
        let total = dims.total_clusters();
        let depth_slices = Self::calculate_depth_slices(&dims, near_plane, far_plane);

        Self {
            dims,
            near_plane,
            far_plane,
            depth_slices,
            cluster_aabbs: Vec::with_capacity(total),
            tiles: vec![ClusterTile::default(); total],
            light_indices: Vec::new(),
            max_lights_per_cluster: 256,
        }
    }

    /// Computes exponential depth slicing boundaries between near and far planes.
    fn calculate_depth_slices(
        dims: &ClusterGridDimensions,
        near_plane: f32,
        far_plane: f32,
    ) -> Vec<f32> {
        let mut slices = Vec::with_capacity(dims.z_slices + 1);
        let ratio = far_plane / near_plane;
        for i in 0..=dims.z_slices {
            let exponent = i as f32 / dims.z_slices as f32;
            let z = near_plane * ratio.powf(exponent);
            slices.push(z);
        }
        slices
    }

    /// Recomputes view-space cluster AABBs based on inverse projection matrix.
    pub fn update_clusters(&mut self, inv_proj: Mat4) {
        self.cluster_aabbs.clear();

        for z in 0..self.dims.z_slices {
            let z_near_slice = self.depth_slices[z];
            let z_far_slice = self.depth_slices[z + 1];

            for y in 0..self.dims.y_tiles {
                for x in 0..self.dims.x_tiles {
                    // Normalized Device Coordinates (NDC) in [-1, 1]
                    let x_min_ndc = (x as f32 / self.dims.x_tiles as f32) * 2.0 - 1.0;
                    let x_max_ndc = ((x + 1) as f32 / self.dims.x_tiles as f32) * 2.0 - 1.0;
                    let y_min_ndc = (y as f32 / self.dims.y_tiles as f32) * 2.0 - 1.0;
                    let y_max_ndc = ((y + 1) as f32 / self.dims.y_tiles as f32) * 2.0 - 1.0;

                    // Unproject near corners
                    let p0 = Self::unproject_ndc(Vec3::new(x_min_ndc, y_min_ndc, 0.0), inv_proj);
                    let p1 = Self::unproject_ndc(Vec3::new(x_max_ndc, y_min_ndc, 0.0), inv_proj);
                    let p2 = Self::unproject_ndc(Vec3::new(x_min_ndc, y_max_ndc, 0.0), inv_proj);
                    let p3 = Self::unproject_ndc(Vec3::new(x_max_ndc, y_max_ndc, 0.0), inv_proj);

                    // Scale rays to current depth slice
                    let scale_near = z_near_slice / p0.z.abs().max(1e-4);
                    let scale_far = z_far_slice / p0.z.abs().max(1e-4);

                    let corners = [
                        p0 * scale_near,
                        p1 * scale_near,
                        p2 * scale_near,
                        p3 * scale_near,
                        p0 * scale_far,
                        p1 * scale_far,
                        p2 * scale_far,
                        p3 * scale_far,
                    ];

                    let mut min = Vec3::splat(f32::INFINITY);
                    let mut max = Vec3::splat(f32::NEG_INFINITY);

                    for pt in &corners {
                        min = min.min(*pt);
                        max = max.max(*pt);
                    }

                    self.cluster_aabbs.push(ClusterAabb { min, max });
                }
            }
        }
    }

    fn unproject_ndc(ndc: Vec3, inv_proj: Mat4) -> Vec3 {
        let clip = Vec4::new(ndc.x, ndc.y, ndc.z, 1.0);
        let view = inv_proj * clip;
        if view.w.abs() > 1e-6 {
            view.truncate() / view.w
        } else {
            view.truncate()
        }
    }

    /// Assigns world-space point lights to view-space clusters.
    pub fn assign_lights(&mut self, view_matrix: Mat4, lights: &[ClusteredPointLight]) {
        self.light_indices.clear();
        for tile in &mut self.tiles {
            tile.offset = 0;
            tile.count = 0;
        }

        // Transform lights to view space
        let view_lights: Vec<(usize, Vec3, f32)> = lights
            .iter()
            .enumerate()
            .map(|(idx, l)| {
                let pos_view = (view_matrix
                    * Vec4::new(l.position.x, l.position.y, l.position.z, 1.0))
                .truncate();
                (idx, pos_view, l.radius)
            })
            .collect();

        // Assign to clusters
        for (cluster_idx, aabb) in self.cluster_aabbs.iter().enumerate() {
            let offset = self.light_indices.len() as u32;
            let mut count = 0u32;

            for &(light_idx, pos_view, radius) in &view_lights {
                if aabb.intersects_sphere(pos_view, radius)
                    && (count as usize) < self.max_lights_per_cluster
                {
                    self.light_indices.push(light_idx as u32);
                    count += 1;
                }
            }

            self.tiles[cluster_idx] = ClusterTile { offset, count };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::EulerRot;

    #[test]
    fn test_cluster_grid_dimensions() {
        let dims = ClusterGridDimensions::default();
        assert_eq!(dims.total_clusters(), 16 * 9 * 24);
        assert_eq!(dims.total_clusters(), 3456);
        assert_eq!(dims.linear_index(0, 0, 0), 0);
        assert_eq!(dims.linear_index(15, 8, 23), 3455);
    }

    #[test]
    fn test_exponential_depth_slices() {
        let dims = ClusterGridDimensions::default();
        let near = 0.1;
        let far = 1000.0;
        let slices = ClusteredLightGrid::calculate_depth_slices(&dims, near, far);
        assert_eq!(slices.len(), 25);
        assert!((slices[0] - near).abs() < 1e-4);
        assert!((slices[24] - far).abs() < 1e-3);
        for i in 0..slices.len() - 1 {
            assert!(slices[i] < slices[i + 1]);
        }
    }

    #[test]
    fn test_cluster_aabb_sphere_intersection() {
        let aabb = ClusterAabb {
            min: Vec3::new(-5.0, -5.0, -10.0),
            max: Vec3::new(5.0, 5.0, -5.0),
        };

        // Center inside AABB
        assert!(aabb.intersects_sphere(Vec3::new(0.0, 0.0, -7.0), 1.0));

        // Outside but intersecting
        assert!(aabb.intersects_sphere(Vec3::new(6.0, 0.0, -7.0), 1.5));

        // Outside and not intersecting
        assert!(!aabb.intersects_sphere(Vec3::new(10.0, 0.0, -7.0), 2.0));
    }

    #[test]
    fn test_light_assignment() {
        let dims = ClusterGridDimensions {
            x_tiles: 4,
            y_tiles: 4,
            z_slices: 4,
        };
        let mut grid = ClusteredLightGrid::new(dims, 0.1, 100.0);
        let proj = Mat4::perspective_rh(60.0_f32.to_radians(), 16.0 / 9.0, 0.1, 100.0);
        grid.update_clusters(proj.inverse());

        assert_eq!(grid.cluster_aabbs.len(), 4 * 4 * 4);

        let view = Mat4::from_rotation_translation(
            glam::Quat::from_euler(EulerRot::YXZ, 0.0, 0.0, 0.0),
            Vec3::ZERO,
        );

        let lights = vec![
            ClusteredPointLight {
                position: Vec3::new(0.0, 0.0, -10.0),
                radius: 5.0,
                color: Vec3::ONE,
                intensity: 100.0,
            },
            ClusteredPointLight {
                position: Vec3::new(500.0, 0.0, -10.0), // Far outside
                radius: 1.0,
                color: Vec3::ONE,
                intensity: 10.0,
            },
        ];

        grid.assign_lights(view, &lights);

        // At least one cluster should have light 0
        let total_assigned: u32 = grid.tiles.iter().map(|t| t.count).sum();
        assert!(total_assigned > 0);

        // Light 1 should never be assigned
        assert!(!grid.light_indices.contains(&1));
    }
}
