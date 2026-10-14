// crates/swg-client-sync/src/remote_interpolation.rs

use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::f32::consts::PI;

/// A discrete network snapshot of a remote entity received from the server.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RemoteSnapshot {
    pub timestamp: f64,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
}

/// The interpolated or extrapolated transform at a given render frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InterpolatedTransform {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub is_extrapolated: bool,
}

/// Cubic Hermite spline evaluator for smooth trajectory curves.
pub struct HermiteSpline;

impl HermiteSpline {
    /// Computes interpolated position and velocity between two snapshots using Cubic Hermite Splines.
    #[must_use]
    pub fn evaluate(s0: &RemoteSnapshot, s1: &RemoteSnapshot, render_time: f64) -> (Vec3, Vec3) {
        let dt = (s1.timestamp - s0.timestamp).max(0.0001) as f32;
        let t = (((render_time - s0.timestamp) as f32) / dt).clamp(0.0, 1.0);

        let t2 = t * t;
        let t3 = t2 * t;

        // Hermite basis functions for position
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        let pos =
            h00 * s0.position + h10 * dt * s0.velocity + h01 * s1.position + h11 * dt * s1.velocity;

        // Hermite derivatives for instantaneous velocity
        let dh00 = 6.0 * t2 - 6.0 * t;
        let dh10 = 3.0 * t2 - 4.0 * t + 1.0;
        let dh01 = -6.0 * t2 + 6.0 * t;
        let dh11 = 3.0 * t2 - 2.0 * t;

        let vel = (dh00 * s0.position + dh01 * s1.position) / dt
            + (dh10 * s0.velocity + dh11 * s1.velocity);

        (pos, vel)
    }

    /// Interpolates angular yaw along the shortest arc.
    #[must_use]
    pub fn lerp_angle(a0: f32, a1: f32, t: f32) -> f32 {
        let mut diff = (a1 - a0) % (2.0 * PI);
        if diff > PI {
            diff -= 2.0 * PI;
        } else if diff < -PI {
            diff += 2.0 * PI;
        }
        a0 + diff * t.clamp(0.0, 1.0)
    }
}

/// Remote entity interpolation manager handling jitter buffering,
/// Hermite spline blending, and graceful loss extrapolation.
#[derive(Debug, Clone)]
pub struct RemoteEntityInterpolator {
    jitter_buffer_secs: f64,
    max_extrapolation_secs: f64,
    max_snapshots_per_entity: usize,
    entities: HashMap<u64, VecDeque<RemoteSnapshot>>,
}

impl RemoteEntityInterpolator {
    #[must_use]
    pub fn new(jitter_buffer_secs: f64) -> Self {
        Self {
            jitter_buffer_secs,
            max_extrapolation_secs: 0.5,
            max_snapshots_per_entity: 64,
            entities: HashMap::new(),
        }
    }

    #[must_use]
    pub fn jitter_buffer_secs(&self) -> f64 {
        self.jitter_buffer_secs
    }

    pub fn set_jitter_buffer_secs(&mut self, buffer_secs: f64) {
        self.jitter_buffer_secs = buffer_secs.max(0.01);
    }

    /// Feeds an incoming server snapshot for a remote entity.
    pub fn push_snapshot(&mut self, entity_id: u64, snapshot: RemoteSnapshot) {
        let buffer = self
            .entities
            .entry(entity_id)
            .or_insert_with(|| VecDeque::with_capacity(32));

        // If snapshot is newer than current back, append it
        if let Some(back) = buffer.back() {
            if snapshot.timestamp > back.timestamp {
                buffer.push_back(snapshot);
            }
        } else {
            buffer.push_back(snapshot);
        }

        // Maintain maximum buffer capacity
        while buffer.len() > self.max_snapshots_per_entity {
            buffer.pop_front();
        }
    }

