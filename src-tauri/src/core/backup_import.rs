use crate::adapters::{
    huggingface_cache::HuggingFaceCacheAdapter, jan::JanAdapter, lmstudio::LmStudioAdapter,
    ollama::OllamaAdapter, omlx::OmlxAdapter, SourceAdapter,
};
use crate::core::drive_monitor::{ensure_drive_mounted, enrich_drives};
use crate::core::operations::read_manifest;
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::types::{
    AppSettings, BackupManifest, ImportFromDriveResult, ModelFileRecord, ModelRecord,
};
use chrono::Utc;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

const MANIFEST_NAME: &str = "model.manifest.json";

/// Walk a backup drive root and find every `model.manifest.json`.
pub fn scan_drive_for_manifests(drive_root: &Path) -> AppResult<Vec<PathBuf>> {
    if !drive_root.is_dir() {
        return Err(AppError::msg(format!(
            "Drive path is not a directory: {}",
            drive_root.display()
        )));
    }

    let mut manifests = Vec::new();
    for entry in WalkDir::new(drive_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name() != MANIFEST_NAME {
            continue;
        }
        manifests.push(entry.path().parent().unwrap().to_path_buf());
    }
    manifests.sort();
    manifests.dedup();
    Ok(manifests)
}

fn normalize_source(source: &str) -> String {
    if source == "unsloth" {
        "huggingface".to_string()
    } else {
        source.to_string()
    }
}

fn hf_repo_dir_name(display_name: &str) -> String {
    let parts: Vec<&str> = display_name.splitn(2, '/').collect();
    if parts.len() == 2 {
        format!("models--{}--{}", parts[0], parts[1].replace('/', "--"))
    } else {
        format!("models--{}", display_name.replace('/', "--"))
    }
}

fn extract_hf_revision(source_path: &str) -> Option<String> {
    let path = Path::new(source_path);
    let components: Vec<_> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    for (i, part) in components.iter().enumerate() {
        if *part == "snapshots" {
            return components.get(i + 1).map(|s| s.to_string());
        }
    }
    None
}

/// Map a manifest's original `source_path` to a sensible local path on this machine.
pub fn resolve_restore_path(manifest: &BackupManifest, settings: &AppSettings) -> AppResult<String> {
    let source_path = Path::new(&manifest.source_path);
    if source_path.exists() {
        return Ok(manifest.source_path.clone());
    }

    let source = normalize_source(&manifest.source);
    match source.as_str() {
        "lmstudio" => {
            let adapter = LmStudioAdapter::new(settings.lmstudio_path_override.clone());
            let roots = adapter.resolve_roots()?;
            let models_root = roots
                .into_iter()
                .next()
                .ok_or_else(|| AppError::msg("Could not resolve LM Studio models directory"))?;
            Ok(models_root
                .join(&manifest.display_name)
                .to_string_lossy()
                .to_string())
        }
        "huggingface" => {
            let adapter = HuggingFaceCacheAdapter::new(settings.hf_cache_path_override.clone());
            let cache_root = adapter
                .resolve_roots()?
                .into_iter()
                .next()
                .ok_or_else(|| AppError::msg("Could not resolve Hugging Face cache"))?;
            let repo_dir = cache_root.join(hf_repo_dir_name(&manifest.display_name));
            let revision = extract_hf_revision(&manifest.source_path)
                .filter(|r| !r.is_empty())
                .unwrap_or_else(|| "restored".to_string());
            Ok(repo_dir
                .join("snapshots")
                .join(revision)
                .to_string_lossy()
                .to_string())
        }
        "omlx" => remapped_under_roots(
            OmlxAdapter::new(settings.omlx_path_override.clone()).resolve_roots()?,
            &manifest.display_name,
        ),
        "ollama" => remapped_under_roots(
            OllamaAdapter::new(settings.ollama_models_override.clone()).resolve_roots()?,
            &manifest.display_name,
        ),
        "jan" => remapped_under_roots(
            JanAdapter::new(settings.jan_data_override.clone()).resolve_roots()?,
            &manifest.display_name,
        ),
        _ => Ok(manifest.source_path.clone()),
    }
}

fn remapped_under_roots(roots: Vec<PathBuf>, display_name: &str) -> AppResult<String> {
    let root = roots
        .into_iter()
        .next()
        .ok_or_else(|| AppError::msg("Could not resolve model directory for source"))?;
    Ok(root
        .join(display_name)
        .to_string_lossy()
        .to_string())
}

