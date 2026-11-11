//! Detection of Roblox Player installations on this machine.

use super::deploy::{FileIndex, FILE_INDEX, STAGING_SUFFIX};
use super::manifest::{is_valid_guid, PLAYER_EXE};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DetectedInstall {
    pub id: String,
    /// "aeolus" | "official" | "bloxstrap" | "fishstrap" | "programfiles" | "custom"
    pub source: String,
    pub source_label: String,
    pub path: String,
    pub version_guid: String,
    pub version: Option<String>,
    pub managed: bool,
    pub valid: bool,
    pub issues: Vec<String>,
    pub detected_at: chrono::DateTime<chrono::Local>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    pub installs: Vec<DetectedInstall>,
    /// Interrupted Aeolus installations that can be resumed.
    pub incomplete: Vec<String>,
    pub scanned: Vec<String>,
    pub detected_at: chrono::DateTime<chrono::Local>,
}

pub struct ScanRoot {
    pub source: &'static str,
    pub label: &'static str,
    pub path: PathBuf,
    pub managed: bool,
}

/// Standard locations, resolved from environment variables (never hard-coded drive letters).
pub fn standard_roots(managed_root: &Path, include_official: bool, include_others: bool) -> Vec<ScanRoot> {
    let mut v = vec![ScanRoot { source: "aeolus", label: "Aeolus", path: managed_root.to_path_buf(), managed: true }];
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    if include_official {
        if let Some(l) = &local {
            v.push(ScanRoot { source: "official", label: "Roblox (official)", path: l.join("Roblox").join("Versions"), managed: false });
        }
        if let Some(pf) = std::env::var_os("ProgramFiles(x86)") {
            v.push(ScanRoot { source: "programfiles", label: "Roblox (all users)", path: PathBuf::from(pf).join("Roblox").join("Versions"), managed: false });
        }
    }
    if include_others {
        if let Some(l) = &local {
            v.push(ScanRoot { source: "bloxstrap", label: "Bloxstrap", path: l.join("Bloxstrap").join("Versions"), managed: false });
            v.push(ScanRoot { source: "fishstrap", label: "Fishstrap", path: l.join("Fishstrap").join("Versions"), managed: false });
        }
    }
    v
}

fn id_for(path: &Path) -> String {
    use sha2::Digest;
    hex::encode(&sha2::Sha256::digest(path.to_string_lossy().to_lowercase().as_bytes())[..8])
}

pub fn inspect(dir: &Path, root: &ScanRoot) -> Option<DetectedInstall> {
    let name = dir.file_name()?.to_string_lossy().to_string();
    if !is_valid_guid(&name) {
        return None;
    }
    let exe = dir.join(PLAYER_EXE);
    let has_exe = exe.is_file();
    // Some versions folders contain only Studio; ignore folders with no player traces.
    if !has_exe && !dir.join(FILE_INDEX).exists() && !dir.join("AppSettings.xml").exists() {
        return None;
    }
    let mut issues = vec![];
    if !has_exe {
        issues.push(format!("{PLAYER_EXE} is missing"));
    }
    if !dir.join("AppSettings.xml").is_file() {
        issues.push("AppSettings.xml is missing".into());
    }
    if !dir.join("content").is_dir() {
        issues.push("content folder is missing".into());
    }
    let mut version = None;
    if root.managed {
        match std::fs::read(dir.join(FILE_INDEX)).ok().and_then(|b| serde_json::from_slice::<FileIndex>(&b).ok()) {
            Some(idx) => version = Some(idx.version),
            None => issues.push("Aeolus file index is missing; run Verify or Repair".into()),
        }
    }
    Some(DetectedInstall {
        id: id_for(dir),
        source: root.source.into(),
        source_label: root.label.into(),
        path: dir.to_string_lossy().to_string(),
        version_guid: name,
        version,
        managed: root.managed,
        valid: issues.is_empty(),
        issues,
        detected_at: chrono::Local::now(),
    })
}

pub fn scan(roots: &[ScanRoot]) -> DetectionResult {
    let mut installs = vec![];
    let mut incomplete = vec![];
    let mut scanned = vec![];
    for root in roots {
        let Ok(rd) = std::fs::read_dir(&root.path) else { continue };
        scanned.push(root.path.to_string_lossy().to_string());
        let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        dirs.sort();
        for d in dirs {
            let n = d.file_name().unwrap_or_default().to_string_lossy().to_string();
            if root.managed && n.ends_with(STAGING_SUFFIX) {
                incomplete.push(n.trim_end_matches(STAGING_SUFFIX).to_string());
                continue;
            }
            if let Some(i) = inspect(&d, root) {
                installs.push(i);
            }
        }
    }
    DetectionResult { installs, incomplete, scanned, detected_at: chrono::Local::now() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_valid_invalid_and_incomplete() {
        let d = tempfile::tempdir().unwrap();
        let ok = d.path().join("version-aaaa1111");
        std::fs::create_dir_all(ok.join("content")).unwrap();
        std::fs::write(ok.join(PLAYER_EXE), b"exe").unwrap();
        std::fs::write(ok.join("AppSettings.xml"), b"x").unwrap();
        let broken = d.path().join("version-bbbb2222");
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join("AppSettings.xml"), b"x").unwrap();
        std::fs::create_dir_all(d.path().join("not-a-version")).unwrap();
        std::fs::create_dir_all(d.path().join(format!("version-cccc3333{STAGING_SUFFIX}"))).unwrap();

        let roots = vec![ScanRoot { source: "official", label: "Roblox", path: d.path().into(), managed: false }];
        let r = scan(&roots);
        assert_eq!(r.installs.len(), 2);
        assert!(r.installs[0].valid);
        assert!(!r.installs[1].valid);
        assert!(r.incomplete.is_empty(), "staging dirs only count for managed roots");

        let roots = vec![ScanRoot { source: "aeolus", label: "Aeolus", path: d.path().into(), managed: true }];
        let r = scan(&roots);
        assert_eq!(r.incomplete, vec!["version-cccc3333".to_string()]);
        assert!(!r.installs[0].valid, "managed installs without index are flagged");
    }
}
