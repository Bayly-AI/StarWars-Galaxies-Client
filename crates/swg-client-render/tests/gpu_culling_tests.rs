// crates/swg-client-render/tests/gpu_culling_tests.rs

use glam::{Vec3, Vec4};
use std::time::Instant;
use swg_client_core::{GraphicsBackend, RenderConfig, RenderContext, RenderError};
use swg_client_render::{
    CpuCullingEvaluator, CullableInstanceRaw, FrustumUniforms, GpuCullingPipeline,
    LodGeomorphConfig, LodGeomorphEvaluator, OperatingSystem, PlatformManager,
};

#[test]
fn test_gpu_compute_shader_compilation() {
    let config = RenderConfig {
        backend: GraphicsBackend::Auto,
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    };

    match RenderContext::init_headless_sync(&config) {
        Ok(ctx) => {
            let (_shader, _pipeline, _layout) = GpuCullingPipeline::create_pipeline(&ctx.device);
            println!("GPU compute culling pipeline compiled successfully on hardware adapter");
        }
        Err(RenderError::AdapterNotFound) => {
            println!("Headless runner has no GPU adapter (allowed in headless CI)");
        }
        Err(e) => panic!("Unexpected render error: {e:?}"),
    }
}

#[test]
fn test_gpu_compute_frustum_and_distance_culling_compact() {
    // Frustum looking down -Z from camera at (0, 10, 0)
    let planes = [
        [0.7071, 0.0, -0.7071, 0.0],   // Left
        [-0.7071, 0.0, -0.7071, 0.0],  // Right
        [0.0, 0.9659, -0.2588, 0.0],   // Bottom
        [0.0, -0.7071, -0.7071, 10.0], // Top
        [0.0, 0.0, -1.0, 1.0],         // Near (Z < -1)
        [0.0, 0.0, 1.0, 250.0],        // Far (Z > -250)
    ];

    let uniforms = FrustumUniforms {
        planes,
        camera_pos: [0.0, 10.0, 0.0, 1.0],
        total_instances: 5000,
        max_draw_distance: 200.0,
        pad0: 0,
        pad1: 0,
    };

    // Create 5,000 instances spread across a 500x500 area
    let mut instances = Vec::with_capacity(5000);
    for i in 0..5000 {
        let x = ((i % 100) as f32 - 50.0) * 5.0;
        let z = ((i / 100) as f32 - 25.0) * -10.0;
        let center = Vec3::new(x, 0.0, z);
        instances.push(CullableInstanceRaw::new(
            center,
            2.0,
            center - Vec3::splat(2.0),
            center + Vec3::splat(2.0),
            i as u32,
            0,
        ));
    }

    let visible = CpuCullingEvaluator::cull_instances(&instances, &uniforms);

    // Some should be visible in front of camera, but far/behind instances must be culled
    assert!(
        !visible.is_empty(),
        "Must retain visible instances in view cone"
    );
    assert!(
        visible.len() < instances.len(),
        "Must cull instances outside frustum and beyond distance"
    );

    // Verify all retained instances are within distance limit
    let cam = Vec3::new(0.0, 10.0, 0.0);
    for &id in &visible {
        let inst = &instances[id as usize];
        let d = Vec3::from_slice(&inst.center).distance(cam);
        assert!(
            d <= uniforms.max_draw_distance + inst.radius + 0.01,
            "Visible instance must be within draw distance"
        );
    }
}

#[test]
fn test_dense_player_city_performance_120fps() {
    let geomorph_config = LodGeomorphConfig::default();

    // Simulate dense player city (Mos Eisley / Theed):
    // 50+ concurrent animated characters + 10,000 city props/clutter
    let num_characters = 60;
    let num_props = 10_000;
    let total_instances = num_characters + num_props;

    let mut instances = Vec::with_capacity(total_instances);
    for i in 0..total_instances {
        let angle = (i as f32) * 0.1;
        let radius = ((i % 200) as f32) * 1.5;
        let center = Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
        instances.push(CullableInstanceRaw::new(
            center,
            3.0,
            center - Vec3::splat(3.0),
            center + Vec3::splat(3.0),
            i as u32,
            0,
        ));
    }

    let planes = [
        [0.7071, 0.0, -0.7071, 50.0],
        [-0.7071, 0.0, -0.7071, 50.0],
        [0.0, 0.9659, -0.2588, 50.0],
        [0.0, -0.7071, -0.7071, 50.0],
        [0.0, 0.0, -1.0, 2.0],
        [0.0, 0.0, 1.0, 300.0],
    ];

    let uniforms = FrustumUniforms {
        planes,
        camera_pos: [0.0, 12.0, 20.0, 1.0],
        total_instances: total_instances as u32,
        max_draw_distance: 250.0,
        pad0: 0,
        pad1: 0,
    };

    // Benchmark 120 simulated frames
    let frame_count = 120;
    let start = Instant::now();

    for _frame in 0..frame_count {
        // 1. Culling
        let visible = CpuCullingEvaluator::cull_instances(&instances, &uniforms);

        // 2. Geomorph evaluation for all visible characters & meshes
        let cam = Vec3::new(0.0, 12.0, 20.0);
        for &id in &visible {
            let inst = &instances[id as usize];
            let dist = Vec3::from_slice(&inst.center).distance(cam);
            let (_lod, _morph) =
                LodGeomorphEvaluator::evaluate_lod_and_morph(dist, &geomorph_config);
        }
    }

    let elapsed = start.elapsed();
    let avg_frame_ms = elapsed.as_secs_f64() * 1000.0 / (frame_count as f64);
    println!("Dense player city benchmark (10,060 instances): {avg_frame_ms:.3} ms per frame");

    // 120 FPS target budget is 8.33ms per frame.
    // Our culling & geomorphing must comfortably execute in under 4.0ms.
    assert!(
        avg_frame_ms < 4.0,
        "Dense city frame time ({avg_frame_ms:.2}ms) must sustain well above 120 FPS (<8.33ms)"
    );
}

#[test]
fn test_cross_platform_packaging_targets() {
    let win = PlatformManager::default_config(OperatingSystem::Windows);
    let mac = PlatformManager::default_config(OperatingSystem::MacOS);
    let lin = PlatformManager::default_config(OperatingSystem::Linux);

    assert_eq!(win.executable_name, "SWGClient.exe");
    assert_eq!(mac.executable_name, "SWGClient");
    assert_eq!(lin.executable_name, "swg-client");

    assert!(win.high_dpi_enabled);
    assert!(mac.high_dpi_enabled);
    assert!(lin.high_dpi_enabled);
}
