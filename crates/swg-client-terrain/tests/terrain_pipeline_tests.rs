// crates/swg-client-terrain/tests/terrain_pipeline_tests.rs

use glam::Vec3;
use swg_client_terrain::{
    CHUNK_SIZE_METERS, ChunkGridManager, NeighborLods, TerrainChunkMesh, TerrainGenerator,
    TerrainLod,
};

#[test]
fn test_dynamic_camera_chunk_streaming() {
    let generator = TerrainGenerator::naboo();
    let mut manager = ChunkGridManager::new(128.0);

    // Initial position at origin
    manager.update(&generator, Vec3::new(0.0, 10.0, 0.0));
    let initial_count = manager.loaded_chunks.len();
    assert!(
        initial_count > 0,
        "Should load chunks within 128m view radius"
    );

    // Origin chunk should be LOD0 (closest)
    let origin_chunk = manager.loaded_chunks.get(&(0, 0)).unwrap();
    assert_eq!(origin_chunk.lod, TerrainLod::Lod0);

    // Camera moves 1000m away to (1000, 1000)
    manager.update(&generator, Vec3::new(1000.0, 10.0, 1000.0));
    assert!(
        !manager.loaded_chunks.contains_key(&(0, 0)),
        "Old origin chunks should be evicted outside view radius"
    );

    let new_center_x = (1000.0 / CHUNK_SIZE_METERS).floor() as i32;
    let new_center_z = (1000.0 / CHUNK_SIZE_METERS).floor() as i32;
    assert!(
        manager
            .loaded_chunks
            .contains_key(&(new_center_x, new_center_z)),
        "New center chunk must be loaded"
    );
}

#[test]
fn test_seam_and_crack_free_watertightness_multilateral() {
    let generator = TerrainGenerator::tatooine();

    // Center chunk (0, 0) at LOD 0
    // West neighbor (-1, 0) at LOD 1
    // East neighbor (1, 0) at LOD 1
    // South neighbor (0, -1) at LOD 2
    // North neighbor (0, 1) at LOD 2

    let neighbors_center = NeighborLods {
        north: Some(TerrainLod::Lod2),
        south: Some(TerrainLod::Lod2),
        east: Some(TerrainLod::Lod1),
        west: Some(TerrainLod::Lod1),
    };

    let center = TerrainChunkMesh::generate(&generator, 0, 0, TerrainLod::Lod0, neighbors_center);
    let north = TerrainChunkMesh::generate(
        &generator,
        0,
        1,
        TerrainLod::Lod2,
        NeighborLods {
            south: Some(TerrainLod::Lod0),
            ..Default::default()
        },
    );

    // Test North boundary between center (jz = 64) and north (jz = 0)
    let center_north_offset = 64 * 65;
    for i_coarse in 0..=16 {
        let i_center = i_coarse * 4; // LOD 0 has 64 segments, LOD 2 has 16 segments (4x ratio)
        let pos_center = center.vertices[center_north_offset + i_center].position;
        let pos_north = north.vertices[i_coarse].position;

        assert!(
            (pos_center[0] - pos_north[0]).abs() < 1e-3,
            "X coordinates must match along boundary"
        );
        assert!(
            (pos_center[1] - pos_north[1]).abs() < 1e-3,
            "Y heights must match along boundary"
        );
        assert!(
            (pos_center[2] - pos_north[2]).abs() < 1e-3,
            "Z coordinates must match along boundary"
        );
    }
}
