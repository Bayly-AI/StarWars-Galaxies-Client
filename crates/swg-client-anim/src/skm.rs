// crates/swg-client-anim/src/skm.rs

use glam::{Mat4, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use swg_client_iff::{IffError, IffNode, IffWriter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SkeletalError {
    #[error("IFF error: {0}")]
    Iff(#[from] IffError),
    #[error("Missing skeleton form or bone hierarchy")]
    MissingSkeleton,
    #[error("Missing vertex weight data")]
    MissingWeights,
    #[error("Invalid bone index {0} out of bounds {1}")]
    InvalidBoneIndex(usize, usize),
    #[error("Malformed buffer data for tag '{0}'")]
    MalformedData(String),
}

/// A skeletal joint in the character skeleton hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bone {
    pub name: String,
    pub parent_index: Option<usize>,
    pub local_bind_transform: Mat4,
    pub inv_bind_pose: Mat4,
}

/// A locator hardpoint attached to a specific bone for modular weapons/items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hardpoint {
    pub name: String,
    pub bone_index: usize,
    pub offset: Mat4,
}

/// A vertex influenced by up to 4 skeletal bones.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkinVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub bone_indices: [u16; 4],
    pub weights: [f32; 4],
}

impl SkinVertex {
    /// Normalizes weights so that sum(weights) == 1.0.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        let sum: f32 = self.weights.iter().sum();
        if sum > 0.0001 {
            let inv = 1.0 / sum;
            for w in &mut self.weights {
                *w *= inv;
            }
        } else {
            self.weights = [1.0, 0.0, 0.0, 0.0];
        }
        self
    }
}

/// 3D Skeletal Mesh (.skm) representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkeletalMesh {
    pub skeleton: Vec<Bone>,
    pub vertices: Vec<SkinVertex>,
    pub indices: Vec<u32>,
    pub hardpoints: Vec<Hardpoint>,
    pub shader_path: String,
}

impl SkeletalMesh {
    /// Parses a skeletal mesh from IFF bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SkeletalError> {
        let root = IffNode::parse(bytes)?;

        if root.form_type() != Some(*b"SKMG") && root.form_type() != Some(*b"SKTM") {
            return Err(SkeletalError::MissingSkeleton);
        }

        // Parse Bones (BONE chunk: count, then name, parent_idx (i32), local_mat 16 f32s, inv_bind 16 f32s)
        let mut skeleton = Vec::new();
        if let Some(bone_data) = root.find_chunk(b"BONE") {
            let mut offset = 0;
            if bone_data.len() >= 4 {
                let bone_count =
                    u32::from_le_bytes([bone_data[0], bone_data[1], bone_data[2], bone_data[3]])
                        as usize;
                offset += 4;

                for _ in 0..bone_count {
                    if offset + 4 > bone_data.len() {
                        break;
                    }
                    let name_len = u32::from_le_bytes([
                        bone_data[offset],
                        bone_data[offset + 1],
                        bone_data[offset + 2],
                        bone_data[offset + 3],
                    ]) as usize;
                    offset += 4;

                    if offset + name_len > bone_data.len() {
                        break;
                    }
                    let name = String::from_utf8_lossy(&bone_data[offset..offset + name_len])
                        .trim_end_matches('\0')
                        .to_string();
                    offset += name_len;

                    if offset + 4 > bone_data.len() {
                        break;
                    }
                    let parent_raw = i32::from_le_bytes([
                        bone_data[offset],
                        bone_data[offset + 1],
                        bone_data[offset + 2],
                        bone_data[offset + 3],
                    ]);
                    offset += 4;
                    let parent_index = if parent_raw >= 0 {
                        Some(parent_raw as usize)
                    } else {
                        None
                    };

                    // Local bind matrix (16 f32s = 64 bytes)
                    if offset + 64 > bone_data.len() {
                        break;
                    }
                    let mut local_cols = [0.0f32; 16];
                    for (i, col) in local_cols.iter_mut().enumerate() {
                        let b = &bone_data[offset + i * 4..offset + (i + 1) * 4];
                        *col = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                    }
                    offset += 64;
                    let local_bind_transform = Mat4::from_cols_array(&local_cols);

                    // Inverse bind matrix (16 f32s = 64 bytes)
                    if offset + 64 > bone_data.len() {
                        break;
                    }
                    let mut inv_cols = [0.0f32; 16];
                    for (i, col) in inv_cols.iter_mut().enumerate() {
                        let b = &bone_data[offset + i * 4..offset + (i + 1) * 4];
                        *col = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                    }
                    offset += 64;
                    let inv_bind_pose = Mat4::from_cols_array(&inv_cols);

                    skeleton.push(Bone {
                        name,
                        parent_index,
                        local_bind_transform,
                        inv_bind_pose,
                    });
                }
            }
        }

