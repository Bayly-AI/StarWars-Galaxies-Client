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
    assert_eq!(app.ham.health.current, 900);
    assert_eq!(app.ham.action.wound, 100);
    assert!(stats.ui_vertices_count > 10);
}
