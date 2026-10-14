// crates/swg-client-anim/src/ans.rs

use crate::skm::Bone;
use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use swg_client_iff::{IffError, IffNode, IffWriter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnsError {
    #[error("IFF error: {0}")]
    Iff(#[from] IffError),
    #[error("Missing ANST animation form")]
    MissingAnstForm,
    #[error("Missing INFO chunk")]
    MissingInfoChunk,
}

/// A time-stamped keyframe value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Keyframe<T> {
    pub time: f32,
    pub value: T,
}

/// Keyframe channels for a single bone over time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformTrack {
    pub bone_name: String,
    pub translations: Vec<Keyframe<Vec3>>,
    pub rotations: Vec<Keyframe<Quat>>,
    pub scales: Vec<Keyframe<Vec3>>,
}

impl TransformTrack {
    #[must_use]
    pub fn new(bone_name: impl Into<String>) -> Self {
        Self {
            bone_name: bone_name.into(),
            translations: Vec::new(),
            rotations: Vec::new(),
            scales: Vec::new(),
        }
    }

    /// Evaluates the local transformation at time `t`.
    #[must_use]
    pub fn sample(&self, time: f32) -> (Vec3, Quat, Vec3) {
        let trans = sample_vec3(&self.translations, time).unwrap_or(Vec3::ZERO);
        let rot = sample_quat(&self.rotations, time).unwrap_or(Quat::IDENTITY);
        let scale = sample_vec3(&self.scales, time).unwrap_or(Vec3::ONE);
        (trans, rot, scale)
    }
}

fn sample_vec3(keys: &[Keyframe<Vec3>], time: f32) -> Option<Vec3> {
    if keys.is_empty() {
        return None;
    }
    if keys.len() == 1 || time <= keys[0].time {
        return Some(keys[0].value);
    }
    if time >= keys.last().unwrap().time {
        return Some(keys.last().unwrap().value);
    }

    for i in 0..keys.len() - 1 {
        let k0 = &keys[i];
        let k1 = &keys[i + 1];
        if time >= k0.time && time <= k1.time {
            let span = (k1.time - k0.time).max(1e-6);
            let factor = ((time - k0.time) / span).clamp(0.0, 1.0);
            return Some(k0.value.lerp(k1.value, factor));
        }
    }

    Some(keys.last().unwrap().value)
}

fn sample_quat(keys: &[Keyframe<Quat>], time: f32) -> Option<Quat> {
    if keys.is_empty() {
        return None;
    }
    if keys.len() == 1 || time <= keys[0].time {
        return Some(keys[0].value);
    }
    if time >= keys.last().unwrap().time {
        return Some(keys.last().unwrap().value);
    }

    for i in 0..keys.len() - 1 {
        let k0 = &keys[i];
        let k1 = &keys[i + 1];
        if time >= k0.time && time <= k1.time {
            let span = (k1.time - k0.time).max(1e-6);
            let factor = ((time - k0.time) / span).clamp(0.0, 1.0);
            return Some(k0.value.slerp(k1.value, factor));
        }
    }

    Some(keys.last().unwrap().value)
}

/// An animation clip containing multiple bone tracks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub looping: bool,
    pub tracks: Vec<TransformTrack>,
}

impl AnimationClip {
    #[must_use]
    pub fn new(name: impl Into<String>, duration: f32, looping: bool) -> Self {
        Self {
            name: name.into(),
            duration: duration.max(0.001),
            looping,
            tracks: Vec::new(),
        }
    }

    pub fn add_track(&mut self, track: TransformTrack) {
        self.tracks.push(track);
    }

