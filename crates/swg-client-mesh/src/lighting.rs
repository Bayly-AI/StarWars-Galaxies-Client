// crates/swg-client-mesh/src/lighting.rs

use crate::material::PbrMaterial;
use crate::msh::RigidMesh;
use glam::{Mat4, Vec3, Vec4};
use swg_client_core::{ClusteredLightGrid, ClusteredPointLight};

/// Planetary sun and ambient scene lighting parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneLighting {
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
    pub ambient_color: Vec3,
}

impl Default for SceneLighting {
    fn default() -> Self {
        Self {
            sun_direction: Vec3::new(-0.5, -1.0, -0.3).normalize(),
            sun_color: Vec3::ONE,
            ambient_color: Vec3::splat(0.1),
        }
    }
}

/// Evaluates clustered Forward+ PBR illumination on a 3D rigid mesh.
pub struct MeshLightingEvaluator;

impl MeshLightingEvaluator {
    /// Computes per-vertex shaded RGB color on a mesh given planetary sun and dynamic point lights.
    #[must_use]
    pub fn shade_mesh_vertices(
        mesh: &RigidMesh,
        model_matrix: Mat4,
        material: &PbrMaterial,
        lighting: &SceneLighting,
        point_lights: &[ClusteredPointLight],
        _light_grid: &ClusteredLightGrid,
    ) -> Vec<Vec3> {
        let normal_matrix = model_matrix.inverse().transpose();
        let mut shaded_colors = Vec::with_capacity(mesh.positions.len());

        let albedo = material.base_color.truncate();
        let emissive_radiance = material.emissive * material.emissive_intensity;

        for (pos_local, norm_local) in mesh.positions.iter().zip(&mesh.normals) {
            let world_pos =
                (model_matrix * Vec4::new(pos_local.x, pos_local.y, pos_local.z, 1.0)).truncate();
            let world_norm = (normal_matrix
                * Vec4::new(norm_local.x, norm_local.y, norm_local.z, 0.0))
            .truncate()
            .normalize();

            // 1. Ambient lighting
            let mut total_color = albedo * lighting.ambient_color;

            // 2. Directional sun lighting
            let n_dot_sun = world_norm.dot(-lighting.sun_direction).max(0.0);
            total_color += albedo * lighting.sun_color * n_dot_sun;

            // 3. Dynamic point lights (blasters, lightsabers, campfires)
            for light in point_lights {
                let to_light = light.position - world_pos;
                let dist_sq = to_light.length_squared();

                if dist_sq <= light.radius * light.radius {
                    let dist = dist_sq.sqrt().max(0.01);
                    let light_dir = to_light / dist;
                    let n_dot_l = world_norm.dot(light_dir).max(0.0);

                    // Inverse-square falloff with smooth radius attenuation
                    let atten = (1.0 - (dist / light.radius)).clamp(0.0, 1.0);
                    let radiance = (light.color * light.intensity / dist_sq.max(1.0)) * atten;

                    total_color += albedo * radiance * n_dot_l;
                }
            }

            // 4. Emissive contribution
            total_color += emissive_radiance;

            shaded_colors.push(total_color);
        }

        shaded_colors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msh::MshWriter;
    use glam::{Vec2, Vec4};

    #[test]
    fn test_point_light_illumination_on_mesh() {
        let mut builder = MshWriter::new();
        // Flat quad facing +Y
        builder.add_vertex(Vec3::new(-1.0, 0.0, -1.0), Vec3::Y, Vec2::ZERO, Vec4::X);
        builder.add_vertex(Vec3::new(1.0, 0.0, -1.0), Vec3::Y, Vec2::X, Vec4::X);
        builder.add_vertex(Vec3::new(0.0, 0.0, 1.0), Vec3::Y, Vec2::Y, Vec4::X);
        builder.add_sub_mesh("shader/default.sht", vec![0, 1, 2]);

        let mesh = RigidMesh::parse(&builder.finish()).unwrap();
        let mat = PbrMaterial::default();

        let lighting = SceneLighting {
            sun_direction: Vec3::new(0.0, -1.0, 0.0),
            sun_color: Vec3::ONE,
            ambient_color: Vec3::splat(0.1),
        };

        // Point light directly above center at (0, 2, 0)
        let lights = vec![ClusteredPointLight {
            position: Vec3::new(0.0, 2.0, 0.0),
            radius: 10.0,
            color: Vec3::new(1.0, 0.0, 0.0), // Red blaster bolt
            intensity: 20.0,
        }];

        let grid = ClusteredLightGrid::new(Default::default(), 0.1, 100.0);
        let colors = MeshLightingEvaluator::shade_mesh_vertices(
            &mesh,
            Mat4::IDENTITY,
            &mat,
            &lighting,
            &lights,
            &grid,
        );

        assert_eq!(colors.len(), 3);
        // Red channel should be boosted by the red point light
        for c in colors {
            assert!(
                c.x > c.y,
                "Red component should be higher due to red blaster point light"
            );
        }
    }
}
