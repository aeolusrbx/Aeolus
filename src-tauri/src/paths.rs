//! Locations of Aeolus data on disk. Everything lives under the per-user
//! application data directories (never a temp directory).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Paths {
    /// Roaming config dir (%APPDATA%\dev.aeolus.app): settings, profiles, flags.
    pub config: PathBuf,
    /// Local data dir (%LOCALAPPDATA%\dev.aeolus.app): Roblox versions, downloads, logs.
    pub local: PathBuf,
}

impl Paths {
    pub fn new(config: PathBuf, local: PathBuf) -> Self {
        Self { config, local }
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        for d in [
            self.config.clone(),
            self.local.clone(),
            self.backups(),
            self.logs(),
            self.downloads(),
            self.default_versions_root(),
            self.plugins(),
            self.skies(),
            self.updates_cache(),
        ] {
            std::fs::create_dir_all(d)?;
        }
        Ok(())
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.config.join(name)
    }
    pub fn backups(&self) -> PathBuf {
        self.config.join("backups")
    }
    pub fn skies(&self) -> PathBuf {
        self.config.join("skies")
    }
    pub fn plugins(&self) -> PathBuf {
        self.config.join("plugins")
    }
    pub fn logs(&self) -> PathBuf {
        self.local.join("logs")
    }
    pub fn downloads(&self) -> PathBuf {
        self.local.join("downloads")
    }
    pub fn updates_cache(&self) -> PathBuf {
        self.local.join("self-update")
    }
    pub fn default_versions_root(&self) -> PathBuf {
        self.local.join("Roblox").join("Versions")
    }
}

/// Reject paths that try to escape a base directory (zip-slip, plugin ids, etc.).
pub fn safe_join(base: &Path, relative: &str) -> Option<PathBuf> {
    let rel = relative.replace('\\', "/");
    let mut out = base.to_path_buf();
    for part in rel.split('/') {
        match part {
            "" | "." => continue,
            ".." => return None,
            p if p.contains(':') => return None,
            p => out.push(p),
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_rejects_traversal() {
        let b = Path::new("/base");
        assert!(safe_join(b, "../etc/passwd").is_none());
        assert!(safe_join(b, "a/../../x").is_none());
        assert!(safe_join(b, "C:/Windows").is_none());
        assert_eq!(safe_join(b, "a\\b.txt").unwrap(), Path::new("/base/a/b.txt"));
        assert_eq!(safe_join(b, "./a//b").unwrap(), Path::new("/base/a/b"));
    }
}
