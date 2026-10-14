// crates/swg-client-interior/tests/interior_pipeline_tests.rs

use glam::{Mat4, Vec3};
use std::f32::consts::FRAC_PI_4;
use swg_client_interior::{
    BuildingInterior, CellObject, CellTransitionManager, DoorState, Portal, PortalOcclusionCuller,
};
use swg_client_world::CameraFrustum;

fn create_cantina_building_interior() -> BuildingInterior {
    // Chalmun's Cantina placed at Mos Eisley canon coordinates rotated 45 degrees
    let cantina_pos = Vec3::new(3528.0, 5.0, -4804.0);
    let world_mat = Mat4::from_translation(cantina_pos) * Mat4::from_rotation_y(FRAC_PI_4);

    let mut cantina = BuildingInterior::new(500_001, world_mat);

    // Cell 1: Foyer (0..6 on Z)
    let mut foyer = CellObject::new(
        1,
        500_001,
        "cantina_foyer",
        Vec3::new(-4.0, 0.0, 0.0),
        Vec3::new(4.0, 4.0, 6.0),
    );
    foyer.add_object(700_001); // Droid scanner terminal
    cantina.add_cell(foyer);

    // Cell 2: Main Cantina Bar (6..25 on Z)
    let mut main_bar = CellObject::new(
        2,
        500_001,
        "cantina_main_bar",
        Vec3::new(-15.0, 0.0, 6.0),
        Vec3::new(15.0, 6.0, 25.0),
    );
    main_bar.add_object(700_002); // Central curved drink bar
    main_bar.add_object(700_003); // Bith band stage
    cantina.add_cell(main_bar);

    // Cell 3: Private Booth Room (25..35 on Z, -10..0 on X)
    let mut booth = CellObject::new(
        3,
        500_001,
        "cantina_private_booth",
        Vec3::new(-10.0, 0.0, 25.0),
        Vec3::new(0.0, 4.0, 35.0),
    );
    booth.add_object(700_004); // Han & Greedo table
    cantina.add_cell(booth);

    // Portal 1: Outdoors (0) <-> Foyer (1) at Z = 0
    cantina.add_portal(Portal::new(
        1,
        0,
        1,
        vec![
            Vec3::new(-1.2, 0.0, 0.0),
            Vec3::new(1.2, 0.0, 0.0),
            Vec3::new(1.2, 2.8, 0.0),
            Vec3::new(-1.2, 2.8, 0.0),
        ],
        DoorState::Open,
    ));

    // Portal 2: Foyer (1) <-> Main Bar (2) at Z = 6
    cantina.add_portal(Portal::new(
        2,
        1,
        2,
        vec![
            Vec3::new(-2.0, 0.0, 6.0),
            Vec3::new(2.0, 0.0, 6.0),
            Vec3::new(2.0, 3.5, 6.0),
            Vec3::new(-2.0, 3.5, 6.0),
        ],
        DoorState::Open,
    ));

    // Portal 3: Main Bar (2) <-> Private Booth (3) at Z = 25
    cantina.add_portal(Portal::new(
        3,
        2,
        3,
        vec![
            Vec3::new(-6.0, 0.0, 25.0),
            Vec3::new(-4.0, 0.0, 25.0),
            Vec3::new(-4.0, 2.5, 25.0),
            Vec3::new(-6.0, 2.5, 25.0),
        ],
        DoorState::Open,
    ));

    cantina
}

#[test]
fn test_cantina_interior_navigation_and_portal_occlusion() {
    let mut cantina = create_cantina_building_interior();
    let mut transition_mgr = CellTransitionManager::new(0);

    let frustum = CameraFrustum::from_view_projection(Mat4::IDENTITY);

    // 1. Player is outside Mos Eisley cantina
    let outside_pos = cantina.cell_to_world_point(Vec3::new(0.0, 0.0, -5.0));
    let cell = transition_mgr.update(&cantina, outside_pos);
    assert_eq!(cell, 0, "Player must be outdoors");

    let vis_outdoors =
        PortalOcclusionCuller::evaluate_visibility(&cantina, 0, outside_pos, &frustum);
    assert!(vis_outdoors.outdoor_terrain_visible);
    // Can see into foyer through open entrance portal
    assert!(vis_outdoors.visible_cells.contains(&1));

    // 2. Player walks through entrance into Foyer
    let foyer_pos = cantina.cell_to_world_point(Vec3::new(0.0, 1.8, 3.0));
    let cell = transition_mgr.update(&cantina, foyer_pos);
    assert_eq!(cell, 1, "Player should now be inside Foyer (cell 1)");

    // 3. Player advances into Main Bar
    let bar_pos = cantina.cell_to_world_point(Vec3::new(0.0, 1.8, 15.0));
    let cell = transition_mgr.update(&cantina, bar_pos);
    assert_eq!(cell, 2, "Player should be inside Main Bar (cell 2)");

    // 4. Close the cantina entrance door (blast door closed)
    cantina.portals[0].door_state = DoorState::Closed;

    let vis_inside_bar = PortalOcclusionCuller::evaluate_visibility(&cantina, 2, bar_pos, &frustum);
    assert!(vis_inside_bar.visible_cells.contains(&2));
    assert!(
        !vis_inside_bar.outdoor_terrain_visible,
        "When entrance door is closed, outdoor Tatooine terrain must be completely culled"
    );

    // 5. Player enters Han Solo private booth (cell 3) and closes booth door
    let booth_pos = cantina.cell_to_world_point(Vec3::new(-5.0, 1.8, 30.0));
    let cell = transition_mgr.update(&cantina, booth_pos);
    assert_eq!(cell, 3, "Player should be inside private booth (cell 3)");

    cantina.portals[2].door_state = DoorState::Closed; // Close booth privacy door

    let vis_booth = PortalOcclusionCuller::evaluate_visibility(&cantina, 3, booth_pos, &frustum);
    assert!(vis_booth.visible_cells.contains(&3));
    assert!(
        !vis_booth.visible_cells.contains(&2),
        "Main bar is culled when booth door is closed"
    );
    assert!(!vis_booth.outdoor_terrain_visible);
}
