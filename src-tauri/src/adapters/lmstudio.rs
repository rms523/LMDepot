use super::{collect_files, SourceAdapter};
use crate::error::{AppError, AppResult};
use crate::types::ModelRecord;
use chrono::Utc;
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &["_active", "_archive", ".cache"];

pub struct LmStudioAdapter {
    path_override: Option<String>,
}

impl LmStudioAdapter {
    pub fn new(path_override: Option<String>) -> Self {
        Self { path_override }
    }

    fn resolve_home(&self) -> AppResult<PathBuf> {
        if let Some(ref p) = self.path_override {
            if !p.is_empty() {
                return Ok(PathBuf::from(p));
            }
        }

        if let Some(home) = dirs::home_dir() {
            let pointer = home.join(".lmstudio-home-pointer");
            if pointer.exists() {
                let content = std::fs::read_to_string(&pointer)?;
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    return Ok(PathBuf::from(trimmed));
                }
            }

            let lmstudio = home.join(".lmstudio");
            if lmstudio.exists() {
                return Ok(lmstudio);
            }

            let cache = home.join(".cache").join("lm-studio");
            if cache.exists() {
                return Ok(cache);
            }

            return Ok(lmstudio);
        }

        Err(AppError::msg("Could not resolve home directory"))
    }

    fn models_dir(&self) -> AppResult<PathBuf> {
        Ok(self.resolve_home()?.join("models"))
    }

    fn scan_directory(&self, models_root: &Path, rel_prefix: &str) -> AppResult<Vec<ModelRecord>> {
        use walkdir::WalkDir;

        let mut found = Vec::new();
        if !models_root.exists() {
            return Ok(found);
        }

        let mut candidate_dirs: Vec<PathBuf> = Vec::new();

        for entry in WalkDir::new(models_root)
            .min_depth(1)
            .max_depth(4)
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

            let has_model_file = WalkDir::new(path)
                .max_depth(3)
                .into_iter()
                .filter_map(|e| e.ok())
                .any(|e| {
                    e.file_type().is_file()
                        && e.path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| {
                                matches!(
                                    ext.to_lowercase().as_str(),
                                    "gguf" | "safetensors" | "bin" | "mlx"
                                )
                            })
                            .unwrap_or(false)
                });

            if !has_model_file {
                continue;
            }

            // Prefer the deepest directory that contains model files.
            candidate_dirs.retain(|c| !path.starts_with(c));
            candidate_dirs.retain(|c| !c.starts_with(path));
            candidate_dirs.push(path.to_path_buf());
        }

        let scanned_at = Utc::now().to_rfc3339();

        for dir in candidate_dirs {
            let rel = dir
                .strip_prefix(models_root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| dir.file_name().unwrap().to_string_lossy().to_string());

            let display_name = if rel_prefix.is_empty() {
                rel.clone()
            } else {
                format!("{rel_prefix}/{rel}")
            };

            let id = format!("lmstudio:{}", display_name.replace('\\', "/"));
            let (total_bytes, file_count, files) = collect_files(&dir)?;

            if file_count == 0 {
                continue;
            }

            found.push(ModelRecord {
                id,
                display_name: display_name.replace('\\', "/"),
                source: "lmstudio".to_string(),
                primary_path: dir.to_string_lossy().to_string(),
                total_bytes,
                file_count,
                scanned_at: scanned_at.clone(),
                revision: None,
                files,
            });
        }

        Ok(found)
    }
}

impl SourceAdapter for LmStudioAdapter {
    fn id(&self) -> &str {
        "lmstudio"
    }

    fn resolve_roots(&self) -> AppResult<Vec<PathBuf>> {
        Ok(vec![self.models_dir()?])
    }

    fn scan(&self) -> AppResult<Vec<ModelRecord>> {
        let models_root = self.models_dir()?;
        self.scan_directory(&models_root, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn detects_gguf_folder() {
        let tmp = TempDir::new().unwrap();
        let models_root = tmp.path().join("models");
        let model_dir = models_root.join("author").join("test-model");
        fs::create_dir_all(&model_dir).unwrap();
        fs::write(model_dir.join("model.gguf"), b"fake").unwrap();

        let adapter = LmStudioAdapter::new(Some(tmp.path().to_string_lossy().to_string()));
        let results = adapter.scan().unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].display_name.contains("test-model"));
    }
}