        // Parse Vertices: VERT chunk with 4-bone skinning
        // Per vertex: pos (12B), norm (12B), uv (8B), bone_indices (8B = 4x u16), weights (16B = 4x f32) = 56 bytes
        let mut vertices = Vec::new();
        if let Some(vert_data) = root.find_chunk(b"VERT") {
            let stride = 56;
            let count = vert_data.len() / stride;
            vertices.reserve(count);

            for i in 0..count {
                let off = i * stride;
                let px = f32::from_le_bytes([
                    vert_data[off],
                    vert_data[off + 1],
                    vert_data[off + 2],
                    vert_data[off + 3],
                ]);
                let py = f32::from_le_bytes([
                    vert_data[off + 4],
                    vert_data[off + 5],
                    vert_data[off + 6],
                    vert_data[off + 7],
                ]);
                let pz = f32::from_le_bytes([
                    vert_data[off + 8],
                    vert_data[off + 9],
                    vert_data[off + 10],
                    vert_data[off + 11],
                ]);

                let nx = f32::from_le_bytes([
                    vert_data[off + 12],
                    vert_data[off + 13],
                    vert_data[off + 14],
                    vert_data[off + 15],
                ]);
                let ny = f32::from_le_bytes([
                    vert_data[off + 16],
                    vert_data[off + 17],
                    vert_data[off + 18],
                    vert_data[off + 19],
                ]);
                let nz = f32::from_le_bytes([
                    vert_data[off + 20],
                    vert_data[off + 21],
                    vert_data[off + 22],
                    vert_data[off + 23],
                ]);

                let u = f32::from_le_bytes([
                    vert_data[off + 24],
                    vert_data[off + 25],
                    vert_data[off + 26],
                    vert_data[off + 27],
                ]);
                let v = f32::from_le_bytes([
                    vert_data[off + 28],
                    vert_data[off + 29],
                    vert_data[off + 30],
                    vert_data[off + 31],
                ]);

                let b0 = u16::from_le_bytes([vert_data[off + 32], vert_data[off + 33]]);
                let b1 = u16::from_le_bytes([vert_data[off + 34], vert_data[off + 35]]);
                let b2 = u16::from_le_bytes([vert_data[off + 36], vert_data[off + 37]]);
                let b3 = u16::from_le_bytes([vert_data[off + 38], vert_data[off + 39]]);

                let w0 = f32::from_le_bytes([
                    vert_data[off + 40],
                    vert_data[off + 41],
                    vert_data[off + 42],
                    vert_data[off + 43],
                ]);
                let w1 = f32::from_le_bytes([
                    vert_data[off + 44],
                    vert_data[off + 45],
                    vert_data[off + 46],
                    vert_data[off + 47],
                ]);
                let w2 = f32::from_le_bytes([
                    vert_data[off + 48],
                    vert_data[off + 49],
                    vert_data[off + 50],
                    vert_data[off + 51],
                ]);
                let w3 = f32::from_le_bytes([
                    vert_data[off + 52],
                    vert_data[off + 53],
                    vert_data[off + 54],
                    vert_data[off + 55],
                ]);

                vertices.push(
                    SkinVertex {
                        position: Vec3::new(px, py, pz),
                        normal: Vec3::new(nx, ny, nz),
                        uv: Vec2::new(u, v),
                        bone_indices: [b0, b1, b2, b3],
                        weights: [w0, w1, w2, w3],
                    }
                    .normalized(),
                );
            }
        }

