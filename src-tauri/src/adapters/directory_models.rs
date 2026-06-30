use super::{collect_files, is_model_file};
use crate::error::AppResult;
use crate::types::ModelRecord;
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const SKIP_DIRS: &[&str] = &["_active", "_archive", ".cache", ".locks"];

fn dir_has_model_files(path: &Path) -> bool {
    WalkDir::new(path)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .any(|e| {
            if !e.file_type().is_file() && !e.file_type().is_symlink() {
                return false;
            }
            let p = e.path();
            if e.file_type().is_symlink() && p.is_dir() {
                return false;
            }
            if !p.is_file() {
                return false;
            }
            p.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    matches!(
                        ext.to_lowercase().as_str(),
                        "gguf" | "safetensors" | "bin" | "mlx" | "pt" | "npz"
                    ) || is_model_file(p.file_name().and_then(|n| n.to_str()).unwrap_or(""))
                })
                .unwrap_or(false)
        })
}

/// Scan one or more roots for model directories (LM Studio, OMLX, Jan, etc.).
pub fn scan_model_directories(
    models_roots: &[PathBuf],
    source: &str,
    id_prefix: &str,
) -> AppResult<Vec<ModelRecord>> {
    let scanned_at = Utc::now().to_rfc3339();
    let mut by_id: HashMap<String, ModelRecord> = HashMap::new();

    for models_root in models_roots {
        if !models_root.exists() {
            continue;
        }

        let mut candidate_dirs: Vec<PathBuf> = Vec::new();

        for entry in WalkDir::new(models_root)
            .min_depth(1)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| SKIP_DIRS.contains(&n))
                .unwrap_or(false)
            {
                continue;
            }
            if !dir_has_model_files(path) {
                continue;
            }

            candidate_dirs.retain(|c| !path.starts_with(c));
            candidate_dirs.retain(|c| !c.starts_with(path));
            candidate_dirs.push(path.to_path_buf());
        }

        for dir in candidate_dirs {
            let rel = dir
                .strip_prefix(models_root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| dir.file_name().unwrap().to_string_lossy().to_string());

            let display_name = rel.replace('\\', "/");
            let id = format!("{id_prefix}{}", display_name);
            let (total_bytes, file_count, files) = collect_files(&dir)?;

            if file_count == 0 {
                continue;
            }

            by_id.insert(
                id.clone(),
                ModelRecord {
                    id,
                    display_name,
                    source: source.to_string(),
                    primary_path: dir.to_string_lossy().to_string(),
                    total_bytes,
                    file_count,
                    scanned_at: scanned_at.clone(),
                    revision: None,
                    files,
                },
            );
        }
    }

    let mut models: Vec<ModelRecord> = by_id.into_values().collect();
    models.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn finds_nested_gguf_directory() {
        let tmp = TempDir::new().unwrap();
        let model_dir = tmp.path().join("org").join("demo-model");
        fs::create_dir_all(&model_dir).unwrap();
        fs::write(model_dir.join("model.gguf"), b"fake").unwrap();

        let found = scan_model_directories(&[tmp.path().to_path_buf()], "test", "test:").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].display_name, "org/demo-model");
    }
}
