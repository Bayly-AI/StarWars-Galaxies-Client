// crates/swg-client-anim/src/state_machine.rs

use crate::ans::AnimationClip;
use crate::blend::AnimationBlender;
use crate::skm::{Bone, Hardpoint, SkeletalMesh};
use glam::Mat4;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pre-CU Locomotion & Posture states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocomotionState {
    StandingIdle,
    Walking,
    Running,
    CombatReady,
    Kneeling,
    Prone,
}

/// Character Animation Controller evaluating states, transitions, and matrix palettes.
pub struct AnimationController {
    pub current_state: LocomotionState,
    pub target_state: Option<LocomotionState>,
    pub current_time: f32,
    pub target_time: f32,
    pub transition_progress: f32,
    pub transition_duration: f32,
    clips: HashMap<LocomotionState, AnimationClip>,
}

impl AnimationController {
    #[must_use]
    pub fn new(initial_state: LocomotionState) -> Self {
        Self {
            current_state: initial_state,
            target_state: None,
            current_time: 0.0,
            target_time: 0.0,
            transition_progress: 1.0,
            transition_duration: 0.25,
            clips: HashMap::new(),
        }
    }

    /// Registers an animation clip for a specific locomotion state.
    pub fn register_clip(&mut self, state: LocomotionState, clip: AnimationClip) {
        self.clips.insert(state, clip);
    }

    /// Triggers a smooth cross-fade transition to a new locomotion state.
    pub fn transition_to(&mut self, state: LocomotionState) {
        if self.current_state == state && self.target_state.is_none() {
            return;
        }
        if self.target_state == Some(state) {
            return;
        }

        if let Some(target) = self.target_state {
            self.current_state = target;
            self.current_time = self.target_time;
        }
        self.target_state = Some(state);
        self.target_time = 0.0;
        self.transition_progress = 0.0;
    }

    /// Advances animation time and evaluates bone global transforms.
    pub fn tick(&mut self, dt: f32, skeleton: &[Bone]) -> Vec<Mat4> {
        self.current_time += dt;

        if let Some(target) = self.target_state {
            self.target_time += dt;
            if self.transition_duration > 0.0 {
                self.transition_progress += dt / self.transition_duration;
            } else {
                self.transition_progress = 1.0;
            }

            if self.transition_progress >= 1.0 {
                self.current_state = target;
                self.current_time = self.target_time;
                self.target_state = None;
                self.transition_progress = 1.0;

                if let Some(clip) = self.clips.get(&self.current_state) {
                    return clip.sample_pose(self.current_time, skeleton);
                }
            } else {
                // Cross-fade blend between current and target
                if let (Some(clip_a), Some(clip_b)) =
                    (self.clips.get(&self.current_state), self.clips.get(&target))
                {
                    return AnimationBlender::blend_two(
                        clip_a,
                        self.current_time,
                        clip_b,
                        self.target_time,
                        self.transition_progress,
                        skeleton,
                    );
                }
            }
        }

        // Single active clip
        if let Some(clip) = self.clips.get(&self.current_state) {
            clip.sample_pose(self.current_time, skeleton)
        } else {
            // Fallback to skeleton rest pose
            skeleton.iter().map(|b| b.local_bind_transform).collect()
        }
    }

    /// Computes the world-space transform for an equipped hardpoint (e.g. right hand weapon socket).
    #[must_use]
    pub fn evaluate_hardpoint(
        hardpoint: &Hardpoint,
        global_bone_transforms: &[Mat4],
        model_matrix: Mat4,
    ) -> Mat4 {
        let bone_global = if hardpoint.bone_index < global_bone_transforms.len() {
            global_bone_transforms[hardpoint.bone_index]
        } else {
            Mat4::IDENTITY
        };
        model_matrix * bone_global * hardpoint.offset
    }

    /// Helper evaluating GPU skinning matrices for a skeletal mesh from the current controller state.
    pub fn evaluate_skinning(&mut self, dt: f32, mesh: &SkeletalMesh) -> (Vec<Mat4>, Vec<Mat4>) {
        let globals = self.tick(dt, &mesh.skeleton);
        let skin_palette = mesh.compute_skinning_matrices(&globals);
        (globals, skin_palette)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ans::TransformTrack;

    #[test]
    fn test_state_machine_transition_and_hardpoint() {
        let mut controller = AnimationController::new(LocomotionState::StandingIdle);

        let bone = Bone {
            name: "hand_r".to_string(),
            parent_index: None,
            local_bind_transform: Mat4::IDENTITY,
            inv_bind_pose: Mat4::IDENTITY,
        };
        let skeleton = vec![bone];

        let clip_idle = AnimationClip::new("idle", 1.0, true);
        let mut clip_run = AnimationClip::new("run", 1.0, true);
        let mut tr = TransformTrack::new("hand_r");
        tr.translations.push(crate::ans::Keyframe {
            time: 0.0,
            value: glam::Vec3::new(1.0, 0.0, 0.0),
        });
        clip_run.add_track(tr);

        controller.register_clip(LocomotionState::StandingIdle, clip_idle);
        controller.register_clip(LocomotionState::Running, clip_run);

        // Tick idle
        let globals = controller.tick(0.1, &skeleton);
        assert_eq!(globals.len(), 1);

        // Transition to Run
        controller.transition_to(LocomotionState::Running);
        assert_eq!(controller.target_state, Some(LocomotionState::Running));

        // Advance 0.25s (reaches target)
        let globals = controller.tick(0.25, &skeleton);
        assert_eq!(controller.current_state, LocomotionState::Running);
        assert_eq!(controller.target_state, None);

        // Verify weapon hardpoint attachment
        let hp = Hardpoint {
            name: "hold_r".to_string(),
            bone_index: 0,
            offset: Mat4::from_translation(glam::Vec3::new(0.0, 0.5, 0.0)),
        };
        let hp_world = AnimationController::evaluate_hardpoint(&hp, &globals, Mat4::IDENTITY);
        let hp_pos = hp_world.w_axis.truncate();
        assert!((hp_pos.x - 1.0).abs() < 1e-4);
        assert!((hp_pos.y - 0.5).abs() < 1e-4);
    }
}
