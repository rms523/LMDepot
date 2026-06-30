use crate::error::{AppError, AppResult};
use crate::types::ModelRecord;
use std::path::PathBuf;

pub trait SourceAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn resolve_roots(&self) -> AppResult<Vec<PathBuf>>;
    fn scan(&self) -> AppResult<Vec<ModelRecord>>;
}

pub mod huggingface_cache;
pub mod lmstudio;

use huggingface_cache::HuggingFaceCacheAdapter;
use lmstudio::LmStudioAdapter;

pub struct AdapterRegistry {
    adapters: Vec<Box<dyn SourceAdapter>>,
}

impl AdapterRegistry {
    pub fn new(lmstudio_override: Option<String>, hf_cache_override: Option<String>) -> Self {
        Self {
            adapters: vec![
                Box::new(LmStudioAdapter::new(lmstudio_override)),
                Box::new(HuggingFaceCacheAdapter::new(hf_cache_override)),
            ],
        }
    }

    pub fn scan_all(&self) -> AppResult<Vec<ModelRecord>> {
        let mut all = Vec::new();
        for adapter in &self.adapters {
            match adapter.scan() {
                Ok(mut models) => all.append(&mut models),
                Err(e) => {
                    tracing::warn!("Adapter {} scan failed: {}", adapter.id(), e);
                }
            }
        }
        Ok(all)
    }
}

pub fn is_model_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".gguf")
        || lower.ends_with(".safetensors")
        || lower.ends_with(".bin")
        || lower.ends_with(".pt")
        || lower.ends_with(".mlx")
        || lower.ends_with(".npz")
        || lower == "config.json"
        || lower == "tokenizer.json"
        || lower == "tokenizer_config.json"
}

pub fn collect_files(base: &std::path::Path) -> AppResult<(u64, u32, Vec<crate::types::ModelFileRecord>)> {
    use walkdir::WalkDir;

    let mut total_bytes = 0u64;
    let mut file_count = 0u32;
    let mut files = Vec::new();

    for entry in WalkDir::new(base).into_iter().filter_map(|e| e.ok()) {
        let ft = entry.file_type();
        // HF Hub snapshots store files as symlinks into blobs/; LM Studio uses real files.
        if !ft.is_file() && !ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_symlink() && path.is_dir() {
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !is_model_file(name) && !name.ends_with(".json") && !name.ends_with(".txt") {
            continue;
        }
        let meta = std::fs::metadata(path)?;
        let rel = path
            .strip_prefix(base)
            .map_err(|e| AppError::msg(e.to_string()))?
            .to_string_lossy()
            .to_string();
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        total_bytes += meta.len();
        file_count += 1;
        files.push(crate::types::ModelFileRecord {
            relative_path: rel,
            size: meta.len(),
            modified_at,
        });
    }
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok((total_bytes, file_count, files))
}

#[cfg(test)]
mod collect_files_tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    #[test]
    fn counts_symlinked_model_files() {
        let tmp = TempDir::new().unwrap();
        let blobs = tmp.path().join("blobs");
        let snapshot = tmp.path().join("snapshot");
        fs::create_dir_all(&blobs).unwrap();
        fs::create_dir_all(&snapshot).unwrap();
        let blob = blobs.join("abc123");
        fs::write(&blob, vec![0u8; 512]).unwrap();
        let link = snapshot.join("model.safetensors");
        #[cfg(unix)]
        symlink(&blob, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&blob, &link).unwrap();

        let (total_bytes, file_count, files) = collect_files(&snapshot).unwrap();
        assert_eq!(file_count, 1);
        assert_eq!(total_bytes, 512);
        assert_eq!(files[0].relative_path, "model.safetensors");
    }
}
