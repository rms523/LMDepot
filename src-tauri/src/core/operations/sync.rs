use super::{files_needing_sync, new_job, prepare_backup_path, read_manifest, run_copy_with_job, write_manifest, JobContext};
use crate::error::{AppError, AppResult};
use crate::types::JobRecord;
use std::path::Path;
use uuid::Uuid;

pub fn execute_sync(
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
    let source = Path::new(&model.primary_path);

    ctx.check_cancelled()?;

    let to_copy = if backup_path.join("model.manifest.json").exists() {
        let manifest = read_manifest(&backup_path)?;
        let needs = files_needing_sync(&model, &manifest);
        if needs.is_empty() {
            ctx.db.upsert_model_backup(
                &Uuid::new_v4().to_string(),
                model_id,
                drive_id,
                &backup_path.to_string_lossy(),
                "backed_up",
            )?;
            return Ok(());
        }
        needs
    } else {
        std::fs::create_dir_all(&backup_path)?;
        model.files.clone()
    };

    run_copy_with_job(ctx, job, source, &backup_path, &to_copy)?;
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

pub fn run(ctx: &JobContext, model_id: &str, drive_id: &str) -> AppResult<()> {
    let mut job = new_job("sync", model_id, drive_id);
    ctx.db.create_job(&job)?;

    let result = execute_sync(ctx, &mut job, model_id, drive_id);

    match result {
        Ok(()) => ctx.finish_job(&mut job, true, "Sync completed")?,
        Err(e) => {
            let msg = e.to_string();
            ctx.finish_job(&mut job, false, &msg)?;
            return Err(e);
        }
    }
    Ok(())
}
