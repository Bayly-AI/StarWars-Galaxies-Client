// crates/swg-client-mesh/src/material.rs

use glam::{Vec3, Vec4};
use serde::{Deserialize, Serialize};

/// Modern PBR material properties translated from legacy SWG shaders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PbrMaterial {
    pub name: String,
    pub base_color: Vec4,
    pub roughness: f32,
    pub metallic: f32,
    pub emissive: Vec3,
    pub emissive_intensity: f32,
    pub diffuse_map: Option<String>,
    pub normal_map: Option<String>,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            name: "default_material".to_string(),
            base_color: Vec4::ONE,
            roughness: 0.5,
            metallic: 0.0,
            emissive: Vec3::ZERO,
            emissive_intensity: 0.0,
            diffuse_map: None,
            normal_map: None,
        }
    }
}

impl PbrMaterial {
    /// Translates legacy SWG shader parameters (diffuse, specular exponent, metallic flag, emissive) into modern PBR.
    #[must_use]
    pub fn from_legacy_parameters(
        name: &str,
        diffuse: Vec4,
        gloss_factor: f32,
        is_metallic: bool,
        emissive_color: Option<Vec3>,
    ) -> Self {
        // Modern roughness conversion from legacy gloss:
        // High gloss (1.0) -> low roughness (~0.04)
        // Matte / low gloss (0.0) -> high roughness (1.0)
        let roughness = (1.0 - gloss_factor.clamp(0.0, 1.0)).sqrt().clamp(0.04, 1.0);
        let metallic = if is_metallic { 0.95 } else { 0.0 };

        let (emissive, emissive_intensity) = if let Some(c) = emissive_color {
            let max_c = c.max_element();
            if max_c > 1.0 {
                (c / max_c, max_c)
            } else {
                (c, 1.0)
            }
        } else {
            (Vec3::ZERO, 0.0)
        };

        Self {
            name: name.to_string(),
            base_color: diffuse,
            roughness,
            metallic,
            emissive,
            emissive_intensity,
            diffuse_map: None,
            normal_map: None,
        }
    }

    /// Lightsaber blade material preset.
    #[must_use]
    pub fn lightsaber_blade(color: Vec3) -> Self {
        Self {
            name: "lightsaber_blade".to_string(),
            base_color: Vec4::new(1.0, 1.0, 1.0, 1.0), // Core white
            roughness: 0.1,
            metallic: 0.0,
            emissive: color,
            emissive_intensity: 50.0,
            diffuse_map: None,
            normal_map: None,
        }
    }

    /// Starship metal hull preset (e.g. X-Wing or TIE Fighter).
    #[must_use]
    pub fn starship_hull() -> Self {
        Self::from_legacy_parameters(
            "starship_hull",
            Vec4::new(0.85, 0.85, 0.88, 1.0),
            0.75, // Moderate gloss
            true, // Metal
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_material_conversion() {
        let mat = PbrMaterial::from_legacy_parameters(
            "blaster_casing",
            Vec4::new(0.2, 0.2, 0.2, 1.0),
            0.8, // High gloss
            true,
            None,
        );

        assert!(mat.roughness < 0.5, "High gloss must produce low roughness");
        assert_eq!(mat.metallic, 0.95);
        assert_eq!(mat.emissive_intensity, 0.0);
    }

    #[test]
    fn test_lightsaber_blade_emissive() {
        let blue = Vec3::new(0.1, 0.3, 1.0);
        let mat = PbrMaterial::lightsaber_blade(blue);
        assert_eq!(mat.emissive, blue);
        assert_eq!(mat.emissive_intensity, 50.0);
    }
}