        // Indices
        let mut indices = Vec::new();
        if let Some(idx_data) = root.find_chunk(b"INDX") {
            let count = idx_data.len() / 4;
            indices.reserve(count);
            for i in 0..count {
                let off = i * 4;
                let idx = u32::from_le_bytes([
                    idx_data[off],
                    idx_data[off + 1],
                    idx_data[off + 2],
                    idx_data[off + 3],
                ]);
                indices.push(idx);
            }
        }

        // Hardpoints: HARD chunk
        let mut hardpoints = Vec::new();
        if let Some(hp_data) = root.find_chunk(b"HARD") {
            let mut off = 0;
            if hp_data.len() >= 4 {
                let hp_count =
                    u32::from_le_bytes([hp_data[0], hp_data[1], hp_data[2], hp_data[3]]) as usize;
                off += 4;

                for _ in 0..hp_count {
                    if off + 4 > hp_data.len() {
                        break;
                    }
                    let name_len = u32::from_le_bytes([
                        hp_data[off],
                        hp_data[off + 1],
                        hp_data[off + 2],
                        hp_data[off + 3],
                    ]) as usize;
                    off += 4;

                    if off + name_len > hp_data.len() {
                        break;
                    }
                    let name = String::from_utf8_lossy(&hp_data[off..off + name_len])
                        .trim_end_matches('\0')
                        .to_string();
                    off += name_len;

                    if off + 4 > hp_data.len() {
                        break;
                    }
                    let bone_index = u32::from_le_bytes([
                        hp_data[off],
                        hp_data[off + 1],
                        hp_data[off + 2],
                        hp_data[off + 3],
                    ]) as usize;
                    off += 4;

                    if off + 64 > hp_data.len() {
                        break;
                    }
                    let mut cols = [0.0f32; 16];
                    for (i, col) in cols.iter_mut().enumerate() {
                        let b = &hp_data[off + i * 4..off + (i + 1) * 4];
                        *col = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                    }
                    off += 64;
                    let offset_mat = Mat4::from_cols_array(&cols);

                    hardpoints.push(Hardpoint {
                        name,
                        bone_index,
                        offset: offset_mat,
                    });
                }
            }
        }

        // Shader path
        let mut shader_path = "shader/default_skin.sht".to_string();
        if let Some(shdr_data) = root.find_chunk(b"SHDR") {
            shader_path = String::from_utf8_lossy(shdr_data)
                .trim_end_matches('\0')
                .to_string();
        }

        Ok(Self {
            skeleton,
            vertices,
            indices,
            hardpoints,
            shader_path,
        })
    }

    /// Evaluates GPU skinning matrices for all bones:
    /// $M_i = G_i \times B_{\text{inv}, i}$
    #[must_use]
    pub fn compute_skinning_matrices(&self, global_transforms: &[Mat4]) -> Vec<Mat4> {
        let mut skin_matrices = Vec::with_capacity(self.skeleton.len());
        for (i, bone) in self.skeleton.iter().enumerate() {
            let global = if i < global_transforms.len() {
                global_transforms[i]
            } else {
                Mat4::IDENTITY
            };
            skin_matrices.push(global * bone.inv_bind_pose);
        }
        skin_matrices
    }

    /// Deforms skin vertices using GPU-equivalent skinning matrix palette on CPU.
    #[must_use]
    pub fn deform_vertices(&self, skin_matrices: &[Mat4]) -> Vec<Vec3> {
        let mut deformed = Vec::with_capacity(self.vertices.len());
        for vert in &self.vertices {
            let mut p = Vec3::ZERO;
            for k in 0..4 {
                let weight = vert.weights[k];
                if weight > 0.0001 {
                    let b_idx = vert.bone_indices[k] as usize;
                    let mat = if b_idx < skin_matrices.len() {
                        skin_matrices[b_idx]
                    } else {
                        Mat4::IDENTITY
                    };
                    let transformed = (mat
                        * Vec4::new(vert.position.x, vert.position.y, vert.position.z, 1.0))
                    .truncate();
                    p += transformed * weight;
                }
            }
            deformed.push(p);
        }
        deformed
    }
}

