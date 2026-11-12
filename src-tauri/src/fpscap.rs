//! FPS cap through Roblox's own settings file.
//!
//! Roblox keeps the in-game "Maximum Frame Rate" option in
//! `%LOCALAPPDATA%\Roblox\GlobalBasicSettings_13.xml` as
//! `<int name="FramerateCap">240</int>`. Aeolus writes that value right before
//! launching. Roblox officially supports up to 240; higher values may be
//! ignored or reset by Roblox.

use crate::error::{AppError, AppResult};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};

pub const MIN_CAP: u32 = 15;
pub const MAX_CAP: u32 = 9999;

static CAP: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(<int name="FramerateCap">)(-?\d+)(</int>)"#).unwrap());

/// `%LOCALAPPDATA%\Roblox\GlobalBasicSettings_13.xml`
pub fn settings_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join("Roblox").join("GlobalBasicSettings_13.xml"))
}

pub fn validate(cap: u32) -> AppResult<()> {
    if !(MIN_CAP..=MAX_CAP).contains(&cap) {
        return Err(AppError::Validation(format!("The FPS cap must be between {MIN_CAP} and {MAX_CAP}.")));
    }
    Ok(())
}

pub fn read_cap(xml: &str) -> Option<i64> {
    CAP.captures(xml).and_then(|c| c[2].parse().ok())
}

/// Return the settings XML with the cap set, or None if the file doesn't look
/// like Roblox's settings file.
pub fn with_cap(xml: &str, cap: u32) -> Option<String> {
    if CAP.is_match(xml) {
        return Some(CAP.replace(xml, format!("${{1}}{cap}${{3}}")).into_owned());
    }
    // Older files may not have the value yet: add it to UserGameSettings.
    let item = xml.find(r#"<Item class="UserGameSettings""#)?;
    let end = item + xml[item..].find("</Properties>")?;
    let line_start = xml[..end].rfind('\n').map(|i| i + 1).unwrap_or(end);
    let indent = &xml[line_start..end];
    let mut out = String::with_capacity(xml.len() + 48);
    out.push_str(&xml[..line_start]);
    out.push_str(&format!("{indent}\t<int name=\"FramerateCap\">{cap}</int>\n"));
    out.push_str(&xml[line_start..]);
    Some(out)
}

/// Write the cap into Roblox's settings file. Returns the previous value.
/// If Roblox has never run (no settings file yet), nothing is written.
pub fn apply(path: &Path, cap: u32) -> AppResult<Option<i64>> {
    validate(cap)?;
    let xml = match std::fs::read_to_string(path) {
        Ok(x) => x,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let previous = read_cap(&xml);
    if previous == Some(cap as i64) {
        return Ok(previous);
    }
    let new = with_cap(&xml, cap).ok_or_else(|| AppError::Other("Roblox's settings file has an unexpected format, so the FPS cap was not changed.".into()))?;
    let tmp = path.with_extension("xml.aeolus-tmp");
    std::fs::write(&tmp, new)?;
    std::fs::rename(&tmp, path)?;
    Ok(previous)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "<roblox version=\"4\">\n\t<External>null</External>\n\t<Item class=\"UserGameSettings\" referent=\"RBX1\">\n\t\t<Properties>\n\t\t\t<bool name=\"AllTutorialsDisabled\">false</bool>\n\t\t\t<int name=\"FramerateCap\">240</int>\n\t\t\t<int name=\"RCCProfilerRecordFrameRate\">1</int>\n\t\t</Properties>\n\t</Item>\n</roblox>";

    #[test]
    fn replaces_existing_value_only() {
        let out = with_cap(SAMPLE, 1000).unwrap();
        assert_eq!(read_cap(&out), Some(1000));
        assert!(out.contains("<int name=\"RCCProfilerRecordFrameRate\">1</int>"));
        assert_eq!(out.len(), SAMPLE.len() + 1);
    }

    #[test]
    fn adds_missing_value() {
        let without = SAMPLE.replace("\t\t\t<int name=\"FramerateCap\">240</int>\n", "");
        let out = with_cap(&without, 144).unwrap();
        assert_eq!(read_cap(&out), Some(144));
        assert!(out.contains("\t\t\t<int name=\"FramerateCap\">144</int>\n\t\t</Properties>"));
        assert!(with_cap("<html/>", 60).is_none());
    }

    #[test]
    fn writes_file_and_validates() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("GlobalBasicSettings_13.xml");
        assert_eq!(apply(&p, 60).unwrap(), None); // no file: nothing to do
        std::fs::write(&p, SAMPLE).unwrap();
        assert_eq!(apply(&p, 60).unwrap(), Some(240));
        assert_eq!(read_cap(&std::fs::read_to_string(&p).unwrap()), Some(60));
        assert!(apply(&p, 5).is_err());
        assert!(apply(&p, 100_000).is_err());
    }
}
