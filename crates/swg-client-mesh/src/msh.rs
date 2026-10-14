// crates/swg-client-mesh/src/msh.rs

use glam::{Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use swg_client_iff::{IffError, IffNode, IffWriter};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MeshError {
    #[error("IFF error while reading MSH model: {0}")]
    Iff(#[from] IffError),
    #[error("Invalid MSH header or missing required chunks: {0}")]
    InvalidModel(String),
}

/// A material sub-mesh partition within a 3D model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubMesh {
    pub shader_path: String,
    pub indices: Vec<u32>,
}

/// 3D Rigid Mesh model parsed from SWG .msh asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RigidMesh {
    pub name: String,
    pub bounds_sphere_center: Vec3,
    pub bounds_sphere_radius: f32,
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub tangents: Vec<Vec4>,
    pub uvs: Vec<Vec2>,
    pub sub_meshes: Vec<SubMesh>,
}

impl RigidMesh {
    /// Total vertex count.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Total triangle count across all sub-meshes.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.sub_meshes.iter().map(|s| s.indices.len() / 3).sum()
    }

    /// Parses an authentic SWG .msh file from raw IFF bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, MeshError> {
        let root = IffNode::parse(bytes)?;
        if root.form_type() != Some(*b"MSH_") {
            return Err(MeshError::InvalidModel(
                "Root container is not FORM MSH_".to_string(),
            ));
        }

        // Parse bounds sphere and AABB
        let (bounds_sphere_center, bounds_sphere_radius) = match root.find_chunk(b"SPHR") {
            Some(data) if data.len() >= 16 => {
                let cx = f32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                let cy = f32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                let cz = f32::from_le_bytes([data[8], data[9], data[10], data[11]]);
                let r = f32::from_le_bytes([data[12], data[13], data[14], data[15]]);
                (Vec3::new(cx, cy, cz), r)
            }
            _ => (Vec3::ZERO, 10.0),
        };

        let (aabb_min, aabb_max) = match root.find_chunk(b"BOX_") {
            Some(data) if data.len() >= 24 => {
                let min_x = f32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                let min_y = f32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                let min_z = f32::from_le_bytes([data[8], data[9], data[10], data[11]]);
                let max_x = f32::from_le_bytes([data[12], data[13], data[14], data[15]]);
                let max_y = f32::from_le_bytes([data[16], data[17], data[18], data[19]]);
                let max_z = f32::from_le_bytes([data[20], data[21], data[22], data[23]]);
                (
                    Vec3::new(min_x, min_y, min_z),
                    Vec3::new(max_x, max_y, max_z),
                )
            }
            _ => (Vec3::splat(-10.0), Vec3::splat(10.0)),
        };

        // Parse vertex geometry
        let mut positions = Vec::new();
        if let Some(pos_data) = root.find_chunk(b"VERT") {
            let count = pos_data.len() / 12;
            for i in 0..count {
                let off = i * 12;
                let x = f32::from_le_bytes([
                    pos_data[off],
                    pos_data[off + 1],
                    pos_data[off + 2],
                    pos_data[off + 3],
                ]);
                let y = f32::from_le_bytes([
                    pos_data[off + 4],
                    pos_data[off + 5],
                    pos_data[off + 6],
                    pos_data[off + 7],
                ]);
                let z = f32::from_le_bytes([
                    pos_data[off + 8],
                    pos_data[off + 9],
                    pos_data[off + 10],
                    pos_data[off + 11],
                ]);
                positions.push(Vec3::new(x, y, z));
            }
        }

        let mut normals = Vec::new();
        if let Some(norm_data) = root.find_chunk(b"NORM") {
            let count = norm_data.len() / 12;
            for i in 0..count {
                let off = i * 12;
                let x = f32::from_le_bytes([
                    norm_data[off],
                    norm_data[off + 1],
                    norm_data[off + 2],
                    norm_data[off + 3],
                ]);
                let y = f32::from_le_bytes([
                    norm_data[off + 4],
                    norm_data[off + 5],
                    norm_data[off + 6],
                    norm_data[off + 7],
                ]);
                let z = f32::from_le_bytes([
                    norm_data[off + 8],
                    norm_data[off + 9],
                    norm_data[off + 10],
                    norm_data[off + 11],
                ]);
                normals.push(Vec3::new(x, y, z));
            }
        }

        let mut uvs = Vec::new();
        if let Some(uv_data) = root.find_chunk(b"TEXC") {
            let count = uv_data.len() / 8;
            for i in 0..count {
                let off = i * 8;
                let u = f32::from_le_bytes([
                    uv_data[off],
                    uv_data[off + 1],
                    uv_data[off + 2],
                    uv_data[off + 3],
                ]);
                let v = f32::from_le_bytes([
                    uv_data[off + 4],
                    uv_data[off + 5],
                    uv_data[off + 6],
                    uv_data[off + 7],
                ]);
                uvs.push(Vec2::new(u, v));
            }
        }

        let mut tangents = Vec::new();
        if let Some(tang_data) = root.find_chunk(b"TANG") {
            let count = tang_data.len() / 16;
            for i in 0..count {
                let off = i * 16;
                let x = f32::from_le_bytes([
                    tang_data[off],
                    tang_data[off + 1],
                    tang_data[off + 2],
                    tang_data[off + 3],
                ]);
                let y = f32::from_le_bytes([
                    tang_data[off + 4],
                    tang_data[off + 5],
                    tang_data[off + 6],
                    tang_data[off + 7],
                ]);
                let z = f32::from_le_bytes([
                    tang_data[off + 8],
                    tang_data[off + 9],
                    tang_data[off + 10],
                    tang_data[off + 11],
                ]);
                let w = f32::from_le_bytes([
                    tang_data[off + 12],
                    tang_data[off + 13],
                    tang_data[off + 14],
                    tang_data[off + 15],
                ]);
                tangents.push(Vec4::new(x, y, z, w));
            }
        }

        // Parse submesh index partitions and shader references
        let mut sub_meshes = Vec::new();
        if let Some(IffNode::Form { children, .. }) = root.find_form(b"PART") {
            for child in children {
                if let IffNode::Form {
                    form_type,
                    children: sub_children,
                } = child
                    && form_type == b"PART"
                {
                    let mut shader_path = String::new();
                    let mut indices = Vec::new();

                    for sub in sub_children {
                        match sub {
                            IffNode::Chunk {
                                tag: [b'S', b'H', b'D', b'R'],
                                data,
                            } => {
                                shader_path = String::from_utf8_lossy(data)
                                    .trim_end_matches('\0')
                                    .to_string();
                            }
                            IffNode::Chunk {
                                tag: [b'I', b'N', b'D', b'X'],
                                data,
                            } => {
                                let idx_count = data.len() / 4;
                                for k in 0..idx_count {
                                    let off = k * 4;
                                    let idx = u32::from_le_bytes([
                                        data[off],
                                        data[off + 1],
                                        data[off + 2],
                                        data[off + 3],
                                    ]);
                                    indices.push(idx);
                                }
                            }
                            _ => {}
                        }
                    }

                    sub_meshes.push(SubMesh {
                        shader_path,
                        indices,
                    });
                }
            }
        }

        // If no explicit PART form, check root INDX
        if sub_meshes.is_empty()
            && let Some(idx_data) = root.find_chunk(b"INDX")
        {
            let idx_count = idx_data.len() / 4;
            let mut indices = Vec::with_capacity(idx_count);
            for k in 0..idx_count {
                let off = k * 4;
                let idx = u32::from_le_bytes([
                    idx_data[off],
                    idx_data[off + 1],
                    idx_data[off + 2],
                    idx_data[off + 3],
                ]);
                indices.push(idx);
            }
            sub_meshes.push(SubMesh {
                shader_path: "shader/default.sht".to_string(),
                indices,
            });
        }

        Ok(Self {
            name: "msh_model".to_string(),
            bounds_sphere_center,
            bounds_sphere_radius,
            aabb_min,
            aabb_max,
            positions,
            normals,
            tangents,
            uvs,
            sub_meshes,
        })
    }
}