fn model_from_manifest(manifest: &BackupManifest, primary_path: &str) -> ModelRecord {
    let files: Vec<ModelFileRecord> = manifest
        .files
        .iter()
        .map(|f| ModelFileRecord {
            relative_path: f.relative_path.clone(),
            size: f.size,
            modified_at: f.modified_at,
        })
        .collect();
    let total_bytes: u64 = files.iter().map(|f| f.size).sum();
    ModelRecord {
        id: manifest.model_id.clone(),
        display_name: manifest.display_name.clone(),
        source: normalize_source(&manifest.source),
        primary_path: primary_path.to_string(),
        total_bytes,
        file_count: files.len() as u32,
        scanned_at: Utc::now().to_rfc3339(),
        revision: extract_hf_revision(&manifest.source_path),
        files,
    }
}

enum ImportAction {
    Imported,
    LinkedOnly,
    Skipped,
}

fn import_one_backup(
    db: &Database,
    drive_id: &str,
    backup_dir: &Path,
    settings: &AppSettings,
) -> AppResult<ImportAction> {
    let manifest = read_manifest(backup_dir)?;
    if manifest.files.is_empty() {
        return Ok(ImportAction::Skipped);
    }

    let backup_path = backup_dir.to_string_lossy().to_string();
    let local_path = resolve_restore_path(&manifest, settings)?;
    let imported_model = model_from_manifest(&manifest, &local_path);

    if let Some(existing) = db.get_model(&manifest.model_id)? {
        if existing.source_present {
            db.upsert_model_backup(
                &Uuid::new_v4().to_string(),
                &manifest.model_id,
                drive_id,
                &backup_path,
                "backed_up",
            )?;
            return Ok(ImportAction::LinkedOnly);
        }
        db.upsert_models(&[imported_model])?;
    } else {
        db.upsert_models(&[imported_model])?;
    }

    db.upsert_model_backup(
        &Uuid::new_v4().to_string(),
        &manifest.model_id,
        drive_id,
        &backup_path,
        "backed_up",
    )?;
    Ok(ImportAction::Imported)
}

pub fn import_from_drive(db: &Database, drive_id: &str) -> AppResult<ImportFromDriveResult> {
    let drive = db
        .get_backup_drive(drive_id)?
        .ok_or_else(|| AppError::msg("Backup drive not found"))?;
    ensure_drive_mounted(&drive)?;

    let settings = db.get_settings()?;
    let backup_dirs = scan_drive_for_manifests(Path::new(&drive.root_path))?;

    let mut imported_count = 0u32;
    let mut linked_count = 0u32;
    let mut skipped_count = 0u32;
    let mut errors: Vec<String> = Vec::new();

    for backup_dir in backup_dirs {
        match import_one_backup(db, drive_id, &backup_dir, &settings) {
            Ok(ImportAction::Imported) => imported_count += 1,
            Ok(ImportAction::LinkedOnly) => linked_count += 1,
            Ok(ImportAction::Skipped) => skipped_count += 1,
            Err(e) => errors.push(format!(
                "{}: {}",
                backup_dir.file_name().unwrap_or_default().to_string_lossy(),
                e
            )),
        }
    }

    let models = db.list_models(None)?;
    let message = if errors.is_empty() {
        if imported_count + linked_count == 0 {
            "No backups found on this drive".to_string()
        } else {
            format!(
                "Imported {imported_count} model(s), linked {linked_count} existing local model(s)"
            )
        }
    } else {
        format!(
            "Imported {imported_count}, linked {linked_count}, {} error(s)",
            errors.len()
        )
    };

    Ok(ImportFromDriveResult {
        drive_id: drive_id.to_string(),
        imported_count,
        linked_count,
        skipped_count,
        error_count: errors.len() as u32,
        errors,
        message,
        models,
    })
}

