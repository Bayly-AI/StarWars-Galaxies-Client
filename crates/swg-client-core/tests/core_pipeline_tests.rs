// crates/swg-client-core/tests/core_pipeline_tests.rs

use glam::{Mat4, Vec3};
use swg_client_core::{
    Camera, ClientWorld, ClusterGridDimensions, ClusteredLightGrid, ClusteredPointLight,
    DisplayMetrics, GlobalTransform, GraphicsBackend, LightGridResource, PresentModeSetting,
    RenderConfig, RenderContext, RenderError, WindowConfig, WindowMode,
};

#[test]
fn test_window_config_and_display_metrics() {
    let config = WindowConfig {
        title: "SWG Test Window".to_string(),
        width: 2560,
        height: 1440,
        mode: WindowMode::Windowed,
        present_mode: PresentModeSetting::Mailbox,
        target_fps: 144.0,
        resizable: true,
        high_dpi_enabled: true,
    };

    assert_eq!(config.target_fps, 144.0);
    assert_eq!(config.present_mode, PresentModeSetting::Mailbox);

    // Test 1.5x scaling on 1440p
    let metrics = DisplayMetrics::new(config.width as f64, config.height as f64, 1.5);
    assert_eq!(metrics.physical_width, 3840);
    assert_eq!(metrics.physical_height, 2160);

    // Test 2.0x scaling on 1080p (Retina)
    let retina_metrics = DisplayMetrics::new(1920.0, 1080.0, 2.0);
    assert_eq!(retina_metrics.physical_width, 3840);
    assert_eq!(retina_metrics.physical_height, 2160);
    assert!((retina_metrics.aspect_ratio() - (16.0 / 9.0)).abs() < 1e-4);
}

#[test]
fn test_clustered_forward_light_grid_slicing() {
    let dims = ClusterGridDimensions::default();
    assert_eq!(dims.x_tiles, 16);
    assert_eq!(dims.y_tiles, 9);
    assert_eq!(dims.z_slices, 24);
    assert_eq!(dims.total_clusters(), 3456);

    let mut grid = ClusteredLightGrid::new(dims, 0.1, 1000.0);
    let proj = Mat4::perspective_rh(60.0_f32.to_radians(), 16.0 / 9.0, 0.1, 1000.0);
    grid.update_clusters(proj.inverse());

    assert_eq!(grid.cluster_aabbs.len(), 3456);

    // View looking forward from origin
    let view = Mat4::IDENTITY;

    // Point lights:
    // 1. Center of near field (inside frustum)
    // 2. Far away to the side (outside frustum)
    let lights = vec![
        ClusteredPointLight {
            position: Vec3::new(0.0, 0.0, -10.0),
            radius: 8.0,
            color: Vec3::new(1.0, 0.8, 0.6),
            intensity: 50.0,
        },
        ClusteredPointLight {
            position: Vec3::new(2000.0, 500.0, -10.0),
            radius: 5.0,
            color: Vec3::new(0.2, 0.2, 1.0),
            intensity: 20.0,
        },
    ];

    grid.assign_lights(view, &lights);

    // Light 0 should be assigned to some clusters
    let active_clusters: Vec<_> = grid.tiles.iter().filter(|t| t.count > 0).collect();
    assert!(
        !active_clusters.is_empty(),
        "Light inside frustum must be assigned to clusters"
    );

    // Light 1 must be culled
    assert!(
        !grid.light_indices.contains(&1),
        "Light outside frustum must be culled from index list"
    );
}

#[test]
fn test_ecs_world_and_simulation_schedule() {
    let mut world = ClientWorld::new();

    let player_pos = Vec3::new(10.0, 0.0, -20.0);
    let player = world.spawn_local_player(player_pos);
    let camera = world.spawn_camera(Vec3::new(10.0, 2.0, -15.0));
    let light = world.spawn_point_light(Vec3::new(10.0, 1.0, -18.0), 5.0, Vec3::new(1.0, 0.9, 0.8));

    // Tick simulation for 10 frames at 144Hz (dt = 1/144 = ~0.00694s)
    let dt = 1.0 / 144.0;
    for _ in 0..10 {
        world.tick(dt);
    }

    // Verify time step
    let time = world
        .world
        .get_resource::<swg_client_core::TimeStep>()
        .unwrap();
    assert_eq!(time.frame_count, 10);
    assert!((time.elapsed_seconds - (10.0 * dt as f64)).abs() < 1e-4);

    // Verify player global transform updated
    let p_transform = world.world.entity(player).get::<GlobalTransform>().unwrap();
    assert_eq!(p_transform.matrix.w_axis.truncate(), player_pos);

    // Verify camera view matrix is valid
    let cam = world.world.entity(camera).get::<Camera>().unwrap();
    assert_ne!(cam.view, Mat4::IDENTITY);
    assert_ne!(cam.view_proj, Mat4::IDENTITY);

    // Verify light assigned in grid resource
    let grid_res = world.world.get_resource::<LightGridResource>().unwrap();
    let total_assigned: u32 = grid_res.grid.tiles.iter().map(|t| t.count).sum();
    assert!(
        total_assigned > 0,
        "Point light should be assigned in clustered grid"
    );

    let _ = light;
}

#[test]
fn test_headless_render_initialization() {
    let config = RenderConfig {
        backend: GraphicsBackend::Auto,
        power_preference: wgpu::PowerPreference::HighPerformance,
        present_mode: PresentModeSetting::Mailbox,
        clear_color: wgpu::Color::BLACK,
    };

    match RenderContext::init_headless_sync(&config) {
        Ok(ctx) => {
            let info = ctx.adapter.get_info();
            println!(
                "Headless adapter initialized: {:?} ({:?})",
                info.name, info.backend
            );
            assert!(ctx.surface.is_none());
        }
        Err(RenderError::AdapterNotFound) => {
            println!("Headless runner has no graphics adapter (expected in some CI environments)");
        }
        Err(e) => panic!("Unexpected render error: {e:?}"),
    }
}
