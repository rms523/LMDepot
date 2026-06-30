use super::directory_models::scan_model_directories;
use super::SourceAdapter;
use crate::error::{AppError, AppResult};
use crate::types::ModelRecord;
use std::path::PathBuf;

pub struct JanAdapter {
    path_override: Option<String>,
}

impl JanAdapter {
    pub fn new(path_override: Option<String>) -> Self {
        Self { path_override }
    }

    pub fn resolve_data_dir() -> AppResult<PathBuf> {
        if let Ok(dir) = std::env::var("JAN_DATA_DIR") {
            if !dir.is_empty() {
                return Ok(PathBuf::from(dir));
            }
        }

        if let Some(home) = dirs::home_dir() {
            let candidates = if cfg!(target_os = "macos") {
                vec![
                    home.join("Library")
                        .join("Application Support")
                        .join("jan"),
                    home.join("Library")
                        .join("Application Support")
                        .join("Jan")
                        .join("data"),
                    home.join("Library").join("jan"),
                ]
            } else if cfg!(target_os = "windows") {
                let mut paths = Vec::new();
                if let Ok(appdata) = std::env::var("APPDATA") {
                    paths.push(PathBuf::from(appdata).join("jan"));
                }
                paths.push(home.join("AppData").join("Roaming").join("jan"));
                paths
            } else {
                vec![
                    home.join(".local").join("share").join("jan"),
                    home.join(".config").join("jan"),
                ]
            };

            for candidate in candidates {
                if candidate.exists() {
                    return Ok(candidate);
                }
            }

            return Ok(home.join("Library").join("Application Support").join("jan"));
        }

        Err(AppError::msg("Could not resolve Jan data directory"))
    }

    fn model_roots(&self) -> AppResult<Vec<PathBuf>> {
        let data_dir = if let Some(ref p) = self.path_override {
            if !p.is_empty() {
                PathBuf::from(p)
            } else {
                Self::resolve_data_dir()?
            }
        } else {
            Self::resolve_data_dir()?
        };

        let mut roots = vec![
            data_dir.join("models"),
            data_dir.join("llamacpp").join("models"),
            data_dir.join("mlx").join("models"),
        ];
        roots.retain(|p| p.exists());
        if roots.is_empty() {
            roots.push(data_dir.join("models"));
        }
        Ok(roots)
    }
}

impl SourceAdapter for JanAdapter {
    fn id(&self) -> &str {
        "jan"
    }

    fn resolve_roots(&self) -> AppResult<Vec<PathBuf>> {
        self.model_roots()
    }

    fn scan(&self) -> AppResult<Vec<ModelRecord>> {
        scan_model_directories(&self.model_roots()?, "jan", "jan:")
    }
}
