// crates/swg-client-terrain/src/height.rs

use glam::Vec3;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TerrainError {
    #[error("IFF error while reading .trn file: {0}")]
    Iff(String),
    #[error("Invalid fractal parameter: {0}")]
    InvalidParameter(String),
}

/// Fractal noise evaluator using multi-octave gradient noise with quintic interpolation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FractalNoise {
    pub octaves: usize,
    pub frequency: f32,
    pub lacunarity: f32,
    pub persistence: f32,
    pub base_amplitude: f32,
    pub seed: u32,
}

impl Default for FractalNoise {
    fn default() -> Self {
        Self {
            octaves: 4,
            frequency: 0.002, // 1 cycle per 500m
            lacunarity: 2.0,
            persistence: 0.5,
            base_amplitude: 40.0,
            seed: 1337,
        }
    }
}

impl FractalNoise {
    #[must_use]
    pub fn sample(&self, x: f32, z: f32) -> f32 {
        let mut total = 0.0;
        let mut freq = self.frequency;
        let mut amp = self.base_amplitude;

        for octave in 0..self.octaves {
            let s_x = x * freq + (self.seed.wrapping_add((octave as u32) * 1013) as f32);
            let s_z = z * freq + (self.seed.wrapping_add((octave as u32) * 2017) as f32);
            let n = Self::gradient_noise_2d(s_x, s_z);
            total += n * amp;

            freq *= self.lacunarity;
            amp *= self.persistence;
        }

        total
    }

    /// 2D Gradient noise with quintic Hermite smoothing.
    fn gradient_noise_2d(x: f32, z: f32) -> f32 {
        let x0 = x.floor() as i32;
        let z0 = z.floor() as i32;
        let x1 = x0 + 1;
        let z1 = z0 + 1;

        let fx = x - x.floor();
        let fz = z - z.floor();

        // Quintic curve: 6t^5 - 15t^4 + 10t^3
        let ux = fx * fx * fx * (fx * (fx * 6.0 - 15.0) + 10.0);
        let uz = fz * fz * fz * (fz * (fz * 6.0 - 15.0) + 10.0);

        let g00 = Self::dot_grid_gradient(x0, z0, fx, fz);
        let g10 = Self::dot_grid_gradient(x1, z0, fx - 1.0, fz);
        let g01 = Self::dot_grid_gradient(x0, z1, fx, fz - 1.0);
        let g11 = Self::dot_grid_gradient(x1, z1, fx - 1.0, fz - 1.0);

        let nx0 = g00 + ux * (g10 - g00);
        let nx1 = g01 + ux * (g11 - g01);

        nx0 + uz * (nx1 - nx0)
    }

    fn dot_grid_gradient(ix: i32, iz: i32, dx: f32, dz: f32) -> f32 {
        let hash = Self::hash_coords(ix, iz);
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let (gx, gz) = match hash & 7 {
            0 => (1.0, 0.0),
            1 => (-1.0, 0.0),
            2 => (0.0, 1.0),
            3 => (0.0, -1.0),
            4 => (s, s),
            5 => (-s, s),
            6 => (s, -s),
            _ => (-s, -s),
        };
        dx * gx + dz * gz
    }

    fn hash_coords(x: i32, z: i32) -> u32 {
        let mut h = (x as u32).wrapping_mul(0x85EB_CA6B);
        h ^= (z as u32).wrapping_mul(0xC2B2_AE35);
        h ^= h >> 16;
        h = h.wrapping_mul(0x27D4_EB2D);
        h ^= h >> 15;
        h
    }
}

/// Boundary rule affecting localized elevation (e.g. flat city landing pads or deep canyons).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularBoundary {
    pub center_x: f32,
    pub center_z: f32,
    pub radius: f32,
    pub feather_width: f32,
    pub target_height: f32,
}

impl CircularBoundary {
    #[must_use]
    pub fn blend_factor(&self, x: f32, z: f32) -> f32 {
        let dx = x - self.center_x;
        let dz = z - self.center_z;
        let dist = (dx * dx + dz * dz).sqrt();

        if dist <= self.radius {
            1.0
        } else if dist >= self.radius + self.feather_width {
            0.0
        } else {
            let t = (dist - self.radius) / self.feather_width;
            // Smoothstep 1.0 - (3t^2 - 2t^3)
            1.0 - (t * t * (3.0 - 2.0 * t))
        }
    }
}

