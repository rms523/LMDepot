use super::directory_models::scan_model_directories;
use super::SourceAdapter;
use crate::error::AppResult;
use crate::types::ModelRecord;
use serde::Deserialize;
use std::path::PathBuf;

pub struct OmlxAdapter {
    path_override: Option<String>,
}

impl OmlxAdapter {
    pub fn new(path_override: Option<String>) -> Self {
        Self { path_override }
    }

    pub fn resolve_model_dirs() -> AppResult<Vec<PathBuf>> {
        let mut dirs = Vec::new();

        if let Ok(dir) = std::env::var("OMLX_MODEL_DIR") {
            if !dir.is_empty() {
                dirs.push(PathBuf::from(dir));
            }
        }

        if let Some(home) = dirs::home_dir() {
            let settings_path = home.join(".omlx").join("settings.json");
            if settings_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&settings_path) {
                    if let Ok(settings) = serde_json::from_str::<OmlxSettings>(&content) {
                        if let Some(model) = settings.model {
                            for dir in model.model_dirs {
                                if !dir.is_empty() {
                                    dirs.push(PathBuf::from(dir));
                                }
                            }
                            if let Some(dir) = model.model_dir {
                                if !dir.is_empty() {
                                    dirs.push(PathBuf::from(dir));
                                }
                            }
                        }
                    }
                }
            }

            let default_dir = home.join(".omlx").join("models");
            if default_dir.exists() {
                dirs.push(default_dir);
            }
        }

        dirs.sort();
        dirs.dedup();
        Ok(dirs)
    }

    fn roots(&self) -> AppResult<Vec<PathBuf>> {
        if let Some(ref p) = self.path_override {
            if !p.is_empty() {
                return Ok(vec![PathBuf::from(p)]);
            }
        }
        Self::resolve_model_dirs()
    }
}

#[derive(Debug, Deserialize)]
struct OmlxSettings {
    model: Option<OmlxModelSettings>,
}

#[derive(Debug, Deserialize)]
struct OmlxModelSettings {
    #[serde(default)]
    model_dir: Option<String>,
    #[serde(default)]
    model_dirs: Vec<String>,
}

impl SourceAdapter for OmlxAdapter {
    fn id(&self) -> &str {
        "omlx"
    }

    fn resolve_roots(&self) -> AppResult<Vec<PathBuf>> {
        self.roots()
    }

    fn scan(&self) -> AppResult<Vec<ModelRecord>> {
        let roots = self.roots()?;
        if roots.is_empty() {
            return Ok(vec![]);
        }
        scan_model_directories(&roots, "omlx", "omlx:")
    }
}