    /// Evaluates forward-kinematics global bone transforms for an entire skeleton at time `t`.
    #[must_use]
    pub fn sample_pose(&self, time: f32, skeleton: &[Bone]) -> Vec<Mat4> {
        let sample_time = if self.looping {
            time.rem_euclid(self.duration)
        } else {
            time.clamp(0.0, self.duration)
        };

        // Build quick lookup table for tracks by bone name
        let mut track_map = HashMap::with_capacity(self.tracks.len());
        for track in &self.tracks {
            track_map.insert(track.bone_name.as_str(), track);
        }

        // First compute local transforms
        let mut local_transforms = Vec::with_capacity(skeleton.len());
        for bone in skeleton {
            if let Some(track) = track_map.get(bone.name.as_str()) {
                let (trans, rot, scale) = track.sample(sample_time);
                let local = Mat4::from_scale_rotation_translation(scale, rot, trans);
                local_transforms.push(local);
            } else {
                local_transforms.push(bone.local_bind_transform);
            }
        }

        // Forward kinematics: hierarchy must guarantee parents appear before children,
        // or we evaluate with parent recursion/dependency.
        let mut global_transforms = vec![Mat4::IDENTITY; skeleton.len()];
        for (i, bone) in skeleton.iter().enumerate() {
            let local = local_transforms[i];
            if let Some(p_idx) = bone.parent_index {
                let p_global = if p_idx < global_transforms.len() {
                    global_transforms[p_idx]
                } else {
                    Mat4::IDENTITY
                };
                global_transforms[i] = p_global * local;
            } else {
                global_transforms[i] = local;
            }
        }

        global_transforms
    }

    /// Parses animation clip from IFF bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AnsError> {
        let root = IffNode::parse(bytes)?;

        if root.form_type() != Some(*b"ANST") {
            return Err(AnsError::MissingAnstForm);
        }

        let info_data = root.find_chunk(b"INFO").ok_or(AnsError::MissingInfoChunk)?;
        if info_data.len() < 9 {
            return Err(AnsError::MissingInfoChunk);
        }

        let name_len =
            u32::from_le_bytes([info_data[0], info_data[1], info_data[2], info_data[3]]) as usize;
        let mut off = 4;
        let name = String::from_utf8_lossy(&info_data[off..off + name_len])
            .trim_end_matches('\0')
            .to_string();
        off += name_len;

        let duration = f32::from_le_bytes([
            info_data[off],
            info_data[off + 1],
            info_data[off + 2],
            info_data[off + 3],
        ]);
        off += 4;
        let looping = info_data[off] != 0;

        let mut clip = Self::new(name, duration, looping);

