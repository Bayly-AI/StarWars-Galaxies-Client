// crates/swg-client-terrain/src/lod.rs

use serde::{Deserialize, Serialize};

/// Discrete Level of Detail tier for terrain chunks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TerrainLod {
    /// 64 quads per edge (1m spacing, 4225 vertices)
    Lod0 = 0,
    /// 32 quads per edge (2m spacing, 1089 vertices)
    Lod1 = 1,
    /// 16 quads per edge (4m spacing, 289 vertices)
    Lod2 = 2,
    /// 8 quads per edge (8m spacing, 81 vertices)
    Lod3 = 3,
}

impl TerrainLod {
    #[must_use]
    pub fn segments_per_edge(self) -> usize {
        match self {
            TerrainLod::Lod0 => 64,
            TerrainLod::Lod1 => 32,
            TerrainLod::Lod2 => 16,
            TerrainLod::Lod3 => 8,
        }
    }

    #[must_use]
    pub fn step_meters(self) -> f32 {
        64.0 / self.segments_per_edge() as f32
    }

    #[must_use]
    pub fn from_distance(dist: f32) -> Self {
        if dist < 128.0 {
            TerrainLod::Lod0
        } else if dist < 256.0 {
            TerrainLod::Lod1
        } else if dist < 512.0 {
            TerrainLod::Lod2
        } else {
            TerrainLod::Lod3
        }
    }
}

/// Boundary neighboring LOD states (North +Z, South -Z, East +X, West -X) for crack-free stitching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NeighborLods {
    pub north: Option<TerrainLod>,
    pub south: Option<TerrainLod>,
    pub east: Option<TerrainLod>,
    pub west: Option<TerrainLod>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_distances_and_segments() {
        assert_eq!(TerrainLod::from_distance(50.0), TerrainLod::Lod0);
        assert_eq!(TerrainLod::from_distance(150.0), TerrainLod::Lod1);
        assert_eq!(TerrainLod::from_distance(300.0), TerrainLod::Lod2);
        assert_eq!(TerrainLod::from_distance(800.0), TerrainLod::Lod3);

        assert_eq!(TerrainLod::Lod0.segments_per_edge(), 64);
        assert_eq!(TerrainLod::Lod3.segments_per_edge(), 8);
        assert_eq!(TerrainLod::Lod0.step_meters(), 1.0);
        assert_eq!(TerrainLod::Lod3.step_meters(), 8.0);
    }
}
