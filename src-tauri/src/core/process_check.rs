use crate::error::{AppError, AppResult};
use crate::types::RunningAppsCheck;
use std::process::Command;

const LM_STUDIO_NAMES: &[&str] = &["LM Studio", "lm-studio", "LMStudio"];
/// Apps that may hold locks on the Hugging Face Hub cache directory.
const HF_CACHE_APP_NAMES: &[&str] = &["unsloth", "Unsloth", "Unsloth Studio", "huggingface-cli"];
const OMLX_NAMES: &[&str] = &["omlx", "OMLX"];
const OLLAMA_NAMES: &[&str] = &["ollama", "Ollama"];
const JAN_NAMES: &[&str] = &["jan", "Jan"];

pub fn check_running_apps() -> RunningAppsCheck {
    RunningAppsCheck {
        lmstudio_running: is_process_running(LM_STUDIO_NAMES),
        huggingface_running: is_process_running(HF_CACHE_APP_NAMES),
        omlx_running: is_process_running(OMLX_NAMES),
        ollama_running: is_process_running(OLLAMA_NAMES),
        jan_running: is_process_running(JAN_NAMES),
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
    let mut apps = Vec::new();
    if check.lmstudio_running {
        apps.push("LM Studio");
    }
    if check.huggingface_running {
        apps.push("Hugging Face");
    }
    if check.omlx_running {
        apps.push("oMLX");
    }
    if check.ollama_running {
        apps.push("Ollama");
    }
    if check.jan_running {
        apps.push("Jan");
    }
    if apps.is_empty() {
        return Ok(());
    }
    Err(AppError::msg(format!(
        "{} is running. Close it before destructive operations or disable the warning in Settings.",
        apps.join(", ")
    )))
}
