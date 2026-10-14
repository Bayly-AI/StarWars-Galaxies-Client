// crates/swg-client-anim/src/sat.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use swg_client_iff::{IffError, IffNode, IffWriter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SatError {
    #[error("IFF error: {0}")]
    Iff(#[from] IffError),
    #[error("Missing STAT form")]
    MissingStatForm,
    #[error("Missing BASE skeleton chunk")]
    MissingBaseSkeleton,
}

/// Attachment type for modular appearance equipment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttachmentType {
    /// Skinned mesh conforming to the character skeleton (e.g. chest armor, trousers).
    SkeletalMesh,
    /// Rigid mesh socketed into a bone or hardpoint locator (e.g. helmet, blaster).
    SocketRigid,
}

/// A wearable modular equipment attachment slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppearanceAttachment {
    pub slot_name: String,
    pub asset_path: String,
    pub attachment_type: AttachmentType,
    pub socket_target: Option<String>,
}

/// Skeletal Appearance Template (.sat) defining a customizable character appearance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkeletalAppearanceTemplate {
    pub base_skeleton: String,
    pub slots: HashMap<String, AppearanceAttachment>,
}

impl SkeletalAppearanceTemplate {
    #[must_use]
    pub fn new(base_skeleton: impl Into<String>) -> Self {
        Self {
            base_skeleton: base_skeleton.into(),
            slots: HashMap::new(),
        }
    }

    /// Equips or attaches a modular item to a designated slot.
    pub fn attach(
        &mut self,
        slot_name: impl Into<String>,
        asset_path: impl Into<String>,
        attachment_type: AttachmentType,
        socket_target: Option<String>,
    ) {
        let name = slot_name.into();
        self.slots.insert(
            name.clone(),
            AppearanceAttachment {
                slot_name: name,
                asset_path: asset_path.into(),
                attachment_type,
                socket_target,
            },
        );
    }

    /// Removes an item from an appearance slot.
    pub fn detach(&mut self, slot_name: &str) -> Option<AppearanceAttachment> {
        self.slots.remove(slot_name)
    }

    /// Gets an equipped attachment by slot name.
    #[must_use]
    pub fn get_slot(&self, slot_name: &str) -> Option<&AppearanceAttachment> {
        self.slots.get(slot_name)
    }

    /// Parses a .sat appearance template from IFF bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SatError> {
        let root = IffNode::parse(bytes)?;

        if root.form_type() != Some(*b"STAT") {
            return Err(SatError::MissingStatForm);
        }

        let base_data = root
            .find_chunk(b"BASE")
            .ok_or(SatError::MissingBaseSkeleton)?;
        let base_skeleton = String::from_utf8_lossy(base_data)
            .trim_end_matches('\0')
            .to_string();

        let mut template = Self::new(base_skeleton);

        // Parse modular slots: SLOT chunk
        // Format: count (u32), then [slot_len: u32, slot_str, asset_len: u32, asset_str, is_skinned: u8, target_len: u32, target_str]
        if let Some(slot_data) = root.find_chunk(b"SLOT") {
            let mut off = 0;
            if slot_data.len() >= 4 {
                let count =
                    u32::from_le_bytes([slot_data[0], slot_data[1], slot_data[2], slot_data[3]])
                        as usize;
                off += 4;

                for _ in 0..count {
                    if off + 4 > slot_data.len() {
                        break;
                    }
                    let s_len = u32::from_le_bytes([
                        slot_data[off],
                        slot_data[off + 1],
                        slot_data[off + 2],
                        slot_data[off + 3],
                    ]) as usize;
                    off += 4;
                    if off + s_len > slot_data.len() {
                        break;
                    }
                    let slot_name = String::from_utf8_lossy(&slot_data[off..off + s_len])
                        .trim_end_matches('\0')
                        .to_string();
                    off += s_len;

                    if off + 4 > slot_data.len() {
                        break;
                    }
                    let a_len = u32::from_le_bytes([
                        slot_data[off],
                        slot_data[off + 1],
                        slot_data[off + 2],
                        slot_data[off + 3],
                    ]) as usize;
                    off += 4;
                    if off + a_len > slot_data.len() {
                        break;
                    }
                    let asset_path = String::from_utf8_lossy(&slot_data[off..off + a_len])
                        .trim_end_matches('\0')
                        .to_string();
                    off += a_len;

                    if off >= slot_data.len() {
                        break;
                    }
                    let is_skinned = slot_data[off] != 0;
                    off += 1;

                    if off + 4 > slot_data.len() {
                        break;
                    }
                    let t_len = u32::from_le_bytes([
                        slot_data[off],
                        slot_data[off + 1],
                        slot_data[off + 2],
                        slot_data[off + 3],
                    ]) as usize;
                    off += 4;
                    let socket_target = if t_len > 0 && off + t_len <= slot_data.len() {
                        let target = String::from_utf8_lossy(&slot_data[off..off + t_len])
                            .trim_end_matches('\0')
                            .to_string();
                        off += t_len;
                        Some(target)
                    } else {
                        None
                    };

                    template.attach(
                        slot_name,
                        asset_path,
                        if is_skinned {
                            AttachmentType::SkeletalMesh
                        } else {
                            AttachmentType::SocketRigid
                        },
                        socket_target,
                    );
                }
            }
        }

