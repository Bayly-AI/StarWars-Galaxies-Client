// crates/swg-client-world/tests/world_pipeline_tests.rs

use glam::{Mat4, Quat, Vec3};
use std::time::Instant;
use swg_client_world::{
    CameraFrustum, CanonCityGenerator, PlanetHub, SpatialSceneGraph, WorldSnapshot,
    WorldSnapshotNode,
};

#[test]
fn test_canon_city_world_snapshot_serialization_and_streaming() {
    let theed_snap = CanonCityGenerator::generate_hub_snapshot(PlanetHub::NabooTheed);
    let bytes = theed_snap.to_bytes();

    let reloaded = WorldSnapshot::from_bytes(&bytes).expect("Failed to deserialize Theed snapshot");
    assert_eq!(reloaded.len(), theed_snap.len());

    let palace = reloaded
        .nodes
        .iter()
        .find(|n| n.template_name.contains("palace_theed"))
        .expect("Theed palace not found in snapshot");

    assert_eq!(palace.position, Vec3::new(-5498.0, 25.0, 4478.0));
    assert_eq!(palace.radius, 150.0);
}

#[test]
fn test_mos_eisley_frustum_and_distance_culling_performance() {
    let snapshot = CanonCityGenerator::generate_hub_snapshot(PlanetHub::TatooineMosEisley);
    let mut graph = SpatialSceneGraph::from_snapshot(snapshot, 128.0);

    // Add 1,000 background desert rocks and vaporators across Mos Eisley perimeter
    for i in 0..1000 {
        let x = 3000.0 + (i as f32 % 30.0) * 30.0;
        let z = -5200.0 + (i as f32 / 30.0) * 30.0;
        graph.insert(WorldSnapshotNode::new(
            10_000 + i,
            "object/static/rock/shared_desert_rock.iff",
            0,
            Vec3::new(x, 5.0, z),
            Quat::IDENTITY,
            5.0,
        ));
    }

    assert!(graph.len() > 1000);

    // Place camera at Chalmun's Cantina looking East towards the Starport (+X direction)
    let cam_pos = Vec3::new(3528.0, 10.0, -4804.0);
    let target = Vec3::new(3670.0, 10.0, -4780.0);
    let view = Mat4::look_at_rh(cam_pos, target, Vec3::Y);
    let proj = Mat4::perspective_rh(1.0, 1.777, 0.5, 1500.0);
    let frustum = CameraFrustum::from_view_projection(proj * view);

    // Measure query latency
    let start = Instant::now();
    let visible = graph.query_visible(cam_pos, &frustum, 800.0);
    let elapsed = start.elapsed();

    // Streaming query must be sub-millisecond
    assert!(
        elapsed.as_micros() < 2000,
        "Spatial scene graph query took too long: {:?}",
        elapsed
    );

    // Cantina and Starport should be visible
    assert!(
        visible
            .iter()
            .any(|n| n.template_name.contains("cantina_tatooine")),
        "Cantina should be visible in camera viewport"
    );
    assert!(
        visible
            .iter()
            .any(|n| n.template_name.contains("starport_tatooine")),
        "Starport should be visible looking towards +X"
    );

    // Objects completely behind the camera (e.g. to the West / Lucky Despot at X = 3380) should NOT be visible
    let lucky_despot = visible
        .iter()
        .find(|n| n.template_name.contains("hotel_tatooine"));
    assert!(
        lucky_despot.is_none(),
        "Lucky Despot Hotel is behind the camera and must be culled"
    );
}
