// crates/swg-client-customization/src/species.rs

use serde::{Deserialize, Serialize};

/// The 8 playable canonical galactic species available in Pre-CU Star Wars Galaxies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Species {
    Human,
    Trandoshan,
    Wookiee,
    Twilek,
    Zabrak,
    Bothan,
    MonCalamari,
    Rodian,
}

impl Species {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Human => "Human",
            Self::Trandoshan => "Trandoshan",
            Self::Wookiee => "Wookiee",
            Self::Twilek => "Twi'lek",
            Self::Zabrak => "Zabrak",
            Self::Bothan => "Bothan",
            Self::MonCalamari => "Mon Calamari",
            Self::Rodian => "Rodian",
        }
    }
}

/// Biological gender determining skeletal template and mesh baselines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
}

impl Gender {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Male => "Male",
            Self::Female => "Female",
        }
    }
}

/// Species physical constraints, anatomical traits, and asset templates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeciesProfile {
    pub species: Species,
    pub gender: Gender,
    pub display_name: String,
    pub base_height_meters: f32,
    pub min_height_scale: f32,
    pub max_height_scale: f32,
    pub skeleton_path: String,
    pub default_skin_palette: String,
    pub default_hair_palette: Option<String>,
    pub default_eye_palette: String,
    pub has_hair: bool,
    pub has_horns: bool,
    pub has_lekku: bool,
    pub racial_traits: Vec<String>,
}

pub struct SpeciesRegistry;

impl SpeciesRegistry {
    pub const ALL_SPECIES: [Species; 8] = [
        Species::Human,
        Species::Trandoshan,
        Species::Wookiee,
        Species::Twilek,
        Species::Zabrak,
        Species::Bothan,
        Species::MonCalamari,
        Species::Rodian,
    ];

    pub const ALL_GENDERS: [Gender; 2] = [Gender::Male, Gender::Female];

