// crates/swg-client-vfs/src/tre.rs

use bytes::Bytes;
use crc32fast::Hasher;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

pub const TRE_MAGIC_EERT: [u8; 4] = *b"EERT";
pub const TRE_MAGIC_TREE: [u8; 4] = *b"TREE";
pub const TRE_VERSION_0005: u32 = 0x35303030; // '0005'
pub const TRE_VERSION_1: u32 = 1;

pub const COMPRESSION_NONE: u32 = 0;
pub const COMPRESSION_ZLIB: u32 = 2;

#[derive(Error, Debug)]
pub enum VfsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid TRE archive magic: expected 'EERT' or 'TREE', got {0:?}")]
    InvalidMagic([u8; 4]),
    #[error("Invalid TRE version: {0:#x}")]
    InvalidVersion(u32),
    #[error("TOC offset {0} or size {1} exceeds file length {2}")]
    InvalidTocBounds(u64, u64, u64),
    #[error("Checksum mismatch for entry '{0}': expected {1:#x}, computed {2:#x}")]
    ChecksumMismatch(String, u32, u32),
    #[error("Decompression failed for entry '{0}': {1}")]
    DecompressionFailed(String, String),
    #[error("File not found in VFS: '{0}'")]
    FileNotFound(String),
}

/// A single file entry within a .tre archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreEntry {
    pub name: String,
    pub checksum: u32,
    pub uncompressed_size: u32,
    pub data_offset: u32,
    pub compression_type: u32,
    pub compressed_size: u32,
}

/// A memory-mapped .tre archive.
pub struct TreArchive {
    pub path: PathBuf,
    mmap: Arc<Mmap>,
    pub(crate) entries: HashMap<String, TreEntry>,
}

impl TreArchive {
    /// Opens and memory-maps a .tre archive, parsing its header and TOC.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, VfsError> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf)?;
        let mmap = unsafe { Mmap::map(&file)? };