        Ok(template)
    }

    /// Serializes the template to IFF bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = IffWriter::new();
        writer.begin_form(b"STAT");

        // BASE
        let mut base_data = self.base_skeleton.clone().into_bytes();
        base_data.push(0);
        writer.write_chunk(b"BASE", &base_data);

        // SLOT
        let mut slot_data = Vec::new();
        slot_data.extend_from_slice(&(self.slots.len() as u32).to_le_bytes());
        for att in self.slots.values() {
            let s_bytes = att.slot_name.as_bytes();
            slot_data.extend_from_slice(&(s_bytes.len() as u32).to_le_bytes());
            slot_data.extend_from_slice(s_bytes);

            let a_bytes = att.asset_path.as_bytes();
            slot_data.extend_from_slice(&(a_bytes.len() as u32).to_le_bytes());
            slot_data.extend_from_slice(a_bytes);

            let is_skinned = match att.attachment_type {
                AttachmentType::SkeletalMesh => 1u8,
                AttachmentType::SocketRigid => 0u8,
            };
            slot_data.push(is_skinned);

            if let Some(target) = &att.socket_target {
                let t_bytes = target.as_bytes();
                slot_data.extend_from_slice(&(t_bytes.len() as u32).to_le_bytes());
                slot_data.extend_from_slice(t_bytes);
            } else {
                slot_data.extend_from_slice(&0u32.to_le_bytes());
            }
        }
        writer.write_chunk(b"SLOT", &slot_data);

        writer.end_form();
        writer.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sat_roundtrip_modular_armor() {
        let mut sat = SkeletalAppearanceTemplate::new("appearance/mesh/human_m.skm");

        // Skinned chest armor
        sat.attach(
            "chest",
            "appearance/mesh/armor_composite_chest_m.skm",
            AttachmentType::SkeletalMesh,
            None,
        );

        // Rigid helmet attached to head bone
        sat.attach(
            "helmet",
            "appearance/mesh/armor_composite_helmet_m.msh",
            AttachmentType::SocketRigid,
            Some("head".to_string()),
        );

        // Blaster rifle attached to right hand socket
        sat.attach(
            "weapon_r",
            "appearance/mesh/wp_rifle_e11.msh",
            AttachmentType::SocketRigid,
            Some("hold_r".to_string()),
        );

        let bytes = sat.to_bytes();
        let parsed = SkeletalAppearanceTemplate::from_bytes(&bytes).expect("Failed to parse SAT");

        assert_eq!(parsed.base_skeleton, "appearance/mesh/human_m.skm");
        assert_eq!(parsed.slots.len(), 3);

        let chest = parsed.get_slot("chest").unwrap();
        assert_eq!(chest.attachment_type, AttachmentType::SkeletalMesh);
        assert_eq!(
            chest.asset_path,
            "appearance/mesh/armor_composite_chest_m.skm"
        );

        let weapon = parsed.get_slot("weapon_r").unwrap();
        assert_eq!(weapon.attachment_type, AttachmentType::SocketRigid);
        assert_eq!(weapon.socket_target.as_deref(), Some("hold_r"));
    }
}
