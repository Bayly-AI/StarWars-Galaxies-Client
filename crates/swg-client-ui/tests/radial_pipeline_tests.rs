// crates/swg-client-ui/tests/radial_pipeline_tests.rs

use glam::{Mat4, Vec2, Vec3};
use swg_client_ui::{RadialMenuGenerator, Ray3D, TargetKind};

#[test]
fn test_3d_cursor_world_picking_to_radial_menu() {
    let proj = Mat4::perspective_rh(1.0, 1.777, 0.1, 1000.0);
    let view = Mat4::look_at_rh(
        Vec3::new(0.0, 3.0, 10.0), // Camera elevated at (0, 3, 10)
        Vec3::new(0.0, 0.0, 0.0),  // Looking down at Dewback corpse
        Vec3::Y,
    );
    let inv_vp = (proj * view).inverse();

    let screen_dims = Vec2::new(1920.0, 1080.0);
    let mouse_click_px = Vec2::new(960.0, 540.0); // Right-click at center of screen

    // 1. Unproject mouse cursor into 3D world ray
    let ray = Ray3D::unproject_mouse(mouse_click_px, screen_dims, inv_vp);

    // Dewback corpse bounding sphere at world origin (0, 0, 0) radius 2.0m
    let corpse_center = Vec3::ZERO;
    let corpse_radius = 2.0;

    let hit_dist = ray.intersects_sphere(corpse_center, corpse_radius);
    assert!(hit_dist.is_some(), "Raycast must intersect Dewback corpse");

    // 2. Spawn context-sensitive radial menu centered at mouse click
    let dewback_net_id = 900_001;
    let menu =
        RadialMenuGenerator::build_menu(dewback_net_id, TargetKind::CreatureCorpse, mouse_click_px);

    assert_eq!(menu.target_id, dewback_net_id);
    assert_eq!(menu.options.len(), 5);
    assert_eq!(menu.options[0].command, "/loot");
    assert_eq!(menu.options[1].command, "/harvest meat");
    assert_eq!(menu.options[2].command, "/harvest hide");

    // 3. Select default option (clicking at center)
    let default_sel = menu.select_by_mouse(mouse_click_px);
    assert_eq!(default_sel.map(|o| o.command.as_str()), Some("/loot"));

    // 4. Select by keyboard hotkey (key 2 -> Harvest Meat)
    let hotkey_sel = menu.select_by_hotkey(2);
    assert_eq!(
        hotkey_sel.map(|o| o.command.as_str()),
        Some("/harvest meat")
    );
}

#[test]
fn test_storage_container_and_player_interaction_menus() {
    let click_pos = Vec2::new(400.0, 300.0);

    // Storage Chest
    let chest_menu =
        RadialMenuGenerator::build_menu(800_002, TargetKind::StorageContainer, click_pos);
    assert_eq!(chest_menu.options[0].command, "/openContainer");
    assert_eq!(chest_menu.options[1].command, "/pickup");

    // Friendly Player
    let player_menu =
        RadialMenuGenerator::build_menu(800_003, TargetKind::PlayerCharacter, click_pos);
    assert_eq!(player_menu.options[0].command, "/trade");
    assert_eq!(player_menu.options[1].command, "/invite");
    assert_eq!(player_menu.options[2].command, "/follow");
}