pub fn import_from_all_mounted_drives(db: &Database) -> AppResult<ImportFromDriveResult> {
    let drives = enrich_drives(db.list_backup_drives()?);
    let mounted: Vec<_> = drives.into_iter().filter(|d| d.is_mounted).collect();
    if mounted.is_empty() {
        return Err(AppError::msg("No mounted backup drives"));
    }

    let mut total_imported = 0u32;
    let mut total_linked = 0u32;
    let mut total_skipped = 0u32;
    let mut total_errors = 0u32;
    let mut all_errors = Vec::new();

    for drive in &mounted {
        match import_from_drive(db, &drive.id) {
            Ok(result) => {
                total_imported += result.imported_count;
                total_linked += result.linked_count;
                total_skipped += result.skipped_count;
                total_errors += result.error_count;
                for err in result.errors {
                    all_errors.push(format!("{}: {err}", drive.label));
                }
            }
            Err(e) => all_errors.push(format!("{}: {e}", drive.label)),
        }
    }

    let models = db.list_models(None)?;
    let message = if total_imported + total_linked == 0 && all_errors.is_empty() {
        "No backups found on mounted drives".to_string()
    } else {
        format!(
            "Imported {total_imported} model(s), linked {total_linked} existing local model(s) from {} drive(s)",
            mounted.len()
        )
    };

    Ok(ImportFromDriveResult {
        drive_id: mounted
            .first()
            .map(|d| d.id.clone())
            .unwrap_or_default(),
        imported_count: total_imported,
        linked_count: total_linked,
        skipped_count: total_skipped,
        error_count: total_errors,
        errors: all_errors,
        message,
        models,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::operations::write_manifest;
    use std::fs;
    use tempfile::TempDir;

    fn sample_manifest(model_id: &str, source_path: &str) -> BackupManifest {
        BackupManifest {
            model_id: model_id.to_string(),
            display_name: "author/demo-model".to_string(),
            source: "lmstudio".to_string(),
            source_path: source_path.to_string(),
            backup_version: 1,
            created_at: Utc::now().to_rfc3339(),
            files: vec![crate::types::ManifestFileEntry {
                relative_path: "model.gguf".to_string(),
                size: 128,
                modified_at: 0,
                sha256: None,
            }],
        }
    }

    #[test]
    fn scan_finds_nested_manifests() {
        let drive = TempDir::new().unwrap();
        let backup = drive.path().join("lmstudio").join("author").join("demo");
        fs::create_dir_all(&backup).unwrap();
        fs::write(backup.join("model.gguf"), b"x").unwrap();
        let manifest = sample_manifest("lmstudio:author/demo", "/old/models/author/demo");
        write_manifest(&backup, &model_from_manifest(&manifest, "/old/models/author/demo"), false)
            .unwrap();

        let found = scan_drive_for_manifests(drive.path()).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("author/demo"));
    }

    #[test]
    fn remaps_lmstudio_path_to_local_models_dir() {
        let tmp = TempDir::new().unwrap();
        let models_root = tmp.path().join("models");
        fs::create_dir_all(&models_root).unwrap();
        let manifest = sample_manifest(
            "lmstudio:author/demo-model",
            "/Users/old/.lmstudio/models/author/demo-model",
        );
        let settings = AppSettings {
            lmstudio_path_override: Some(tmp.path().to_string_lossy().to_string()),
            ..Default::default()
        };
        let path = resolve_restore_path(&manifest, &settings).unwrap();
        assert!(path.ends_with("author/demo-model"));
        assert!(path.contains("models"));
    }

    #[test]
    fn remaps_hf_path_with_snapshot_revision() {
        let tmp = TempDir::new().unwrap();
        let manifest = BackupManifest {
            model_id: "hf:demo/Test-Model".to_string(),
            display_name: "demo/Test-Model".to_string(),
            source: "huggingface".to_string(),
            source_path: "/old/hub/models--demo--Test-Model/snapshots/abc123".to_string(),
            backup_version: 1,
            created_at: Utc::now().to_rfc3339(),
            files: vec![crate::types::ManifestFileEntry {
                relative_path: "model.safetensors".to_string(),
                size: 1,
                modified_at: 0,
                sha256: None,
            }],
        };
        let settings = AppSettings {
            hf_cache_path_override: Some(tmp.path().to_string_lossy().to_string()),
            ..Default::default()
        };
        let path = resolve_restore_path(&manifest, &settings).unwrap();
        assert!(path.contains("models--demo--Test-Model"));
        assert!(path.ends_with("snapshots/abc123"));
    }

    #[test]
    fn import_registers_model_and_backup_link() {
        let app_data = TempDir::new().unwrap();
        let drive_root = TempDir::new().unwrap();
        let db = Database::open(app_data.path()).unwrap();

        let backup = drive_root
            .path()
            .join("lmstudio")
            .join("author")
            .join("demo");
        fs::create_dir_all(&backup).unwrap();
        fs::write(backup.join("model.gguf"), b"x").unwrap();
        let manifest = sample_manifest(
            "lmstudio:author/demo",
            "/nonexistent/.lmstudio/models/author/demo",
        );
        write_manifest(&backup, &model_from_manifest(&manifest, &manifest.source_path), false)
            .unwrap();

        let drive = crate::types::BackupDrive {
            id: "drive-1".to_string(),
            label: "Test".to_string(),
            root_path: drive_root.path().to_string_lossy().to_string(),
            volume_id: None,
            is_default: true,
            last_seen_at: None,
            is_mounted: true,
        };
        db.add_backup_drive(&drive).unwrap();

        let result = import_from_drive(&db, "drive-1").unwrap();
        assert_eq!(result.imported_count, 1);
        assert_eq!(result.models.len(), 1);
        assert!(!result.models[0].source_present);
        assert_eq!(result.models[0].backups.len(), 1);
    }
}
