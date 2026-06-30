use crate::error::{AppError, AppResult};
use crate::types::{BackupDrive, ModelRecord};
use std::path::{Path, PathBuf};

pub fn is_path_mounted(path: &str) -> bool {
    let p = Path::new(path);
    p.exists() && p.is_dir()
}

pub fn enrich_drives(mut drives: Vec<BackupDrive>) -> Vec<BackupDrive> {
    for drive in &mut drives {
        drive.is_mounted = is_path_mounted(&drive.root_path);
        if drive.is_mounted {
            drive.last_seen_at = Some(chrono::Utc::now().to_rfc3339());
        }
    }
    drives
}

pub fn count_mounted(drives: &[BackupDrive]) -> u32 {
    drives.iter().filter(|d| d.is_mounted).count() as u32
}

pub fn backup_layout_path(drive_root: &Path, model: &ModelRecord) -> PathBuf {
    let source_dir = match model.source.as_str() {
        "lmstudio" => "lmstudio",
        "huggingface" | "unsloth" => "hf", // unsloth: legacy source tag
        _ => "other",
    };
    let name = model.display_name.replace('\\', "/").replace(':', "_");
    drive_root.join(source_dir).join(name)
}

pub fn ensure_drive_mounted(drive: &BackupDrive) -> AppResult<()> {
    if !is_path_mounted(&drive.root_path) {
        return Err(AppError::msg(format!(
            "Backup drive '{}' is not mounted at {}",
            drive.label, drive.root_path
        )));
    }
    Ok(())
}

pub fn volume_id_for_path(path: &Path) -> Option<String> {
    use std::process::Command;

    if cfg!(target_os = "macos") {
        if let Some(mount) = path.to_str().and_then(|p| p.strip_prefix("/Volumes/")) {
            let vol = mount.split('/').next().unwrap_or(mount);
            let output = Command::new("diskutil")
                .args(["info", &format!("/Volumes/{vol}")])
                .output()
                .ok()?;
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("Volume UUID") {
                    return line.split(':').nth(1).map(|s| s.trim().to_string());
                }
            }
            return Some(format!("mac-vol-{vol}"));
        }
    }

    if cfg!(target_os = "linux") {
        if let Ok(output) = Command::new("findmnt").args(["-n", "-o", "UUID"]).arg(path).output() {
            let uuid = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !uuid.is_empty() {
                return Some(uuid);
            }
        }
    }

    path.canonicalize()
        .ok()
        .map(|p| format!("path-{}", p.to_string_lossy()))
}
