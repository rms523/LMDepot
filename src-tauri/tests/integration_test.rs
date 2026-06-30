use model_backup_lib::adapters::huggingface_cache::HuggingFaceCacheAdapter;
use model_backup_lib::adapters::lmstudio::LmStudioAdapter;
use model_backup_lib::adapters::SourceAdapter;
use model_backup_lib::core::copy_engine;
use model_backup_lib::core::drive_monitor::backup_layout_path;
use model_backup_lib::core::operations::{read_manifest, write_manifest};
use model_backup_lib::db::Database;
use model_backup_lib::types::ModelRecord;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn sample_model(source_root: &Path, id: &str, source: &str) -> ModelRecord {
    let model_dir = source_root.join("sample-model");
    fs::create_dir_all(&model_dir).unwrap();
    fs::write(model_dir.join("weights.gguf"), vec![0u8; 1024]).unwrap();
    let (total_bytes, file_count, files) = model_backup_lib::adapters::collect_files(&model_dir).unwrap();
    ModelRecord {
        id: id.to_string(),
        display_name: "sample-model".to_string(),
        source: source.to_string(),
        primary_path: model_dir.to_string_lossy().to_string(),
        total_bytes,
        file_count,
        scanned_at: chrono::Utc::now().to_rfc3339(),
        revision: None,
        files,
    }
}

#[test]
fn backup_and_sync_roundtrip() {
    let app_data = TempDir::new().unwrap();
    let source_root = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();

    let db = Database::open(app_data.path()).unwrap();
    let model = sample_model(source_root.path(), "lmstudio:sample-model", "lmstudio");

    let backup_path = backup_layout_path(drive_root.path(), &model);
    fs::create_dir_all(&backup_path).unwrap();

    copy_engine::copy_model_files(
        Path::new(&model.primary_path),
        &backup_path,
        &model.files,
        |_| {},
    )
    .unwrap();

    write_manifest(&backup_path, &model, false).unwrap();
    let manifest = read_manifest(&backup_path).unwrap();
    assert_eq!(manifest.model_id, model.id);
    assert_eq!(manifest.files.len(), model.files.len());

    db.upsert_models(&[model.clone()]).unwrap();
    let stored = db.list_models(None).unwrap();
    assert_eq!(stored.len(), 1);
}

#[test]
fn lmstudio_adapter_finds_models() {
    let tmp = TempDir::new().unwrap();
    let models = tmp.path().join("models").join("org").join("demo");
    fs::create_dir_all(&models).unwrap();
    fs::write(models.join("model.gguf"), b"data").unwrap();

    let adapter = LmStudioAdapter::new(Some(tmp.path().to_string_lossy().to_string()));
    let found = adapter.scan().unwrap();
    assert_eq!(found.len(), 1);
}

#[test]
fn hf_cache_adapter_finds_repo() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("models--demo--Test-Model");
    let snapshot = repo.join("snapshots").join("abc123");
    fs::create_dir_all(&snapshot).unwrap();
    fs::write(snapshot.join("model.safetensors"), b"data").unwrap();

    let adapter = HuggingFaceCacheAdapter::new(Some(tmp.path().to_string_lossy().to_string()), "unsloth");
    let found = adapter.scan().unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].display_name, "demo/Test-Model");
}
