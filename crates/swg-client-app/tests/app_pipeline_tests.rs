// crates/swg-client-app/tests/app_pipeline_tests.rs

use swg_client_app::{ClientConfig, SwgClientApp};
use swg_client_audio::ThemeType;
use swg_client_customization::{Gender, Species};
use swg_client_vfs::{AssetCategory, TreArchiveWriter};
use tempfile::tempdir;

#[test]
fn test_end_to_end_client_startup_and_vfs_ingestion() {
    let dir = tempdir().unwrap();
    let tre1 = dir.path().join("data_sku1_00.tre");
    let patch = dir.path().join("patch_14_00.tre");

    // Base archive
    let mut base_w = TreArchiveWriter::new();
    base_w.add_file("terrain/tatooine.trn", b"TATOOINE_TERRAIN".to_vec(), false);
    base_w.add_file("texture/ground.dds", b"GROUND_DDS".to_vec(), false);
    base_w.add_file("appearance/human_m.skm", b"SKM_DATA".to_vec(), false);
    base_w.add_file("ui/hud.dds", b"BASE_HUD".to_vec(), false);
    base_w.write_to_file(&tre1).unwrap();

    // Patch archive (overrides HUD)
    let mut patch_w = TreArchiveWriter::new();
    patch_w.add_file("ui/hud.dds", b"PATCH_14_HUD".to_vec(), false);
    patch_w.add_file("sound/wep_blaster_e11_fire.wav", b"AUDIO_DATA".to_vec(), false);
    patch_w.write_to_file(&patch).unwrap();

    let mut config = ClientConfig::default();
    config.headless = true;
    config.asset_paths = vec![dir.path().to_path_buf()];
    config.character_name = "Wedge Antilles".to_string();
    config.species = Species::Human;
    config.gender = Gender::Male;
    config.planet = "tatooine".to_string();

    let mut app = SwgClientApp::new(config);
    let mounted = app.initialize_vfs().expect("Failed to initialize VFS");

    assert_eq!(mounted, 2);
    assert_eq!(app.manifest.total_files, 6); // 6 total entries across both archives
    assert_eq!(app.vfs.file_count(), 5); // 5 unique deduped across VFS
    assert_eq!(app.manifest.get_count(AssetCategory::Terrain), 1);
    assert_eq!(app.manifest.get_count(AssetCategory::Texture), 1);
    assert_eq!(app.manifest.get_count(AssetCategory::Mesh), 1);
    assert_eq!(app.manifest.get_count(AssetCategory::UserInterface), 2); // 1 in base + 1 in patch
    assert_eq!(app.manifest.get_count(AssetCategory::Audio), 1);

    // Verify patch precedence via VFS
    let hud_bytes = app.vfs.read_file("ui/hud.dds").unwrap();
    assert_eq!(hud_bytes.as_ref(), b"PATCH_14_HUD");

    // Step simulation frames
    for _ in 0..20 {
        let stats = app.step_frame();
        assert!(stats.fps > 0.0);
        assert!(stats.ui_vertices_count > 0);
    }
}

#[test]
fn test_client_character_skins_and_hud_pipeline() {
    let mut config = ClientConfig::default();
    config.headless = true;
    config.species = Species::Twilek;
    config.gender = Gender::Female;
    config.character_name = "Aayla Secura".to_string();

    let mut app = SwgClientApp::new(config);
    assert_eq!(app.player.species, Species::Twilek);
    assert_eq!(app.player.gender, Gender::Female);

    // Lethan Red skin customization
    app.player.set_color(
        swg_client_customization::VAR_SKIN_COLOR,
        "palette/twilek_skin.pal",
        1,
    );
    let skin_tint = app
        .player
        .resolve_tint(swg_client_customization::VAR_SKIN_COLOR, &app.palettes)
        .unwrap();
    assert_eq!(skin_tint, [210, 50, 45, 255]); // Lethan Red

    // Engage in combat and take damage
    app.music.enter_combat(0.85); // High threat -> BossFight
    app.music.tick(1.5); // Complete crossfade transition
    app.ham.health.apply_damage(300);
    app.ham.action.apply_wound(100);

    let stats = app.step_frame();
    assert_eq!(stats.active_music_theme, ThemeType::BossFight);
    assert_eq!(
        stats.active_music_track,
        "music/mus_player_v_player_b_lp.mp3"
    );
    assert_eq!(app.ham.health.current, 900);
    assert_eq!(app.ham.action.wound, 100);
    assert!(stats.ui_vertices_count > 10);
}

#[test]
fn test_retail_tre_ingestion_and_dds_decoding() {
    let tre_path = std::path::Path::new("../../../downloads/tre_test/SWG_Files/default_patch.tre");
    if !tre_path.exists() {
        println!("Skipping retail TRE test: archive not present");
        return;
    }

    let archive = swg_client_vfs::TreArchive::open(tre_path).expect("Failed to open retail TRE");
    let entry = archive.get_entry("texture/defaultshader.dds").expect("Entry not found");
    let raw_dds = archive.read_entry(entry).expect("Failed to read DDS file");

    let image =
        swg_client_render::dds::DdsImage::parse(&raw_dds).expect("Failed to parse DDS image");
    assert_eq!(image.width, 128);
    assert_eq!(image.height, 128);
    assert_eq!(image.format, swg_client_render::dds::DdsFormat::Dxt3);
    assert_eq!(image.mipmap_count, 8);

    // Decompress top mip level to RGBA8
    let rgba = image.decode_to_rgba8().expect("Failed to decompress DXT3 to RGBA8");
    assert_eq!(rgba.len(), 128 * 128 * 4);
}

#[test]
fn test_client_retail_audio_and_soundtrack_pipeline() {
    let tre_path = std::path::Path::new("../../../downloads/tre_test/SWG_Files/data_sample_03.tre");
    if !tre_path.exists() {
        println!("Skipping retail audio pipeline test: archive not present");
        return;
    }

    let mut config = ClientConfig::default();
    config.headless = true;
    config.asset_paths = vec![std::path::PathBuf::from(
        "../../../downloads/tre_test/SWG_Files",
    )];

    let mut app = SwgClientApp::new(config);
    let mounted = app.initialize_vfs().expect("Failed to initialize VFS");
    assert!(mounted > 0);

    // Read blaster fire sample via client VFS
    let blaster_bytes = app
        .vfs
        .read_file("sample/wep_blaster_fire_2.wav")
        .expect("Blaster WAV not found in client VFS");
    let clip = std::sync::Arc::new(
        swg_client_audio::WavClip::parse(&blaster_bytes).expect("Failed to parse blaster WAV"),
    );

    // Play 3D blaster sound in client mixer
    let emitter = swg_client_audio::AudioEmitter::new(
        101,
        glam::Vec3::new(10.0, 0.0, 0.0),
        5.0,
        150.0,
    );
    app.mixer.play_3d(clip, emitter, 1.0, false);

    let stats = app.step_frame();
    assert_eq!(stats.audio_active_voices, 1);
    assert_eq!(
        stats.active_music_track,
        "music/mus_theme_corellia.mp3"
    );
}
