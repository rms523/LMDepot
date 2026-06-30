mod schema_embed {
    pub const SCHEMA: &str = include_str!("schema.sql");
}

use crate::error::{AppError, AppResult};
use crate::types::{
    AppSettings, BackupDrive, DashboardStats, JobRecord, ModelBackupStatus, ModelFileRecord,
    ModelRecord, ModelWithBackups,
};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(app_data_dir: &Path) -> AppResult<Self> {
        std::fs::create_dir_all(app_data_dir)?;
        let db_path = app_data_dir.join("model-backup.db");
        let conn = Connection::open(db_path)?;
        conn.execute_batch(schema_embed::SCHEMA)?;
        conn.execute(
            "UPDATE models SET source = 'huggingface' WHERE source = 'unsloth'",
            [],
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn get_settings(&self) -> AppResult<AppSettings> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        let mut settings = AppSettings::default();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (key, value) = row?;
            match key.as_str() {
                "lmstudio_path_override" => settings.lmstudio_path_override = Some(value),
                "hf_cache_path_override" => settings.hf_cache_path_override = Some(value),
                "omlx_path_override" => settings.omlx_path_override = Some(value),
                "ollama_models_override" => settings.ollama_models_override = Some(value),
                "jan_data_override" => settings.jan_data_override = Some(value),
                "verify_hashes" => settings.verify_hashes = value == "true",
                "warn_if_app_running" => settings.warn_if_app_running = value == "true",
                _ => {}
            }
        }
        Ok(settings)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        let pairs = [
            (
                "lmstudio_path_override",
                settings
                    .lmstudio_path_override
                    .clone()
                    .unwrap_or_default(),
            ),
            (
                "hf_cache_path_override",
                settings
                    .hf_cache_path_override
                    .clone()
                    .unwrap_or_default(),
            ),
            (
                "omlx_path_override",
                settings.omlx_path_override.clone().unwrap_or_default(),
            ),
            (
                "ollama_models_override",
                settings
                    .ollama_models_override
                    .clone()
                    .unwrap_or_default(),
            ),
            (
                "jan_data_override",
                settings.jan_data_override.clone().unwrap_or_default(),
            ),
            (
                "verify_hashes",
                settings.verify_hashes.to_string(),
            ),
            (
                "warn_if_app_running",
                settings.warn_if_app_running.to_string(),
            ),
        ];
        for (key, value) in pairs {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
        }
        Ok(())
    }

    pub fn upsert_models(&self, models: &[ModelRecord]) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;
        for model in models {
            tx.execute(
                "INSERT INTO models (id, display_name, source, primary_path, total_bytes, file_count, scanned_at, revision)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                   display_name = excluded.display_name,
                   primary_path = excluded.primary_path,
                   total_bytes = excluded.total_bytes,
                   file_count = excluded.file_count,
                   scanned_at = excluded.scanned_at,
                   revision = excluded.revision",
                params![
                    model.id,
                    model.display_name,
                    model.source,
                    model.primary_path,
                    model.total_bytes,
                    model.file_count,
                    model.scanned_at,
                    model.revision,
                ],
            )?;
            tx.execute("DELETE FROM model_files WHERE model_id = ?1", params![model.id])?;
            for file in &model.files {
                tx.execute(
                    "INSERT INTO model_files (model_id, relative_path, size, modified_at) VALUES (?1, ?2, ?3, ?4)",
                    params![model.id, file.relative_path, file.size, file.modified_at],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_models(&self, source_filter: Option<&str>) -> AppResult<Vec<ModelWithBackups>> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        let query = match source_filter {
            Some(_) => "SELECT id, display_name, source, primary_path, total_bytes, file_count, scanned_at, revision FROM models WHERE source = ?1 ORDER BY display_name",
            None => "SELECT id, display_name, source, primary_path, total_bytes, file_count, scanned_at, revision FROM models ORDER BY display_name",
        };
        let mut stmt = conn.prepare(query)?;
        let rows: Vec<ModelRecord> = if let Some(src) = source_filter {
            stmt.query_map(params![src], map_model_row)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map([], map_model_row)?
                .collect::<Result<Vec<_>, _>>()?
        };

        let mut result = Vec::new();
        for mut model in rows {
            model.files = self.get_model_files_inner(&conn, &model.id)?;
            let backups = self.get_backup_status_inner(&conn, &model.id)?;
            result.push(ModelWithBackups { model, backups });
        }
        Ok(result)
    }

    pub fn get_model(&self, model_id: &str) -> AppResult<Option<ModelWithBackups>> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, display_name, source, primary_path, total_bytes, file_count, scanned_at, revision FROM models WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![model_id], map_model_row)?;
        if let Some(row) = rows.next() {
            let mut model = row?;
            model.files = self.get_model_files_inner(&conn, &model.id)?;
            let backups = self.get_backup_status_inner(&conn, &model.id)?;
            return Ok(Some(ModelWithBackups { model, backups }));
        }
        Ok(None)
    }

    fn get_model_files_inner(
        &self,
        conn: &Connection,
        model_id: &str,
    ) -> AppResult<Vec<ModelFileRecord>> {
        let mut stmt = conn.prepare(
            "SELECT relative_path, size, modified_at FROM model_files WHERE model_id = ?1 ORDER BY relative_path",
        )?;
        let files = stmt
            .query_map(params![model_id], |row| {
                Ok(ModelFileRecord {
                    relative_path: row.get(0)?,
                    size: row.get(1)?,
                    modified_at: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(files)
    }

    fn get_backup_status_inner(
        &self,
        conn: &Connection,
        model_id: &str,
    ) -> AppResult<Vec<ModelBackupStatus>> {
        let mut stmt = conn.prepare(
            "SELECT mb.drive_id, bd.label, mb.status, mb.backup_path, mb.last_synced_at
             FROM model_backups mb
             JOIN backup_drives bd ON bd.id = mb.drive_id
             WHERE mb.model_id = ?1",
        )?;
        let backups = stmt
            .query_map(params![model_id], |row| {
                Ok(ModelBackupStatus {
                    drive_id: row.get(0)?,
                    drive_label: row.get(1)?,
                    status: row.get(2)?,
                    backup_path: row.get(3)?,
                    last_synced_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(backups)
    }

    pub fn remove_stale_models(&self, current_ids: &[String]) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        if current_ids.is_empty() {
            conn.execute("DELETE FROM models", [])?;
            return Ok(());
        }
        let placeholders = current_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!("DELETE FROM models WHERE id NOT IN ({placeholders})");
        let params: Vec<&dyn rusqlite::ToSql> = current_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();
        conn.execute(&sql, params.as_slice())?;
        Ok(())
    }

    pub fn add_backup_drive(&self, drive: &BackupDrive) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        if drive.is_default {
            conn.execute("UPDATE backup_drives SET is_default = 0", [])?;
        }
        conn.execute(
            "INSERT INTO backup_drives (id, label, root_path, volume_id, is_default, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                drive.id,
                drive.label,
                drive.root_path,
                drive.volume_id,
                drive.is_default as i32,
                drive.last_seen_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_backup_drives(&self) -> AppResult<Vec<BackupDrive>> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, label, root_path, volume_id, is_default, last_seen_at FROM backup_drives ORDER BY label",
        )?;
        let drives = stmt
            .query_map([], |row| {
                Ok(BackupDrive {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    root_path: row.get(2)?,
                    volume_id: row.get(3)?,
                    is_default: row.get::<_, i32>(4)? != 0,
                    last_seen_at: row.get(5)?,
                    is_mounted: false,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(drives)
    }

    pub fn remove_backup_drive(&self, drive_id: &str) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        conn.execute("DELETE FROM backup_drives WHERE id = ?1", params![drive_id])?;
        Ok(())
    }

    pub fn get_backup_drive(&self, drive_id: &str) -> AppResult<Option<BackupDrive>> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, label, root_path, volume_id, is_default, last_seen_at FROM backup_drives WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![drive_id], |row| {
            Ok(BackupDrive {
                id: row.get(0)?,
                label: row.get(1)?,
                root_path: row.get(2)?,
                volume_id: row.get(3)?,
                is_default: row.get::<_, i32>(4)? != 0,
                last_seen_at: row.get(5)?,
                is_mounted: false,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    pub fn upsert_model_backup(
        &self,
        id: &str,
        model_id: &str,
        drive_id: &str,
        backup_path: &str,
        status: &str,
    ) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        conn.execute(
            "INSERT INTO model_backups (id, model_id, drive_id, backup_path, status, last_synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(model_id, drive_id) DO UPDATE SET
               backup_path = excluded.backup_path,
               status = excluded.status,
               last_synced_at = excluded.last_synced_at",
            params![id, model_id, drive_id, backup_path, status, now],
        )?;
        Ok(())
    }

    pub fn delete_model_backup(&self, model_id: &str, drive_id: &str) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        conn.execute(
            "DELETE FROM model_backups WHERE model_id = ?1 AND drive_id = ?2",
            params![model_id, drive_id],
        )?;
        Ok(())
    }

    pub fn delete_model(&self, model_id: &str) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        conn.execute("DELETE FROM models WHERE id = ?1", params![model_id])?;
        Ok(())
    }

    pub fn create_job(&self, job: &JobRecord) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        conn.execute(
            "INSERT INTO jobs (id, job_type, status, model_id, drive_id, progress_pct, bytes_done, bytes_total, current_file, message, created_at, finished_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                job.id,
                job.job_type,
                job.status,
                job.model_id,
                job.drive_id,
                job.progress_pct,
                job.bytes_done,
                job.bytes_total,
                job.current_file,
                job.message,
                job.created_at,
                job.finished_at,
            ],
        )?;
        Ok(())
    }

    pub fn update_job(&self, job: &JobRecord) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        conn.execute(
            "UPDATE jobs SET status = ?2, progress_pct = ?3, bytes_done = ?4, bytes_total = ?5,
             current_file = ?6, message = ?7, finished_at = ?8 WHERE id = ?1",
            params![
                job.id,
                job.status,
                job.progress_pct,
                job.bytes_done,
                job.bytes_total,
                job.current_file,
                job.message,
                job.finished_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_jobs(&self, limit: u32) -> AppResult<Vec<JobRecord>> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, job_type, status, model_id, drive_id, progress_pct, bytes_done, bytes_total, current_file, message, created_at, finished_at
             FROM jobs ORDER BY created_at DESC LIMIT ?1",
        )?;
        let jobs = stmt
            .query_map(params![limit], map_job_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(jobs)
    }

    pub fn get_job(&self, job_id: &str) -> AppResult<Option<JobRecord>> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, job_type, status, model_id, drive_id, progress_pct, bytes_done, bytes_total, current_file, message, created_at, finished_at
             FROM jobs WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![job_id], map_job_row)?;
        Ok(rows.next().transpose()?)
    }

    /// Mark jobs left in `running` after a crash or force-quit as failed.
    pub fn fail_stale_running_jobs(&self, message: &str) -> AppResult<u32> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let updated = conn.execute(
            "UPDATE jobs SET status = 'failed', message = ?1, finished_at = ?2 WHERE status = 'running'",
            params![message, now],
        )?;
        Ok(updated as u32)
    }

    /// Force-finish a single job if it is still marked running (orphaned / no worker).
    pub fn force_finish_job(&self, job_id: &str, status: &str, message: &str) -> AppResult<bool> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let updated = conn.execute(
            "UPDATE jobs SET status = ?2, message = ?3, finished_at = ?4
             WHERE id = ?1 AND status = 'running'",
            params![job_id, status, message, now],
        )?;
        Ok(updated > 0)
    }

    pub fn dashboard_stats(&self) -> AppResult<DashboardStats> {
        let conn = self.conn.lock().map_err(|e| AppError::msg(e.to_string()))?;
        let total_models: u32 =
            conn.query_row("SELECT COUNT(*) FROM models", [], |r| r.get(0))?;
        let total_bytes: u64 =
            conn.query_row("SELECT COALESCE(SUM(total_bytes), 0) FROM models", [], |r| {
                r.get(0)
            })?;
        let lmstudio_bytes: u64 = conn.query_row(
            "SELECT COALESCE(SUM(total_bytes), 0) FROM models WHERE source = 'lmstudio'",
            [],
            |r| r.get(0),
        )?;
        let huggingface_bytes: u64 = conn.query_row(
            "SELECT COALESCE(SUM(total_bytes), 0) FROM models WHERE source IN ('huggingface', 'unsloth')",
            [],
            |r| r.get(0),
        )?;
        let omlx_bytes: u64 = conn.query_row(
            "SELECT COALESCE(SUM(total_bytes), 0) FROM models WHERE source = 'omlx'",
            [],
            |r| r.get(0),
        )?;
        let ollama_bytes: u64 = conn.query_row(
            "SELECT COALESCE(SUM(total_bytes), 0) FROM models WHERE source = 'ollama'",
            [],
            |r| r.get(0),
        )?;
        let jan_bytes: u64 = conn.query_row(
            "SELECT COALESCE(SUM(total_bytes), 0) FROM models WHERE source = 'jan'",
            [],
            |r| r.get(0),
        )?;
        let backed_up_count: u32 = conn.query_row(
            "SELECT COUNT(DISTINCT model_id) FROM model_backups WHERE status = 'backed_up'",
            [],
            |r| r.get(0),
        )?;
        let drive_count: u32 =
            conn.query_row("SELECT COUNT(*) FROM backup_drives", [], |r| r.get(0))?;
        let coverage = if total_models > 0 {
            (backed_up_count as f64 / total_models as f64) * 100.0
        } else {
            0.0
        };
        Ok(DashboardStats {
            total_models,
            total_bytes,
            backed_up_count,
            backup_coverage_pct: coverage,
            drive_count,
            mounted_drives: 0,
            lmstudio_bytes,
            huggingface_bytes,
            omlx_bytes,
            ollama_bytes,
            jan_bytes,
        })
    }
}

fn map_model_row(row: &rusqlite::Row) -> rusqlite::Result<ModelRecord> {
    Ok(ModelRecord {
        id: row.get(0)?,
        display_name: row.get(1)?,
        source: row.get(2)?,
        primary_path: row.get(3)?,
        total_bytes: row.get(4)?,
        file_count: row.get(5)?,
        scanned_at: row.get(6)?,
        revision: row.get(7)?,
        files: vec![],
    })
}

fn map_job_row(row: &rusqlite::Row) -> rusqlite::Result<JobRecord> {
    Ok(JobRecord {
        id: row.get(0)?,
        job_type: row.get(1)?,
        status: row.get(2)?,
        model_id: row.get(3)?,
        drive_id: row.get(4)?,
        progress_pct: row.get(5)?,
        bytes_done: row.get(6)?,
        bytes_total: row.get(7)?,
        current_file: row.get(8)?,
        message: row.get(9)?,
        created_at: row.get(10)?,
        finished_at: row.get(11)?,
    })
}
