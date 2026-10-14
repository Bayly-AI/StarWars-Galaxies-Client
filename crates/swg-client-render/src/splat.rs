// crates/swg-client-render/src/splat.rs

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// 4-Layer terrain splat weight distribution.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SplatWeights {
    pub sand: f32,
    pub dirt: f32,
    pub rock: f32,
    pub mesa: f32,
}

impl Default for SplatWeights {
    fn default() -> Self {
        Self {
            sand: 1.0,
            dirt: 0.0,
            rock: 0.0,
            mesa: 0.0,
        }
    }
}

impl SplatWeights {
    /// Computes normalized multi-layer splat weights from surface normal slope and altitude elevation.
    #[must_use]
    pub fn compute(normal: Vec3, elevation: f32) -> Self {
        let slope = normal.y.clamp(0.0, 1.0); // 1.0 = horizontal flat, 0.0 = vertical cliff

        // Flat ground: slope > 0.85
        // Moderate slope: 0.65 < slope <= 0.85
        // Steep rock cliff: slope <= 0.65
        let (mut sand, mut dirt, rock, mesa);

        if slope > 0.85 {
            sand = 1.0;
            dirt = 0.0;
            rock = 0.0;
        } else if slope > 0.65 {
            let t = (slope - 0.65) / 0.20;
            sand = t;
            dirt = 1.0 - t;
            rock = 0.0;
        } else if slope > 0.40 {
            let t = (slope - 0.40) / 0.25;
            sand = 0.0;
            dirt = t;
            rock = 1.0 - t;
        } else {
            sand = 0.0;
            dirt = 0.0;
            rock = 1.0;
        }

        // Elevation influence (Mesa plateaus above 50m)
        if elevation > 50.0 {
            let elevation_factor = ((elevation - 50.0) / 30.0).clamp(0.0, 1.0);
            mesa = elevation_factor * (sand + dirt * 0.5);
            sand *= 1.0 - elevation_factor;
            dirt *= 1.0 - elevation_factor * 0.5;
        } else {
            mesa = 0.0;
        }

        // Normalize weights so sum == 1.0
        let total = sand + dirt + rock + mesa;
        if total > 1e-4 {
            let inv = 1.0 / total;
            Self {
                sand: sand * inv,
                dirt: dirt * inv,
                rock: rock * inv,
                mesa: mesa * inv,
            }
        } else {
            Self::default()
        }
    }
}

/// PBR material definition for a terrain ground layer.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainPbrMaterial {
    pub albedo: Vec3,
    pub roughness: f32,
    pub metallic: f32,
}

/// Set of 4 terrain material layers blended by splatting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainMaterialPalette {
    pub sand: TerrainPbrMaterial,
    pub dirt: TerrainPbrMaterial,
    pub rock: TerrainPbrMaterial,
    pub mesa: TerrainPbrMaterial,
}

impl Default for TerrainMaterialPalette {
    fn default() -> Self {
        Self::tatooine()
    }
}

impl TerrainMaterialPalette {
    /// Authentic Tatooine desert palette: golden dune sand, canyon dirt, granite cliff rock, and red mesa.
    #[must_use]
    pub fn tatooine() -> Self {
        Self {
            sand: TerrainPbrMaterial {
                albedo: Vec3::new(0.85, 0.72, 0.50), // Golden dunes
                roughness: 0.9,
                metallic: 0.0,
            },
            dirt: TerrainPbrMaterial {
                albedo: Vec3::new(0.65, 0.48, 0.32), // Canyon floor gravel
                roughness: 0.85,
                metallic: 0.0,
            },
            rock: TerrainPbrMaterial {
                albedo: Vec3::new(0.45, 0.38, 0.30), // Steep rock cliffs
                roughness: 0.75,
                metallic: 0.05,
            },
            mesa: TerrainPbrMaterial {
                albedo: Vec3::new(0.70, 0.35, 0.22), // Red sandstone mesa
                roughness: 0.8,
                metallic: 0.0,
            },
        }
    }

    /// Blends PBR parameters using splat weights.
    #[must_use]
    pub fn evaluate_pbr(&self, weights: &SplatWeights) -> TerrainPbrMaterial {
        let albedo = self.sand.albedo * weights.sand
            + self.dirt.albedo * weights.dirt
            + self.rock.albedo * weights.rock
            + self.mesa.albedo * weights.mesa;

        let roughness = self.sand.roughness * weights.sand
            + self.dirt.roughness * weights.dirt
            + self.rock.roughness * weights.rock
            + self.mesa.roughness * weights.mesa;

        let metallic = self.sand.metallic * weights.sand
            + self.dirt.metallic * weights.dirt
            + self.rock.metallic * weights.rock
            + self.mesa.metallic * weights.mesa;

        TerrainPbrMaterial {
            albedo,
            roughness,
            metallic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_sand_splatting() {
        let norm_flat = Vec3::new(0.0, 1.0, 0.0);
        let weights = SplatWeights::compute(norm_flat, 10.0);

        assert!((weights.sand - 1.0).abs() < 1e-3);
        assert_eq!(weights.rock, 0.0);
        assert_eq!(weights.mesa, 0.0);

        let palette = TerrainMaterialPalette::tatooine();
        let pbr = palette.evaluate_pbr(&weights);
        assert!((pbr.albedo - palette.sand.albedo).length() < 1e-3);
    }

    #[test]
    fn test_steep_cliff_splatting() {
        let norm_cliff = Vec3::new(0.9, 0.1, 0.0).normalize();
        let weights = SplatWeights::compute(norm_cliff, 10.0);

        assert!(weights.rock > 0.9, "Steep slope must be dominated by rock");
        assert!(weights.sand < 0.1);
    }

    #[test]
    fn test_mesa_elevation_splatting() {
        let norm_flat = Vec3::new(0.0, 1.0, 0.0);
        let weights = SplatWeights::compute(norm_flat, 85.0); // High plateau

        assert!(
            weights.mesa > 0.8,
            "High flat elevation should blend red mesa"
        );
    }
}
