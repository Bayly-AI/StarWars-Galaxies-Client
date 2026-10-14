// crates/swg-client-vfs/src/async_loader.rs

use crate::tre::VfsError;
use crate::vfs::VfsManager;
use bytes::Bytes;
use std::sync::Arc;

/// Asynchronous wrapper around VfsManager for multi-threaded background asset streaming.
#[derive(Clone)]
pub struct AsyncVfs {
    vfs: Arc<VfsManager>,
}

impl AsyncVfs {
    #[must_use]
    pub fn new(vfs: VfsManager) -> Self {
        Self { vfs: Arc::new(vfs) }
    }

    #[must_use]
    pub fn from_arc(vfs: Arc<VfsManager>) -> Self {
        Self { vfs }
    }

    /// Asynchronously extracts and decompresses a file on the blocking thread pool.
    pub async fn read_file_async(&self, path: &str) -> Result<Bytes, VfsError> {
        let vfs = self.vfs.clone();
        let path_str = path.to_string();

        tokio::task::spawn_blocking(move || vfs.read_file(&path_str))
            .await
            .map_err(|e| VfsError::Io(std::io::Error::other(e)))?
    }

    /// Asynchronously loads a batch of files concurrently, preserving input order.
    pub async fn read_batch_async(&self, paths: &[&str]) -> Vec<Result<Bytes, VfsError>> {
        let mut set = tokio::task::JoinSet::new();

        for (idx, &path) in paths.iter().enumerate() {
            let vfs = self.vfs.clone();
            let path_str = path.to_string();
            set.spawn_blocking(move || (idx, vfs.read_file(&path_str)));
        }

        let mut results: Vec<Option<Result<Bytes, VfsError>>> =
            (0..paths.len()).map(|_| None).collect();

        while let Some(res) = set.join_next().await {
            match res {
                Ok((idx, data)) => {
                    results[idx] = Some(data);
                }
                Err(e) => {
                    // Task panicked or cancelled
                    let err = VfsError::Io(std::io::Error::other(e));
                    for slot in results.iter_mut() {
                        if slot.is_none() {
                            *slot = Some(Err(err));
                            break;
                        }
                    }
                }
            }
        }

        results
            .into_iter()
            .map(|opt| {
                opt.unwrap_or_else(|| {
                    Err(VfsError::FileNotFound(
                        "Failed to load in batch".to_string(),
                    ))
                })
            })
            .collect()
    }

    /// Direct reference to underlying VfsManager.
    #[must_use]
    pub fn inner(&self) -> &VfsManager {
        &self.vfs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tre::TreArchiveWriter;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_async_vfs_batch_loading() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        let mut writer = TreArchiveWriter::new();
        writer.add_file("shader/default.vsh", b"VERTEX_SHADER".to_vec(), true);
        writer.add_file("shader/default.fsh", b"FRAGMENT_SHADER".to_vec(), true);
        writer.add_file("texture/noise.dds", b"NOISE_TEXTURE".to_vec(), true);
        writer.write_to_file(path).unwrap();

        let mut vfs = VfsManager::new();
        vfs.mount_path(path).unwrap();

        let async_vfs = AsyncVfs::new(vfs);

        // Single async read
        let data = async_vfs
            .read_file_async("shader/default.vsh")
            .await
            .unwrap();
        assert_eq!(data, Bytes::from_static(b"VERTEX_SHADER"));

        // Batch async read
        let batch = async_vfs
            .read_batch_async(&["shader/default.fsh", "texture/noise.dds"])
            .await;
        assert_eq!(batch.len(), 2);
        for item in batch {
            assert!(item.is_ok());
        }
    }
}
