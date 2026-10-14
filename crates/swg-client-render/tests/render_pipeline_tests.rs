// crates/swg-client-render/tests/render_pipeline_tests.rs

use glam::Vec3;
use std::time::Instant;
use swg_client_render::{ClutterGenerator, SplatWeights, TerrainMaterialPalette};
use swg_client_terrain::TerrainGenerator;

#[test]
fn test_splat_blending_gradient() {
    let palette = TerrainMaterialPalette::tatooine();

    // 1. Flat sand
    let flat_norm = Vec3::new(0.0, 1.0, 0.0);
    let weights_flat = SplatWeights::compute(flat_norm, 10.0);
    let pbr_flat = palette.evaluate_pbr(&weights_flat);
    assert!(weights_flat.sand > 0.95);
    assert!((pbr_flat.albedo.x - 0.85).abs() < 0.05);

    // 2. Steep cliff
    let cliff_norm = Vec3::new(0.95, 0.05, 0.0).normalize();
    let weights_cliff = SplatWeights::compute(cliff_norm, 10.0);
    let pbr_cliff = palette.evaluate_pbr(&weights_cliff);
    assert!(weights_cliff.rock > 0.90);
    assert!(pbr_cliff.roughness < 0.80);

    // 3. High mesa plateau
    let weights_mesa = SplatWeights::compute(flat_norm, 90.0);
    let pbr_mesa = palette.evaluate_pbr(&weights_mesa);
    assert!(weights_mesa.mesa > 0.80);
    assert!(pbr_mesa.albedo.x > 0.65);
}

#[test]
fn test_clutter_instancing_performance_120fps() {
    let generator = TerrainGenerator::tatooine();
    let clutter_gen = ClutterGenerator {
        max_distance: 128.0,
        instances_per_chunk: 512,
    };

    let camera_pos = Vec3::new(0.0, 15.0, 0.0);

    // In a 120 FPS runtime, at most 1 chunk is streamed in on a frame tick.
    // Verify per-chunk generation easily satisfies the 8.3ms 120 FPS frame budget.
    let start = Instant::now();
    let instances = clutter_gen.generate_chunk_clutter(&generator, 0, 0, camera_pos);
    let elapsed = start.elapsed();

    println!(
        "Streamed chunk with {} clutter instances in {:?} (budget: 8.3ms for 120 FPS)",
        instances.len(),
        elapsed
    );

    assert!(
        instances.len() > 100,
        "Should generate dense clutter near camera"
    );
    assert!(
        elapsed.as_millis() < 8,
        "Per-chunk clutter generation must take < 8.3ms to sustain 120+ FPS"
    );
}