/// Builder helper to create SkeletalMesh IFF buffers for testing and tools.
pub struct SkeletalMeshWriter {
    skeleton: Vec<Bone>,
    vertices: Vec<SkinVertex>,
    indices: Vec<u32>,
    hardpoints: Vec<Hardpoint>,
    shader_path: String,
}

impl SkeletalMeshWriter {
    #[must_use]
    pub fn new(shader_path: impl Into<String>) -> Self {
        Self {
            skeleton: Vec::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
            hardpoints: Vec::new(),
            shader_path: shader_path.into(),
        }
    }

    pub fn add_bone(&mut self, bone: Bone) {
        self.skeleton.push(bone);
    }

    pub fn add_vertex(&mut self, vert: SkinVertex) {
        self.vertices.push(vert.normalized());
    }

    pub fn add_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }

    pub fn add_hardpoint(&mut self, hp: Hardpoint) {
        self.hardpoints.push(hp);
    }

    #[must_use]
    pub fn build(self) -> Vec<u8> {
        let mut writer = IffWriter::new();
        writer.begin_form(b"SKMG");

        // SHDR
        let mut shdr_data = self.shader_path.into_bytes();
        shdr_data.push(0);
        writer.write_chunk(b"SHDR", &shdr_data);

        // BONE
        let mut bone_data = Vec::new();
        bone_data.extend_from_slice(&(self.skeleton.len() as u32).to_le_bytes());
        for bone in &self.skeleton {
            let name_bytes = bone.name.as_bytes();
            bone_data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            bone_data.extend_from_slice(name_bytes);
            let parent_val = bone.parent_index.map_or(-1i32, |p| p as i32);
            bone_data.extend_from_slice(&parent_val.to_le_bytes());
            for col in bone.local_bind_transform.to_cols_array() {
                bone_data.extend_from_slice(&col.to_le_bytes());
            }
            for col in bone.inv_bind_pose.to_cols_array() {
                bone_data.extend_from_slice(&col.to_le_bytes());
            }
        }
        writer.write_chunk(b"BONE", &bone_data);

        // VERT (pos 12, norm 12, uv 8, bones 8, weights 16 = 56B)
        let mut vert_data = Vec::with_capacity(self.vertices.len() * 56);
        for v in &self.vertices {
            vert_data.extend_from_slice(&v.position.x.to_le_bytes());
            vert_data.extend_from_slice(&v.position.y.to_le_bytes());
            vert_data.extend_from_slice(&v.position.z.to_le_bytes());
            vert_data.extend_from_slice(&v.normal.x.to_le_bytes());
            vert_data.extend_from_slice(&v.normal.y.to_le_bytes());
            vert_data.extend_from_slice(&v.normal.z.to_le_bytes());
            vert_data.extend_from_slice(&v.uv.x.to_le_bytes());
            vert_data.extend_from_slice(&v.uv.y.to_le_bytes());
            for b in v.bone_indices {
                vert_data.extend_from_slice(&b.to_le_bytes());
            }
            for w in v.weights {
                vert_data.extend_from_slice(&w.to_le_bytes());
            }
        }
        writer.write_chunk(b"VERT", &vert_data);

        // INDX
        let mut idx_data = Vec::with_capacity(self.indices.len() * 4);
        for idx in &self.indices {
            idx_data.extend_from_slice(&idx.to_le_bytes());
        }
        writer.write_chunk(b"INDX", &idx_data);

        // HARD
        let mut hard_data = Vec::new();
        hard_data.extend_from_slice(&(self.hardpoints.len() as u32).to_le_bytes());
        for hp in &self.hardpoints {
            let name_bytes = hp.name.as_bytes();
            hard_data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            hard_data.extend_from_slice(name_bytes);
            hard_data.extend_from_slice(&(hp.bone_index as u32).to_le_bytes());
            for col in hp.offset.to_cols_array() {
                hard_data.extend_from_slice(&col.to_le_bytes());
            }
        }
        writer.write_chunk(b"HARD", &hard_data);

        writer.end_form();
        writer.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeletal_mesh_roundtrip() {
        let mut builder = SkeletalMeshWriter::new("shader/humanoid.sht");

        // Root pelvis bone
        builder.add_bone(Bone {
            name: "root_pelvis".to_string(),
            parent_index: None,
            local_bind_transform: Mat4::IDENTITY,
            inv_bind_pose: Mat4::IDENTITY,
        });

        // Spine bone
        builder.add_bone(Bone {
            name: "spine_01".to_string(),
            parent_index: Some(0),
            local_bind_transform: Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            inv_bind_pose: Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
        });

        // Vertex attached to pelvis (bone 0)
        builder.add_vertex(SkinVertex {
            position: Vec3::new(0.0, 0.5, 0.0),
            normal: Vec3::Y,
            uv: Vec2::ZERO,
            bone_indices: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        });

        // Vertex attached half to pelvis and half to spine
        builder.add_vertex(SkinVertex {
            position: Vec3::new(0.0, 1.0, 0.0),
            normal: Vec3::Y,
            uv: Vec2::X,
            bone_indices: [0, 1, 0, 0],
            weights: [0.5, 0.5, 0.0, 0.0],
        });

        builder.add_vertex(SkinVertex {
            position: Vec3::new(0.5, 1.0, 0.0),
            normal: Vec3::Y,
            uv: Vec2::ONE,
            bone_indices: [1, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        });

        builder.add_triangle(0, 1, 2);

        // Right hand weapon socket
        builder.add_hardpoint(Hardpoint {
            name: "hold_r".to_string(),
            bone_index: 1,
            offset: Mat4::from_translation(Vec3::new(0.2, 0.0, 0.0)),
        });

        let bytes = builder.build();
        let parsed = SkeletalMesh::from_bytes(&bytes).expect("Failed to parse SKM");

        assert_eq!(parsed.skeleton.len(), 2);
        assert_eq!(parsed.skeleton[0].name, "root_pelvis");
        assert_eq!(parsed.skeleton[1].parent_index, Some(0));
        assert_eq!(parsed.vertices.len(), 3);
        assert_eq!(parsed.indices.len(), 3);
        assert_eq!(parsed.hardpoints.len(), 1);
        assert_eq!(parsed.hardpoints[0].name, "hold_r");
    }

    #[test]
    fn test_skinning_matrix_deformation() {
        let mut builder = SkeletalMeshWriter::new("shader/test.sht");
        builder.add_bone(Bone {
            name: "root".to_string(),
            parent_index: None,
            local_bind_transform: Mat4::IDENTITY,
            inv_bind_pose: Mat4::IDENTITY,
        });

        builder.add_vertex(SkinVertex {
            position: Vec3::new(1.0, 2.0, 3.0),
            normal: Vec3::Y,
            uv: Vec2::ZERO,
            bone_indices: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        });

        let mesh = SkeletalMesh::from_bytes(&builder.build()).unwrap();

        // Translate bone by (10, 0, 0)
        let global_transforms = vec![Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0))];
        let skin_mats = mesh.compute_skinning_matrices(&global_transforms);
        let deformed = mesh.deform_vertices(&skin_mats);

        assert_eq!(deformed.len(), 1);
        assert!((deformed[0].x - 11.0).abs() < 1e-4);
        assert!((deformed[0].y - 2.0).abs() < 1e-4);
        assert!((deformed[0].z - 3.0).abs() < 1e-4);
    }
}
