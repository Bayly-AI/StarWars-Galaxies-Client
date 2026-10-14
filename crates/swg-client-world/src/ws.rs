// crates/swg-client-world/src/ws.rs

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use swg_client_iff::{IffError, IffNode, IffWriter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WsError {
    #[error("IFF error: {0}")]
    Iff(#[from] IffError),
    #[error("Missing WSNP or SNAP form in World Snapshot")]
    MissingSnapshotForm,
    #[error("Malformed node data chunk")]
    MalformedNodeData,
}

/// A static world object node from a SWG World Snapshot (.ws).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldSnapshotNode {
    pub network_id: u64,
    pub template_name: String,
    pub cell_id: u64,
    pub position: Vec3,
    pub orientation: Quat,
    pub scale: Vec3,
    pub radius: f32,
}

impl WorldSnapshotNode {
    #[must_use]
    pub fn new(
        network_id: u64,
        template_name: impl Into<String>,
        cell_id: u64,
        position: Vec3,
        orientation: Quat,
        radius: f32,
    ) -> Self {
        Self {
            network_id,
            template_name: template_name.into(),
            cell_id,
            position,
            orientation,
            scale: Vec3::ONE,
            radius: radius.max(1.0),
        }
    }

    /// Computes the 4x4 model transformation matrix.
    #[must_use]
    pub fn transform_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.orientation, self.position)
    }

    /// Whether this object is an outdoor landscape object (cell 0).
    #[must_use]
    pub fn is_outdoor(&self) -> bool {
        self.cell_id == 0
    }
}

/// A World Snapshot containing all planetary static structures, buildings, and props.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub nodes: Vec<WorldSnapshotNode>,
}

