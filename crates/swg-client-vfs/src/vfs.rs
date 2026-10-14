// crates/swg-client-vfs/src/vfs.rs

use crate::tre::{TreArchive, TreEntry, VfsError};
use bytes::Bytes;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Virtual File System managing multiple mounted .tre archives with patch cascading priority.
pub struct VfsManager {
    archives: Vec<Arc<TreArchive>>,
    /// Map of normalized file path -> (archive_index, entry)
    /// Lowest archive_index indicates highest priority.
    index: HashMap<String, (usize, TreEntry)>,
}

impl Default for VfsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VfsManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            archives: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Mounts an opened .tre archive at the next priority tier.
    /// Earlier mounted archives have higher priority than later mounted archives.
    pub fn mount(&mut self, archive: TreArchive) {
        let archive_idx = self.archives.len();
        let arc = Arc::new(archive);

        // Index entries: only insert if not already present (so earlier mounted archives win)
        for (norm_path, entry) in &arc.entries {
            self.index
                .entry(norm_path.clone())
                .or_insert_with(|| (archive_idx, entry.clone()));
        }

        self.archives.push(arc);
    }

    /// Mounts an archive from file path.
    pub fn mount_path(&mut self, path: impl AsRef<Path>) -> Result<(), VfsError> {
        let archive = TreArchive::open(path)?;
        self.mount(archive);
        Ok(())
    }

    /// Mounts an entire directory of .tre archives, sorting them by standard SWG patch priority:
    /// `patch_XX_YY.tre` (descending) -> `data_sku1_XX.tre` (descending) -> `data_other_XX.tre`.
    pub fn mount_directory(&mut self, dir: impl AsRef<Path>) -> Result<usize, VfsError> {
        let dir_path = dir.as_ref();
        if !dir_path.is_dir() {
            return Ok(0);
        }

        let mut files = Vec::new();
        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("tre"))
            {
                files.push(path);
            }
        }

        // Sort by cascading priority
        files.sort_by(|a, b| Self::compare_tre_priority(a, b));

        let count = files.len();
        for file in files {
            self.mount_path(file)?;
        }

        Ok(count)
    }

    /// Priority comparator: Higher priority archives should come FIRST (Ordering::Less).
    fn compare_tre_priority(a: &Path, b: &Path) -> Ordering {
        let name_a = a
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        let name_b = b
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        let is_patch_a = name_a.starts_with("patch");
        let is_patch_b = name_b.starts_with("patch");

        match (is_patch_a, is_patch_b) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => {
                // Reverse alphabetical so patch_14 comes before patch_01
                name_b.cmp(&name_a)
            }
        }
    }

    /// Checks if a file exists in any mounted archive.
    #[must_use]
    pub fn exists(&self, path: &str) -> bool {
        let norm = TreArchive::normalize_path(path);
        self.index.contains_key(&norm)
    }

    /// Total unique virtual files across all mounted archives.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.index.len()
    }

    /// Number of mounted archives.
    #[must_use]
    pub fn archive_count(&self) -> usize {
        self.archives.len()
    }

    /// Synchronously reads and decompresses a file from the highest-priority archive containing it.
    pub fn read_file(&self, path: &str) -> Result<Bytes, VfsError> {
        let norm = TreArchive::normalize_path(path);
        let &(archive_idx, ref entry) = self
            .index
            .get(&norm)
            .ok_or_else(|| VfsError::FileNotFound(path.to_string()))?;

        self.archives[archive_idx].read_entry(entry)
    }

    /// Resolves which archive file contains the active version of a given path.
    #[must_use]
    pub fn resolve_archive_path(&self, path: &str) -> Option<&PathBuf> {
        let norm = TreArchive::normalize_path(path);
        let &(archive_idx, _) = self.index.get(&norm)?;
        Some(&self.archives[archive_idx].path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tre::TreArchiveWriter;
    use tempfile::tempdir;

    #[test]
    fn test_patch_priority_cascading() {
        let dir = tempdir().unwrap();
        let base_path = dir.path().join("data_sku1_00.tre");
        let patch_path = dir.path().join("patch_14_00.tre");

        // Write base archive
        let mut base_writer = TreArchiveWriter::new();
        base_writer.add_file("ui/hud.dds", b"BASE_HUD".to_vec(), false);
        base_writer.add_file("terrain/tatooine.trn", b"BASE_TERRAIN".to_vec(), false);
        base_writer.write_to_file(&base_path).unwrap();

        // Write patch archive (overrides ui/hud.dds)
        let mut patch_writer = TreArchiveWriter::new();
        patch_writer.add_file("ui/hud.dds", b"PATCH_14_HUD".to_vec(), false);
        patch_writer.add_file("quest/new_quest.iff", b"NEW_QUEST".to_vec(), false);
        patch_writer.write_to_file(&patch_path).unwrap();

        let mut vfs = VfsManager::new();
        let mounted = vfs.mount_directory(dir.path()).unwrap();
        assert_eq!(mounted, 2);

        // Verify total files (ui/hud.dds deduped, total = 3: ui/hud.dds, terrain/tatooine.trn, quest/new_quest.iff)
        assert_eq!(vfs.file_count(), 3);

        // Verify patch_14 overrides base
        let hud_data = vfs.read_file("UI/HUD.DDS").unwrap();
        assert_eq!(hud_data, Bytes::from_static(b"PATCH_14_HUD"));

        // Verify base file still accessible
        let terrain_data = vfs.read_file("terrain/tatooine.trn").unwrap();
        assert_eq!(terrain_data, Bytes::from_static(b"BASE_TERRAIN"));

        // Verify patch-only file accessible
        let quest_data = vfs.read_file("quest/new_quest.iff").unwrap();
        assert_eq!(quest_data, Bytes::from_static(b"NEW_QUEST"));
    }
}
