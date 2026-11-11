//! Optional registration of Aeolus as the handler for `roblox-player:` and
//! `roblox:` links (per-user, under HKCU\Software\Classes). The previous handler
//! is recorded so unregistering restores it exactly.

use crate::configuration::store;
use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const SCHEMES: &[&str] = &["roblox-player", "roblox"];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProtocolBackup {
    /// scheme -> previous open command (None = key did not exist)
    pub previous: std::collections::BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolStatus {
    pub supported: bool,
    pub handlers: Vec<(String, Option<String>)>,
    pub registered_to_aeolus: bool,
}

#[cfg(windows)]
mod imp {
    use super::*;
    use winreg::enums::*;
    use winreg::RegKey;

    fn classes() -> AppResult<RegKey> {
        Ok(RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Software\\Classes", KEY_ALL_ACCESS)?)
    }

    pub fn current(scheme: &str) -> Option<String> {
        let c = RegKey::predef(HKEY_CURRENT_USER).open_subkey(format!("Software\\Classes\\{scheme}\\shell\\open\\command")).ok()?;
        c.get_value::<String, _>("").ok()
    }

    pub fn set(scheme: &str, command: &str) -> AppResult<()> {
        let (key, _) = classes()?.create_subkey(scheme)?;
        key.set_value("", &format!("URL: Roblox Protocol ({scheme})"))?;
        key.set_value("URL Protocol", &"")?;
        let (cmd, _) = key.create_subkey("shell\\open\\command")?;
        cmd.set_value("", &command)?;
        Ok(())
    }

    pub fn remove(scheme: &str) -> AppResult<()> {
        match classes()?.delete_subkey_all(scheme) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    use crate::error::AppError;
    pub fn current(_: &str) -> Option<String> {
        None
    }
    pub fn set(_: &str, _: &str) -> AppResult<()> {
        Err(AppError::Unsupported("Link handling is only available on Windows".into()))
    }
    pub fn remove(_: &str) -> AppResult<()> {
        Err(AppError::Unsupported("Link handling is only available on Windows".into()))
    }
}

pub fn command_for(exe: &Path) -> String {
    format!("\"{}\" \"%1\"", exe.display())
}

pub fn status(exe: &Path) -> ProtocolStatus {
    let ours = command_for(exe);
    let handlers: Vec<(String, Option<String>)> = SCHEMES.iter().map(|s| (s.to_string(), imp::current(s))).collect();
    let registered = handlers.iter().all(|(_, h)| h.as_deref() == Some(ours.as_str()));
    ProtocolStatus { supported: cfg!(windows), handlers, registered_to_aeolus: registered }
}

pub fn register(exe: &Path, backup_file: &Path) -> AppResult<()> {
    let ours = command_for(exe);
    let mut backup = store::load_json_or_default::<ProtocolBackup>(backup_file)?.value;
    for s in SCHEMES {
        let cur = imp::current(s);
        if cur.as_deref() != Some(ours.as_str()) {
            backup.previous.insert(s.to_string(), cur);
        }
    }
    store::write_json(backup_file, &backup)?;
    for s in SCHEMES {
        imp::set(s, &ours)?;
    }
    Ok(())
}

pub fn unregister(exe: &Path, backup_file: &Path) -> AppResult<()> {
    let ours = command_for(exe);
    let backup = store::load_json_or_default::<ProtocolBackup>(backup_file)?.value;
    for s in SCHEMES {
        if imp::current(s).as_deref() != Some(ours.as_str()) {
            continue; // someone else owns it now; leave it alone
        }
        match backup.previous.get(*s).cloned().flatten() {
            Some(prev) => imp::set(s, &prev)?,
            None => imp::remove(s)?,
        }
    }
    Ok(())
}
