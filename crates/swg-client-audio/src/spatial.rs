// crates/swg-client-audio/src/spatial.rs

use crate::listener::AudioListener;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Distance attenuation curve model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttenuationModel {
    InverseSquare,
    Linear,
    Exponential,
}

/// A 3D audio emitter placed in the game world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioEmitter {
    pub id: u64,
    pub position: Vec3,
    pub velocity: Vec3,
    pub min_distance: f32, // Distance below which volume is 1.0
    pub max_distance: f32, // Maximum audible radius
    pub rolloff_factor: f32,
    pub base_volume: f32,
    pub attenuation_model: AttenuationModel,
}

impl AudioEmitter {
    #[must_use]
    pub fn new(id: u64, position: Vec3, min_dist: f32, max_dist: f32) -> Self {
        Self {
            id,
            position,
            velocity: Vec3::ZERO,
            min_distance: min_dist.max(0.1),
            max_distance: max_dist.max(min_dist),
            rolloff_factor: 1.0,
            base_volume: 1.0,
            attenuation_model: AttenuationModel::InverseSquare,
        }
    }

    #[must_use]
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.base_volume = volume.clamp(0.0, 1.0);
        self
    }
}

/// 7.1 surround sound multi-channel gain distribution.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SurroundGains {
    pub left: f32,
    pub right: f32,
    pub center: f32,
    pub lfe: f32, // Low Frequency Effects (Subwoofer)
    pub surround_left: f32,
    pub surround_right: f32,
    pub back_left: f32,
    pub back_right: f32,
}

impl SurroundGains {
    /// Extracts simplified two-channel stereo gains (left, right).
    #[must_use]
    pub fn stereo_pair(&self) -> (f32, f32) {
        let left_total =
            self.left + self.center * 0.707 + self.surround_left * 0.707 + self.back_left * 0.5;
        let right_total =
            self.right + self.center * 0.707 + self.surround_right * 0.707 + self.back_right * 0.5;
        (left_total.min(1.0), right_total.min(1.0))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpatialAudioEvaluator;

impl SpatialAudioEvaluator {
    /// Computes distance attenuation gain [0.0, 1.0].
    #[must_use]
    pub fn compute_attenuation(emitter: &AudioEmitter, distance: f32) -> f32 {
        if distance <= emitter.min_distance {
            return 1.0;
        }
        if distance >= emitter.max_distance {
            return 0.0;
        }

        match emitter.attenuation_model {
            AttenuationModel::InverseSquare => {
                let range = emitter.max_distance - emitter.min_distance;
                let excess = distance - emitter.min_distance;
                let factor =
                    emitter.min_distance / (emitter.min_distance + emitter.rolloff_factor * excess);
                let fade = 1.0 - (excess / range).clamp(0.0, 1.0);
                (factor * fade).clamp(0.0, 1.0)
            }
            AttenuationModel::Linear => {
                let range = emitter.max_distance - emitter.min_distance;
                (1.0 - ((distance - emitter.min_distance) / range)).clamp(0.0, 1.0)
            }
            AttenuationModel::Exponential => {
                let norm = (distance - emitter.min_distance)
                    / (emitter.max_distance - emitter.min_distance);
                ((-norm * emitter.rolloff_factor * 3.0).exp()).clamp(0.0, 1.0)
            }
        }
    }

    /// Evaluates 7.1 surround sound gains for an emitter relative to listener position and orientation.
    #[must_use]
    pub fn evaluate_spatial(emitter: &AudioEmitter, listener: &AudioListener) -> SurroundGains {
        let delta = emitter.position - listener.position;
        let dist = delta.length();
        let atten = Self::compute_attenuation(emitter, dist) * emitter.base_volume;

        if atten <= 0.001 || dist < 1e-4 {
            let half = atten * 0.5;
            return SurroundGains {
                left: half,
                right: half,
                center: atten,
                lfe: atten * 0.2,
                surround_left: half,
                surround_right: half,
                back_left: half * 0.5,
                back_right: half * 0.5,
            };
        }

        let dir = delta / dist;
        let forward = listener.forward;
        let right = listener.right();

        // Project direction onto listener horizontal axes
        let front_dot = dir.dot(forward); // +1 front, -1 rear
        let right_dot = dir.dot(right); // +1 right, -1 left

        // Front vs rear split
        let front_weight = (front_dot + 1.0) * 0.5; // 1.0 full front, 0.0 full rear
        let rear_weight = 1.0 - front_weight;

        // Left vs right split
        let right_weight = (right_dot + 1.0) * 0.5; // 1.0 full right, 0.0 full left
        let left_weight = 1.0 - right_weight;

        let center_weight = (front_dot.max(0.0) * (1.0 - right_dot.abs())).max(0.0);

        SurroundGains {
            left: atten * left_weight * front_weight,
            right: atten * right_weight * front_weight,
            center: atten * center_weight,
            lfe: atten * 0.2, // Subtle low-frequency presence
            surround_left: atten * left_weight * (1.0 - front_dot.abs()),
            surround_right: atten * right_weight * (1.0 - front_dot.abs()),
            back_left: atten * left_weight * rear_weight,
            back_right: atten * right_weight * rear_weight,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_attenuation_and_cutoff() {
        let emitter = AudioEmitter::new(1, Vec3::new(0.0, 0.0, 50.0), 10.0, 100.0);

        // Within min distance: full volume
        assert!((SpatialAudioEvaluator::compute_attenuation(&emitter, 5.0) - 1.0).abs() < 1e-4);

        // Midway: attenuated
        let mid_gain = SpatialAudioEvaluator::compute_attenuation(&emitter, 50.0);
        assert!(mid_gain > 0.0 && mid_gain < 1.0);

        // At or beyond max distance: silence
        assert_eq!(
            SpatialAudioEvaluator::compute_attenuation(&emitter, 100.0),
            0.0
        );
        assert_eq!(
            SpatialAudioEvaluator::compute_attenuation(&emitter, 150.0),
            0.0
        );
    }

    #[test]
    fn test_directional_panning_left_and_right() {
        let listener = AudioListener::new(Vec3::ZERO, -Vec3::Z, Vec3::Y);

        // Emitter on the far right (+X)
        let emitter_right = AudioEmitter::new(2, Vec3::new(20.0, 0.0, 0.0), 5.0, 100.0);
        let gains_right = SpatialAudioEvaluator::evaluate_spatial(&emitter_right, &listener);
        let (left_r, right_r) = gains_right.stereo_pair();
        assert!(
            right_r > left_r * 2.0,
            "Right channel should dominate for right emitter"
        );

        // Emitter on the far left (-X)
        let emitter_left = AudioEmitter::new(3, Vec3::new(-20.0, 0.0, 0.0), 5.0, 100.0);
        let gains_left = SpatialAudioEvaluator::evaluate_spatial(&emitter_left, &listener);
        let (left_l, right_l) = gains_left.stereo_pair();
        assert!(
            left_l > right_l * 2.0,
            "Left channel should dominate for left emitter"
        );
    }
}
