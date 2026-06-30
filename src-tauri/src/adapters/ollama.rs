use super::SourceAdapter;
use crate::error::{AppError, AppResult};
use crate::types::{ModelFileRecord, ModelRecord};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct OllamaAdapter {
    path_override: Option<String>,
}

impl OllamaAdapter {
    pub fn new(path_override: Option<String>) -> Self {
        Self { path_override }
    }

    pub fn resolve_models_dir() -> AppResult<PathBuf> {
        if let Ok(dir) = std::env::var("OLLAMA_MODELS") {
            if !dir.is_empty() {
                return Ok(PathBuf::from(dir));
            }
        }

        if cfg!(target_os = "linux") {
            let system = PathBuf::from("/usr/share/ollama/.ollama/models");
            if system.exists() {
                return Ok(system);
            }
        }

        if let Some(home) = dirs::home_dir() {
            return Ok(home.join(".ollama").join("models"));
        }

        Err(AppError::msg("Could not resolve Ollama models directory"))
    }

    fn models_dir(&self) -> AppResult<PathBuf> {
        if let Some(ref p) = self.path_override {
            if !p.is_empty() {
                return Ok(PathBuf::from(p));
            }
        }
        Self::resolve_models_dir()
    }

    fn blob_relpath(digest: &str) -> String {
        let hash = digest.strip_prefix("sha256:").unwrap_or(digest);
        format!("blobs/sha256-{hash}")
    }

    fn file_record(models_root: &Path, rel: &str) -> AppResult<ModelFileRecord> {
        let path = models_root.join(rel);
        let meta = std::fs::metadata(&path)?;
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Ok(ModelFileRecord {
            relative_path: rel.to_string(),
            size: meta.len(),
            modified_at,
        })
    }
}

#[derive(Debug, Deserialize)]
struct OllamaManifest {
    config: OllamaDigest,
    layers: Option<Vec<OllamaDigest>>,
}

#[derive(Debug, Deserialize)]
struct OllamaDigest {
    digest: String,
    #[allow(dead_code)]
    size: u64,
}

impl SourceAdapter for OllamaAdapter {
    fn id(&self) -> &str {
        "ollama"
    }

    fn resolve_roots(&self) -> AppResult<Vec<PathBuf>> {
        Ok(vec![self.models_dir()?])
    }

    fn scan(&self) -> AppResult<Vec<ModelRecord>> {
        let models_root = self.models_dir()?;
        if !models_root.exists() {
            return Ok(vec![]);
        }

        let manifests_root = models_root
            .join("manifests")
            .join("registry.ollama.ai");
        if !manifests_root.exists() {
            return Ok(vec![]);
        }

        let scanned_at = Utc::now().to_rfc3339();
        let mut models = Vec::new();

        for entry in WalkDir::new(&manifests_root)
            .min_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let manifest_path = entry.path();
            let rel_manifest = manifest_path
                .strip_prefix(&models_root)
                .map_err(|e| AppError::msg(e.to_string()))?
                .to_string_lossy()
                .to_string();

            let rel_to_registry = manifest_path
                .strip_prefix(&manifests_root)
                .map_err(|e| AppError::msg(e.to_string()))?;
            let parts: Vec<&str> = rel_to_registry
                .components()
                .map(|c| c.as_os_str().to_str().unwrap_or(""))
                .collect();
            if parts.len() < 3 {
                continue;
            }
            let namespace = parts[0];
            let model_name = parts[1];
            let tag = parts[2];
            let display_name = if namespace == "library" {
                format!("{model_name}:{tag}")
            } else {
                format!("{namespace}/{model_name}:{tag}")
            };

            let content = std::fs::read_to_string(manifest_path)?;
            let manifest: OllamaManifest = serde_json::from_str(&content)?;

            let layers = manifest.layers.unwrap_or_default();
            if layers.is_empty() {
                continue;
            }

            let mut blob_paths = HashSet::new();
            blob_paths.insert(Self::blob_relpath(&manifest.config.digest));
            for layer in &layers {
                blob_paths.insert(Self::blob_relpath(&layer.digest));
            }

            let mut files = Vec::new();
            let mut total_bytes = 0u64;
            for blob in &blob_paths {
                let path = models_root.join(blob);
                if !path.is_file() {
                    continue;
                }
                let record = Self::file_record(&models_root, blob)?;
                total_bytes += record.size;
                files.push(record);
            }

            files.push(Self::file_record(&models_root, &rel_manifest)?);
            total_bytes += files.last().map(|f| f.size).unwrap_or(0);

            if files.len() <= 1 {
                continue;
            }

            files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
            let id = format!("ollama:{}", display_name.replace(':', "_"));

            models.push(ModelRecord {
                id,
                display_name,
                source: "ollama".to_string(),
                primary_path: models_root.to_string_lossy().to_string(),
                total_bytes,
                file_count: files.len() as u32,
                scanned_at: scanned_at.clone(),
                revision: Some(tag.to_string()),
                files,
            });
        }

        models.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn blob_relpath_strips_prefix() {
        assert_eq!(
            OllamaAdapter::blob_relpath("sha256:abc123"),
            "blobs/sha256-abc123"
        );
    }

    #[test]
    fn finds_manifest_backed_model() {
        let tmp = TempDir::new().unwrap();
        let models_root = tmp.path();
        let manifest_path = models_root
            .join("manifests")
            .join("registry.ollama.ai")
            .join("library")
            .join("demo")
            .join("latest");
        fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();
        fs::create_dir_all(models_root.join("blobs")).unwrap();

        let config_digest = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
        let layer_digest = "sha256:2222222222222222222222222222222222222222222222222222222222222222";
        fs::write(
            models_root.join("blobs").join("sha256-1111111111111111111111111111111111111111111111111111111111111111"),
            b"config",
        )
        .unwrap();
        fs::write(
            models_root.join("blobs").join("sha256-2222222222222222222222222222222222222222222222222222222222222222"),
            vec![0u8; 128],
        )
        .unwrap();

        let manifest = format!(
            r#"{{
              "config": {{ "digest": "{config_digest}", "size": 6 }},
              "layers": [{{ "digest": "{layer_digest}", "size": 128 }}]
            }}"#
        );
        fs::write(&manifest_path, manifest).unwrap();

        let adapter = OllamaAdapter::new(Some(models_root.to_string_lossy().to_string()));
        let found = adapter.scan().unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].display_name, "demo:latest");
        assert!(found[0].file_count >= 2);
    }
}
