use super::{new_job, prepare_backup_path, run_copy_with_job, write_manifest, JobContext};
use crate::error::{AppError, AppResult};
use crate::types::JobRecord;
use std::path::Path;
use uuid::Uuid;

pub fn execute_backup(
    ctx: &JobContext,
    job: &mut JobRecord,
    model_id: &str,
    drive_id: &str,
) -> AppResult<()> {
    let settings = ctx.db.get_settings()?;
    let model = ctx
        .db
        .get_model(model_id)?
        .ok_or_else(|| AppError::msg(format!("Model not found: {model_id}")))?
        .model;

    let (_drive, backup_path) = prepare_backup_path(&ctx.db, drive_id, &model)?;

    ctx.check_cancelled()?;
    std::fs::create_dir_all(&backup_path)?;

    let source = Path::new(&model.primary_path);
    run_copy_with_job(ctx, job, source, &backup_path, &model.files)?;
    ctx.check_cancelled()?;
    write_manifest(&backup_path, &model, settings.verify_hashes)?;

    ctx.db.upsert_model_backup(
        &Uuid::new_v4().to_string(),
        model_id,
        drive_id,
        &backup_path.to_string_lossy(),
        "backed_up",
    )?;
    Ok(())
}

pub fn run(ctx: &JobContext, job_id: &str, model_id: &str, drive_id: &str) -> AppResult<()> {
    let mut job = new_job(job_id, "backup", model_id, drive_id);
    ctx.db.create_job(&job)?;

    let result = execute_backup(ctx, &mut job, model_id, drive_id);

    match result {
        Ok(()) => ctx.finish_job(&mut job, true, "Backup completed")?,
        Err(e) => {
            let msg = e.to_string();
            ctx.finish_job(&mut job, false, &msg)?;
            return Err(e);
        }
    }
    Ok(())
}
