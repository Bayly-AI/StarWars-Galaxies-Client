// crates/swg-client-customization/tests/customization_pipeline_tests.rs

use glam::Vec3;
use swg_client_customization::{
    BaseMesh, CharacterAppearance, ColorRamp, Gender, MORPH_EYE_SIZE, MORPH_HEIGHT, MORPH_JAW_LINE,
    MORPH_LEKKU_LENGTH, MORPH_MUSCLE, MORPH_WEIGHT, MorphEvaluator, MorphTarget, PaletteLibrary,
    Species, SpeciesRegistry, VAR_CLOTH_PRIMARY, VAR_CLOTH_SECONDARY, VAR_EYE_COLOR,
    VAR_HAIR_COLOR, VAR_SKIN_COLOR,
};

#[test]
fn test_species_and_gender_matrix_coverage() {
    for &species in &SpeciesRegistry::ALL_SPECIES {
        for &gender in &SpeciesRegistry::ALL_GENDERS {
            let profile = SpeciesRegistry::get_profile(species, gender);
            assert_eq!(profile.species, species);
            assert_eq!(profile.gender, gender);
            assert!(!profile.display_name.is_empty());
            assert!(profile.min_height_scale > 0.70 && profile.max_height_scale < 1.35);
            assert!(profile.base_height_meters > 1.30 && profile.base_height_meters < 2.40);
            assert!(profile.skeleton_path.ends_with(".sat"));
            assert!(profile.default_skin_palette.ends_with(".pal"));
        }
    }
}

#[test]
fn test_realtime_morph_slider_deformation_pipeline() {
    // Generate a cylindrical torso/head mesh with 24 vertices
    let mut base_positions = Vec::new();
    let mut base_normals = Vec::new();
    let mut indices = Vec::new();

    for i in 0..24 {
        let angle = (i as f32) * std::f32::consts::TAU / 24.0;
        let x = angle.cos();
        let z = angle.sin();
        let y = (i / 8) as f32 * 0.5;
        base_positions.push(Vec3::new(x, y, z));
        base_normals.push(Vec3::new(x, 0.0, z).normalize());
        indices.push(i as u32);
    }

    let base_mesh = BaseMesh::new(base_positions, base_normals, indices);

    // Create morph targets:
    // 1. Muscle: expands outward laterally
    let muscle_deltas: Vec<Vec3> = base_mesh
        .positions
        .iter()
        .map(|p| Vec3::new(p.x * 0.25, 0.0, p.z * 0.25))
        .collect();
    let muscle_normals: Vec<Vec3> = vec![Vec3::ZERO; 24];
    let muscle_target = MorphTarget::new(MORPH_MUSCLE, muscle_deltas, muscle_normals);

    // 2. Height: expands vertically
    let height_deltas: Vec<Vec3> = base_mesh
        .positions
        .iter()
        .map(|p| Vec3::new(0.0, p.y * 0.30, 0.0))
        .collect();
    let height_normals: Vec<Vec3> = vec![Vec3::ZERO; 24];
    let height_target = MorphTarget::new(MORPH_HEIGHT, height_deltas, height_normals);

    // 3. Lekku length: pulls top head vertices backwards and downwards
    let lekku_deltas: Vec<Vec3> = base_mesh
        .positions
        .iter()
        .enumerate()
        .map(|(idx, _p)| {
            if idx >= 16 {
                Vec3::new(0.0, -0.4, -0.3)
            } else {
                Vec3::ZERO
            }
        })
        .collect();
    let lekku_normals: Vec<Vec3> = vec![Vec3::ZERO; 24];
    let lekku_target = MorphTarget::new(MORPH_LEKKU_LENGTH, lekku_deltas, lekku_normals);

    let targets = vec![muscle_target, height_target, lekku_target];

    // Apply morph sliders: muscle = 0.8, height = 0.5, lekku = 1.0
    let mut sliders = std::collections::HashMap::new();
    sliders.insert(MORPH_MUSCLE.to_string(), 0.8);
    sliders.insert(MORPH_HEIGHT.to_string(), 0.5);
    sliders.insert(MORPH_LEKKU_LENGTH.to_string(), 1.0);

    let deformed = MorphEvaluator::evaluate(&base_mesh, &targets, &sliders);

    assert_eq!(deformed.positions.len(), 24);
    assert_eq!(deformed.normals.len(), 24);

    // Check that vertices expanded laterally due to muscle slider
    for i in 0..8 {
        let base_rad = (base_mesh.positions[i].x.powi(2) + base_mesh.positions[i].z.powi(2)).sqrt();
        let deformed_rad =
            (deformed.positions[i].x.powi(2) + deformed.positions[i].z.powi(2)).sqrt();
        assert!(
            deformed_rad > base_rad * 1.15,
            "Muscle morph must expand mesh laterally"
        );
    }

    // Check that top vertices elongated backwards due to lekku slider
    assert!(
        deformed.positions[20].z < base_mesh.positions[20].z - 0.2,
        "Lekku morph must pull head vertices backwards"
    );

    // Check normals are valid unit vectors without NaN or zero length
    for n in &deformed.normals {
        assert!(!n.is_nan());
        assert!((n.length() - 1.0).abs() < 0.01);
    }
}

