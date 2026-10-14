// crates/swg-client-render/src/clutter.rs

use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use swg_client_terrain::TerrainGenerator;

/// Type of planetary clutter detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClutterType {
    GrassTuft,
    Pebble,
    Shrub,
    DesertRock,
}

/// Raw GPU layout for an instanced clutter model.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ClutterInstanceRaw {
    pub model_matrix: [f32; 16],
    pub color_tint: [f32; 4],
}

/// Logical representation of a single ground clutter instance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClutterInstance {
    pub clutter_type: ClutterType,
    pub position: Vec3,
    pub rotation_y: f32,
    pub scale: f32,
    pub tint: [f32; 4],
}

impl ClutterInstance {
    #[must_use]
    pub fn to_raw(&self) -> ClutterInstanceRaw {
        let matrix = Mat4::from_scale_rotation_translation(
            Vec3::splat(self.scale),
            glam::Quat::from_rotation_y(self.rotation_y),
            self.position,
        );

        ClutterInstanceRaw {
            model_matrix: matrix.to_cols_array(),
            color_tint: self.tint,
        }
    }
}

/// Generator for procedurally scattering dense flora and debris instances across terrain.
pub struct ClutterGenerator {
    pub max_distance: f32,
    pub instances_per_chunk: usize,
}

impl Default for ClutterGenerator {
    fn default() -> Self {
        Self {
            max_distance: 128.0,
            instances_per_chunk: 128,
        }
    }
}

impl ClutterGenerator {
    /// Generates clutter instances across a terrain chunk.
    #[must_use]
    pub fn generate_chunk_clutter(
        &self,
        generator: &TerrainGenerator,
        chunk_x: i32,
        chunk_z: i32,
        camera_pos: Vec3,
    ) -> Vec<ClutterInstance> {
        let mut instances = Vec::with_capacity(self.instances_per_chunk);
        let origin_x = chunk_x as f32 * 64.0;
        let origin_z = chunk_z as f32 * 64.0;

        for i in 0..self.instances_per_chunk {
            // Pseudo-random coordinates using spatial hash
            let h = Self::hash(chunk_x, chunk_z, i as u32);
            let rx = (h & 0xFFFF) as f32 / 65535.0 * 64.0;
            let rz = ((h >> 16) & 0xFFFF) as f32 / 65535.0 * 64.0;

            let wx = origin_x + rx;
            let wz = origin_z + rz;

            // Distance cull against camera
            let dx = wx - camera_pos.x;
            let dz = wz - camera_pos.z;
            let dist_sq = dx * dx + dz * dz;

            if dist_sq > self.max_distance * self.max_distance {
                continue;
            }

            // Slope check: do not place grass/shrubs on vertical cliffs
            let norm = generator.get_normal(wx, wz);
            if norm.y < 0.75 {
                continue;
            }

            let wy = generator.get_height(wx, wz);

            let rot_y = ((h.wrapping_mul(31)) % 360) as f32 * std::f32::consts::PI / 180.0;
            let scale = 0.8 + ((h >> 8) & 0x3F) as f32 / 100.0; // 0.8 to 1.43

            let clutter_type = match (h >> 4) & 3 {
                0 => ClutterType::GrassTuft,
                1 => ClutterType::Pebble,
                2 => ClutterType::Shrub,
                _ => ClutterType::DesertRock,
            };

            instances.push(ClutterInstance {
                clutter_type,
                position: Vec3::new(wx, wy, wz),
                rotation_y: rot_y,
                scale,
                tint: [1.0, 1.0, 1.0, 1.0],
            });
        }

        instances
    }

    fn hash(x: i32, z: i32, i: u32) -> u32 {
        let mut h = (x as u32).wrapping_mul(0x9E37_79B9);
        h ^= (z as u32).wrapping_mul(0x85EB_CA6B);
        h ^= i.wrapping_mul(0xC2B2_AE35);
        h ^= h >> 16;
        h = h.wrapping_mul(0x27D4_EB2D);
        h ^= h >> 15;
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clutter_generation_and_culling() {
        let generator = TerrainGenerator::tatooine();
        let clutter_gen = ClutterGenerator {
            max_distance: 100.0,
            instances_per_chunk: 200,
        };

        let camera_pos = Vec3::new(32.0, 10.0, 32.0);
        let instances = clutter_gen.generate_chunk_clutter(&generator, 0, 0, camera_pos);

        assert!(!instances.is_empty(), "Should generate clutter near camera");
        for inst in &instances {
            assert!(
                (inst.position - camera_pos).length() <= 100.0,
                "Instances beyond max_distance must be culled"
            );
            let raw = inst.to_raw();
            assert_eq!(raw.color_tint[3], 1.0);
        }

        // Test distant chunk (1000m away)
        let distant_instances = clutter_gen.generate_chunk_clutter(&generator, 20, 20, camera_pos);
        assert!(
            distant_instances.is_empty(),
            "Distant chunk clutter should be 100% culled"
        );
    }
}
