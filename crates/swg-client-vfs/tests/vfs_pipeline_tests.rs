// crates/swg-client-vfs/tests/vfs_pipeline_tests.rs

use bytes::Bytes;
use std::time::Instant;
use swg_client_vfs::{AsyncVfs, TreArchiveWriter, VfsManager};
use tempfile::tempdir;

#[test]
fn test_multi_layer_cascading_patch_priority() {
    let dir = tempdir().unwrap();
    let base_path = dir.path().join("data_sku1_00.tre");
    let patch1_path = dir.path().join("patch_01_00.tre");
    let patch14_path = dir.path().join("patch_14_00.tre");

    // 1. Base archive
    let mut base = TreArchiveWriter::new();
    base.add_file("texture/ground.dds", b"BASE_GROUND".to_vec(), false);
    base.add_file("texture/sky.dds", b"BASE_SKY".to_vec(), false);
    base.add_file("model/creature.msh", b"BASE_CREATURE".to_vec(), true);
    base.write_to_file(&base_path).unwrap();

    // 2. Patch 01 (updates creature, adds weapon)
    let mut patch1 = TreArchiveWriter::new();
    patch1.add_file("model/creature.msh", b"PATCH_01_CREATURE".to_vec(), true);
    patch1.add_file("model/blaster.msh", b"PATCH_01_BLASTER".to_vec(), false);
    patch1.write_to_file(&patch1_path).unwrap();

    // 3. Patch 14 (updates ground, updates creature again)
    let mut patch14 = TreArchiveWriter::new();
    patch14.add_file("texture/ground.dds", b"PATCH_14_GROUND".to_vec(), true);
    patch14.add_file("model/creature.msh", b"PATCH_14_CREATURE".to_vec(), true);
    patch14.write_to_file(&patch14_path).unwrap();

    // Mount entire directory and measure time
    let mut vfs = VfsManager::new();
    let start = Instant::now();
    let count = vfs.mount_directory(dir.path()).unwrap();
    let mount_duration = start.elapsed();

    assert_eq!(count, 3);
    println!("Mounted 3 TRE archives in {mount_duration:?}");
    assert!(
        mount_duration.as_millis() < 250,
        "Mount must be under 250ms"
    );

    // Verify cascade resolution
    // Ground should be PATCH_14
    let ground = vfs.read_file("texture/ground.dds").unwrap();
    assert_eq!(ground, Bytes::from_static(b"PATCH_14_GROUND"));

    // Creature should be PATCH_14
    let creature = vfs.read_file("model/creature.msh").unwrap();
    assert_eq!(creature, Bytes::from_static(b"PATCH_14_CREATURE"));

    // Blaster should be PATCH_01
    let blaster = vfs.read_file("model/blaster.msh").unwrap();
    assert_eq!(blaster, Bytes::from_static(b"PATCH_01_BLASTER"));

    // Sky should be BASE
    let sky = vfs.read_file("texture/sky.dds").unwrap();
    assert_eq!(sky, Bytes::from_static(b"BASE_SKY"));
}

#[tokio::test]
async fn test_async_vfs_parallel_streaming() {
    let dir = tempdir().unwrap();
    let archive_path = dir.path().join("data_other_00.tre");

    let mut writer = TreArchiveWriter::new();
    for i in 0..50 {
        let name = format!("assets/sound_effect_{i:03}.wav");
        let content = format!("AUDIO_SAMPLE_DATA_PAYLOAD_FOR_EFFECT_{i}").into_bytes();
        writer.add_file(&name, content, true);
    }
    writer.write_to_file(&archive_path).unwrap();

    let mut vfs = VfsManager::new();
    vfs.mount_directory(dir.path()).unwrap();
    let async_vfs = AsyncVfs::new(vfs);

    let paths: Vec<String> = (0..50)
        .map(|i| format!("assets/sound_effect_{i:03}.wav"))
        .collect();
    let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();

    let results = async_vfs.read_batch_async(&path_refs).await;
    assert_eq!(results.len(), 50);

    for (i, res) in results.into_iter().enumerate() {
        let data = res.unwrap();
        let expected = format!("AUDIO_SAMPLE_DATA_PAYLOAD_FOR_EFFECT_{i}");
        assert_eq!(data, Bytes::from(expected));
    }
}

#[test]
fn test_retail_tre_archive_extraction() {
    let tre_path = std::path::Path::new("../../../downloads/tre_test/SWG_Files/default_patch.tre");
    if !tre_path.exists() {
        println!("Skipping retail TRE test: archive not present at {:?}", tre_path);
        return;
    }

    let archive = swg_client_vfs::TreArchive::open(tre_path).expect("Failed to open retail TRE");
    assert_eq!(archive.file_count(), 3);
    assert!(archive.contains("texture/defaultshader.dds"));
    assert!(archive.contains("texture/defaultcubemap.dds"));
    assert!(archive.contains("texture/defaulttexture.dds"));

    let entry = archive.get_entry("texture/defaultshader.dds").expect("Entry not found");
    let data = archive.read_entry(entry).expect("Failed to read DDS entry");
    assert_eq!(data.len(), 22000);
    assert_eq!(&data[0..4], b"DDS ");
}
