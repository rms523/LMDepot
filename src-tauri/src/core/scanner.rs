use crate::adapters::AdapterRegistry;
use crate::db::Database;
use crate::error::AppResult;

pub fn scan_and_persist(db: &Database) -> AppResult<Vec<crate::types::ModelRecord>> {
    let settings = db.get_settings()?;
    let registry = AdapterRegistry::new(
        settings.lmstudio_path_override.clone(),
        settings.hf_cache_path_override.clone(),
        settings.omlx_path_override.clone(),
        settings.ollama_models_override.clone(),
        settings.jan_data_override.clone(),
    );
    let models = registry.scan_all()?;
    let ids: Vec<String> = models.iter().map(|m| m.id.clone()).collect();
    db.upsert_models(&models)?;
    db.remove_stale_models(&ids)?;
    Ok(models)
}
