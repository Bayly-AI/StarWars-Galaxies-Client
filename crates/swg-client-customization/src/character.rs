// crates/swg-client-customization/src/character.rs

use crate::morph::MorphSliderState;
use crate::palette::{CustomizationVariable, PaletteLibrary};
use crate::species::{Gender, Species, SpeciesProfile, SpeciesRegistry};
use glam::Vec4;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard customization variable paths.
pub const VAR_SKIN_COLOR: &str = "/shared_owner/index_color_skin";
pub const VAR_HAIR_COLOR: &str = "/shared_owner/index_color_hair";
pub const VAR_EYE_COLOR: &str = "/shared_owner/index_color_eyes";
pub const VAR_CLOTH_PRIMARY: &str = "/shared_owner/index_color_cloth_primary";
pub const VAR_CLOTH_SECONDARY: &str = "/shared_owner/index_color_cloth_secondary";

/// Complete customized appearance configuration for a player or NPC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterAppearance {
    pub species: Species,
    pub gender: Gender,
    pub scale: f32,
    pub morphs: MorphSliderState,
    pub customization_vars: HashMap<String, CustomizationVariable>,
}

impl CharacterAppearance {
    #[must_use]
    pub fn new(species: Species, gender: Gender) -> Self {
        let profile = SpeciesRegistry::get_profile(species, gender);
        let mut appearance = Self {
            species,
            gender,
            scale: 1.0,
            morphs: MorphSliderState::new(),
            customization_vars: HashMap::new(),
        };

        // Initialize default skin, hair, and eye palette variables
        appearance.set_color(VAR_SKIN_COLOR, &profile.default_skin_palette, 0);
        if let Some(hair_pal) = &profile.default_hair_palette {
            appearance.set_color(VAR_HAIR_COLOR, hair_pal, 0);
        }
        appearance.set_color(VAR_EYE_COLOR, &profile.default_eye_palette, 0);

        appearance
    }

    #[must_use]
    pub fn profile(&self) -> SpeciesProfile {
        SpeciesRegistry::get_profile(self.species, self.gender)
    }

    pub fn set_morph(&mut self, slider_name: impl Into<String>, value: f32) {
        self.morphs.set_slider(slider_name, value);
    }

    pub fn set_color(
        &mut self,
        var_path: impl Into<String>,
        palette_path: impl Into<String>,
        color_index: u16,
    ) {
        let path = var_path.into();
        self.customization_vars.insert(
            path.clone(),
            CustomizationVariable {
                path,
                palette_path: palette_path.into(),
                color_index,
            },
        );
    }

    pub fn set_scale(&mut self, scale: f32) {
        let profile = self.profile();
        self.scale = scale.clamp(profile.min_height_scale, profile.max_height_scale);
    }

    /// Resolves the RGBA color bytes for a customization variable.
    #[must_use]
    pub fn resolve_tint(&self, var_name: &str, lib: &PaletteLibrary) -> Option<[u8; 4]> {
        let var = self.customization_vars.get(var_name)?;
        lib.sample_color(&var.palette_path, var.color_index as usize)
    }

    /// Resolves normalized Vec4 RGBA tint for a customization variable.
    #[must_use]
    pub fn resolve_tint_vec4(&self, var_name: &str, lib: &PaletteLibrary) -> Option<Vec4> {
        let [r, g, b, a] = self.resolve_tint(var_name, lib)?;
        Some(Vec4::new(
            (r as f32) / 255.0,
            (g as f32) / 255.0,
            (b as f32) / 255.0,
            (a as f32) / 255.0,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_appearance_defaults_and_tint_resolution() {
        let lib = PaletteLibrary::new();
        let mut appearance = CharacterAppearance::new(Species::Twilek, Gender::Female);

        // Twilek skin index 1 is Lethan Red ([210, 50, 45, 255])
        appearance.set_color(VAR_SKIN_COLOR, "palette/twilek_skin.pal", 1);

        let tint = appearance.resolve_tint(VAR_SKIN_COLOR, &lib).unwrap();
        assert_eq!(tint, [210, 50, 45, 255]);

        let v4 = appearance.resolve_tint_vec4(VAR_SKIN_COLOR, &lib).unwrap();
        assert!((v4.x - (210.0 / 255.0)).abs() < 0.001);
    }

    #[test]
    fn test_character_scale_clamping() {
        let mut appearance = CharacterAppearance::new(Species::Wookiee, Gender::Male);
        let profile = appearance.profile();

        appearance.set_scale(0.5); // Below min
        assert_eq!(appearance.scale, profile.min_height_scale);

        appearance.set_scale(2.0); // Above max
        assert_eq!(appearance.scale, profile.max_height_scale);
    }
}
