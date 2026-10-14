// crates/swg-client-terrain/src/chunk.rs

use crate::height::TerrainGenerator;
use crate::lod::{NeighborLods, TerrainLod};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CHUNK_SIZE_METERS: f32 = 64.0;

/// Vertex structure for terrain chunk meshes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// Generated 3D mesh for a 64m x 64m terrain chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainChunkMesh {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub lod: TerrainLod,
    pub vertices: Vec<TerrainVertex>,
    pub indices: Vec<u32>,
}

impl TerrainChunkMesh {
    /// Generates a 64m x 64m terrain mesh with edge stitching for crack-free LOD transitions.
    #[must_use]
    pub fn generate(
        generator: &TerrainGenerator,
        chunk_x: i32,
        chunk_z: i32,
        lod: TerrainLod,
        neighbors: NeighborLods,
    ) -> Self {
        let n = lod.segments_per_edge();
        let step = CHUNK_SIZE_METERS / n as f32;
        let origin_x = chunk_x as f32 * CHUNK_SIZE_METERS;
        let origin_z = chunk_z as f32 * CHUNK_SIZE_METERS;

        let num_verts = (n + 1) * (n + 1);
        let mut vertices = Vec::with_capacity(num_verts);

        // Generate base grid vertices
        for jz in 0..=n {
            let wz = origin_z + jz as f32 * step;
            let v_uv = jz as f32 / n as f32;

            for ix in 0..=n {
                let wx = origin_x + ix as f32 * step;
                let u_uv = ix as f32 / n as f32;

                let wy = generator.get_height(wx, wz);
                let norm = generator.get_normal(wx, wz);

                vertices.push(TerrainVertex {
                    position: [wx, wy, wz],
                    normal: [norm.x, norm.y, norm.z],
                    uv: [u_uv, v_uv],
                });
            }
        }

        // Apply edge stitching geomorphing to prevent seams / cracks
        // South edge (jz = 0)
        if let Some(s_lod) = neighbors.south
            && s_lod > lod
        {
            for ix in (1..n).step_by(2) {
                let idx = ix;
                let prev_pos = vertices[ix - 1].position;
                let next_pos = vertices[ix + 1].position;
                vertices[idx].position = [
                    (prev_pos[0] + next_pos[0]) * 0.5,
                    (prev_pos[1] + next_pos[1]) * 0.5,
                    (prev_pos[2] + next_pos[2]) * 0.5,
                ];
            }
        }

        // North edge (jz = n)
        if let Some(n_lod) = neighbors.north
            && n_lod > lod
        {
            let row_offset = n * (n + 1);
            for ix in (1..n).step_by(2) {
                let idx = row_offset + ix;
                let prev_pos = vertices[row_offset + ix - 1].position;
                let next_pos = vertices[row_offset + ix + 1].position;
                vertices[idx].position = [
                    (prev_pos[0] + next_pos[0]) * 0.5,
                    (prev_pos[1] + next_pos[1]) * 0.5,
                    (prev_pos[2] + next_pos[2]) * 0.5,
                ];
            }
        }

        // West edge (ix = 0)
        if let Some(w_lod) = neighbors.west
            && w_lod > lod
        {
            for jz in (1..n).step_by(2) {
                let idx = jz * (n + 1);
                let prev_pos = vertices[(jz - 1) * (n + 1)].position;
                let next_pos = vertices[(jz + 1) * (n + 1)].position;
                vertices[idx].position = [
                    (prev_pos[0] + next_pos[0]) * 0.5,
                    (prev_pos[1] + next_pos[1]) * 0.5,
                    (prev_pos[2] + next_pos[2]) * 0.5,
                ];
            }
        }

        // East edge (ix = n)
        if let Some(e_lod) = neighbors.east
            && e_lod > lod
        {
            for jz in (1..n).step_by(2) {
                let idx = jz * (n + 1) + n;
                let prev_pos = vertices[(jz - 1) * (n + 1) + n].position;
                let next_pos = vertices[(jz + 1) * (n + 1) + n].position;
                vertices[idx].position = [
                    (prev_pos[0] + next_pos[0]) * 0.5,
                    (prev_pos[1] + next_pos[1]) * 0.5,
                    (prev_pos[2] + next_pos[2]) * 0.5,
                ];
            }
        }

        // Generate triangle indices
        let num_quads = n * n;
        let mut indices = Vec::with_capacity(num_quads * 6);

        for jz in 0..n {
            for ix in 0..n {
                let v00 = (jz * (n + 1) + ix) as u32;
                let v10 = (jz * (n + 1) + (ix + 1)) as u32;
                let v01 = ((jz + 1) * (n + 1) + ix) as u32;
                let v11 = ((jz + 1) * (n + 1) + (ix + 1)) as u32;

                // Triangle 1
                indices.push(v00);
                indices.push(v01);
                indices.push(v10);

                // Triangle 2
                indices.push(v10);
                indices.push(v01);
                indices.push(v11);
            }
        }

        Self {
            chunk_x,
            chunk_z,
            lod,
            vertices,
            indices,
        }
    }
}

/// Dynamic manager updating terrain chunks centered around camera world position.
pub struct ChunkGridManager {
    pub view_radius_meters: f32,
    pub loaded_chunks: HashMap<(i32, i32), TerrainChunkMesh>,
}

impl Default for ChunkGridManager {
    fn default() -> Self {
        Self::new(256.0)
    }
}

