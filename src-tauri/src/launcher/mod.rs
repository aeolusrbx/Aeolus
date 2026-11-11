//! Launching the Roblox Player.
//!
//! Supported launch inputs are deliberately limited to what the Roblox Player
//! itself supports: opening the Roblox app (`--app`) or handing over a
//! `roblox-player:` / `roblox:` link produced by the Roblox website.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LaunchResult {
    Started,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRecord {
    pub id: String,
    pub at: chrono::DateTime<chrono::Local>,
    pub version: Option<String>,
    pub install_path: Option<String>,
    pub profile: Option<String>,
    pub fast_flag_profile: Option<String>,
    /// "app" or "link"
    pub mode: String,
    pub result: LaunchResult,
    pub error: Option<String>,
    pub suggestions: Vec<String>,
    pub pid: Option<u32>,
}

/// Validate a protocol link before passing it to the player.
pub fn validate_link(link: &str) -> AppResult<()> {
    let lower = link.to_ascii_lowercase();
    if !(lower.starts_with("roblox-player:") || lower.starts_with("roblox:")) {
        return Err(AppError::Validation("Only roblox-player: and roblox: links can be launched".into()));
    }
    if link.len() > 8192 || link.chars().any(|c| c.is_control() || c == '"') {
        return Err(AppError::Validation("The Roblox link contains invalid characters".into()));
    }
    Ok(())
}

pub fn build_args(link: Option<&str>) -> AppResult<Vec<String>> {
    match link {
        Some(l) => {
            validate_link(l)?;
            Ok(vec![l.to_string()])
        }
        None => Ok(vec!["--app".into()]),
    }
}

/// Spawn the player and watch it briefly. A process that exits with a failure
/// code within the watch window is reported as a failed launch. Returns the pid
/// and, if the player is still running after the watch window, its process handle
/// (a clean early exit means it handed off to an already-running Roblox).
pub async fn spawn_and_watch(exe: &Path, args: &[String], watch: Duration) -> AppResult<(u32, Option<std::process::Child>)> {
    if !exe.is_file() {
        return Err(AppError::NotFound(format!("{} was not found", exe.display())));
    }
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args);
    if let Some(dir) = exe.parent() {
        cmd.current_dir(dir);
    }
    let mut child = cmd.spawn().map_err(|e| {
        AppError::Io(std::io::Error::new(e.kind(), format!("Windows refused to start Roblox: {e}")))
    })?;
    let pid = child.id();
    let exe_s = exe.to_path_buf();
    tokio::task::spawn_blocking(move || -> AppResult<(u32, Option<std::process::Child>)> {
        let start = std::time::Instant::now();
        while start.elapsed() < watch {
            match child.try_wait()? {
                Some(status) if !status.success() => {
                    return Err(AppError::Other(format!(
                        "Roblox exited immediately ({status}). {} may be damaged or blocked.",
                        exe_s.file_name().unwrap_or_default().to_string_lossy()
                    )))
                }
                Some(_) => return Ok((pid, None)), // clean exit, e.g. handed off to an existing instance
                None => std::thread::sleep(Duration::from_millis(250)),
            }
        }
        Ok((pid, Some(child)))
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?
}

pub fn troubleshooting(err: &AppError) -> Vec<String> {
    let mut v = vec![];
    match err.kind() {
        "not_found" => {
            v.push("Install Roblox from the Installations page, or choose another detected installation.".into());
        }
        "permission" => {
            v.push("Check whether antivirus software quarantined RobloxPlayerBeta.exe.".into());
            v.push("Make sure the installation folder is not read-only.".into());
        }
        "integrity" => v.push("Run Repair on the Installations page to restore damaged files.".into()),
        "network" => v.push("Check your connection; Aeolus needs it to check for Roblox updates.".into()),
        _ => {
            v.push("Run Verify on the Installations page.".into());
            v.push("If it keeps failing, reinstall Roblox from the Installations page.".into());
        }
    }
    v.push("Open Diagnostics to review the log for details.".into());
    v
}

pub fn player_exe(install_dir: &Path) -> PathBuf {
    install_dir.join(crate::installation::manifest::PLAYER_EXE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_and_links() {
        assert_eq!(build_args(None).unwrap(), vec!["--app"]);
        assert!(build_args(Some("roblox-player:1+launchmode:play+placeId:1")).is_ok());
        assert!(build_args(Some("roblox://experiences/start?placeId=1")).is_ok());
        assert!(build_args(Some("https://evil")).is_err());
        assert!(build_args(Some("roblox-player:1\" --other")).is_err());
        assert!(build_args(Some("roblox-player:\n")).is_err());
    }

    #[tokio::test]
    async fn missing_exe_is_not_found() {
        let e = spawn_and_watch(Path::new("/nonexistent/RobloxPlayerBeta.exe"), &[], Duration::from_millis(10)).await.unwrap_err();
        assert_eq!(e.kind(), "not_found");
        assert!(!troubleshooting(&e).is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn detects_immediate_failure() {
        let d = tempfile::tempdir().unwrap();
        let script = d.path().join("fail.sh");
        std::fs::write(&script, "#!/bin/sh\nexit 3\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(spawn_and_watch(&script, &[], Duration::from_secs(3)).await.is_err());
        let ok = d.path().join("ok.sh");
        std::fs::write(&ok, "#!/bin/sh\nsleep 1\n").unwrap();
        std::fs::set_permissions(&ok, std::fs::Permissions::from_mode(0o755)).unwrap();
        let (_, child) = spawn_and_watch(&ok, &[], Duration::from_millis(300)).await.unwrap();
        let mut child = child.expect("still running after the watch window");
        assert!(child.wait().unwrap().success());
    }
}