    /// Samples the continuous visual transform for a remote entity at the given client time.
    #[must_use]
    pub fn sample(&self, entity_id: u64, client_time: f64) -> Option<InterpolatedTransform> {
        let buffer = self.entities.get(&entity_id)?;
        if buffer.is_empty() {
            return None;
        }

        let render_time = client_time - self.jitter_buffer_secs;

        // If only 1 snapshot exists, return it directly
        if buffer.len() == 1 {
            let s = &buffer[0];
            return Some(InterpolatedTransform {
                position: s.position,
                velocity: s.velocity,
                yaw: s.yaw,
                is_extrapolated: false,
            });
        }

        let oldest = buffer.front().unwrap();
        let newest = buffer.back().unwrap();

        // If render time is older than our oldest snapshot, clamp to oldest
        if render_time <= oldest.timestamp {
            return Some(InterpolatedTransform {
                position: oldest.position,
                velocity: oldest.velocity,
                yaw: oldest.yaw,
                is_extrapolated: false,
            });
        }

        // If render time exceeds newest snapshot, extrapolate smoothly along last velocity
        if render_time > newest.timestamp {
            let extra_dt = (render_time - newest.timestamp).min(self.max_extrapolation_secs) as f32;
            let ext_pos = newest.position + newest.velocity * extra_dt;
            return Some(InterpolatedTransform {
                position: ext_pos,
                velocity: newest.velocity,
                yaw: newest.yaw,
                is_extrapolated: true,
            });
        }

        // Otherwise find the two bounding snapshots around render_time
        for i in 0..(buffer.len() - 1) {
            let s0 = &buffer[i];
            let s1 = &buffer[i + 1];

            if s0.timestamp <= render_time && render_time <= s1.timestamp {
                let (pos, vel) = HermiteSpline::evaluate(s0, s1, render_time);
                let dt = (s1.timestamp - s0.timestamp).max(0.0001) as f32;
                let t = (((render_time - s0.timestamp) as f32) / dt).clamp(0.0, 1.0);
                let yaw = HermiteSpline::lerp_angle(s0.yaw, s1.yaw, t);

                return Some(InterpolatedTransform {
                    position: pos,
                    velocity: vel,
                    yaw,
                    is_extrapolated: false,
                });
            }
        }

        None
    }

    /// Cleans up snapshots that are older than render_time across all entities.
    pub fn prune_old_snapshots(&mut self, client_time: f64) {
        let cutoff = client_time - self.jitter_buffer_secs - 0.2; // Keep 200ms grace period
        for buffer in self.entities.values_mut() {
            while buffer.len() > 2 {
                if let Some(second) = buffer.get(1) {
                    if second.timestamp < cutoff {
                        buffer.pop_front();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }

    /// Removes an entity when despawned.
    pub fn remove_entity(&mut self, entity_id: u64) {
        self.entities.remove(&entity_id);
    }

    /// Clears all tracked entities.
    pub fn clear(&mut self) {
        self.entities.clear();
    }
}

impl Default for RemoteEntityInterpolator {
    fn default() -> Self {
        Self::new(0.100) // 100ms default jitter buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hermite_interpolation_continuity() {
        let s0 = RemoteSnapshot {
            timestamp: 10.0,
            position: Vec3::new(0.0, 0.0, 0.0),
            velocity: Vec3::new(10.0, 0.0, 0.0),
            yaw: 0.0,
        };
        let s1 = RemoteSnapshot {
            timestamp: 11.0,
            position: Vec3::new(10.0, 0.0, 0.0),
            velocity: Vec3::new(10.0, 0.0, 0.0),
            yaw: 0.0,
        };

        let (mid_pos, mid_vel) = HermiteSpline::evaluate(&s0, &s1, 10.5);
        assert!((mid_pos.x - 5.0).abs() < 0.01);
        assert!((mid_vel.x - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_remote_interpolator_jitter_buffering() {
        let mut interpolator = RemoteEntityInterpolator::new(0.100);
        let entity_id = 42;

        // Server sends snapshots at 10.0, 10.05, 10.10, 10.15
        interpolator.push_snapshot(
            entity_id,
            RemoteSnapshot {
                timestamp: 10.0,
                position: Vec3::new(0.0, 0.0, 0.0),
                velocity: Vec3::new(5.0, 0.0, 0.0),
                yaw: 0.0,
            },
        );
        interpolator.push_snapshot(
            entity_id,
            RemoteSnapshot {
                timestamp: 10.05,
                position: Vec3::new(0.25, 0.0, 0.0),
                velocity: Vec3::new(5.0, 0.0, 0.0),
                yaw: 0.0,
            },
        );
        interpolator.push_snapshot(
            entity_id,
            RemoteSnapshot {
                timestamp: 10.10,
                position: Vec3::new(0.50, 0.0, 0.0),
                velocity: Vec3::new(5.0, 0.0, 0.0),
                yaw: 0.0,
            },
        );

        // At client time 10.15, render time is 10.15 - 0.100 = 10.05
        let transform = interpolator.sample(entity_id, 10.15).unwrap();
        assert!(!transform.is_extrapolated);
        assert!((transform.position.x - 0.25).abs() < 0.01);

        // At client time 10.25 (starved of snapshots), extrapolates past 10.10
        let ext_transform = interpolator.sample(entity_id, 10.25).unwrap();
        assert!(ext_transform.is_extrapolated);
        assert!(ext_transform.position.x > 0.50);
    }
}
