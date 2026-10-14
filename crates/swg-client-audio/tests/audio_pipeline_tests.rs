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

#[test]
fn test_retail_wav_audio_decoding() {
    let tre_path = std::path::Path::new("../../../downloads/tre_test/SWG_Files/data_sample_03.tre");
    if !tre_path.exists() {
        println!("Skipping retail WAV test: archive not present at {:?}", tre_path);
        return;
    }

    let archive = swg_client_vfs::TreArchive::open(tre_path).expect("Failed to open sample TRE");
    let entry = archive
        .get_entry("sample/wep_blaster_fire_2.wav")
        .expect("Blaster WAV entry not found");
    let data = archive.read_entry(entry).expect("Failed to read WAV entry");

    let clip = swg_client_audio::WavClip::parse(&data).expect("Failed to decode WAV file");
    assert_eq!(clip.channels, 1);
    assert_eq!(clip.sample_rate, 22050);
    assert_eq!(clip.bits_per_sample, 16);
    assert!(clip.duration_seconds > 1.5 && clip.duration_seconds < 2.0);
    assert!(clip.samples.len() > 30000);

    // Verify non-silent samples
    let max_amp = clip.samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(max_amp > 0.5, "Blaster sample should have loud peak amplitude");
}

#[test]
fn test_retail_snd_template_parsing() {
    let tre_path = std::path::Path::new("../../../downloads/tre_test/SWG_Files/patch_00.tre");
    if !tre_path.exists() {
        println!("Skipping retail SND test: archive not present at {:?}", tre_path);
        return;
    }

    let archive = swg_client_vfs::TreArchive::open(tre_path).expect("Failed to open patch TRE");

    // 1. UI 2D sound template
    let entry_2d = archive
        .get_entry("sound/ui_select_rotate.snd")
        .expect("UI rotate SND entry not found");
    let data_2d = archive.read_entry(entry_2d).expect("Failed to read 2D SND entry");
    let template_2d =
        swg_client_audio::SoundTemplate::parse(&data_2d).expect("Failed to parse 2D SND");
    assert!(!template_2d.is_3d);
    assert_eq!(
        template_2d.primary_sample(),
        Some("sample/ui_select_rotate.wav")
    );

    // 2. Container creak 3D sound template
    let entry_3d = archive
        .get_entry("sound/item_creak_close.snd")
        .expect("Creak close SND entry not found");
    let data_3d = archive.read_entry(entry_3d).expect("Failed to read 3D SND entry");
    let template_3d =
        swg_client_audio::SoundTemplate::parse(&data_3d).expect("Failed to parse 3D SND");
    assert!(template_3d.is_3d);
    assert_eq!(
        template_3d.primary_sample(),
        Some("sample/item_creak_close.wav")
    );
}

#[test]
fn test_retail_mp3_soundtrack_parsing() {
    let tre_path = std::path::Path::new("../../../downloads/tre_test/SWG_Files/data_music_00.tre");
    if !tre_path.exists() {
        println!("Skipping retail MP3 test: archive not present at {:?}", tre_path);
        return;
    }

    let archive = swg_client_vfs::TreArchive::open(tre_path).expect("Failed to open music TRE");

    // 1. Combat skirmish loop theme
    let entry_combat = archive
        .get_entry("music/mus_combat_b_lp.mp3")
        .expect("Combat MP3 entry not found");
    let data_combat = archive
        .read_entry(entry_combat)
        .expect("Failed to read combat MP3 entry");

    let track_combat = swg_client_audio::Mp3Track::parse("mus_combat_b_lp.mp3", &data_combat)
        .expect("Failed to parse combat MP3 track");
    assert_eq!(track_combat.sample_rate, 44100);
    assert_eq!(track_combat.channels, 2);
    assert!(track_combat.duration_seconds > 20.0);

    // 2. Cantina band performance theme (Max Rebo Song 1)
    let entry_cantina = archive
        .get_entry("music/mus_max_rebo_song_1.mp3")
        .expect("Cantina MP3 entry not found");
    let data_cantina = archive
        .read_entry(entry_cantina)
        .expect("Failed to read cantina MP3 entry");

    let track_cantina = swg_client_audio::Mp3Track::parse("mus_max_rebo_song_1.mp3", &data_cantina)
        .expect("Failed to parse cantina MP3 track");
    assert_eq!(track_cantina.sample_rate, 44100);
    assert!(track_cantina.duration_seconds > 60.0);
}

#[test]
fn test_retail_blaster_3d_spatial_mixing() {
    let tre_path = std::path::Path::new("../../../downloads/tre_test/SWG_Files/data_sample_03.tre");
    if !tre_path.exists() {
        println!("Skipping retail spatial mixing test: archive not present");
        return;
    }

    let archive = swg_client_vfs::TreArchive::open(tre_path).expect("Failed to open sample TRE");
    let entry = archive
        .get_entry("sample/wep_blaster_fire_2.wav")
        .expect("Blaster WAV not found");
    let data = archive.read_entry(entry).expect("Failed to read blaster WAV");
    let clip = std::sync::Arc::new(
        swg_client_audio::WavClip::parse(&data).expect("Failed to decode blaster WAV"),
    );

    let mut mixer = swg_client_audio::AudioMixer::new(22050);
    // Listener at origin facing north (-Z)
    mixer.set_listener(swg_client_audio::AudioListener::new(
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::Y,
    ));

    // Blaster fired 15m to the right (+X)
    let emitter = swg_client_audio::AudioEmitter::new(42, Vec3::new(15.0, 0.0, 0.0), 5.0, 150.0);
    mixer.play_3d(clip, emitter, 1.0, false);

    assert_eq!(mixer.active_voice_count(), 1);

    // Render 1024 stereo frames
    let frames = mixer.render_stereo_frames(1024);
    assert_eq!(frames.len(), 2048);

    // Sum left and right channel energies
    let mut energy_l = 0.0f32;
    let mut energy_r = 0.0f32;
    for i in 0..1024 {
        energy_l += frames[i * 2].abs();
        energy_r += frames[i * 2 + 1].abs();
    }

    assert!(
        energy_r > energy_l * 2.0,
        "Right ear should hear blaster on right (+X) significantly louder (L={energy_l}, R={energy_r})"
    );
}