/// Procedural planetary terrain generator calculating elevations and surface normals.
#[derive(Debug, Clone, Default)]
pub struct TerrainGenerator {
    pub base_height: f32,
    pub primary_fractal: FractalNoise,
    pub detail_fractal: FractalNoise,
    pub boundaries: Vec<CircularBoundary>,
}

impl TerrainGenerator {
    /// Tatooine planetary preset: vast rolling desert dunes and flat canyon floors.
    #[must_use]
    pub fn tatooine() -> Self {
        Self {
            base_height: 10.0,
            primary_fractal: FractalNoise {
                octaves: 4,
                frequency: 0.0015,
                lacunarity: 2.1,
                persistence: 0.45,
                base_amplitude: 65.0,
                seed: 42,
            },
            detail_fractal: FractalNoise {
                octaves: 3,
                frequency: 0.01,
                lacunarity: 2.0,
                persistence: 0.5,
                base_amplitude: 4.0,
                seed: 99,
            },
            boundaries: vec![CircularBoundary {
                center_x: 0.0,
                center_z: 0.0,
                radius: 120.0,
                feather_width: 60.0,
                target_height: 8.0, // Mos Eisley spaceport flat
            }],
        }
    }

    /// Naboo planetary preset: green rolling hills and plains.
    #[must_use]
    pub fn naboo() -> Self {
        Self {
            base_height: 25.0,
            primary_fractal: FractalNoise {
                octaves: 5,
                frequency: 0.001,
                lacunarity: 2.0,
                persistence: 0.5,
                base_amplitude: 45.0,
                seed: 777,
            },
            detail_fractal: FractalNoise {
                octaves: 2,
                frequency: 0.008,
                lacunarity: 2.0,
                persistence: 0.4,
                base_amplitude: 2.5,
                seed: 888,
            },
            boundaries: Vec::new(),
        }
    }

    /// Evaluates terrain height $H(x, z)$ at any world coordinate.
    #[must_use]
    pub fn get_height(&self, x: f32, z: f32) -> f32 {
        let mut h = self.base_height;
        h += self.primary_fractal.sample(x, z);
        h += self.detail_fractal.sample(x, z);

        for boundary in &self.boundaries {
            let factor = boundary.blend_factor(x, z);
            if factor > 0.0 {
                h = h * (1.0 - factor) + boundary.target_height * factor;
            }
        }

        h
    }

    /// Computes analytic / central-difference surface normal vector at $(x, z)$.
    #[must_use]
    pub fn get_normal(&self, x: f32, z: f32) -> Vec3 {
        let eps = 0.5;
        let h_l = self.get_height(x - eps, z);
        let h_r = self.get_height(x + eps, z);
        let h_d = self.get_height(x, z - eps);
        let h_u = self.get_height(x, z + eps);

        let dx = (h_r - h_l) / (2.0 * eps);
        let dz = (h_u - h_d) / (2.0 * eps);

        Vec3::new(-dx, 1.0, -dz).normalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tatooine_height_and_boundary() {
        let generator = TerrainGenerator::tatooine();

        // Origin is inside the flat Mos Eisley boundary
        let h_origin = generator.get_height(0.0, 0.0);
        assert!(
            (h_origin - 8.0).abs() < 1e-3,
            "Origin must be flattened to target height 8.0"
        );

        // Normal at origin should point nearly straight up (flat ground)
        let norm_origin = generator.get_normal(0.0, 0.0);
        assert!((norm_origin.y - 1.0).abs() < 1e-2);

        // Outside boundary, dune fractal dominates
        let h_far = generator.get_height(1000.0, 1000.0);
        assert!(
            (h_far - 8.0).abs() > 5.0,
            "Dune terrain outside boundary should vary"
        );
    }

    #[test]
    fn test_normal_vector_normalization() {
        let generator = TerrainGenerator::naboo();
        let norm = generator.get_normal(123.4, 567.8);
        assert!((norm.length() - 1.0).abs() < 1e-4);
        assert!(norm.y > 0.0, "Surface normal Y must point upward");
    }
}