/// Builder for constructing valid .msh IFF models.
pub struct MshWriter {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    uvs: Vec<Vec2>,
    tangents: Vec<Vec4>,
    sub_meshes: Vec<SubMesh>,
    sphere_center: Vec3,
    sphere_radius: f32,
    aabb_min: Vec3,
    aabb_max: Vec3,
}

impl Default for MshWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl MshWriter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            tangents: Vec::new(),
            sub_meshes: Vec::new(),
            sphere_center: Vec3::ZERO,
            sphere_radius: 10.0,
            aabb_min: Vec3::splat(-10.0),
            aabb_max: Vec3::splat(10.0),
        }
    }

    pub fn set_bounds(&mut self, center: Vec3, radius: f32, min: Vec3, max: Vec3) {
        self.sphere_center = center;
        self.sphere_radius = radius;
        self.aabb_min = min;
        self.aabb_max = max;
    }

    pub fn add_vertex(&mut self, pos: Vec3, norm: Vec3, uv: Vec2, tangent: Vec4) {
        self.positions.push(pos);
        self.normals.push(norm);
        self.uvs.push(uv);
        self.tangents.push(tangent);
    }

    pub fn add_sub_mesh(&mut self, shader_path: &str, indices: Vec<u32>) {
        self.sub_meshes.push(SubMesh {
            shader_path: shader_path.to_string(),
            indices,
        });
    }

    #[must_use]
    pub fn finish(&self) -> Vec<u8> {
        let mut writer = IffWriter::new();
        writer.begin_form(b"MSH_");
        {
            writer.write_chunk(b"0005", &[0, 0, 0, 0]);

            // SPHR chunk
            let mut sphr = Vec::with_capacity(16);
            sphr.extend_from_slice(&self.sphere_center.x.to_le_bytes());
            sphr.extend_from_slice(&self.sphere_center.y.to_le_bytes());
            sphr.extend_from_slice(&self.sphere_center.z.to_le_bytes());
            sphr.extend_from_slice(&self.sphere_radius.to_le_bytes());
            writer.write_chunk(b"SPHR", &sphr);

            // BOX_ chunk
            let mut box_data = Vec::with_capacity(24);
            box_data.extend_from_slice(&self.aabb_min.x.to_le_bytes());
            box_data.extend_from_slice(&self.aabb_min.y.to_le_bytes());
            box_data.extend_from_slice(&self.aabb_min.z.to_le_bytes());
            box_data.extend_from_slice(&self.aabb_max.x.to_le_bytes());
            box_data.extend_from_slice(&self.aabb_max.y.to_le_bytes());
            box_data.extend_from_slice(&self.aabb_max.z.to_le_bytes());
            writer.write_chunk(b"BOX_", &box_data);

            // VERT chunk
            let mut vert_data = Vec::with_capacity(self.positions.len() * 12);
            for p in &self.positions {
                vert_data.extend_from_slice(&p.x.to_le_bytes());
                vert_data.extend_from_slice(&p.y.to_le_bytes());
                vert_data.extend_from_slice(&p.z.to_le_bytes());
            }
            writer.write_chunk(b"VERT", &vert_data);

            // NORM chunk
            let mut norm_data = Vec::with_capacity(self.normals.len() * 12);
            for n in &self.normals {
                norm_data.extend_from_slice(&n.x.to_le_bytes());
                norm_data.extend_from_slice(&n.y.to_le_bytes());
                norm_data.extend_from_slice(&n.z.to_le_bytes());
            }
            writer.write_chunk(b"NORM", &norm_data);

            // TEXC chunk
            let mut uv_data = Vec::with_capacity(self.uvs.len() * 8);
            for uv in &self.uvs {
                uv_data.extend_from_slice(&uv.x.to_le_bytes());
                uv_data.extend_from_slice(&uv.y.to_le_bytes());
            }
            writer.write_chunk(b"TEXC", &uv_data);

            // TANG chunk
            let mut tang_data = Vec::with_capacity(self.tangents.len() * 16);
            for t in &self.tangents {
                tang_data.extend_from_slice(&t.x.to_le_bytes());
                tang_data.extend_from_slice(&t.y.to_le_bytes());
                tang_data.extend_from_slice(&t.z.to_le_bytes());
                tang_data.extend_from_slice(&t.w.to_le_bytes());
            }
            writer.write_chunk(b"TANG", &tang_data);

            // PART form
            writer.begin_form(b"PART");
            for part in &self.sub_meshes {
                writer.begin_form(b"PART");
                writer.write_chunk(b"SHDR", part.shader_path.as_bytes());

                let mut idx_data = Vec::with_capacity(part.indices.len() * 4);
                for idx in &part.indices {
                    idx_data.extend_from_slice(&idx.to_le_bytes());
                }
                writer.write_chunk(b"INDX", &idx_data);
                writer.end_form();
            }
            writer.end_form();
        }
        writer.end_form();

        writer.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msh_model_roundtrip() {
        let mut builder = MshWriter::new();
        builder.set_bounds(
            Vec3::new(0.0, 2.0, 0.0),
            5.0,
            Vec3::new(-2.0, 0.0, -2.0),
            Vec3::new(2.0, 4.0, 2.0),
        );

        // Add 3 vertices (1 triangle)
        builder.add_vertex(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::Y,
            Vec2::new(0.0, 0.0),
            Vec4::new(1.0, 0.0, 0.0, 1.0),
        );
        builder.add_vertex(
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::Y,
            Vec2::new(1.0, 0.0),
            Vec4::new(1.0, 0.0, 0.0, 1.0),
        );
        builder.add_vertex(
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::Y,
            Vec2::new(0.5, 1.0),
            Vec4::new(1.0, 0.0, 0.0, 1.0),
        );

        builder.add_sub_mesh("shader/vehicle/xwing_hull.sht", vec![0, 1, 2]);

        let bytes = builder.finish();
        let mesh = RigidMesh::parse(&bytes).unwrap();

        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.bounds_sphere_radius, 5.0);
        assert_eq!(mesh.sub_meshes.len(), 1);
        assert_eq!(
            mesh.sub_meshes[0].shader_path,
            "shader/vehicle/xwing_hull.sht"
        );
        assert_eq!(mesh.sub_meshes[0].indices, vec![0, 1, 2]);
    }
}
