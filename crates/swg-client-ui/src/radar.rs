// crates/swg-client-ui/src/radar.rs

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Category of blip displayed on the planetary radar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlipType {
    Waypoint,
    FriendlyPlayer,
    NeutralNpc,
    HostileEnemy,
    Creature,
    Corpse,
}

/// A target marker on the radar screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadarBlip {
    pub id: u64,
    pub world_pos: Vec3,
    pub blip_type: BlipType,
    pub label: String,
}

/// Planetary radar minimap maintaining world coordinates, heading, and blip positions.
pub struct RadarMinimap {
    pub center_pos: Vec3,
    pub heading: f32, // Heading in radians (0 = North, PI/2 = East)
    pub range: f32,   // Radar radius in world meters
    pub blips: Vec<RadarBlip>,
}

impl RadarMinimap {
    #[must_use]
    pub fn new(center_pos: Vec3, heading: f32, range: f32) -> Self {
        Self {
            center_pos,
            heading,
            range: range.max(32.0),
            blips: Vec::new(),
        }
    }

    pub fn add_blip(&mut self, blip: RadarBlip) {
        self.blips.push(blip);
    }

    pub fn clear_blips(&mut self) {
        self.blips.clear();
    }

    /// Returns human-readable 8-point cardinal compass heading ("N", "NE", "E", etc.).
    #[must_use]
    pub fn cardinal_heading_name(&self) -> &'static str {
        let norm = self.heading.rem_euclid(2.0 * PI);
        let deg = norm.to_degrees();

        if (337.5..=360.0).contains(&deg) || (0.0..22.5).contains(&deg) {
            "N"
        } else if (22.5..67.5).contains(&deg) {
            "NE"
        } else if (67.5..112.5).contains(&deg) {
            "E"
        } else if (112.5..157.5).contains(&deg) {
            "SE"
        } else if (157.5..202.5).contains(&deg) {
            "S"
        } else if (202.5..247.5).contains(&deg) {
            "SW"
        } else if (247.5..292.5).contains(&deg) {
            "W"
        } else {
            "NW"
        }
    }

    /// Transforms a world position into radar screen pixel offsets relative to radar center.
    /// Clamps distant off-radar markers to the circular outer rim.
    #[must_use]
    pub fn compute_blip_screenspace(
        &self,
        blip_world_pos: Vec3,
        radar_radius_px: f32,
    ) -> (Vec2, bool) {
        let dx = blip_world_pos.x - self.center_pos.x;
        let dz = blip_world_pos.z - self.center_pos.z;
        let world_dist = (dx * dx + dz * dz).sqrt();

        // North is -Z, East is +X
        let world_angle = dx.atan2(-dz);
        let rel_angle = world_angle - self.heading;

        let (pixel_dist, is_clamped) = if world_dist <= self.range {
            ((world_dist / self.range) * radar_radius_px, false)
        } else {
            (radar_radius_px, true)
        };

        let sx = pixel_dist * rel_angle.sin();
        let sy = -pixel_dist * rel_angle.cos();

        (Vec2::new(sx, sy), is_clamped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radar_cardinal_headings() {
        let radar_n = RadarMinimap::new(Vec3::ZERO, 0.0, 256.0);
        assert_eq!(radar_n.cardinal_heading_name(), "N");

        let radar_e = RadarMinimap::new(Vec3::ZERO, PI / 2.0, 256.0);
        assert_eq!(radar_e.cardinal_heading_name(), "E");

        let radar_s = RadarMinimap::new(Vec3::ZERO, PI, 256.0);
        assert_eq!(radar_s.cardinal_heading_name(), "S");

        let radar_w = RadarMinimap::new(Vec3::ZERO, 3.0 * PI / 2.0, 256.0);
        assert_eq!(radar_w.cardinal_heading_name(), "W");
    }

    #[test]
    fn test_blip_screenspace_and_perimeter_clamping() {
        let radar = RadarMinimap::new(Vec3::ZERO, 0.0, 100.0); // Facing North, 100m range
        let radius_px = 120.0;

        // Target directly North at 50m (in front of player)
        let (pos_n, clamped_n) =
            radar.compute_blip_screenspace(Vec3::new(0.0, 0.0, -50.0), radius_px);
        assert!(!clamped_n);
        assert!((pos_n.x).abs() < 1e-4);
        assert!((pos_n.y - (-60.0)).abs() < 1e-4); // Directly up on screen

        // Target far East at 500m (outside radar range -> clamped to perimeter edge)
        let (pos_far, clamped_far) =
            radar.compute_blip_screenspace(Vec3::new(500.0, 0.0, 0.0), radius_px);
        assert!(clamped_far);
        assert!((pos_far.x - 120.0).abs() < 1e-4); // Pinned to right edge
        assert!((pos_far.y).abs() < 1e-4);
    }
}
