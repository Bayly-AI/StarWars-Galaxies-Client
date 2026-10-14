// crates/swg-client-audio/src/listener.rs

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// 3D audio listener representing the player or camera in world space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioListener {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub velocity: Vec3,
}

impl Default for AudioListener {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            forward: -Vec3::Z, // Default facing forward (-Z)
            up: Vec3::Y,
            velocity: Vec3::ZERO,
        }
    }
}

impl AudioListener {
    #[must_use]
    pub fn new(position: Vec3, forward: Vec3, up: Vec3) -> Self {
        Self {
            position,
            forward: forward.normalize_or_zero(),
            up: up.normalize_or_zero(),
            velocity: Vec3::ZERO,
        }
    }

    /// Computes the normalized right vector (+X direction relative to listener orientation).
    #[must_use]
    pub fn right(&self) -> Vec3 {
        self.forward.cross(self.up).normalize_or_zero()
    }

    pub fn set_velocity(&mut self, velocity: Vec3) {
        self.velocity = velocity;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listener_orientation_vectors() {
        let listener = AudioListener::new(Vec3::ZERO, -Vec3::Z, Vec3::Y);
        let right = listener.right();

        // Cross product of -Z and +Y is +X (Right)
        assert!((right.x - 1.0).abs() < 1e-4);
        assert!(right.y.abs() < 1e-4);
        assert!(right.z.abs() < 1e-4);
    }
}
