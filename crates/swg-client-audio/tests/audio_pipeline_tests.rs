// crates/swg-client-audio/tests/audio_pipeline_tests.rs

use glam::Vec3;
use swg_client_audio::{
    AudioEmitter, AudioListener, DynamicMusicDirector, SoundBank, SoundId, SpatialAudioEvaluator,
    SurfaceType, ThemeType,
};

#[test]
fn test_3d_spatial_audio_surround_panning() {
    let listener = AudioListener::new(
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, -1.0), // Facing North (-Z)
        Vec3::Y,
    );

    // 1. Bith band playing to the East / Right at (15, 0, 0)
    let (min_d, max_d) = SoundBank::default_distances(SoundId::CantinaBandPerformance);
    let band_emitter = AudioEmitter::new(1, Vec3::new(15.0, 0.0, 0.0), min_d, max_d);
    let band_gains = SpatialAudioEvaluator::evaluate_spatial(&band_emitter, &listener);

    assert!(
        band_gains.right > band_gains.left * 3.0,
        "Right channel must dominate for East emitter"
    );
    let (stereo_l, stereo_r) = band_gains.stereo_pair();
    assert!(stereo_r > stereo_l * 2.0);

    // 2. Stormtrooper firing E-11 to the Front-Left at (-20, 0, -20)
    let (blaster_min, blaster_max) = SoundBank::default_distances(SoundId::BlasterE11Fire);
    let blaster_emitter =
        AudioEmitter::new(2, Vec3::new(-20.0, 0.0, -20.0), blaster_min, blaster_max);
    let blaster_gains = SpatialAudioEvaluator::evaluate_spatial(&blaster_emitter, &listener);

    assert!(
        blaster_gains.left > blaster_gains.right * 2.0,
        "Left channel must dominate for West emitter"
    );
    assert!(
        blaster_gains.left > blaster_gains.back_left,
        "Front left should be louder than rear left"
    );

    // 3. Ambush from directly behind at (0, 0, 25)
    let ambush_emitter = AudioEmitter::new(3, Vec3::new(0.0, 0.0, 25.0), 5.0, 100.0);
    let ambush_gains = SpatialAudioEvaluator::evaluate_spatial(&ambush_emitter, &listener);

    let rear_total = ambush_gains.back_left + ambush_gains.back_right;
    let front_total = ambush_gains.left + ambush_gains.right;
    assert!(
        rear_total > front_total * 2.0,
        "Rear channels must dominate for sounds directly behind listener"
    );
}

#[test]
fn test_dynamic_interactive_soundtrack_state_machine() {
    let mut director = DynamicMusicDirector::new();
    assert_eq!(director.current_theme, ThemeType::PlanetaryAmbient);
    assert_eq!(director.get_volume(ThemeType::PlanetaryAmbient), 1.0);

    // 1. Skirmish attack (Threat 0.4)
    director.enter_combat(0.4);
    assert_eq!(director.target_theme, Some(ThemeType::CombatSkirmish));

    // Cross-fade for 1.5s
    director.tick(1.5);
    assert_eq!(director.current_theme, ThemeType::CombatSkirmish);
    assert_eq!(director.get_volume(ThemeType::CombatSkirmish), 1.0);
    assert_eq!(director.get_volume(ThemeType::PlanetaryAmbient), 0.0);

    // 2. Escalate to Boss Fight (Krayt Dragon, Threat 0.95)
    director.enter_combat(0.95);
    assert_eq!(director.target_theme, Some(ThemeType::BossFight));
    director.tick(1.5);
    assert_eq!(director.current_theme, ThemeType::BossFight);
    assert_eq!(director.get_volume(ThemeType::BossFight), 1.0);

    // 3. Victorious completion -> Victory Fanfare
    director.transition_to(ThemeType::VictoryFanfare);
    director.tick(1.5);
    assert_eq!(director.current_theme, ThemeType::VictoryFanfare);
    assert_eq!(director.get_volume(ThemeType::VictoryFanfare), 1.0);

    // 4. Return to peaceful planetary exploration
    director.exit_combat();
    director.tick(1.5);
    assert_eq!(director.current_theme, ThemeType::PlanetaryAmbient);
    assert_eq!(director.get_volume(ThemeType::PlanetaryAmbient), 1.0);
}

#[test]
fn test_terrain_footstep_sound_resolution() {
    assert_eq!(
        SoundBank::resolve_asset_path(SoundId::Footstep(SurfaceType::Sand)),
        "sound/fs_sand_walk_01.wav"
    );
    assert_eq!(
        SoundBank::resolve_asset_path(SoundId::Footstep(SurfaceType::Rock)),
        "sound/fs_rock_walk_01.wav"
    );
    assert_eq!(
        SoundBank::resolve_asset_path(SoundId::Footstep(SurfaceType::Metal)),
        "sound/fs_metal_walk_01.wav"
    );
}