impl ChunkGridManager {
    #[must_use]
    pub fn new(view_radius_meters: f32) -> Self {
        Self {
            view_radius_meters,
            loaded_chunks: HashMap::new(),
        }
    }

    /// Updates loaded chunk collection relative to camera position.
    pub fn update(&mut self, generator: &TerrainGenerator, camera_pos: Vec3) {
        let center_cx = (camera_pos.x / CHUNK_SIZE_METERS).floor() as i32;
        let center_cz = (camera_pos.z / CHUNK_SIZE_METERS).floor() as i32;

        let radius_chunks = (self.view_radius_meters / CHUNK_SIZE_METERS).ceil() as i32;

        // Determine LOD for each chunk in radius
        let mut target_lods = HashMap::new();
        for cz in (center_cz - radius_chunks)..=(center_cz + radius_chunks) {
            for cx in (center_cx - radius_chunks)..=(center_cx + radius_chunks) {
                let chunk_world_x = cx as f32 * CHUNK_SIZE_METERS + CHUNK_SIZE_METERS * 0.5;
                let chunk_world_z = cz as f32 * CHUNK_SIZE_METERS + CHUNK_SIZE_METERS * 0.5;

                let dx = chunk_world_x - camera_pos.x;
                let dz = chunk_world_z - camera_pos.z;
                let dist = (dx * dx + dz * dz).sqrt();

                if dist <= self.view_radius_meters {
                    target_lods.insert((cx, cz), TerrainLod::from_distance(dist));
                }
            }
        }

        // Remove chunks outside view
        self.loaded_chunks
            .retain(|coord, _| target_lods.contains_key(coord));

        // Generate / update meshes
        for (&(cx, cz), &lod) in &target_lods {
            let neighbors = NeighborLods {
                north: target_lods.get(&(cx, cz + 1)).copied(),
                south: target_lods.get(&(cx, cz - 1)).copied(),
                east: target_lods.get(&(cx + 1, cz)).copied(),
                west: target_lods.get(&(cx - 1, cz)).copied(),
            };

            let needs_gen = match self.loaded_chunks.get(&(cx, cz)) {
                Some(existing) => existing.lod != lod,
                None => true,
            };

            if needs_gen {
                let mesh = TerrainChunkMesh::generate(generator, cx, cz, lod, neighbors);
                self.loaded_chunks.insert((cx, cz), mesh);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_mesh_generation_vertex_counts() {
        let generator = TerrainGenerator::tatooine();
        let neighbors = NeighborLods::default();

        let mesh_lod0 = TerrainChunkMesh::generate(&generator, 0, 0, TerrainLod::Lod0, neighbors);
        assert_eq!(mesh_lod0.vertices.len(), 65 * 65);
        assert_eq!(mesh_lod0.indices.len(), 64 * 64 * 6);

        let mesh_lod3 = TerrainChunkMesh::generate(&generator, 0, 0, TerrainLod::Lod3, neighbors);
        assert_eq!(mesh_lod3.vertices.len(), 9 * 9);
        assert_eq!(mesh_lod3.indices.len(), 8 * 8 * 6);
    }

    #[test]
    fn test_crack_free_edge_stitching() {
        let generator = TerrainGenerator::tatooine();

        // Chunk A at LOD 0, bordering Chunk B at LOD 1 on North (+Z)
        let neighbors_a = NeighborLods {
            north: Some(TerrainLod::Lod1),
            ..Default::default()
        };
        let chunk_a = TerrainChunkMesh::generate(&generator, 0, 0, TerrainLod::Lod0, neighbors_a);

        // Chunk B at LOD 1 (north neighbor at (0, 1))
        let neighbors_b = NeighborLods {
            south: Some(TerrainLod::Lod0),
            ..Default::default()
        };
        let chunk_b = TerrainChunkMesh::generate(&generator, 0, 1, TerrainLod::Lod1, neighbors_b);

        // On the shared boundary Z = 64.0:
        // Chunk A North boundary: row jz = 64
        let row_a = 64 * 65;
        // Chunk B South boundary: row jz = 0
        let row_b = 0;

        // Check that corresponding even vertices match exactly
        for i_b in 0..=32 {
            let i_a = i_b * 2;
            let pos_a = chunk_a.vertices[row_a + i_a].position;
            let pos_b = chunk_b.vertices[row_b + i_b].position;

            assert!(
                (pos_a[0] - pos_b[0]).abs() < 1e-4,
                "X mismatch at boundary: a={}, b={}",
                pos_a[0],
                pos_b[0]
            );
            assert!(
                (pos_a[1] - pos_b[1]).abs() < 1e-4,
                "Height Y mismatch at boundary: a={}, b={}",
                pos_a[1],
                pos_b[1]
            );
            assert!(
                (pos_a[2] - pos_b[2]).abs() < 1e-4,
                "Z mismatch at boundary: a={}, b={}",
                pos_a[2],
                pos_b[2]
            );
        }

        // Check that Chunk A odd boundary vertices lie exactly on Chunk B line segments
        for i_odd in (1..64).step_by(2) {
            let pos_odd = chunk_a.vertices[row_a + i_odd].position;
            let prev = chunk_a.vertices[row_a + i_odd - 1].position;
            let next = chunk_a.vertices[row_a + i_odd + 1].position;

            let expected_y = (prev[1] + next[1]) * 0.5;
            assert!(
                (pos_odd[1] - expected_y).abs() < 1e-4,
                "Odd vertex must be geomorphically stitched to prevent cracks"
            );
        }
    }
}