        if mmap.len() < 32 {
            return Err(VfsError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "File too small to contain a valid TRE header",
            )));
        }

        let magic: [u8; 4] = [mmap[0], mmap[1], mmap[2], mmap[3]];
        if magic != TRE_MAGIC_EERT && magic != TRE_MAGIC_TREE {
            return Err(VfsError::InvalidMagic(magic));
        }

        let version = u32::from_le_bytes([mmap[4], mmap[5], mmap[6], mmap[7]]);
        let file_count = u32::from_le_bytes([mmap[8], mmap[9], mmap[10], mmap[11]]) as usize;
        let toc_offset = u32::from_le_bytes([mmap[12], mmap[13], mmap[14], mmap[15]]) as usize;
        let toc_compressed_size =
            u32::from_le_bytes([mmap[16], mmap[17], mmap[18], mmap[19]]) as usize;
        let toc_uncompressed_size =
            u32::from_le_bytes([mmap[20], mmap[21], mmap[22], mmap[23]]) as usize;

        let toc_data = if toc_compressed_size > 0 {
            // Decompress TOC
            let end = toc_offset + toc_compressed_size;
            if end > mmap.len() {
                return Err(VfsError::InvalidTocBounds(
                    toc_offset as u64,
                    toc_compressed_size as u64,
                    mmap.len() as u64,
                ));
            }
            let mut decoder = ZlibDecoder::new(&mmap[toc_offset..end]);
            let mut decompressed = Vec::with_capacity(toc_uncompressed_size);
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| VfsError::DecompressionFailed("TOC".to_string(), e.to_string()))?;
            decompressed
        } else {
            let end = toc_offset + toc_uncompressed_size;
            if end > mmap.len() {
                return Err(VfsError::InvalidTocBounds(
                    toc_offset as u64,
                    toc_uncompressed_size as u64,
                    mmap.len() as u64,
                ));
            }
            mmap[toc_offset..end].to_vec()
        };

        // Parse TOC entries
        let entry_size = 24;
        let entries_total_bytes = file_count * entry_size;
        if toc_data.len() < entries_total_bytes {
            return Err(VfsError::InvalidTocBounds(
                0,
                entries_total_bytes as u64,
                toc_data.len() as u64,
            ));
        }

        let names_block = &toc_data[entries_total_bytes..];
        let mut entries = HashMap::with_capacity(file_count);

        for i in 0..file_count {
            let offset = i * entry_size;
            let checksum = u32::from_le_bytes([
                toc_data[offset],
                toc_data[offset + 1],
                toc_data[offset + 2],
                toc_data[offset + 3],
            ]);
            let uncompressed_size = u32::from_le_bytes([
                toc_data[offset + 4],
                toc_data[offset + 5],
                toc_data[offset + 6],
                toc_data[offset + 7],
            ]);
            let data_offset = u32::from_le_bytes([
                toc_data[offset + 8],
                toc_data[offset + 9],
                toc_data[offset + 10],
                toc_data[offset + 11],
            ]);
            let compression_type = u32::from_le_bytes([
                toc_data[offset + 12],
                toc_data[offset + 13],
                toc_data[offset + 14],
                toc_data[offset + 15],
            ]);
            let compressed_size = u32::from_le_bytes([
                toc_data[offset + 16],
                toc_data[offset + 17],
                toc_data[offset + 18],
                toc_data[offset + 19],
            ]);
            let name_offset = u32::from_le_bytes([
                toc_data[offset + 20],
                toc_data[offset + 21],
                toc_data[offset + 22],
                toc_data[offset + 23],
            ]) as usize;

            // Extract null-terminated name
            if name_offset < names_block.len() {
                let name_slice = &names_block[name_offset..];
                let end = name_slice
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(name_slice.len());
                let raw_name = String::from_utf8_lossy(&name_slice[..end]).to_string();
                let normalized = Self::normalize_path(&raw_name);

                entries.insert(
                    normalized,
                    TreEntry {
                        name: raw_name,
                        checksum,
                        uncompressed_size,
                        data_offset,
                        compression_type,
                        compressed_size,
                    },
                );
            }
        }

        let _ = version;

        Ok(Self {
            path: path_buf,
            mmap: Arc::new(mmap),
            entries,
        })
    }

    /// Normalizes path for case-insensitive and slash-agnostic lookup.
    #[must_use]
    pub fn normalize_path(path: &str) -> String {
        path.trim().replace('\\', "/").to_lowercase()
    }

    /// Checks if a file exists in this archive.
    #[must_use]
    pub fn contains(&self, path: &str) -> bool {
        self.entries.contains_key(&Self::normalize_path(path))
    }

    /// Retrieves entry metadata by path.
    #[must_use]
    pub fn get_entry(&self, path: &str) -> Option<&TreEntry> {
        self.entries.get(&Self::normalize_path(path))
    }

    /// Number of files in the archive.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.entries.len()
    }

    /// Reads and decompresses file data from the memory map.
    pub fn read_entry(&self, entry: &TreEntry) -> Result<Bytes, VfsError> {
        let start = entry.data_offset as usize;
        let end = start + entry.compressed_size as usize;

        if end > self.mmap.len() {
            return Err(VfsError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Entry {} data bounds exceed archive length", entry.name),
            )));
        }

        let raw_slice = &self.mmap[start..end];
        let decompressed = match entry.compression_type {
            COMPRESSION_NONE => raw_slice.to_vec(),
            COMPRESSION_ZLIB => {
                let mut decoder = ZlibDecoder::new(raw_slice);
                let mut buffer = Vec::with_capacity(entry.uncompressed_size as usize);
                decoder.read_to_end(&mut buffer).map_err(|e| {
                    VfsError::DecompressionFailed(entry.name.clone(), e.to_string())
                })?;
                buffer
            }
            other => {
                return Err(VfsError::DecompressionFailed(
                    entry.name.clone(),
                    format!("Unsupported compression type: {other}"),
                ));
            }
        };

        // Verify CRC32 checksum
        let mut hasher = Hasher::new();
        hasher.update(&decompressed);
        let computed = hasher.finalize();

        if computed != entry.checksum {
            return Err(VfsError::ChecksumMismatch(
                entry.name.clone(),
                entry.checksum,
                computed,
            ));
        }

        Ok(Bytes::from(decompressed))
    }
}

/// Helper for building and writing valid .tre archives for tests and asset packages.
pub struct TreArchiveWriter {
    files: Vec<(String, Vec<u8>, bool)>, // (name, data, compress)
}

impl Default for TreArchiveWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl TreArchiveWriter {
    #[must_use]
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add_file(&mut self, name: &str, data: Vec<u8>, compress: bool) {
        self.files.push((name.to_string(), data, compress));
    }

    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<(), VfsError> {
        let file = File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);