        // TRCK chunk
        if let Some(trck_data) = root.find_chunk(b"TRCK") {
            let mut t_off = 0;
            if trck_data.len() >= 4 {
                let track_count =
                    u32::from_le_bytes([trck_data[0], trck_data[1], trck_data[2], trck_data[3]])
                        as usize;
                t_off += 4;

                for _ in 0..track_count {
                    if t_off + 4 > trck_data.len() {
                        break;
                    }
                    let b_len = u32::from_le_bytes([
                        trck_data[t_off],
                        trck_data[t_off + 1],
                        trck_data[t_off + 2],
                        trck_data[t_off + 3],
                    ]) as usize;
                    t_off += 4;
                    let bone_name = String::from_utf8_lossy(&trck_data[t_off..t_off + b_len])
                        .trim_end_matches('\0')
                        .to_string();
                    t_off += b_len;

                    let mut track = TransformTrack::new(bone_name);

                    // Translations: count (u32), then [time: f32, x, y, z: f32]
                    if t_off + 4 > trck_data.len() {
                        break;
                    }
                    let trans_count = u32::from_le_bytes([
                        trck_data[t_off],
                        trck_data[t_off + 1],
                        trck_data[t_off + 2],
                        trck_data[t_off + 3],
                    ]) as usize;
                    t_off += 4;
                    for _ in 0..trans_count {
                        if t_off + 16 > trck_data.len() {
                            break;
                        }
                        let time = f32::from_le_bytes([
                            trck_data[t_off],
                            trck_data[t_off + 1],
                            trck_data[t_off + 2],
                            trck_data[t_off + 3],
                        ]);
                        let x = f32::from_le_bytes([
                            trck_data[t_off + 4],
                            trck_data[t_off + 5],
                            trck_data[t_off + 6],
                            trck_data[t_off + 7],
                        ]);
                        let y = f32::from_le_bytes([
                            trck_data[t_off + 8],
                            trck_data[t_off + 9],
                            trck_data[t_off + 10],
                            trck_data[t_off + 11],
                        ]);
                        let z = f32::from_le_bytes([
                            trck_data[t_off + 12],
                            trck_data[t_off + 13],
                            trck_data[t_off + 14],
                            trck_data[t_off + 15],
                        ]);
                        t_off += 16;
                        track.translations.push(Keyframe {
                            time,
                            value: Vec3::new(x, y, z),
                        });
                    }

                    // Rotations: count (u32), then [time: f32, x, y, z, w: f32]
                    if t_off + 4 > trck_data.len() {
                        break;
                    }
                    let rot_count = u32::from_le_bytes([
                        trck_data[t_off],
                        trck_data[t_off + 1],
                        trck_data[t_off + 2],
                        trck_data[t_off + 3],
                    ]) as usize;
                    t_off += 4;
                    for _ in 0..rot_count {
                        if t_off + 20 > trck_data.len() {
                            break;
                        }
                        let time = f32::from_le_bytes([
                            trck_data[t_off],
                            trck_data[t_off + 1],
                            trck_data[t_off + 2],
                            trck_data[t_off + 3],
                        ]);
                        let x = f32::from_le_bytes([
                            trck_data[t_off + 4],
                            trck_data[t_off + 5],
                            trck_data[t_off + 6],
                            trck_data[t_off + 7],
                        ]);
                        let y = f32::from_le_bytes([
                            trck_data[t_off + 8],
                            trck_data[t_off + 9],
                            trck_data[t_off + 10],
                            trck_data[t_off + 11],
                        ]);
                        let z = f32::from_le_bytes([
                            trck_data[t_off + 12],
                            trck_data[t_off + 13],
                            trck_data[t_off + 14],
                            trck_data[t_off + 15],
                        ]);
                        let w = f32::from_le_bytes([
                            trck_data[t_off + 16],
                            trck_data[t_off + 17],
                            trck_data[t_off + 18],
                            trck_data[t_off + 19],
                        ]);
                        t_off += 20;
                        track.rotations.push(Keyframe {
                            time,
                            value: Quat::from_xyzw(x, y, z, w).normalize(),
                        });
                    }

                    // Scales: count (u32), then [time: f32, x, y, z: f32]
                    if t_off + 4 > trck_data.len() {
                        break;
                    }
                    let scale_count = u32::from_le_bytes([
                        trck_data[t_off],
                        trck_data[t_off + 1],
                        trck_data[t_off + 2],
                        trck_data[t_off + 3],
                    ]) as usize;
                    t_off += 4;
                    for _ in 0..scale_count {
                        if t_off + 16 > trck_data.len() {
                            break;
                        }
                        let time = f32::from_le_bytes([
                            trck_data[t_off],
                            trck_data[t_off + 1],
                            trck_data[t_off + 2],
                            trck_data[t_off + 3],
                        ]);
                        let x = f32::from_le_bytes([
                            trck_data[t_off + 4],
                            trck_data[t_off + 5],
                            trck_data[t_off + 6],
                            trck_data[t_off + 7],
                        ]);
                        let y = f32::from_le_bytes([
                            trck_data[t_off + 8],
                            trck_data[t_off + 9],
                            trck_data[t_off + 10],
                            trck_data[t_off + 11],
                        ]);
                        let z = f32::from_le_bytes([
                            trck_data[t_off + 12],
                            trck_data[t_off + 13],
                            trck_data[t_off + 14],
                            trck_data[t_off + 15],
                        ]);
                        t_off += 16;
                        track.scales.push(Keyframe {
                            time,
                            value: Vec3::new(x, y, z),
                        });
                    }

                    clip.add_track(track);
                }
            }
        }

