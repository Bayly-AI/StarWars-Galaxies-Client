// crates/swg-client-render/src/geomorph.rs

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// WGSL vertex shader snippet for GPU-side LOD geomorphing.
pub const GEOMORPH_WGSL: &str = r#"
fn evaluate_geomorph(pos_lod0: vec3<f32>, pos_lod1: vec3<f32>, morph_factor: f32) -> vec3<f32> {
    let t = clamp(morph_factor, 0.0, 1.0);
    return mix(pos_lod0, pos_lod1, t);
}
"#;

/// Configuration defining LOD transition boundaries and geomorphing blend bands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LodGeomorphConfig {
    pub lod_thresholds: [f32; 4], // e.g. [50.0, 100.0, 200.0, 400.0]
    pub transition_band: f32,     // e.g. 15.0 meters
}

impl Default for LodGeomorphConfig {
    fn default() -> Self {
        Self {
            lod_thresholds: [50.0, 100.0, 200.0, 400.0],
            transition_band: 15.0,
        }
    }
}

/// Evaluates continuous geomorph blend factors to eliminate visual mesh popping.
pub struct LodGeomorphEvaluator;

impl LodGeomorphEvaluator {
    /// Determines active LOD level and continuous morph factor `t` in `0.0..=1.0`
    /// blending towards the next lower-detail LOD.
    #[must_use]
    pub fn evaluate_lod_and_morph(distance: f32, config: &LodGeomorphConfig) -> (usize, f32) {
        let thresholds = &config.lod_thresholds;
        let band = config.transition_band;

        for (lod, &threshold) in thresholds.iter().enumerate() {
            if distance < threshold - band {
                return (lod, 0.0);
            } else if distance < threshold {
                let t = (distance - (threshold - band)) / band;
                return (lod, t.clamp(0.0, 1.0));
            }
        }

        (thresholds.len(), 0.0)
    }

    /// Morphs a fine vertex buffer towards a coarse vertex buffer by factor `t`.
    #[must_use]
    pub fn morph_vertices(fine: &[Vec3], coarse: &[Vec3], t: f32) -> Vec<Vec3> {
        let t_clamped = t.clamp(0.0, 1.0);
        let count = fine.len().min(coarse.len());
        let mut result = Vec::with_capacity(count);

        for i in 0..count {
            let v = fine[i].lerp(coarse[i], t_clamped);
            result.push(v);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_geomorph_transitions() {
        let config = LodGeomorphConfig {
            lod_thresholds: [50.0, 100.0, 200.0, 400.0],
            transition_band: 10.0, // Transitions occur in [40..50], [90..100], etc.
        };

        // Well before transition: LOD 0, morph = 0.0
        let (lod, morph) = LodGeomorphEvaluator::evaluate_lod_and_morph(30.0, &config);
        assert_eq!(lod, 0);
        assert_eq!(morph, 0.0);

        // At start of transition band (40m): LOD 0, morph = 0.0
        let (lod_start, morph_start) = LodGeomorphEvaluator::evaluate_lod_and_morph(40.0, &config);
        assert_eq!(lod_start, 0);
        assert!((morph_start - 0.0).abs() < 0.001);

        // In middle of transition band (45m): LOD 0, morph = 0.5
        let (lod_mid, morph_mid) = LodGeomorphEvaluator::evaluate_lod_and_morph(45.0, &config);
        assert_eq!(lod_mid, 0);
        assert!((morph_mid - 0.5).abs() < 0.001);

        // At threshold (50m): LOD 1, morph = 0.0
        let (lod_next, morph_next) = LodGeomorphEvaluator::evaluate_lod_and_morph(51.0, &config);
        assert_eq!(lod_next, 1);
        assert_eq!(morph_next, 0.0);
    }

    #[test]
    fn test_vertex_morph_interpolation() {
        let fine = vec![Vec3::new(0.0, 10.0, 0.0), Vec3::new(10.0, 10.0, 0.0)];
        let coarse = vec![Vec3::new(0.0, 5.0, 0.0), Vec3::new(10.0, 5.0, 0.0)];

        let morphed = LodGeomorphEvaluator::morph_vertices(&fine, &coarse, 0.5);
        assert!((morphed[0].y - 7.5).abs() < 0.001);
        assert!((morphed[1].y - 7.5).abs() < 0.001);
    }
}
