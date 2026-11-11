//! Structured, rotating (per-day) log files in JSON-lines format.

use crate::error::AppResult;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "UPPERCASE")]
pub enum Level {
    Debug,
    Info,
    Warning,
    Error,
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warning => "WARNING",
            Level::Error => "ERROR",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub ts: chrono::DateTime<chrono::Local>,
    pub level: Level,
    pub source: String,
    pub message: String,
}

static SECRET_PATTERNS: Lazy<Vec<regex::Regex>> = Lazy::new(|| {
    vec![
        // .ROBLOSECURITY cookie values
        regex::Regex::new(r"_\|WARNING:-DO-NOT-SHARE-THIS[^\s;\x22']*").unwrap(),
        regex::Regex::new(r"(?i)(roblosecurity|authorization|cookie|token|password|secret)(\s*[=:]\s*)[^\s,;\x22']+").unwrap(),
        // Windows user names inside paths
        regex::Regex::new(r"(?i)([A-Z]:\\Users\\)[^\\\s\x22']+").unwrap(),
    ]
});

/// Remove credentials and personal identifiers before writing or exporting.
pub fn redact(input: &str) -> String {
    let s = SECRET_PATTERNS[0].replace_all(input, "[REDACTED]");
    let s = SECRET_PATTERNS[1].replace_all(&s, "${1}${2}[REDACTED]");
    let s = SECRET_PATTERNS[2].replace_all(&s, "${1}<user>");
    s.into_owned()
}

pub struct Logger {
    dir: PathBuf,
    debug: AtomicBool,
    file: Mutex<Option<(String, std::fs::File)>>,
}

impl Logger {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, debug: AtomicBool::new(false), file: Mutex::new(None) }
    }

    pub fn set_debug(&self, on: bool) {
        self.debug.store(on, Ordering::Relaxed);
    }

    pub fn log(&self, level: Level, source: &str, message: impl AsRef<str>) {
        if level == Level::Debug && !self.debug.load(Ordering::Relaxed) {
            return;
        }
        let entry = LogEntry {
            ts: chrono::Local::now(),
            level,
            source: source.to_string(),
            message: redact(message.as_ref()),
        };
        #[cfg(debug_assertions)]
        eprintln!("[{:?}] {}: {}", entry.level, entry.source, entry.message);
        let Ok(line) = serde_json::to_string(&entry) else { return };
        let day = entry.ts.format("%Y-%m-%d").to_string();
        let mut guard = match self.file.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if guard.as_ref().map(|(d, _)| d != &day).unwrap_or(true) {
            let _ = std::fs::create_dir_all(&self.dir);
            let path = self.dir.join(format!("aeolus-{day}.log"));
            match std::fs::OpenOptions::new().create(true).append(true).open(path) {
                Ok(f) => *guard = Some((day, f)),
                Err(_) => return,
            }
        }
        if let Some((_, f)) = guard.as_mut() {
            let _ = writeln!(f, "{line}");
        }
    }

    pub fn info(&self, src: &str, m: impl AsRef<str>) {
        self.log(Level::Info, src, m)
    }
    pub fn warn(&self, src: &str, m: impl AsRef<str>) {
        self.log(Level::Warning, src, m)
    }
    pub fn error(&self, src: &str, m: impl AsRef<str>) {
        self.log(Level::Error, src, m)
    }
    pub fn debug(&self, src: &str, m: impl AsRef<str>) {
        self.log(Level::Debug, src, m)
    }

    fn files(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(&self.dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| {
                        let n = p.file_name().unwrap_or_default().to_string_lossy();
                        n.starts_with("aeolus-") && n.ends_with(".log")
                    })
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }

    /// Query logs, newest first.
    pub fn query(&self, q: &LogQuery) -> AppResult<Vec<LogEntry>> {
        let needle = q.search.as_deref().map(str::to_lowercase).filter(|s| !s.is_empty());
        let mut out = vec![];
        for f in self.files().into_iter().rev() {
            let text = std::fs::read_to_string(&f).unwrap_or_default();
            for line in text.lines().rev() {
                let Ok(e) = serde_json::from_str::<LogEntry>(line) else { continue };
                if let Some(levels) = &q.levels {
                    if !levels.contains(&e.level) {
                        continue;
                    }
                }
                if let Some(from) = q.from {
                    if e.ts < from {
                        continue;
                    }
                }
                if let Some(to) = q.to {
                    if e.ts > to {
                        continue;
                    }
                }
                if let Some(n) = &needle {
                    if !e.message.to_lowercase().contains(n) && !e.source.to_lowercase().contains(n) {
                        continue;
                    }
                }
                out.push(e);
                if out.len() >= q.limit.unwrap_or(2000) {
                    return Ok(out);
                }
            }
        }
        Ok(out)
    }

    /// Remove all log files (the current file is truncated).
    pub fn clear(&self) -> AppResult<usize> {
        let mut guard = self.file.lock().unwrap_or_else(|p| p.into_inner());
        *guard = None;
        let mut n = 0;
        for f in self.files() {
            if std::fs::remove_file(&f).is_ok() {
                n += 1;
            }
        }
        Ok(n)
    }

    /// Delete log files older than `days`.
    pub fn enforce_retention(&self, days: u32) -> usize {
        let cutoff = chrono::Local::now().date_naive() - chrono::Duration::days(days as i64);
        let mut n = 0;
        for f in self.files() {
            let name = f.file_stem().unwrap_or_default().to_string_lossy().to_string();
            if let Some(d) = name.strip_prefix("aeolus-") {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d") {
                    if date < cutoff && std::fs::remove_file(&f).is_ok() {
                        n += 1;
                    }
                }
            }
        }
        n
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LogQuery {
    pub search: Option<String>,
    pub levels: Option<Vec<Level>>,
    pub from: Option<chrono::DateTime<chrono::Local>>,
    pub to: Option<chrono::DateTime<chrono::Local>>,
    pub limit: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secrets_and_usernames() {
        let s = redact(r"cookie=abc123 path C:\Users\matthew\AppData _|WARNING:-DO-NOT-SHARE-THIS.--xyz");
        assert!(!s.contains("abc123"));
        assert!(!s.contains("matthew"));
        assert!(!s.contains("xyz"));
        assert!(s.contains(r"C:\Users\<user>\AppData"));
    }

    #[test]
    fn writes_filters_and_clears() {
        let d = tempfile::tempdir().unwrap();
        let l = Logger::new(d.path().to_path_buf());
        l.info("app", "started");
        l.debug("app", "hidden");
        l.set_debug(true);
        l.debug("app", "visible");
        l.error("install", "Download failed");
        let all = l.query(&LogQuery::default()).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].message, "Download failed");
        let errs = l.query(&LogQuery { levels: Some(vec![Level::Error]), ..Default::default() }).unwrap();
        assert_eq!(errs.len(), 1);
        let s = l.query(&LogQuery { search: Some("START".into()), ..Default::default() }).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(l.clear().unwrap(), 1);
        assert!(l.query(&LogQuery::default()).unwrap().is_empty());
        l.info("app", "after clear");
        assert_eq!(l.query(&LogQuery::default()).unwrap().len(), 1);
    }

    #[test]
    fn retention_removes_old_files() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("aeolus-2001-01-01.log"), "").unwrap();
        let l = Logger::new(d.path().to_path_buf());
        l.info("x", "today");
        assert_eq!(l.enforce_retention(14), 1);
        assert_eq!(l.query(&LogQuery::default()).unwrap().len(), 1);
    }
}
