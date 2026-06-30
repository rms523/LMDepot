use super::{new_job, prepare_backup_path, run_copy_with_job, write_manifest, JobContext};
use crate::core::copy_engine::{self, create_symlink, move_dir};
use crate::core::process_check::validate_apps_not_running;
use crate::error::{AppError, AppResult};
use std::path::Path;
use uuid::Uuid;

pub fn run(ctx: &JobContext, job_id: &str, model_id: &str, drive_id: &str) -> AppResult<()> {
    let mut job = new_job(job_id, "offload", model_id, drive_id);
    ctx.db.create_job(&job)?;

    let result = (|| -> AppResult<()> {
        let settings = ctx.db.get_settings()?;
        validate_apps_not_running(settings.warn_if_app_running)?;

        let model = ctx
            .db
            .get_model(model_id)?
            .ok_or_else(|| AppError::msg("Model not found"))?
            .model;

        let source_path = Path::new(&model.primary_path);
        if copy_engine::is_symlink(source_path) {
            return Err(AppError::msg("Model is already offloaded"));
        }

        let (_drive, backup_path) = prepare_backup_path(&ctx.db, drive_id, &model)?;

        ctx.check_cancelled()?;
        std::fs::create_dir_all(&backup_path)?;

        let source = Path::new(&model.primary_path);
        run_copy_with_job(ctx, &mut job, source, &backup_path, &model.files)?;
        ctx.check_cancelled()?;
        write_manifest(&backup_path, &model, settings.verify_hashes)?;

        copy_engine::remove_dir_all(source)?;
        create_symlink(&backup_path, source)?;

        ctx.db.upsert_model_backup(
            &Uuid::new_v4().to_string(),
            model_id,
            drive_id,
            &backup_path.to_string_lossy(),
            "backed_up",
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => ctx.finish_job(&mut job, true, "Offload completed")?,
        Err(e) => {
            let msg = e.to_string();
            ctx.finish_job(&mut job, false, &msg)?;
            return Err(e);
        }
    }
    Ok(())
}

pub fn reverse_offload(ctx: &JobContext, job_id: &str, model_id: &str, drive_id: &str) -> AppResult<()> {
    let mut job = new_job(job_id, "reverse_offload", model_id, drive_id);
    job.message = Some("Reversing offload...".to_string());
    ctx.db.create_job(&job)?;

    let result = (|| -> AppResult<()> {
        let settings = ctx.db.get_settings()?;
        validate_apps_not_running(settings.warn_if_app_running)?;

        let model = ctx
            .db
            .get_model(model_id)?
            .ok_or_else(|| AppError::msg("Model not found"))?
            .model;

        let source_path = Path::new(&model.primary_path);
        if !copy_engine::is_symlink(source_path) {
            return Err(AppError::msg("Model is not offloaded"));
        }

        let target = source_path
            .read_link()
            .map_err(|e| AppError::msg(e.to_string()))?;

        ctx.check_cancelled()?;
        copy_engine::remove_file_or_dir(source_path)?;
        move_dir(&target, source_path)?;
        Ok(())
    })();

    match result {
        Ok(()) => ctx.finish_job(&mut job, true, "Offload reversed")?,
        Err(e) => {
            let msg = e.to_string();
            ctx.finish_job(&mut job, false, &msg)?;
            return Err(e);
        }
    }
    Ok(())
}