    #[must_use]
    pub fn get_profile(species: Species, gender: Gender) -> SpeciesProfile {
        let gender_code = match gender {
            Gender::Male => "m",
            Gender::Female => "f",
        };

        match species {
            Species::Human => SpeciesProfile {
                species,
                gender,
                display_name: format!("Human {}", gender.name()),
                base_height_meters: if gender == Gender::Male { 1.80 } else { 1.68 },
                min_height_scale: 0.90,
                max_height_scale: 1.15,
                skeleton_path: format!("appearance/skeleton/all_{gender_code}.sat"),
                default_skin_palette: "palette/human_skin.pal".into(),
                default_hair_palette: Some("palette/human_hair.pal".into()),
                default_eye_palette: "palette/human_eyes.pal".into(),
                has_hair: true,
                has_horns: false,
                has_lekku: false,
                racial_traits: vec!["Adaptable".into(), "Leadership".into()],
            },
            Species::Trandoshan => SpeciesProfile {
                species,
                gender,
                display_name: format!("Trandoshan {}", gender.name()),
                base_height_meters: if gender == Gender::Male { 2.05 } else { 1.95 },
                min_height_scale: 0.92,
                max_height_scale: 1.20,
                skeleton_path: format!("appearance/skeleton/trando_{gender_code}.sat"),
                default_skin_palette: "palette/trando_scales.pal".into(),
                default_hair_palette: None, // Reptilian, no hair
                default_eye_palette: "palette/trando_eyes.pal".into(),
                has_hair: false,
                has_horns: false,
                has_lekku: false,
                racial_traits: vec!["Regeneration".into(), "Unarmed Damage".into()],
            },
            Species::Wookiee => SpeciesProfile {
                species,
                gender,
                display_name: format!("Wookiee {}", gender.name()),
                base_height_meters: if gender == Gender::Male { 2.25 } else { 2.10 },
                min_height_scale: 0.95,
                max_height_scale: 1.25,
                skeleton_path: format!("appearance/skeleton/wookiee_{gender_code}.sat"),
                default_skin_palette: "palette/wookiee_fur.pal".into(),
                default_hair_palette: None,
                default_eye_palette: "palette/wookiee_eyes.pal".into(),
                has_hair: false, // Entire body covered in fur
                has_horns: false,
                has_lekku: false,
                racial_traits: vec!["Wookiee Roar".into(), "High Strength".into()],
            },
            Species::Twilek => SpeciesProfile {
                species,
                gender,
                display_name: format!("Twi'lek {}", gender.name()),
                base_height_meters: if gender == Gender::Male { 1.82 } else { 1.70 },
                min_height_scale: 0.88,
                max_height_scale: 1.12,
                skeleton_path: format!("appearance/skeleton/twilek_{gender_code}.sat"),
                default_skin_palette: "palette/twilek_skin.pal".into(),
                default_hair_palette: None, // Lekku instead of hair
                default_eye_palette: "palette/twilek_eyes.pal".into(),
                has_hair: false,
                has_horns: false,
                has_lekku: true,
                racial_traits: vec!["Lekku Communication".into(), "Desert Resistance".into()],
            },
            Species::Zabrak => SpeciesProfile {
                species,
                gender,
                display_name: format!("Zabrak {}", gender.name()),
                base_height_meters: if gender == Gender::Male { 1.80 } else { 1.70 },
                min_height_scale: 0.90,
                max_height_scale: 1.15,
                skeleton_path: format!("appearance/skeleton/all_{gender_code}.sat"),
                default_skin_palette: "palette/zabrak_skin.pal".into(),
                default_hair_palette: Some("palette/human_hair.pal".into()),
                default_eye_palette: "palette/human_eyes.pal".into(),
                has_hair: true,
                has_horns: true,
                has_lekku: false,
                racial_traits: vec!["Equilibrium".into(), "Second Heart Vitality".into()],
            },
            Species::Bothan => SpeciesProfile {
                species,
                gender,
                display_name: format!("Bothan {}", gender.name()),
                base_height_meters: if gender == Gender::Male { 1.55 } else { 1.48 },
                min_height_scale: 0.85,
                max_height_scale: 1.10,
                skeleton_path: format!("appearance/skeleton/bothan_{gender_code}.sat"),
                default_skin_palette: "palette/bothan_fur.pal".into(),
                default_hair_palette: Some("palette/bothan_mane.pal".into()),
                default_eye_palette: "palette/bothan_eyes.pal".into(),
                has_hair: true,
                has_horns: false,
                has_lekku: false,
                racial_traits: vec!["Information Gathering".into(), "Agility".into()],
            },
            Species::MonCalamari => SpeciesProfile {
                species,
                gender,
                display_name: format!("Mon Calamari {}", gender.name()),
                base_height_meters: if gender == Gender::Male { 1.78 } else { 1.68 },
                min_height_scale: 0.90,
                max_height_scale: 1.15,
                skeleton_path: format!("appearance/skeleton/moncal_{gender_code}.sat"),
                default_skin_palette: "palette/moncal_skin.pal".into(),
                default_hair_palette: None,
                default_eye_palette: "palette/moncal_eyes.pal".into(),
                has_hair: false,
                has_horns: false,
                has_lekku: false,
                racial_traits: vec!["Aquatic Breathing".into(), "Advanced Engineering".into()],
            },
            Species::Rodian => SpeciesProfile {
                species,
                gender,
                display_name: format!("Rodian {}", gender.name()),
                base_height_meters: if gender == Gender::Male { 1.65 } else { 1.58 },
                min_height_scale: 0.88,
                max_height_scale: 1.12,
                skeleton_path: format!("appearance/skeleton/rodian_{gender_code}.sat"),
                default_skin_palette: "palette/rodian_skin.pal".into(),
                default_hair_palette: None,
                default_eye_palette: "palette/rodian_eyes.pal".into(),
                has_hair: false,
                has_horns: false,
                has_lekku: false,
                racial_traits: vec!["Infrared Vision".into(), "Tracking".into()],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_eight_species_profiles_exist() {
        for &species in &SpeciesRegistry::ALL_SPECIES {
            for &gender in &SpeciesRegistry::ALL_GENDERS {
                let profile = SpeciesRegistry::get_profile(species, gender);
                assert!(!profile.display_name.is_empty());
                assert!(profile.base_height_meters > 1.2 && profile.base_height_meters < 2.5);
                assert!(profile.min_height_scale < profile.max_height_scale);
                assert!(!profile.skeleton_path.is_empty());
                assert!(!profile.default_skin_palette.is_empty());
            }
        }
    }

    #[test]
    fn test_species_specific_anatomical_traits() {
        let twilek = SpeciesRegistry::get_profile(Species::Twilek, Gender::Female);
        assert!(twilek.has_lekku);
        assert!(!twilek.has_hair);

        let zabrak = SpeciesRegistry::get_profile(Species::Zabrak, Gender::Male);
        assert!(zabrak.has_horns);
        assert!(zabrak.has_hair);

        let wookiee = SpeciesRegistry::get_profile(Species::Wookiee, Gender::Male);
        assert!(wookiee.base_height_meters > 2.10);
    }
}
