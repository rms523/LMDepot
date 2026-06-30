use crate::error::{AppError, AppResult};
use crate::types::RunningAppsCheck;
use std::process::Command;

const LM_STUDIO_NAMES: &[&str] = &["LM Studio", "lm-studio", "LMStudio"];
/// Apps that may hold locks on the Hugging Face Hub cache directory.
const HF_CACHE_APP_NAMES: &[&str] = &["unsloth", "Unsloth", "Unsloth Studio", "huggingface-cli"];

pub fn check_running_apps() -> RunningAppsCheck {
    RunningAppsCheck {
        lmstudio_running: is_process_running(LM_STUDIO_NAMES),
        huggingface_running: is_process_running(HF_CACHE_APP_NAMES),
    }
}

fn is_process_running(names: &[&str]) -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("tasklist").output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            return names.iter().any(|n| text.contains(&n.to_lowercase()));
        }
        false
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("ps").args(["-A", "-o", "comm="]).output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            return names.iter().any(|n| text.contains(&n.to_lowercase()));
        }
        false
    }
}

pub fn validate_apps_not_running(warn: bool) -> AppResult<()> {
    if !warn {
        return Ok(());
    }
    let check = check_running_apps();
    if check.lmstudio_running || check.huggingface_running {
        let mut apps = Vec::new();
        if check.lmstudio_running {
            apps.push("LM Studio");
        }
        if check.huggingface_running {
            apps.push("Hugging Face");
        }
        return Err(AppError::msg(format!(
            "{} is running. Close it before destructive operations or disable the warning in Settings.",
            apps.join(", ")
        )));
    }
    Ok(())
}
