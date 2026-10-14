# Playbook: Issue 02 — VFS: Memory-Mapped .tre Archive Virtual File System & Priority Cascading Loader (`swg-client-vfs`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#2  
> **Branch:** `enhancement/2-vfs-tre-memory-mapped-archive`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement high-performance client-side asset streaming in `crates/swg-client-vfs`:

1. **Memory-Mapped .tre Archive Reader (`tre.rs`):**
   - Memory-map `.tre` archives via `memmap2::Mmap` for instant zero-copy header and TOC ingestion.
   - Support both compressed (zlib Deflate) and raw uncompressed entry payloads.
   - CRC32 integrity validation via `crc32fast`.
   - Complete `.tre` serializer / writer for test asset bundling and validation.

2. **Cascading Patch Priority Virtual File System (`vfs.rs`):**
   - Hierarchical mount table supporting reverse-patch priority cascading (`patch_14_00.tre` > `patch_00.tre` > `data_sku1_00.tre`).
   - Case-insensitive normalized path indexing with $O(1)$ lookups.
   - Mount full archive collections in under 250ms.

3. **Asynchronous Parallel Decompression (`async_loader.rs`):**
   - Multi-threaded parallel extraction using `tokio` and worker thread offloading (`spawn_blocking`).
   - Thread-safe caching and extraction pipelines for textures, meshes, and shader assets.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Memory-maps `.tre` archives with zero allocation on index read.
- [x] Cascading patch loader correctly resolves higher-priority archive files over base files.
- [x] Asynchronously decompresses and validates asset payloads with CRC32 verification.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all gates.