impl WorldSnapshot {
    #[must_use]
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: WorldSnapshotNode) {
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

    /// Returns an iterator over outdoor static objects.
    pub fn outdoor_nodes(&self) -> impl Iterator<Item = &WorldSnapshotNode> {
        self.nodes.iter().filter(|n| n.is_outdoor())
    }

    /// Returns an iterator over objects inside a specific interior cell.
    pub fn cell_nodes(&self, cell_id: u64) -> impl Iterator<Item = &WorldSnapshotNode> {
        self.nodes.iter().filter(move |n| n.cell_id == cell_id)
    }

    /// Parses a .ws file from raw IFF bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WsError> {
        let root = IffNode::parse(bytes)?;

        if root.form_type() != Some(*b"WSNP") && root.form_type() != Some(*b"SNAP") {
            return Err(WsError::MissingSnapshotForm);
        }

        let mut snapshot = Self::new();

        // Parse NODS chunk
        // Format: count (u32), then each node:
        // net_id (u64, 8B), cell_id (u64, 8B), t_len (u32, 4B), t_str (t_len B),
        // pos (3x f32, 12B), rot (4x f32, 16B), scale (3x f32, 12B), radius (f32, 4B)
        if let Some(nods_data) = root.find_chunk(b"NODS") {
            let mut off = 0;
            if nods_data.len() >= 4 {
                let node_count =
                    u32::from_le_bytes([nods_data[0], nods_data[1], nods_data[2], nods_data[3]])
                        as usize;
                off += 4;
                snapshot.nodes.reserve(node_count);

                for _ in 0..node_count {
                    if off + 20 > nods_data.len() {
                        break;
                    }
                    let net_id = u64::from_le_bytes([
                        nods_data[off],
                        nods_data[off + 1],
                        nods_data[off + 2],
                        nods_data[off + 3],
                        nods_data[off + 4],
                        nods_data[off + 5],
                        nods_data[off + 6],
                        nods_data[off + 7],
                    ]);
                    off += 8;

                    let cell_id = u64::from_le_bytes([
                        nods_data[off],
                        nods_data[off + 1],
                        nods_data[off + 2],
                        nods_data[off + 3],
                        nods_data[off + 4],
                        nods_data[off + 5],
                        nods_data[off + 6],
                        nods_data[off + 7],
                    ]);
                    off += 8;

                    let t_len = u32::from_le_bytes([
                        nods_data[off],
                        nods_data[off + 1],
                        nods_data[off + 2],
                        nods_data[off + 3],
                    ]) as usize;
                    off += 4;

                    if off + t_len > nods_data.len() {
                        break;
                    }
                    let template_name = String::from_utf8_lossy(&nods_data[off..off + t_len])
                        .trim_end_matches('\0')
                        .to_string();
                    off += t_len;

                    // pos 12, rot 16, scale 12, rad 4 = 44 bytes
                    if off + 44 > nods_data.len() {
                        break;
                    }
                    let px = f32::from_le_bytes([
                        nods_data[off],
                        nods_data[off + 1],
                        nods_data[off + 2],
                        nods_data[off + 3],
                    ]);
                    let py = f32::from_le_bytes([
                        nods_data[off + 4],
                        nods_data[off + 5],
                        nods_data[off + 6],
                        nods_data[off + 7],
                    ]);
                    let pz = f32::from_le_bytes([
                        nods_data[off + 8],
                        nods_data[off + 9],
                        nods_data[off + 10],
                        nods_data[off + 11],
                    ]);
                    off += 12;

                    let rx = f32::from_le_bytes([
                        nods_data[off],
                        nods_data[off + 1],
                        nods_data[off + 2],
                        nods_data[off + 3],
                    ]);
                    let ry = f32::from_le_bytes([
                        nods_data[off + 4],
                        nods_data[off + 5],
                        nods_data[off + 6],
                        nods_data[off + 7],
                    ]);
                    let rz = f32::from_le_bytes([
                        nods_data[off + 8],
                        nods_data[off + 9],
                        nods_data[off + 10],
                        nods_data[off + 11],
                    ]);
                    let rw = f32::from_le_bytes([
                        nods_data[off + 12],
                        nods_data[off + 13],
                        nods_data[off + 14],
                        nods_data[off + 15],
                    ]);
                    off += 16;

                    let sx = f32::from_le_bytes([
                        nods_data[off],
                        nods_data[off + 1],
                        nods_data[off + 2],
                        nods_data[off + 3],
                    ]);
                    let sy = f32::from_le_bytes([
                        nods_data[off + 4],
                        nods_data[off + 5],
                        nods_data[off + 6],
                        nods_data[off + 7],
                    ]);
                    let sz = f32::from_le_bytes([
                        nods_data[off + 8],
                        nods_data[off + 9],
                        nods_data[off + 10],
                        nods_data[off + 11],
                    ]);
                    off += 12;

                    let radius = f32::from_le_bytes([
                        nods_data[off],
                        nods_data[off + 1],
                        nods_data[off + 2],
                        nods_data[off + 3],
                    ]);
                    off += 4;

                    snapshot.nodes.push(WorldSnapshotNode {
                        network_id: net_id,
                        template_name,
                        cell_id,
                        position: Vec3::new(px, py, pz),
                        orientation: Quat::from_xyzw(rx, ry, rz, rw).normalize(),
                        scale: Vec3::new(sx, sy, sz),
                        radius,
                    });
                }
            }
        }

        Ok(snapshot)
    }

    /// Serializes the snapshot to IFF bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = IffWriter::new();
        writer.begin_form(b"WSNP");

        // Version chunk 0000
        writer.write_chunk(b"0000", &[1, 0, 0, 0]);

        // NODS chunk
        let mut nods_data = Vec::new();
        nods_data.extend_from_slice(&(self.nodes.len() as u32).to_le_bytes());
        for node in &self.nodes {
            nods_data.extend_from_slice(&node.network_id.to_le_bytes());
            nods_data.extend_from_slice(&node.cell_id.to_le_bytes());

            let t_bytes = node.template_name.as_bytes();
            nods_data.extend_from_slice(&(t_bytes.len() as u32).to_le_bytes());
            nods_data.extend_from_slice(t_bytes);

            nods_data.extend_from_slice(&node.position.x.to_le_bytes());
            nods_data.extend_from_slice(&node.position.y.to_le_bytes());
            nods_data.extend_from_slice(&node.position.z.to_le_bytes());

            nods_data.extend_from_slice(&node.orientation.x.to_le_bytes());
            nods_data.extend_from_slice(&node.orientation.y.to_le_bytes());
            nods_data.extend_from_slice(&node.orientation.z.to_le_bytes());
            nods_data.extend_from_slice(&node.orientation.w.to_le_bytes());

            nods_data.extend_from_slice(&node.scale.x.to_le_bytes());
            nods_data.extend_from_slice(&node.scale.y.to_le_bytes());
            nods_data.extend_from_slice(&node.scale.z.to_le_bytes());

            nods_data.extend_from_slice(&node.radius.to_le_bytes());
        }
        writer.write_chunk(b"NODS", &nods_data);

        writer.end_form();
        writer.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_snapshot_roundtrip() {
        let mut snapshot = WorldSnapshot::new();
        snapshot.add_node(WorldSnapshotNode::new(
            1001,
            "shared_cantina_tatooine.iff",
            0,
            Vec3::new(3528.0, 5.0, -4804.0),
            Quat::IDENTITY,
            45.0,
        ));
        snapshot.add_node(WorldSnapshotNode::new(
            1002,
            "shared_table_round.iff",
            1001, // Inside cantina cell
            Vec3::new(5.0, 0.0, 3.0),
            Quat::IDENTITY,
            2.0,
        ));

        let bytes = snapshot.to_bytes();
        let reloaded = WorldSnapshot::from_bytes(&bytes).expect("Failed to parse WSNP");

        assert_eq!(reloaded.len(), 2);
        assert_eq!(reloaded.nodes[0].network_id, 1001);
        assert_eq!(
            reloaded.nodes[0].template_name,
            "shared_cantina_tatooine.iff"
        );
        assert_eq!(reloaded.nodes[0].position, Vec3::new(3528.0, 5.0, -4804.0));
        assert_eq!(reloaded.nodes[0].radius, 45.0);
        assert!(reloaded.nodes[0].is_outdoor());

        assert_eq!(reloaded.nodes[1].network_id, 1002);
        assert_eq!(reloaded.nodes[1].cell_id, 1001);
        assert!(!reloaded.nodes[1].is_outdoor());
    }
}