        let file_count = self.files.len() as u32;
        let header_size = 32u32;

        // Collect compressed payloads and metadata
        let mut data_payloads = Vec::new();
        let mut entry_meta = Vec::new();
        let mut names_block = Vec::new();

        let mut current_data_offset = header_size;

        for (name, data, compress) in &self.files {
            let mut hasher = Hasher::new();
            hasher.update(data);
            let checksum = hasher.finalize();

            let (comp_type, comp_data) = if *compress {
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(data)?;
                let compressed = encoder.finish()?;
                (COMPRESSION_ZLIB, compressed)
            } else {
                (COMPRESSION_NONE, data.clone())
            };

            let name_offset = names_block.len() as u32;
            names_block.extend_from_slice(name.as_bytes());
            names_block.push(0); // Null terminator

            let comp_size = comp_data.len() as u32;
            entry_meta.push((
                checksum,
                data.len() as u32,
                current_data_offset,
                comp_type,
                comp_size,
                name_offset,
            ));

            current_data_offset += comp_size;
            data_payloads.push(comp_data);
        }

        let toc_offset = current_data_offset;

        // Build uncompressed TOC buffer
        let mut toc_buffer = Vec::new();
        for (checksum, uncomp_size, offset, comp_type, comp_size, name_offset) in entry_meta {
            toc_buffer.extend_from_slice(&checksum.to_le_bytes());
            toc_buffer.extend_from_slice(&uncomp_size.to_le_bytes());
            toc_buffer.extend_from_slice(&offset.to_le_bytes());
            toc_buffer.extend_from_slice(&comp_type.to_le_bytes());
            toc_buffer.extend_from_slice(&comp_size.to_le_bytes());
            toc_buffer.extend_from_slice(&name_offset.to_le_bytes());
        }
        toc_buffer.extend_from_slice(&names_block);

        let toc_uncompressed_size = toc_buffer.len() as u32;

        // Compress TOC
        let mut toc_encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        toc_encoder.write_all(&toc_buffer)?;
        let toc_compressed = toc_encoder.finish()?;
        let toc_compressed_size = toc_compressed.len() as u32;

        // Write header
        writer.write_all(&TRE_MAGIC_EERT)?;
        writer.write_all(&TRE_VERSION_0005.to_le_bytes())?;
        writer.write_all(&file_count.to_le_bytes())?;
        writer.write_all(&toc_offset.to_le_bytes())?;
        writer.write_all(&toc_compressed_size.to_le_bytes())?;
        writer.write_all(&toc_uncompressed_size.to_le_bytes())?;
        writer.write_all(&0u32.to_le_bytes())?; // block size
        writer.write_all(&0u32.to_le_bytes())?; // reserved

        // Write data payloads
        for payload in data_payloads {
            writer.write_all(&payload)?;
        }

        // Write TOC
        writer.write_all(&toc_compressed)?;
        writer.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_tre_archive_roundtrip() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        let mut writer = TreArchiveWriter::new();
        writer.add_file(
            "texture/terrain/ground.dds",
            b"RAW_UNCOMPRESSED_TEXTURE_BYTES_12345".to_vec(),
            false,
        );
        writer.add_file(
            "model/ship/xwing.msh",
            b"COMPRESSED_XWING_MESH_DATA_REPEATING_AAAAABBBBBCCCCCDDDDDEEEEE".to_vec(),
            true,
        );

        writer.write_to_file(path).unwrap();

        // Open archive
        let archive = TreArchive::open(path).unwrap();
        assert_eq!(archive.file_count(), 2);

        // Read uncompressed entry
        let entry1 = archive.get_entry("texture/terrain/ground.dds").unwrap();
        let data1 = archive.read_entry(entry1).unwrap();
        assert_eq!(
            data1,
            Bytes::from_static(b"RAW_UNCOMPRESSED_TEXTURE_BYTES_12345")
        );

        // Case-insensitivity and slash normalization
        let entry2 = archive.get_entry(r"MODEL\SHIP\XWING.MSH").unwrap();
        let data2 = archive.read_entry(entry2).unwrap();
        assert_eq!(
            data2,
            Bytes::from_static(b"COMPRESSED_XWING_MESH_DATA_REPEATING_AAAAABBBBBCCCCCDDDDDEEEEE")
        );
    }
}
