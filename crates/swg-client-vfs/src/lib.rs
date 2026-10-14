// crates/swg-client-vfs/src/lib.rs

pub mod async_loader;
pub mod tre;
pub mod vfs;

pub use async_loader::AsyncVfs;
pub use tre::{
    COMPRESSION_NONE, COMPRESSION_ZLIB, TRE_MAGIC_EERT, TRE_MAGIC_TREE, TRE_VERSION_1,
    TRE_VERSION_0005, TreArchive, TreArchiveWriter, TreEntry, VfsError,
};
pub use vfs::VfsManager;
