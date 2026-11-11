//! Atomic JSON persistence with automatic backups and corruption recovery.

use crate::error::{AppError, AppResult};
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};

/// Write `bytes` to `path` atomically: write a sibling temp file, flush it, then rename.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp-{}",
        path.extension().and_then(|e| e.to_str()).unwrap_or("dat"),
        uuid::Uuid::new_v4().simple()
    ));
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    atomic_write(path, &bytes)
}

/// Result of loading a JSON document.
pub struct Loaded<T> {
    pub value: T,
    /// Set when the file existed but could not be parsed and was moved aside.
    pub recovered_from: Option<PathBuf>,
}

/// Load a JSON document, falling back to defaults. A corrupt file is preserved
/// (renamed to `*.corrupt-<timestamp>`) rather than deleted.
pub fn load_json_or_default<T: DeserializeOwned + Default>(path: &Path) -> AppResult<Loaded<T>> {
    match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice::<T>(&bytes) {
            Ok(v) => Ok(Loaded { value: v, recovered_from: None }),
            Err(_) => {
                let aside = path.with_extension(format!(
                    "corrupt-{}.json",
                    chrono::Local::now().format("%Y%m%d-%H%M%S")
                ));
                std::fs::rename(path, &aside)?;
                Ok(Loaded { value: T::default(), recovered_from: Some(aside) })
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(Loaded { value: T::default(), recovered_from: None })
        }
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub id: String,
    pub category: String,
    pub label: String,
    pub created_at: chrono::DateTime<chrono::Local>,
    pub size: u64,
}

/// Simple versioned backup folder: `<root>/<category>/<timestamp>__<label>.json`.
pub struct Backups {
    root: PathBuf,
}

impl Backups {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn sanitize(label: &str) -> String {
        label
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
            .take(48)
            .collect()
    }

    /// Back up raw bytes. Keeps at most `keep` entries per category (oldest pruned).
    pub fn create(&self, category: &str, label: &str, bytes: &[u8], keep: usize) -> AppResult<BackupEntry> {
        let dir = self.root.join(Self::sanitize(category));
        std::fs::create_dir_all(&dir)?;
        let now = chrono::Local::now();
        let id = format!("{}__{}", now.format("%Y%m%d-%H%M%S-%3f"), Self::sanitize(label));
        let path = dir.join(format!("{id}.json"));
        atomic_write(&path, bytes)?;
        self.prune(category, keep.max(1))?;
        Ok(BackupEntry {
            id,
            category: category.to_string(),
            label: label.to_string(),
            created_at: now,
            size: bytes.len() as u64,
        })
    }

    /// Back up an existing file if it exists.
    pub fn create_from_file(&self, category: &str, label: &str, file: &Path, keep: usize) -> AppResult<Option<BackupEntry>> {
        match std::fs::read(file) {
            Ok(b) => Ok(Some(self.create(category, label, &b, keep)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list(&self, category: &str) -> AppResult<Vec<BackupEntry>> {
        let dir = self.root.join(Self::sanitize(category));
        let mut out = vec![];
        let rd = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(e.into()),
        };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let id = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let meta = ent.metadata()?;
            let created_at = meta
                .modified()
                .map(chrono::DateTime::<chrono::Local>::from)
                .unwrap_or_else(|_| chrono::Local::now());
            let label = id.split_once("__").map(|(_, l)| l.to_string()).unwrap_or_default();
            out.push(BackupEntry { id, category: category.into(), label, created_at, size: meta.len() });
        }
        out.sort_by(|a, b| b.id.cmp(&a.id));
        Ok(out)
    }

    pub fn read(&self, category: &str, id: &str) -> AppResult<Vec<u8>> {
        if id.contains(['/', '\\']) || id.contains("..") {
            return Err(AppError::Validation("invalid backup id".into()));
        }
        let p = self.root.join(Self::sanitize(category)).join(format!("{id}.json"));
        std::fs::read(&p).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => AppError::NotFound(format!("backup {id} not found")),
            _ => e.into(),
        })
    }

    fn prune(&self, category: &str, keep: usize) -> AppResult<()> {
        let list = self.list(category)?;
        for old in list.into_iter().skip(keep) {
            let p = self.root.join(Self::sanitize(category)).join(format!("{}.json", old.id));
            let _ = std::fs::remove_file(p);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize, Default, PartialEq, Debug)]
    struct Doc {
        a: u32,
    }

    #[test]
    fn atomic_roundtrip_and_corruption_recovery() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("doc.json");
        write_json(&p, &Doc { a: 7 }).unwrap();
        let l: Loaded<Doc> = load_json_or_default(&p).unwrap();
        assert_eq!(l.value, Doc { a: 7 });
        std::fs::write(&p, b"{not json").unwrap();
        let l: Loaded<Doc> = load_json_or_default(&p).unwrap();
        assert_eq!(l.value, Doc::default());
        let aside = l.recovered_from.unwrap();
        assert!(aside.exists(), "corrupt file must be preserved");
        assert_eq!(std::fs::read(aside).unwrap(), b"{not json");
        // no leftover tmp files
        let tmp = std::fs::read_dir(d.path()).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains(".tmp-")).count();
        assert_eq!(tmp, 0);
    }

    #[test]
    fn backups_prune_and_restore() {
        let d = tempfile::tempdir().unwrap();
        let b = Backups::new(d.path().to_path_buf());
        for i in 0..5 {
            b.create("flags", &format!("v{i}"), format!("{{\"i\":{i}}}").as_bytes(), 3).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let list = b.list("flags").unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].label, "v4");
        assert_eq!(b.read("flags", &list[0].id).unwrap(), b"{\"i\":4}");
        assert!(b.read("flags", "../x").is_err());
    }
}