        Ok(clip)
    }

    /// Serializes animation clip to IFF bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = IffWriter::new();
        writer.begin_form(b"ANST");

        // INFO
        let mut info_data = Vec::new();
        let name_bytes = self.name.as_bytes();
        info_data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        info_data.extend_from_slice(name_bytes);
        info_data.extend_from_slice(&self.duration.to_le_bytes());
        info_data.push(if self.looping { 1 } else { 0 });
        writer.write_chunk(b"INFO", &info_data);

        // TRCK
        let mut trck_data = Vec::new();
        trck_data.extend_from_slice(&(self.tracks.len() as u32).to_le_bytes());
        for track in &self.tracks {
            let b_bytes = track.bone_name.as_bytes();
            trck_data.extend_from_slice(&(b_bytes.len() as u32).to_le_bytes());
            trck_data.extend_from_slice(b_bytes);

            trck_data.extend_from_slice(&(track.translations.len() as u32).to_le_bytes());
            for k in &track.translations {
                trck_data.extend_from_slice(&k.time.to_le_bytes());
                trck_data.extend_from_slice(&k.value.x.to_le_bytes());
                trck_data.extend_from_slice(&k.value.y.to_le_bytes());
                trck_data.extend_from_slice(&k.value.z.to_le_bytes());
            }

            trck_data.extend_from_slice(&(track.rotations.len() as u32).to_le_bytes());
            for k in &track.rotations {
                trck_data.extend_from_slice(&k.time.to_le_bytes());
                trck_data.extend_from_slice(&k.value.x.to_le_bytes());
                trck_data.extend_from_slice(&k.value.y.to_le_bytes());
                trck_data.extend_from_slice(&k.value.z.to_le_bytes());
                trck_data.extend_from_slice(&k.value.w.to_le_bytes());
            }

            trck_data.extend_from_slice(&(track.scales.len() as u32).to_le_bytes());
            for k in &track.scales {
                trck_data.extend_from_slice(&k.time.to_le_bytes());
                trck_data.extend_from_slice(&k.value.x.to_le_bytes());
                trck_data.extend_from_slice(&k.value.y.to_le_bytes());
                trck_data.extend_from_slice(&k.value.z.to_le_bytes());
            }
        }
        writer.write_chunk(b"TRCK", &trck_data);

        writer.end_form();
        writer.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_animation_clip_roundtrip_and_interpolation() {
        let mut clip = AnimationClip::new("run_fwd", 1.0, true);

        let mut leg_track = TransformTrack::new("leg_l");
        // Translation oscillates up and down
        leg_track.translations.push(Keyframe {
            time: 0.0,
            value: Vec3::new(0.0, 0.0, 0.0),
        });
        leg_track.translations.push(Keyframe {
            time: 0.5,
            value: Vec3::new(0.0, 0.5, 0.0),
        });
        leg_track.translations.push(Keyframe {
            time: 1.0,
            value: Vec3::new(0.0, 0.0, 0.0),
        });

        // Rotation rotates around X axis from 0 to 90 deg and back
        let q0 = Quat::IDENTITY;
        let q_mid = Quat::from_rotation_x(PI / 2.0);
        let q1 = Quat::IDENTITY;
        leg_track.rotations.push(Keyframe {
            time: 0.0,
            value: q0,
        });
        leg_track.rotations.push(Keyframe {
            time: 0.5,
            value: q_mid,
        });
        leg_track.rotations.push(Keyframe {
            time: 1.0,
            value: q1,
        });

        clip.add_track(leg_track);

        let bytes = clip.to_bytes();
        let parsed = AnimationClip::from_bytes(&bytes).expect("Failed to parse ANS");

        assert_eq!(parsed.name, "run_fwd");
        assert_eq!(parsed.duration, 1.0);
        assert!(parsed.looping);
        assert_eq!(parsed.tracks.len(), 1);

        // Test sample at 0.25 (halfway between 0 and 0.5)
        let (trans, rot, scale) = parsed.tracks[0].sample(0.25);
        assert!((trans.y - 0.25).abs() < 1e-4);
        assert_eq!(scale, Vec3::ONE);

        // Angle should be ~ 45 degrees
        let angle = rot.to_axis_angle().1;
        assert!((angle - (PI / 4.0)).abs() < 0.05);
    }
}