#[test]
fn test_pal_color_ramp_tinting_and_clothing_dyes() {
    let lib = PaletteLibrary::new();
    let mut character = CharacterAppearance::new(Species::Twilek, Gender::Female);

    // Rutian blue skin = index 0
    character.set_color(VAR_SKIN_COLOR, "palette/twilek_skin.pal", 0);
    // Yellow eyes = index 4 in twilek_skin (used as accent) or index in eyes
    character.set_color(VAR_EYE_COLOR, "palette/human_eyes.pal", 0); // Blue eyes
    // Dye clothing: Corellian Navy Blue primary (index 0), Imperial Crimson secondary (index 1)
    character.set_color(VAR_CLOTH_PRIMARY, "palette/clothing_dyes.pal", 0);
    character.set_color(VAR_CLOTH_SECONDARY, "palette/clothing_dyes.pal", 1);

    let skin_tint = character
        .resolve_tint(VAR_SKIN_COLOR, &lib)
        .expect("Twilek skin tint must resolve");
    assert_eq!(skin_tint, [70, 140, 220, 255]); // Rutian Blue

    let cloth_prim = character
        .resolve_tint(VAR_CLOTH_PRIMARY, &lib)
        .expect("Cloth primary must resolve");
    assert_eq!(cloth_prim, [20, 50, 120, 255]); // Navy blue

    let cloth_sec = character
        .resolve_tint(VAR_CLOTH_SECONDARY, &lib)
        .expect("Cloth secondary must resolve");
    assert_eq!(cloth_sec, [180, 30, 30, 255]); // Imperial crimson
}

#[test]
fn test_appearance_serialization_and_deserialization() {
    let mut character = CharacterAppearance::new(Species::Zabrak, Gender::Male);
    character.set_morph(MORPH_HEIGHT, 0.7);
    character.set_morph(MORPH_WEIGHT, 0.4);
    character.set_morph(MORPH_MUSCLE, 0.85);
    character.set_morph(MORPH_JAW_LINE, 0.3);
    character.set_morph(MORPH_EYE_SIZE, 0.5);
    character.set_scale(1.08);

    let json = serde_json::to_string_pretty(&character)
        .expect("Character appearance must serialize to JSON");
    let roundtrip: CharacterAppearance =
        serde_json::from_str(&json).expect("Character appearance must deserialize from JSON");

    assert_eq!(roundtrip.species, Species::Zabrak);
    assert_eq!(roundtrip.gender, Gender::Male);
    assert_eq!(roundtrip.scale, 1.08);
    assert_eq!(roundtrip.morphs.get_slider(MORPH_MUSCLE), 0.85);
    assert_eq!(roundtrip.morphs.get_slider(MORPH_HEIGHT), 0.7);
}
