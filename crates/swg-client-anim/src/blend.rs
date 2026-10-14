// crates/swg-client-anim/src/blend.rs

use crate::ans::AnimationClip;
use crate::skm::Bone;
use glam::{Mat4, Quat, Vec3};
use std::collections::HashMap;

/// Decomposed local SRT transform for a single bone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoneTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl BoneTransform {
    #[must_use]
    pub fn from_mat4(mat: Mat4) -> Self {
        let (scale, rotation, translation) = mat.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }

    #[must_use]
    pub fn to_mat4(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Linearly and spherically interpolates between two bone transforms.
    #[must_use]
    pub fn blend(self, other: Self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        Self {
            translation: self.translation.lerp(other.translation, factor),
            rotation: self.rotation.slerp(other.rotation, factor),
            scale: self.scale.lerp(other.scale, factor),
        }
    }
}

/// Blends multiple animation clips across a common skeleton.
pub struct AnimationBlender;

impl AnimationBlender {
    /// Cross-fades between two animation clips with factor alpha in [0.0, 1.0].
    /// Alpha = 0.0 gives 100% Clip A; Alpha = 1.0 gives 100% Clip B.
    #[must_use]
    pub fn blend_two(
        clip_a: &AnimationClip,
        time_a: f32,
        clip_b: &AnimationClip,
        time_b: f32,
        alpha: f32,
        skeleton: &[Bone],
    ) -> Vec<Mat4> {
        let alpha = alpha.clamp(0.0, 1.0);

        let mut tracks_a = HashMap::new();
        for t in &clip_a.tracks {
            tracks_a.insert(t.bone_name.as_str(), t);
        }

        let mut tracks_b = HashMap::new();
        for t in &clip_b.tracks {
            tracks_b.insert(t.bone_name.as_str(), t);
        }

        let sample_time_a = if clip_a.looping {
            time_a.rem_euclid(clip_a.duration)
        } else {
            time_a.clamp(0.0, clip_a.duration)
        };

        let sample_time_b = if clip_b.looping {
            time_b.rem_euclid(clip_b.duration)
        } else {
            time_b.clamp(0.0, clip_b.duration)
        };

        // Evaluate and blend local transforms
        let mut local_transforms = Vec::with_capacity(skeleton.len());
        for bone in skeleton {
            let srt_a = if let Some(tr) = tracks_a.get(bone.name.as_str()) {
                let (t, r, s) = tr.sample(sample_time_a);
                BoneTransform {
                    translation: t,
                    rotation: r,
                    scale: s,
                }
            } else {
                BoneTransform::from_mat4(bone.local_bind_transform)
            };

            let srt_b = if let Some(tr) = tracks_b.get(bone.name.as_str()) {
                let (t, r, s) = tr.sample(sample_time_b);
                BoneTransform {
                    translation: t,
                    rotation: r,
                    scale: s,
                }
            } else {
                BoneTransform::from_mat4(bone.local_bind_transform)
            };

            let blended = srt_a.blend(srt_b, alpha);
            local_transforms.push(blended.to_mat4());
        }

        // Forward kinematics: compute global transforms
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ans::{Keyframe, TransformTrack};

    #[test]
    fn test_blend_two_animations_midpoint() {
        let bone = Bone {
            name: "arm_r".to_string(),
            parent_index: None,
            local_bind_transform: Mat4::IDENTITY,
            inv_bind_pose: Mat4::IDENTITY,
        };
        let skeleton = vec![bone];

        // Clip A: translation at (0, 0, 0)
        let mut clip_a = AnimationClip::new("idle", 1.0, true);
        let mut tr_a = TransformTrack::new("arm_r");
        tr_a.translations.push(Keyframe {
            time: 0.0,
            value: Vec3::ZERO,
        });
        clip_a.add_track(tr_a);

        // Clip B: translation at (0, 10, 0)
        let mut clip_b = AnimationClip::new("reach", 1.0, true);
        let mut tr_b = TransformTrack::new("arm_r");
        tr_b.translations.push(Keyframe {
            time: 0.0,
            value: Vec3::new(0.0, 10.0, 0.0),
        });
        clip_b.add_track(tr_b);

        // Blend at alpha = 0.5 -> expect translation at (0, 5, 0)
        let globals = AnimationBlender::blend_two(&clip_a, 0.0, &clip_b, 0.0, 0.5, &skeleton);
        assert_eq!(globals.len(), 1);
        let trans = globals[0].w_axis.truncate();
        assert!((trans.y - 5.0).abs() < 1e-4);
    }
}
