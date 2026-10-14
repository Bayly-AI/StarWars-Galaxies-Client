// crates/swg-client-vfs/src/extractor.rs

use crate::tre::{TreArchive, VfsError};
use crate::vfs::VfsManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Categories of assets cataloged across SWG game archives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetCategory {
    Terrain,
    Shader,
    Texture,
    Mesh,
    Animation,
    Palette,
    UserInterface,
    Audio,
    Other,
}

impl AssetCategory {
    #[must_use]
    pub fn categorize(path: &str) -> Self {
        let lower = path.to_lowercase();
        if lower.ends_with(".trn") || lower.contains("terrain/") {
            Self::Terrain
        } else if lower.ends_with(".sht") || lower.contains("shader/") {
            Self::Shader
        } else if lower.ends_with(".dds") || lower.ends_with(".tga") {
            if lower.contains("ui/") || lower.contains("interface/") {
                Self::UserInterface
            } else {
                Self::Texture
            }
        } else if lower.ends_with(".msh") || lower.ends_with(".skm") || lower.ends_with(".sat") {
            Self::Mesh
        } else if lower.ends_with(".ans") || lower.contains("animation/") {
            Self::Animation
        } else if lower.ends_with(".pal") || lower.contains("palette/") {
            Self::Palette
        } else if lower.ends_with(".wav") || lower.ends_with(".mp3") || lower.contains("sound/") {
            Self::Audio
        } else if lower.contains("ui/") || lower.contains("ui_") {
            Self::UserInterface
        } else {
            Self::Other
        }
    }
}

/// A statistical manifest of assets discovered and verified across mounted archives.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssetManifest {
    pub total_files: usize,
    pub category_counts: HashMap<AssetCategory, usize>,
    pub mounted_archives: Vec<PathBuf>,
}

impl AssetManifest {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_file(&mut self, path: &str) {
        self.total_files += 1;
        let cat = AssetCategory::categorize(path);
        *self.category_counts.entry(cat).or_insert(0) += 1;
    }

    #[must_use]
    pub fn get_count(&self, cat: AssetCategory) -> usize {
        self.category_counts.get(&cat).copied().unwrap_or(0)
    }
}

/// Ingestion and cataloging utility for Star Wars Galaxies client data assets.
pub struct AssetExtractor;

impl AssetExtractor {
    /// Discovers all .tre archives within a directory, mounts them into VfsManager,
    /// and generates a comprehensive AssetManifest.
    pub fn ingest_directory(
        dir_path: impl AsRef<Path>,
        vfs: &mut VfsManager,
    ) -> Result<AssetManifest, VfsError> {
        let dir = dir_path.as_ref();
        let mut manifest = AssetManifest::new();

        if !dir.is_dir() {
            return Ok(manifest);
        }

        // Mount all archives using priority cascade
        vfs.mount_directory(dir)?;

        // Discover and categorize all entries across mounted archives
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("tre"))
            {
                manifest.mounted_archives.push(path.clone());
                if let Ok(archive) = TreArchive::open(&path) {
                    for entry_name in archive.entries.keys() {
                        manifest.record_file(entry_name);
                    }
                }
            }
        }

        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tre::TreArchiveWriter;
    use tempfile::tempdir;

    #[test]
    fn test_asset_categorization() {
        assert_eq!(
            AssetCategory::categorize("terrain/tatooine.trn"),
            AssetCategory::Terrain
        );
        assert_eq!(
            AssetCategory::categorize("shader/vehicle/xwing.sht"),
            AssetCategory::Shader
        );
        assert_eq!(
            AssetCategory::categorize("texture/ground_sand.dds"),
            AssetCategory::Texture
        );
        assert_eq!(
            AssetCategory::categorize("appearance/human_m.msh"),
            AssetCategory::Mesh
        );
        assert_eq!(
            AssetCategory::categorize("appearance/animation/run_fwd.ans"),
            AssetCategory::Animation
        );
        assert_eq!(
            AssetCategory::categorize("palette/human_skin.pal"),
            AssetCategory::Palette
        );
        assert_eq!(
            AssetCategory::categorize("ui/radar_bezel.dds"),
            AssetCategory::UserInterface
        );
        assert_eq!(
            AssetCategory::categorize("sound/blaster_e11.wav"),
            AssetCategory::Audio
        );
    }

    #[test]
    fn test_asset_manifest_ingestion() {
        let dir = tempdir().unwrap();
        let tre_path = dir.path().join("data_sku1_00.tre");

        let mut writer = TreArchiveWriter::new();
        writer.add_file("terrain/tatooine.trn", b"TERRAIN".to_vec(), false);
        writer.add_file("texture/sand.dds", b"SAND".to_vec(), false);
        writer.add_file("appearance/xwing.msh", b"XWING".to_vec(), false);
        writer.add_file("palette/skin.pal", b"PALETTE".to_vec(), false);
        writer.add_file("ui/hud.dds", b"HUD".to_vec(), false);
        writer.add_file("sound/fire.wav", b"AUDIO".to_vec(), false);
        writer.write_to_file(&tre_path).unwrap();

        let mut vfs = VfsManager::new();
        let manifest = AssetExtractor::ingest_directory(dir.path(), &mut vfs).unwrap();

        assert_eq!(manifest.total_files, 6);
        assert_eq!(manifest.get_count(AssetCategory::Terrain), 1);
        assert_eq!(manifest.get_count(AssetCategory::Texture), 1);
        assert_eq!(manifest.get_count(AssetCategory::Mesh), 1);
        assert_eq!(manifest.get_count(AssetCategory::Palette), 1);
        assert_eq!(manifest.get_count(AssetCategory::UserInterface), 1);
        assert_eq!(manifest.get_count(AssetCategory::Audio), 1);
        assert_eq!(manifest.mounted_archives.len(), 1);
    }
}
