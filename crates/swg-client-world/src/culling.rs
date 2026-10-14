use glam::{Mat4, Vec3};

/// An infinite geometric 3D plane defined by ax + by + cz + d = 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrustumPlane {
    pub normal: Vec3,
    pub distance: f32,
}

impl FrustumPlane {
    #[must_use]
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
        let n = Vec3::new(a, b, c);
        let len = n.length();
        if len > 1e-6 {
            let inv = 1.0 / len;
            Self {
                normal: n * inv,
                distance: d * inv,
            }
        } else {
            Self {
                normal: Vec3::Y,
                distance: 0.0,
            }
        }
    }

    /// Signed distance from point P to the plane.
    #[must_use]
    pub fn distance_to_point(&self, p: Vec3) -> f32 {
        self.normal.dot(p) + self.distance
    }
}

/// Camera view frustum consisting of 6 clipping planes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraFrustum {
    pub planes: [FrustumPlane; 6],
}

impl CameraFrustum {
    /// Extracts 6 normalized frustum planes from a combined View-Projection matrix (Gribb-Hartmann).
    #[must_use]
    pub fn from_view_projection(vp: Mat4) -> Self {
        // Transpose to access rows easily via cols of transpose
        let m = vp.transpose();
        let r0 = m.x_axis;
        let r1 = m.y_axis;
        let r2 = m.z_axis;
        let r3 = m.w_axis;

        // Left plane: r3 + r0
        let left = r3 + r0;
        // Right plane: r3 - r0
        let right = r3 - r0;
        // Bottom plane: r3 + r1
        let bottom = r3 + r1;
        // Top plane: r3 - r1
        let top = r3 - r1;
        // Near plane: in standard reverse-z or 0..1 depth, r2 or r3 + r2
        let near = r2;
        // Far plane: r3 - r2
        let far = r3 - r2;

        let planes = [
            FrustumPlane::new(left.x, left.y, left.z, left.w),
            FrustumPlane::new(right.x, right.y, right.z, right.w),
            FrustumPlane::new(bottom.x, bottom.y, bottom.z, bottom.w),
            FrustumPlane::new(top.x, top.y, top.z, top.w),
            FrustumPlane::new(near.x, near.y, near.z, near.w),
            FrustumPlane::new(far.x, far.y, far.z, far.w),
        ];

        Self { planes }
    }

    /// Tests if a bounding sphere is within or intersects the frustum.
    /// Returns false if the sphere is completely outside (culled).
    #[must_use]
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(center) < -radius {
                return false;
            }
        }
        true
    }

    /// Tests if an axis-aligned bounding box (AABB) is within or intersects the frustum.
    #[must_use]
    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            // Find positive vertex along plane normal
            let p = Vec3::new(
                if plane.normal.x > 0.0 { max.x } else { min.x },
                if plane.normal.y > 0.0 { max.y } else { min.y },
                if plane.normal.z > 0.0 { max.z } else { min.z },
            );
            if plane.distance_to_point(p) < 0.0 {
                return false;
            }
        }
        true
    }
}

/// Evaluates distance-based visibility against atmospheric view limits.
pub struct DistanceCuller {
    pub max_view_distance: f32,
}

impl DistanceCuller {
    #[must_use]
    pub fn new(max_view_distance: f32) -> Self {
        Self {
            max_view_distance: max_view_distance.max(10.0),
        }
    }

    /// Checks if an object is within max draw distance from the camera.
    #[must_use]
    pub fn is_visible(&self, camera_pos: Vec3, object_pos: Vec3, radius: f32) -> bool {
        let dist = camera_pos.distance(object_pos);
        dist - radius <= self.max_view_distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frustum_sphere_culling_in_front_and_behind() {
        let proj = Mat4::perspective_rh(1.0, 1.333, 0.1, 1000.0);
        let view = Mat4::look_at_rh(
            Vec3::new(0.0, 0.0, 0.0),  // Eye at origin
            Vec3::new(0.0, 0.0, -1.0), // Looking down -Z
            Vec3::Y,
        );
        let vp = proj * view;
        let frustum = CameraFrustum::from_view_projection(vp);

        // Object directly in front of camera at (0, 0, -50)
        assert!(frustum.intersects_sphere(Vec3::new(0.0, 0.0, -50.0), 5.0));

        // Object directly behind camera at (0, 0, 50)
        assert!(!frustum.intersects_sphere(Vec3::new(0.0, 0.0, 50.0), 5.0));

        // Object far off to the side at (5000, 0, -50)
        assert!(!frustum.intersects_sphere(Vec3::new(5000.0, 0.0, -50.0), 5.0));
    }

    #[test]
    fn test_distance_culler() {
        let culler = DistanceCuller::new(500.0);
        let cam = Vec3::new(100.0, 10.0, 100.0);

        // Within 500m
        assert!(culler.is_visible(cam, Vec3::new(200.0, 10.0, 100.0), 10.0));

        // Beyond 500m
        assert!(!culler.is_visible(cam, Vec3::new(1000.0, 10.0, 100.0), 10.0));
    }
}
