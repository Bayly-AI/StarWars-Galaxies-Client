// crates/swg-client-ui/tests/hud_pipeline_tests.rs

use glam::Vec3;
use std::f32::consts::PI;
use swg_client_ui::{
    BlipType, ButtonBar, CharacterHam, RadarBlip, RadarMinimap, ResolutionScaler, TargetStatus,
};

#[test]
fn test_hud_combat_damage_and_wound_pipeline() {
    let mut ham = CharacterHam::new(1200, 1000, 800);

    // Initial state: 100% full
    assert_eq!(ham.health.current_ratio(), 1.0);
    assert_eq!(ham.health.wound_ratio(), 0.0);
    assert!(!ham.is_incapacitated());

    // Server sends damage delta: -400 health, 150 health wounds (blaster rifle bolt)
    ham.health.apply_damage(400);
    ham.health.apply_wound(150);

    assert_eq!(ham.health.current, 800);
    assert_eq!(ham.health.wound, 150);
    assert_eq!(ham.health.effective_max(), 1050);
    assert!((ham.health.current_ratio() - (800.0 / 1200.0)).abs() < 1e-4);
    assert!((ham.health.wound_ratio() - (150.0 / 1200.0)).abs() < 1e-4);

    // Combat Medic applies Bacta stim healing 500 health
    ham.health.heal_damage(500);
    assert_eq!(ham.health.current, 1050, "Health healed up to wound cap");

    // Doctor treats wounds with medicine
    ham.health.heal_wound(150);
    assert_eq!(ham.health.wound, 0);
    assert_eq!(ham.health.effective_max(), 1200);

    // Catastrophic mind damage causes incapacitation
    ham.mind.apply_damage(800);
    assert!(ham.is_incapacitated());
}

#[test]
fn test_radar_and_target_status_tracking() {
    let player_pos = Vec3::new(3528.0, 5.0, -4804.0); // Mos Eisley cantina
    let mut radar = RadarMinimap::new(player_pos, 0.0, 256.0); // Facing North

    // Waypoint at Starport (East of cantina at 3670, 5, -4780)
    let starport_blip = RadarBlip {
        id: 1,
        world_pos: Vec3::new(3670.0, 5.0, -4780.0),
        blip_type: BlipType::Waypoint,
        label: "Mos Eisley Starport".to_string(),
    };
    radar.add_blip(starport_blip);

    // Distant waypoint 2,000m away in Jundland Wastes
    let wastes_blip = RadarBlip {
        id: 2,
        world_pos: Vec3::new(5500.0, 5.0, -4804.0),
        blip_type: BlipType::Waypoint,
        label: "Fort Tusken".to_string(),
    };
    radar.add_blip(wastes_blip);

    let radar_radius_px = 90.0;

    // 1. Facing North: Starport is to the right (+X, East)
    let (sp_screen, clamped) =
        radar.compute_blip_screenspace(radar.blips[0].world_pos, radar_radius_px);
    assert!(!clamped);
    assert!(sp_screen.x > 0.0, "Starport should be to the right (+X)");

    // Fort Tusken is way outside 256m -> clamped to perimeter circle
    let (tusken_screen, clamped_far) =
        radar.compute_blip_screenspace(radar.blips[1].world_pos, radar_radius_px);
    assert!(clamped_far);
    let dist_from_center = tusken_screen.length();
    assert!(
        (dist_from_center - radar_radius_px).abs() < 1e-3,
        "Off-radar blip must pin to perimeter"
    );

    // 2. Rotate player 90 degrees East (heading = PI/2)
    radar.heading = PI / 2.0;
    let (sp_screen_turned, _) =
        radar.compute_blip_screenspace(radar.blips[0].world_pos, radar_radius_px);
    // Looking directly at Starport -> now Starport is centered vertically at top of radar (-Y on screen)
    assert!(sp_screen_turned.y < 0.0);
}

#[test]
fn test_macro_hotkey_execution_and_target_vitals() {
    let mut bar = ButtonBar::new();
    bar.assign(0, "Headshot", "/headshot2", 4.0);

    let mut target = TargetStatus::new();
    target.set_target(501, "Tusken Raider", true, None);
    target.update_vitals(1.0, 1.0, 1.0, 25.0, "Standing");

    // Trigger F1 hotkey
    let executed = bar.press_slot(0);
    assert_eq!(executed.as_deref(), Some("/headshot2"));

    // Shot landed: update target status frame
    target.update_vitals(0.3, 0.9, 0.2, 25.0, "Kneeling");
    assert!((target.health_ratio - 0.3).abs() < 1e-4);
    assert_eq!(target.posture, "Kneeling");
}

#[test]
fn test_crisp_resolution_scaling_4k() {
    let scaler_4k = ResolutionScaler::new(3840.0, 2160.0);
    let ham_rect = scaler_4k.ham_bars_rect();
    let radar_rect = scaler_4k.radar_rect();
    let button_rect = scaler_4k.button_bar_rect();

    // Verify non-overlapping layout
    assert!(ham_rect.x + ham_rect.width < radar_rect.x);
    assert!(ham_rect.y + ham_rect.height < button_rect.y);
    assert_eq!(scaler_4k.scale_factor, 2.0);
}
