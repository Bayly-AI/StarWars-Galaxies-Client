// crates/swg-client-ui/src/cursor_ray.rs

use glam::{Mat4, Vec2, Vec3, Vec4};

/// A 3D ray defined by an origin and normalized direction vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray3D {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray3D {
    #[must_use]
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    #[must_use]
    pub fn point_at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// Unprojects 2D screen coordinates into a 3D world ray using the inverted View-Projection matrix.
    #[must_use]
    pub fn unproject_mouse(mouse_px: Vec2, screen_dims: Vec2, inv_view_proj: Mat4) -> Self {
        let width = screen_dims.x.max(1.0);
        let height = screen_dims.y.max(1.0);

        // Map to Normalized Device Coordinates [-1.0, 1.0]
        let ndc_x = (2.0 * mouse_px.x / width) - 1.0;
        let ndc_y = 1.0 - (2.0 * mouse_px.y / height); // Screen Y increases downwards

        let near_clip = Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
        let far_clip = Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        let near_world_unscaled = inv_view_proj * near_clip;
        let far_world_unscaled = inv_view_proj * far_clip;

        let near_world = near_world_unscaled.truncate() / near_world_unscaled.w.max(1e-6);
        let far_world = far_world_unscaled.truncate() / far_world_unscaled.w.max(1e-6);

        let direction = (far_world - near_world).normalize();
        Self::new(near_world, direction)
    }

    /// Tests intersection against a 3D bounding sphere. Returns distance t if hit in front of ray.
    #[must_use]
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> Option<f32> {
        let l = center - self.origin;
        let t_ca = l.dot(self.direction);
        let d2 = l.dot(l) - t_ca * t_ca;
        let r2 = radius * radius;

        if d2 > r2 {
            return None;
        }

        let t_hc = (r2 - d2).sqrt();
        let t0 = t_ca - t_hc;
        let t1 = t_ca + t_hc;

        if t0 > 0.001 {
            Some(t0)
        } else if t1 > 0.001 {
            Some(t1)
        } else {
            None
        }
    }

    /// Tests intersection against an Axis-Aligned Bounding Box (AABB) using the slab method.
    #[must_use]
    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> Option<f32> {
        let mut tmin = (min.x - self.origin.x) / self.direction.x;
        let mut tmax = (max.x - self.origin.x) / self.direction.x;

        if tmin > tmax {
            std::mem::swap(&mut tmin, &mut tmax);
        }

        let mut tymin = (min.y - self.origin.y) / self.direction.y;
        let mut tymax = (max.y - self.origin.y) / self.direction.y;

        if tymin > tymax {
            std::mem::swap(&mut tymin, &mut tymax);
        }

        if (tmin > tymax) || (tymin > tmax) {
            return None;
        }

        if tymin > tmin {
            tmin = tymin;
        }
        if tymax < tmax {
            tmax = tymax;
        }

        let mut tzmin = (min.z - self.origin.z) / self.direction.z;
        let mut tzmax = (max.z - self.origin.z) / self.direction.z;

        if tzmin > tzmax {
            std::mem::swap(&mut tzmin, &mut tzmax);
        }

        if (tmin > tzmax) || (tzmin > tmax) {
            return None;
        }

        if tzmin > tmin {
            tmin = tzmin;
        }
        if tzmax < tmax {
            tmax = tzmax;
        }

        if tmax < 0.001 {
            None
        } else {
            Some(tmin.max(0.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_sphere_and_aabb_intersection() {
        let ray = Ray3D::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0));

        // Sphere at (0, 0, -10) radius 2.0
        let hit_dist = ray.intersects_sphere(Vec3::new(0.0, 0.0, -10.0), 2.0);
        assert!(hit_dist.is_some());
        // Front surface of sphere is at Z = -8 (distance 8.0)
        assert!((hit_dist.unwrap() - 8.0).abs() < 1e-3);

        // AABB at (-1, -1, -22) to (1, 1, -20)
        let aabb_dist =
            ray.intersects_aabb(Vec3::new(-1.0, -1.0, -22.0), Vec3::new(1.0, 1.0, -20.0));
        assert!(aabb_dist.is_some());
        assert!((aabb_dist.unwrap() - 20.0).abs() < 1e-3);

        // Miss sphere far off to the right
        let miss = ray.intersects_sphere(Vec3::new(100.0, 0.0, -10.0), 2.0);
        assert!(miss.is_none());
    }

    #[test]
    fn test_mouse_unproject_straight_forward() {
        let proj = Mat4::perspective_rh(1.0, 1.0, 0.1, 1000.0);
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0), Vec3::Y);
        let inv_vp = (proj * view).inverse();

        // Mouse centered at (500, 500) on 1000x1000 screen
        let ray =
            Ray3D::unproject_mouse(Vec2::new(500.0, 500.0), Vec2::new(1000.0, 1000.0), inv_vp);

        // Direction should point down -Z
        assert!((ray.direction.z - (-1.0)).abs() < 1e-2);
        assert!(ray.direction.x.abs() < 1e-2);
        assert!(ray.direction.y.abs() < 1e-2);
    }
}
