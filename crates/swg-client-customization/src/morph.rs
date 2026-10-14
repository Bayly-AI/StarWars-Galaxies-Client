// crates/swg-client-customization/src/morph.rs

use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard morph slider identifiers in Star Wars Galaxies character creation.
pub const MORPH_HEIGHT: &str = "height";
pub const MORPH_WEIGHT: &str = "weight";
pub const MORPH_MUSCLE: &str = "muscle";
pub const MORPH_NOSE_WIDTH: &str = "nose_width";
pub const MORPH_JAW_LINE: &str = "jaw_line";
pub const MORPH_EYE_SIZE: &str = "eye_size";
pub const MORPH_LEKKU_LENGTH: &str = "lekku_length";
pub const MORPH_HORN_SIZE: &str = "horn_size";

/// A base rigid or skinned 3D mesh before morph blend shapes are applied.
#[derive(Debug, Clone, PartialEq)]
pub struct BaseMesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
}

impl BaseMesh {
    #[must_use]
    pub fn new(positions: Vec<Vec3>, normals: Vec<Vec3>, indices: Vec<u32>) -> Self {
        assert_eq!(
            positions.len(),
            normals.len(),
            "Positions and normals must have identical length"
        );
        Self {
            positions,
            normals,
            indices,
        }
    }
}

/// A single vertex blend shape target containing delta displacements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MorphTarget {
    pub name: String,
    pub delta_positions: Vec<Vec3>,
    pub delta_normals: Vec<Vec3>,
}

impl MorphTarget {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        delta_positions: Vec<Vec3>,
        delta_normals: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            delta_positions,
            delta_normals,
        }
    }
}

/// Resulting deformed mesh ready for GPU vertex buffer upload.
#[derive(Debug, Clone, PartialEq)]
pub struct DeformedMesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
}

/// Evaluator that blends multiple morph targets in real-time onto a base mesh.
pub struct MorphEvaluator;

impl MorphEvaluator {
    /// Deforms the given base mesh by weighting and accumulating all active morph targets.
    #[must_use]
    pub fn evaluate(
        base: &BaseMesh,
        targets: &[MorphTarget],
        slider_weights: &HashMap<String, f32>,
    ) -> DeformedMesh {
        let count = base.positions.len();
        let mut deformed_positions = base.positions.clone();
        let mut accumulated_normals = base.normals.clone();

        for target in targets {
            let weight = slider_weights.get(&target.name).copied().unwrap_or(0.0);
            if weight.abs() < 0.0001 {
                continue;
            }

            // Ensure morph delta buffer has matching vertex count
            if target.delta_positions.len() != count {
                continue;
            }

            for i in 0..count {
                deformed_positions[i] += target.delta_positions[i] * weight;
                if target.delta_normals.len() == count {
                    accumulated_normals[i] += target.delta_normals[i] * weight;
                }
            }
        }

        // Re-normalize all normals to maintain smooth shading
        for normal in &mut accumulated_normals {
            let len_sq = normal.length_squared();
            if len_sq > 0.00001 {
                *normal /= len_sq.sqrt();
            } else {
                *normal = Vec3::Y;
            }
        }

        DeformedMesh {
            positions: deformed_positions,
            normals: accumulated_normals,
            indices: base.indices.clone(),
        }
    }
}

/// Dynamic map of morph slider names to blend factors (typically clamped to 0.0..=1.0).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MorphSliderState {
    weights: HashMap<String, f32>,
}

impl MorphSliderState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            weights: HashMap::new(),
        }
    }

    pub fn set_slider(&mut self, name: impl Into<String>, value: f32) {
        self.weights.insert(name.into(), value.clamp(0.0, 1.0));
    }

    #[must_use]
    pub fn get_slider(&self, name: &str) -> f32 {
        self.weights.get(name).copied().unwrap_or(0.0)
    }

    #[must_use]
    pub fn weights(&self) -> &HashMap<String, f32> {
        &self.weights
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morph_evaluation_zero_weights() {
        let base = BaseMesh::new(
            vec![Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 1.0, 0.0)],
            vec![Vec3::Y, Vec3::Y],
            vec![0, 1],
        );

        let targets = vec![MorphTarget::new(
            MORPH_MUSCLE,
            vec![Vec3::new(0.1, 0.0, 0.0), Vec3::new(0.1, 0.0, 0.0)],
            vec![Vec3::ZERO, Vec3::ZERO],
        )];

        let sliders = HashMap::new();
        let deformed = MorphEvaluator::evaluate(&base, &targets, &sliders);

        assert_eq!(deformed.positions, base.positions);
        assert_eq!(deformed.normals, base.normals);
    }

    #[test]
    fn test_morph_evaluation_blend_shapes() {
        let base = BaseMesh::new(
            vec![Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 1.0, 0.0)],
            vec![Vec3::Y, Vec3::Y],
            vec![0, 1],
        );

        let muscle_target = MorphTarget::new(
            MORPH_MUSCLE,
            vec![Vec3::new(0.2, 0.0, 0.0), Vec3::new(0.4, 0.0, 0.0)],
            vec![Vec3::X, Vec3::X],
        );
        let height_target = MorphTarget::new(
            MORPH_HEIGHT,
            vec![Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.0, 0.5, 0.0)],
            vec![Vec3::ZERO, Vec3::ZERO],
        );

        let mut sliders = HashMap::new();
        sliders.insert(MORPH_MUSCLE.to_string(), 0.5);
        sliders.insert(MORPH_HEIGHT.to_string(), 1.0);

        let deformed = MorphEvaluator::evaluate(&base, &[muscle_target, height_target], &sliders);

        // Vertex 0: (0.0, 1.0, 0.0) + 0.5*(0.2, 0.0, 0.0) + 1.0*(0.0, 0.5, 0.0) = (0.1, 1.5, 0.0)
        assert!((deformed.positions[0].x - 0.1).abs() < 0.001);
        assert!((deformed.positions[0].y - 1.5).abs() < 0.001);

        // Vertex 1: (1.0, 1.0, 0.0) + 0.5*(0.4, 0.0, 0.0) + 1.0*(0.0, 0.5, 0.0) = (1.2, 1.5, 0.0)
        assert!((deformed.positions[1].x - 1.2).abs() < 0.001);
        assert!((deformed.positions[1].y - 1.5).abs() < 0.001);

        // Normal must remain normalized
        assert!((deformed.normals[0].length() - 1.0).abs() < 0.001);
    }
}
