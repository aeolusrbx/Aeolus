pub mod activity;
pub mod logging;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub os_build: String,
    pub architecture: String,
    pub cpu: String,
    pub cpu_cores: usize,
    pub gpus: Vec<String>,
    pub total_memory: u64,
    pub available_memory: u64,
}

/// Collect hardware/OS details. Deliberately excludes user names, host names,
/// serial numbers, network identifiers and anything account-related.
pub fn system_info() -> SystemInfo {
    use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
    let sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_memory(MemoryRefreshKind::nothing().with_ram())
            .with_cpu(CpuRefreshKind::nothing()),
    );
    let cpu = sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_else(|| "Unknown".into());
    SystemInfo {
        os_name: System::long_os_version().unwrap_or_else(|| std::env::consts::OS.into()),
        os_version: System::os_version().unwrap_or_default(),
        os_build: System::kernel_version().unwrap_or_default(),
        architecture: System::cpu_arch(),
        cpu,
        cpu_cores: sys.cpus().len(),
        gpus: gpus(),
        total_memory: sys.total_memory(),
        available_memory: sys.available_memory(),
    }
}

#[cfg(windows)]
fn gpus() -> Vec<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => vec![],
    }
}

#[cfg(not(windows))]
fn gpus() -> Vec<String> {
    vec![]
}
