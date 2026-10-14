// crates/swg-client-core/src/ecs.rs

use crate::cluster::{ClusteredLightGrid, ClusteredPointLight};
use bevy_ecs::prelude::*;
use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// 3D Spatial transform component representing local position, orientation, and scale.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    #[must_use]
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self::from_translation(Vec3::new(x, y, z))
    }

    #[must_use]
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    #[must_use]
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }

    #[must_use]
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    #[must_use]
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }
}

/// Global world-space 4x4 transform matrix computed from entity hierarchy.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct GlobalTransform {
    pub matrix: Mat4,
}

impl Default for GlobalTransform {
    fn default() -> Self {
        Self {
            matrix: Mat4::IDENTITY,
        }
    }
}

/// Camera component supporting perspective projection and view matrix updates.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    pub aspect_ratio: f32,
    pub view: Mat4,
    pub proj: Mat4,
    pub view_proj: Mat4,
}

impl Default for Camera {
    fn default() -> Self {
        let fov_y = 60.0_f32.to_radians();
        let near = 0.1;
        let far = 2000.0;
        let aspect_ratio = 16.0 / 9.0;
        let proj = Mat4::perspective_rh(fov_y, aspect_ratio, near, far);
        let view = Mat4::IDENTITY;

        Self {
            fov_y,
            near,
            far,
            aspect_ratio,
            view,
            proj,
            view_proj: proj * view,
        }
    }
}

impl Camera {
    pub fn update(&mut self, transform: &Transform) {
        self.view = Mat4::look_to_rh(transform.translation, transform.forward(), transform.up());
        self.proj = Mat4::perspective_rh(self.fov_y, self.aspect_ratio, self.near, self.far);
        self.view_proj = self.proj * self.view;
    }
}

/// Directional light (e.g. planetary suns Tatooine 1 & 2).
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.5, -1.0, -0.3).normalize(),
            color: Vec3::ONE,
            intensity: 1.0,
        }
    }
}

/// Omnidirectional point light (lamps, blaster bolts, campfires).
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointLight {
    pub radius: f32,
    pub color: Vec3,
    pub intensity: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            radius: 10.0,
            color: Vec3::ONE,
            intensity: 10.0,
        }
    }
}

/// Tag component marking the local controllable player entity.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LocalPlayer;

/// Frame timing resource.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct TimeStep {
    pub delta_seconds: f32,
    pub elapsed_seconds: f64,
    pub frame_count: u64,
}

/// Resource holding the clustered forward light grid for rendering passes.
#[derive(Resource, Debug)]
pub struct LightGridResource {
    pub grid: ClusteredLightGrid,
}

/// System: Updates GlobalTransform components from Transform.
pub fn update_transforms_system(mut query: Query<(&Transform, &mut GlobalTransform)>) {
    for (transform, mut global) in &mut query {
        global.matrix = transform.matrix();
    }
}

/// System: Updates Camera view and projection matrices.
pub fn update_cameras_system(mut query: Query<(&Transform, &mut Camera)>) {
    for (transform, mut camera) in &mut query {
        camera.update(transform);
    }
}

/// System: Updates clustered light grid with active point lights.
pub fn update_light_grid_system(
    camera_query: Query<&Camera>,
    light_query: Query<(&Transform, &PointLight)>,
    light_grid: Option<ResMut<LightGridResource>>,
) {
    let Some(mut light_grid) = light_grid else {
        return;
    };

    let Some(camera) = camera_query.iter().next() else {
        return;
    };

    // Recompute cluster AABBs with camera projection
    light_grid.grid.update_clusters(camera.proj.inverse());

    let lights: Vec<ClusteredPointLight> = light_query
        .iter()
        .map(|(t, p)| ClusteredPointLight {
            position: t.translation,
            radius: p.radius,
            color: p.color,
            intensity: p.intensity,
        })
        .collect();

    light_grid.grid.assign_lights(camera.view, &lights);
}

/// High-level ECS Client World managing simulation schedules and entity state.
pub struct ClientWorld {
    pub world: World,
    pub schedule: Schedule,
}

impl Default for ClientWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientWorld {
    #[must_use]
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(TimeStep::default());
        world.insert_resource(LightGridResource {
            grid: ClusteredLightGrid::new(Default::default(), 0.1, 1000.0),
        });

        let mut schedule = Schedule::default();
        schedule.add_systems((
            update_transforms_system,
            update_cameras_system,
            update_light_grid_system,
        ));

        Self { world, schedule }
    }

    /// Advances simulation by delta time `dt`.
    pub fn tick(&mut self, dt: f32) {
        if let Some(mut time) = self.world.get_resource_mut::<TimeStep>() {
            time.delta_seconds = dt;
            time.elapsed_seconds += dt as f64;
            time.frame_count += 1;
        }
        self.schedule.run(&mut self.world);
    }

    /// Spawns a camera entity with transform.
    pub fn spawn_camera(&mut self, position: Vec3) -> Entity {
        self.world
            .spawn((
                Transform::from_translation(position),
                GlobalTransform::default(),
                Camera::default(),
            ))
            .id()
    }

    /// Spawns the local player entity.
    pub fn spawn_local_player(&mut self, position: Vec3) -> Entity {
        self.world
            .spawn((
                LocalPlayer,
                Transform::from_translation(position),
                GlobalTransform::default(),
            ))
            .id()
    }

    /// Spawns a point light entity.
    pub fn spawn_point_light(&mut self, position: Vec3, radius: f32, color: Vec3) -> Entity {
        self.world
            .spawn((
                Transform::from_translation(position),
                GlobalTransform::default(),
                PointLight {
                    radius,
                    color,
                    intensity: 10.0,
                },
            ))
            .id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_world_tick() {
        let mut client_world = ClientWorld::new();
        let cam_id = client_world.spawn_camera(Vec3::new(0.0, 5.0, 10.0));
        let player_id = client_world.spawn_local_player(Vec3::new(0.0, 0.0, 0.0));
        let light_id = client_world.spawn_point_light(Vec3::new(0.0, 2.0, -5.0), 10.0, Vec3::ONE);

        client_world.tick(0.016);

        // Verify camera matrices updated
        let cam = client_world.world.entity(cam_id).get::<Camera>().unwrap();
        assert_ne!(cam.view, Mat4::IDENTITY);

        // Verify player transform
        let player_trans = client_world
            .world
            .entity(player_id)
            .get::<GlobalTransform>()
            .unwrap();
        assert_eq!(player_trans.matrix, Mat4::IDENTITY);

        // Verify light grid assignment
        let light_res = client_world
            .world
            .get_resource::<LightGridResource>()
            .unwrap();
        let total_assigned: u32 = light_res.grid.tiles.iter().map(|t| t.count).sum();
        assert!(total_assigned > 0);

        let _ = light_id;
    }
}
