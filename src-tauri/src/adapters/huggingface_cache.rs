use super::{collect_files, SourceAdapter};
use crate::error::{AppError, AppResult};
use crate::types::ModelRecord;
use chrono::Utc;
use std::path::{Path, PathBuf};

pub struct HuggingFaceCacheAdapter {
    path_override: Option<String>,
}

impl HuggingFaceCacheAdapter {
    pub fn new(path_override: Option<String>) -> Self {
        Self { path_override }
    }

    pub fn resolve_hf_cache() -> AppResult<PathBuf> {
        if let Ok(cache) = std::env::var("HF_HUB_CACHE") {
            if !cache.is_empty() {
                return Ok(PathBuf::from(cache));
            }
        }
        if let Ok(home) = std::env::var("HF_HOME") {
            if !home.is_empty() {
                return Ok(PathBuf::from(home).join("hub"));
            }
        }
        if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
            return Ok(PathBuf::from(xdg).join("huggingface").join("hub"));
        }
        if let Some(home) = dirs::home_dir() {
            return Ok(home.join(".cache").join("huggingface").join("hub"));
        }
        Err(AppError::msg("Could not resolve Hugging Face cache directory"))
    }

    fn parse_repo_dir(name: &str) -> Option<(String, String)> {
        // models--org--repo-name
        if !name.starts_with("models--") {
            return None;
        }
        let rest = &name[8..];
        let parts: Vec<&str> = rest.splitn(2, "--").collect();
        if parts.len() != 2 {
            return None;
        }
        Some((parts[0].to_string(), parts[1].replace("--", "/")))
    }

    fn resolve_snapshot(repo_dir: &Path) -> AppResult<PathBuf> {
        let snapshots = repo_dir.join("snapshots");
        if !snapshots.exists() {
            return Err(AppError::msg("No snapshots directory"));
        }

        let refs_main = repo_dir.join("refs").join("main");
        if refs_main.exists() {
            let hash = std::fs::read_to_string(&refs_main)?.trim().to_string();
            let snap = snapshots.join(&hash);
            if snap.exists() {
                return Ok(snap);
            }
        }

        let mut best: Option<PathBuf> = None;
        for entry in std::fs::read_dir(&snapshots)? {
            let entry = entry?;
            if entry.path().is_dir() {
                best = Some(entry.path());
            }
        }
        best.ok_or_else(|| AppError::msg("No snapshot found"))
    }
}

impl SourceAdapter for HuggingFaceCacheAdapter {
    fn id(&self) -> &str {
        "huggingface"
    }

    fn resolve_roots(&self) -> AppResult<Vec<PathBuf>> {
        if let Some(ref p) = self.path_override {
            if !p.is_empty() {
                return Ok(vec![PathBuf::from(p)]);
            }
        }
        Ok(vec![Self::resolve_hf_cache()?])
    }

    fn scan(&self) -> AppResult<Vec<ModelRecord>> {
        let cache_root = self.resolve_roots()?.into_iter().next().unwrap();
        if !cache_root.exists() {
            return Ok(vec![]);
        }

        let scanned_at = Utc::now().to_rfc3339();
        let mut models = Vec::new();

        for entry in std::fs::read_dir(&cache_root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let Some((org, repo)) = Self::parse_repo_dir(dir_name) else {
                continue;
            };

            let snapshot = match Self::resolve_snapshot(&path) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let revision = snapshot
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from);

            let display_name = format!("{org}/{repo}");
            let id = format!("hf:{org}/{repo}");
            let (total_bytes, file_count, files) = collect_files(&snapshot)?;

            if file_count == 0 {
                continue;
            }

            models.push(ModelRecord {
                id,
                display_name,
                source: "huggingface".to_string(),
                primary_path: snapshot.to_string_lossy().to_string(),
                total_bytes,
                file_count,
                scanned_at: scanned_at.clone(),
                revision,
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

    #[test]
    fn parses_repo_dir_name() {
        let (org, repo) = HuggingFaceCacheAdapter::parse_repo_dir("models--unsloth--Meta-Llama-3.1-8B").unwrap();
        assert_eq!(org, "unsloth");
        assert_eq!(repo, "Meta-Llama-3.1-8B");
    }
}
