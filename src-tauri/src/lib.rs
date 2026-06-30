pub mod adapters;
pub mod core;
pub mod db;
pub mod error;
pub mod types;

use core::backup_import::{import_from_all_mounted_drives, import_from_drive};
use core::drive_monitor::{count_mounted, enrich_drives, volume_id_for_path};
use core::operations::{backup, batch, delete, offload, restore, sync, JobContext};
use core::process_check::check_running_apps;
use core::scanner::scan_and_persist;
use db::Database;
use error::{AppError, AppResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use types::*;
use uuid::Uuid;

struct AppState {
    db: Arc<Database>,
    cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

#[tauri::command]
fn get_dashboard_stats(state: State<AppState>) -> AppResult<DashboardStats> {
    let mut stats = state.db.dashboard_stats()?;
    let drives = enrich_drives(state.db.list_backup_drives()?);
    stats.mounted_drives = count_mounted(&drives);
    Ok(stats)
}

#[tauri::command]
fn list_models(state: State<AppState>, source: Option<String>) -> AppResult<Vec<ModelWithBackups>> {
    let models = state.db.list_models(source.as_deref())?;
    let drives = enrich_drives(state.db.list_backup_drives()?);
    let drive_map: HashMap<String, _> = drives.iter().map(|d| (d.id.clone(), d)).collect();

    let enriched = models
        .into_iter()
        .map(|mut m| {
            for b in &mut m.backups {
                if let Some(d) = drive_map.get(&b.drive_id) {
                    if !d.is_mounted {
                        b.status = "missing".to_string();
                    }
                }
            }
            m
        })
        .collect();
    Ok(enriched)
}

#[tauri::command]
fn get_model(state: State<AppState>, model_id: String) -> AppResult<Option<ModelWithBackups>> {
    state.db.get_model(&model_id)
}

#[tauri::command]
fn rescan_models(state: State<AppState>) -> AppResult<Vec<ModelRecord>> {
    scan_and_persist(&state.db)
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> AppResult<AppSettings> {
    state.db.get_settings()
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: AppSettings) -> AppResult<()> {
    state.db.save_settings(&settings)
}

#[tauri::command]
fn check_running_apps_cmd() -> RunningAppsCheck {
    check_running_apps()
}

#[tauri::command]
fn list_backup_drives(state: State<AppState>) -> AppResult<Vec<BackupDrive>> {
    Ok(enrich_drives(state.db.list_backup_drives()?))
}

#[tauri::command]
fn add_backup_drive(
    state: State<AppState>,
    label: String,
    root_path: String,
    is_default: bool,
) -> AppResult<BackupDrive> {
    let path = PathBuf::from(&root_path);
    if !path.exists() {
        return Err(AppError::msg("Path does not exist"));
    }
    let drive = BackupDrive {
        id: Uuid::new_v4().to_string(),
        label,
        root_path,
        volume_id: volume_id_for_path(&path),
        is_default,
        last_seen_at: Some(chrono::Utc::now().to_rfc3339()),
        is_mounted: true,
    };
    state.db.add_backup_drive(&drive)?;
    Ok(drive)
}

#[tauri::command]
fn remove_backup_drive(state: State<AppState>, drive_id: String) -> AppResult<()> {
    state.db.remove_backup_drive(&drive_id)
}

#[tauri::command]
fn import_from_backup_drive(
    state: State<AppState>,
    drive_id: Option<String>,
) -> AppResult<ImportFromDriveResult> {
    match drive_id {
        Some(id) => import_from_drive(&state.db, &id),
        None => import_from_all_mounted_drives(&state.db),
    }
}

#[tauri::command]
fn list_jobs(state: State<AppState>, limit: Option<u32>) -> AppResult<Vec<JobRecord>> {
    state.db.list_jobs(limit.unwrap_or(50))
}

#[tauri::command]
fn get_job(state: State<AppState>, job_id: String) -> AppResult<Option<JobRecord>> {
    state.db.get_job(&job_id)
}

fn spawn_job<F>(state: &AppState, app: AppHandle, job_id: String, work: F)
where
    F: FnOnce(JobContext) -> AppResult<()> + Send + 'static,
{
    let db = state.db.clone();
    let cancel_flags = state.cancel_flags.clone();
    let flag = Arc::new(AtomicBool::new(false));
    cancel_flags
        .lock()
        .unwrap()
        .insert(job_id.clone(), flag.clone());

    std::thread::spawn(move || {
        let ctx = JobContext {
            db: db.clone(),
            app: app.clone(),
            cancelled: flag,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(ctx)));
        match &result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("Job {} ended with error: {}", job_id, e),
            Err(_) => tracing::error!("Job {} panicked", job_id),
        }

        let reconcile_ctx = JobContext {
            db: db.clone(),
            app,
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let orphan_msg = match &result {
            Err(_) => "Job crashed unexpectedly",
            _ => "Job stopped unexpectedly",
        };
        let _ = reconcile_ctx.reconcile_orphan_job(&job_id, orphan_msg);

        cancel_flags.lock().unwrap().remove(&job_id);
    });
}

#[tauri::command]
fn start_backup(
    state: State<AppState>,
    app: AppHandle,
    model_id: String,
    drive_id: String,
) -> AppResult<String> {
    let job_id = Uuid::new_v4().to_string();
    let jid = job_id.clone();
    let mid = model_id.clone();
    let did = drive_id.clone();
    spawn_job(&state, app, job_id.clone(), move |ctx| {
        backup::run(&ctx, &jid, &mid, &did)
    });
    Ok(job_id)
}

#[tauri::command]
fn start_sync(
    state: State<AppState>,
    app: AppHandle,
    model_id: String,
    drive_id: String,
) -> AppResult<String> {
    let job_id = Uuid::new_v4().to_string();
    let jid = job_id.clone();
    let mid = model_id.clone();
    let did = drive_id.clone();
    spawn_job(&state, app, job_id.clone(), move |ctx| {
        sync::run(&ctx, &jid, &mid, &did)
    });
    Ok(job_id)
}

#[tauri::command]
fn start_restore(
    state: State<AppState>,
    app: AppHandle,
    model_id: String,
    drive_id: String,
    target_path: Option<String>,
) -> AppResult<String> {
    let job_id = Uuid::new_v4().to_string();
    let jid = job_id.clone();
    let mid = model_id.clone();
    let did = drive_id.clone();
    spawn_job(&state, app, job_id.clone(), move |ctx| {
        restore::run(&ctx, &jid, &mid, &did, target_path)
    });
    Ok(job_id)
}

#[tauri::command]
fn start_delete(
    state: State<AppState>,
    app: AppHandle,
    model_id: String,
    drive_id: Option<String>,
    scope: DeleteScope,
) -> AppResult<String> {
    let job_id = Uuid::new_v4().to_string();
    let jid = job_id.clone();
    let mid = model_id.clone();
    spawn_job(&state, app, job_id.clone(), move |ctx| {
        delete::run(&ctx, &jid, &mid, drive_id, scope)
    });
    Ok(job_id)
}

#[tauri::command]
fn start_offload(
    state: State<AppState>,
    app: AppHandle,
    model_id: String,
    drive_id: String,
) -> AppResult<String> {
    let job_id = Uuid::new_v4().to_string();
    let jid = job_id.clone();
    let mid = model_id.clone();
    let did = drive_id.clone();
    spawn_job(&state, app, job_id.clone(), move |ctx| {
        offload::run(&ctx, &jid, &mid, &did)
    });
    Ok(job_id)
}

#[tauri::command]
fn reverse_offload(
    state: State<AppState>,
    app: AppHandle,
    model_id: String,
    drive_id: String,
) -> AppResult<String> {
    let job_id = Uuid::new_v4().to_string();
    let jid = job_id.clone();
    let mid = model_id.clone();
    let did = drive_id.clone();
    spawn_job(&state, app, job_id.clone(), move |ctx| {
        offload::reverse_offload(&ctx, &jid, &mid, &did)
    });
    Ok(job_id)
}

#[tauri::command]
fn start_backup_all(
    state: State<AppState>,
    app: AppHandle,
    drive_id: String,
) -> AppResult<String> {
    let job_id = Uuid::new_v4().to_string();
    let jid = job_id.clone();
    let did = drive_id.clone();
    spawn_job(&state, app, job_id.clone(), move |ctx| {
        batch::backup_all(&ctx, &jid, &did)
    });
    Ok(job_id)
}

#[tauri::command]
fn start_sync_all(
    state: State<AppState>,
    app: AppHandle,
    drive_id: String,
) -> AppResult<String> {
    let job_id = Uuid::new_v4().to_string();
    let jid = job_id.clone();
    let did = drive_id.clone();
    spawn_job(&state, app, job_id.clone(), move |ctx| {
        batch::sync_all(&ctx, &jid, &did)
    });
    Ok(job_id)
}

#[tauri::command]
fn start_restore_all(
    state: State<AppState>,
    app: AppHandle,
    drive_id: String,
) -> AppResult<String> {
    let job_id = Uuid::new_v4().to_string();
    let jid = job_id.clone();
    let did = drive_id.clone();
    spawn_job(&state, app, job_id.clone(), move |ctx| {
        batch::restore_all(&ctx, &jid, &did)
    });
    Ok(job_id)
}

#[tauri::command]
fn cancel_job(state: State<AppState>, app: AppHandle, job_id: String) -> AppResult<()> {
    if let Some(flag) = state.cancel_flags.lock().unwrap().get(&job_id) {
        flag.store(true, Ordering::Relaxed);
        return Ok(());
    }

    // No active worker (orphaned after restart/crash) — force-finish in DB.
    if state
        .db
        .force_finish_job(&job_id, "cancelled", "Cancelled by user")?
    {
        if let Some(job) = state.db.get_job(&job_id)? {
            let ctx = JobContext {
                db: state.db.clone(),
                app,
                cancelled: Arc::new(AtomicBool::new(false)),
            };
            ctx.emit_progress(&job, "cancelled");
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data = app
                .path()
                .app_data_dir()
                .map_err(|e| AppError::msg(e.to_string()))?;
            let db = Database::open(&app_data)?;
            let stale = db.fail_stale_running_jobs(
                "Interrupted — app was closed or the job was orphaned",
            )?;
            if stale > 0 {
                tracing::info!("Marked {stale} stale running job(s) as failed on startup");
            }
            app.manage(AppState {
                db: Arc::new(db),
                cancel_flags: Arc::new(Mutex::new(HashMap::new())),
            });

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                if let Some(state) = handle.try_state::<AppState>() {
                    let _ = scan_and_persist(&state.db);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_dashboard_stats,
            list_models,
            get_model,
            rescan_models,
            get_settings,
            save_settings,
            check_running_apps_cmd,
            list_backup_drives,
            add_backup_drive,
            remove_backup_drive,
            import_from_backup_drive,
            list_jobs,
            get_job,
            start_backup,
            start_sync,
            start_backup_all,
            start_sync_all,
            start_restore_all,
            start_restore,
            start_delete,
            start_offload,
            reverse_offload,
            cancel_job,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
