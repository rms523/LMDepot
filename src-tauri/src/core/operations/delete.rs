use super::{new_job, JobContext};
use crate::core::copy_engine;
use crate::core::drive_monitor::ensure_drive_mounted;
use crate::core::process_check::validate_apps_not_running;
use crate::error::{AppError, AppResult};
use crate::types::DeleteScope;
use std::path::Path;

pub fn run(
    ctx: &JobContext,
    model_id: &str,
    drive_id: Option<String>,
    scope: DeleteScope,
) -> AppResult<()> {
    let settings = ctx.db.get_settings()?;
    validate_apps_not_running(settings.warn_if_app_running)?;

    let model_with = ctx
        .db
        .get_model(model_id)?
        .ok_or_else(|| AppError::msg("Model not found"))?;

    let mut job = new_job("delete", model_id, drive_id.as_deref().unwrap_or(""));
    if drive_id.is_none() {
        job.drive_id = None;
    }
    ctx.db.create_job(&job)?;

    let result = (|| -> AppResult<()> {
        ctx.check_cancelled()?;

        match scope {
            DeleteScope::SourceOnly => {
                let path = Path::new(&model_with.model.primary_path);
                if copy_engine::is_symlink(path) {
                    copy_engine::remove_file_or_dir(path)?;
                } else {
                    copy_engine::remove_dir_all(path)?;
                }
            }
            DeleteScope::BackupOnly => {
                let drive_id = drive_id.ok_or_else(|| AppError::msg("Drive required"))?;
                let backup = model_with
                    .backups
                    .iter()
                    .find(|b| b.drive_id == drive_id)
                    .ok_or_else(|| AppError::msg("Backup not found"))?;
                let backup_path = backup
                    .backup_path
                    .as_ref()
                    .ok_or_else(|| AppError::msg("Backup path missing"))?;
                let drive = ctx
                    .db
                    .get_backup_drive(&drive_id)?
                    .ok_or_else(|| AppError::msg("Drive not found"))?;
                ensure_drive_mounted(&drive)?;
                copy_engine::remove_dir_all(Path::new(backup_path))?;
                ctx.db.delete_model_backup(model_id, &drive_id)?;
            }
            DeleteScope::Both => {
                let drive_id = drive_id.ok_or_else(|| AppError::msg("Drive required"))?;
                let path = Path::new(&model_with.model.primary_path);
                if copy_engine::is_symlink(path) {
                    copy_engine::remove_file_or_dir(path)?;
                } else {
                    copy_engine::remove_dir_all(path)?;
                }
                let backup = model_with
                    .backups
                    .iter()
                    .find(|b| b.drive_id == drive_id)
                    .ok_or_else(|| AppError::msg("Backup not found"))?;
                if let Some(backup_path) = &backup.backup_path {
                    let drive = ctx
                        .db
                        .get_backup_drive(&drive_id)?
                        .ok_or_else(|| AppError::msg("Drive not found"))?;
                    ensure_drive_mounted(&drive)?;
                    copy_engine::remove_dir_all(Path::new(backup_path))?;
                }
                ctx.db.delete_model_backup(model_id, &drive_id)?;
                ctx.db.delete_model(model_id)?;
                return Ok(());
            }
        }

        if matches!(scope, DeleteScope::SourceOnly) {
            ctx.db.delete_model(model_id)?;
        }

        Ok(())
    })();

    match result {
        Ok(()) => ctx.finish_job(&mut job, true, "Delete completed")?,
        Err(e) => {
            let msg = e.to_string();
            ctx.finish_job(&mut job, false, &msg)?;
            return Err(e);
        }
    }
    Ok(())
}
