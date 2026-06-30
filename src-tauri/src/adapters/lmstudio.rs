use super::directory_models::scan_model_directories;
use super::SourceAdapter;
use crate::error::{AppError, AppResult};
use crate::types::ModelRecord;
use std::path::PathBuf;

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
}

impl SourceAdapter for LmStudioAdapter {
    fn id(&self) -> &str {
        "lmstudio"
    }

    fn resolve_roots(&self) -> AppResult<Vec<PathBuf>> {
        Ok(vec![self.models_dir()?])
    }

    fn scan(&self) -> AppResult<Vec<ModelRecord>> {
        scan_model_directories(&[self.models_dir()?], "lmstudio", "lmstudio:")
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
