// crates/swg-client-mesh/tests/mesh_pipeline_tests.rs

use glam::{Mat4, Vec2, Vec3, Vec4};
use swg_client_core::{ClusteredLightGrid, ClusteredPointLight};
use swg_client_mesh::{MeshLightingEvaluator, MshWriter, PbrMaterial, RigidMesh, SceneLighting};

#[test]
fn test_complex_multi_submesh_building_model() {
    let mut builder = MshWriter::new();
    builder.set_bounds(
        Vec3::new(0.0, 5.0, 0.0),
        15.0,
        Vec3::new(-10.0, 0.0, -10.0),
        Vec3::new(10.0, 10.0, 10.0),
    );

    // Cantina Walls vertices (0..4)
    builder.add_vertex(Vec3::new(-5.0, 0.0, -5.0), Vec3::X, Vec2::ZERO, Vec4::Z);
    builder.add_vertex(Vec3::new(-5.0, 5.0, -5.0), Vec3::X, Vec2::Y, Vec4::Z);
    builder.add_vertex(Vec3::new(-5.0, 0.0, 5.0), Vec3::X, Vec2::X, Vec4::Z);
    builder.add_vertex(Vec3::new(-5.0, 5.0, 5.0), Vec3::X, Vec2::ONE, Vec4::Z);

    // Cantina Roof vertices (4..7)
    builder.add_vertex(Vec3::new(-5.0, 5.0, -5.0), Vec3::Y, Vec2::ZERO, Vec4::X);
    builder.add_vertex(Vec3::new(5.0, 5.0, -5.0), Vec3::Y, Vec2::X, Vec4::X);
    builder.add_vertex(Vec3::new(-5.0, 5.0, 5.0), Vec3::Y, Vec2::Y, Vec4::X);
    builder.add_vertex(Vec3::new(5.0, 5.0, 5.0), Vec3::Y, Vec2::ONE, Vec4::X);

    // Submesh 1: Walls
    builder.add_sub_mesh("shader/building/cantina_wall.sht", vec![0, 1, 2, 2, 1, 3]);

    // Submesh 2: Roof
    builder.add_sub_mesh("shader/building/cantina_roof.sht", vec![4, 5, 6, 6, 5, 7]);

    let bytes = builder.finish();
    let mesh = RigidMesh::parse(&bytes).unwrap();

    assert_eq!(mesh.vertex_count(), 8);
    assert_eq!(mesh.triangle_count(), 4);
    assert_eq!(mesh.sub_meshes.len(), 2);
    assert_eq!(
        mesh.sub_meshes[0].shader_path,
        "shader/building/cantina_wall.sht"
    );
    assert_eq!(
        mesh.sub_meshes[1].shader_path,
        "shader/building/cantina_roof.sht"
    );
}

#[test]
fn test_lightsaber_and_campfire_clustered_illumination() {
    let mut builder = MshWriter::new();
    // 1 Triangle on the ground
    builder.add_vertex(Vec3::new(0.0, 0.0, 0.0), Vec3::Y, Vec2::ZERO, Vec4::X);
    builder.add_vertex(Vec3::new(2.0, 0.0, 0.0), Vec3::Y, Vec2::X, Vec4::X);
    builder.add_vertex(Vec3::new(1.0, 0.0, 2.0), Vec3::Y, Vec2::Y, Vec4::X);
    builder.add_sub_mesh("shader/terrain/sand.sht", vec![0, 1, 2]);

    let mesh = RigidMesh::parse(&builder.finish()).unwrap();

    let mat = PbrMaterial::from_legacy_parameters(
        "sand",
        Vec4::new(0.8, 0.7, 0.5, 1.0),
        0.1,   // Low gloss
        false, // Not metallic
        None,
    );

    // Lightsaber point light (blue) at (1, 1, 1) and Campfire (orange) at (10, 1, 1)
    let lights = vec![
        ClusteredPointLight {
            position: Vec3::new(1.0, 1.0, 1.0),
            radius: 5.0,
            color: Vec3::new(0.0, 0.2, 1.0), // Blue lightsaber
            intensity: 15.0,
        },
        ClusteredPointLight {
            position: Vec3::new(10.0, 1.0, 1.0),
            radius: 20.0,
            color: Vec3::new(1.0, 0.4, 0.0), // Orange campfire
            intensity: 30.0,
        },
    ];

    let grid = ClusteredLightGrid::new(Default::default(), 0.1, 100.0);
    let lighting = SceneLighting {
        sun_direction: Vec3::new(0.0, -1.0, 0.0),
        sun_color: Vec3::splat(0.2), // Night-time faint moon
        ambient_color: Vec3::splat(0.05),
    };

    let colors = MeshLightingEvaluator::shade_mesh_vertices(
        &mesh,
        Mat4::IDENTITY,
        &mat,
        &lighting,
        &lights,
        &grid,
    );

    assert_eq!(colors.len(), 3);
    for color in colors {
        // Shaded surface should receive dominant blue from nearby lightsaber and warm orange from campfire
        assert!(
            color.z > 0.3,
            "Blue component should be high from nearby lightsaber"
        );
        assert!(
            color.x > 0.2,
            "Red component should be present from campfire"
        );
    }
}
