// crates/swg-client-terrain/src/lib.rs

pub mod chunk;
pub mod height;
pub mod lod;

pub use chunk::{CHUNK_SIZE_METERS, ChunkGridManager, TerrainChunkMesh, TerrainVertex};
pub use height::{CircularBoundary, FractalNoise, TerrainError, TerrainGenerator};
pub use lod::{NeighborLods, TerrainLod};
