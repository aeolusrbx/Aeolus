//! Custom skies.
//!
//! A sky pack is the six Roblox sky textures (`sky512_bk/dn/ft/lf/rt/up.tex`),
//! optionally with extra `.tex` files from Roblox's `content/sky` folder and a
//! preview image. Packs are stored under the Aeolus config folder. Applying a
//! pack copies its textures over `<install>/content/sky`, after saving the
//! originals once; restoring puts the originals back. Only files that already
//! exist in Roblox's sky folder are ever replaced.
//!
//! Note: games that define their own Sky object override the default sky, so a
//! custom sky only shows in games that use Roblox's default one.

use crate::configuration::store;
use crate::error::{AppError, AppResult};
use crate::paths::safe_join;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const FACES: [&str; 6] = ["bk", "dn", "ft", "lf", "rt", "up"];
pub const PACK_FILE: &str = "aeolus-sky.json";
pub const OVERLAY_FILE: &str = ".aeolus-overlay.json";
pub const ORIGINALS_DIR: &str = ".aeolus-originals";
const SKY_DIR: &str = "content/sky";
const MAX_FILE: u64 = 32 * 1024 * 1024;
const MAX_FILES: usize = 64;

/// Profile value meaning "use Roblox's own sky, even if a global sky is set".
pub const ROBLOX_DEFAULT: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkyPack {
    pub id: String,
    pub name: String,
    pub files: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Local>,
    #[serde(default)]
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Overlay {
    pub sky_pack_id: Option<String>,
    pub sky_pack_name: Option<String>,
    /// Paths (relative to the install) that currently hold Aeolus-provided files.
    pub files: Vec<String>,
}

fn is_tex(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".tex") && n.len() <= 80 && n.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn is_image(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".png") || n.ends_with(".jpg") || n.ends_with(".jpeg") || n.ends_with(".webp")
}

pub fn required_names() -> Vec<String> {
    FACES.iter().map(|f| format!("sky512_{f}.tex")).collect()
}

/// Files found in a source (folder or zip): lowercase file name -> bytes.
type Found = Vec<(String, Vec<u8>)>;

fn read_capped(mut r: impl Read, name: &str) -> AppResult<Vec<u8>> {
    let mut buf = Vec::new();
    r.by_ref().take(MAX_FILE + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > MAX_FILE {
        return Err(AppError::Validation(format!("{name} is larger than 32 MB, which is too big for a sky texture")));
    }
    Ok(buf)
}

/// Group candidate files by the folder they're in, then pick the folder that has all six faces.
fn choose(entries: Vec<(String, String, Vec<u8>)>) -> AppResult<Found> {
    use std::collections::BTreeMap;
    let mut by_dir: BTreeMap<String, Found> = BTreeMap::new();
    for (dir, name, bytes) in entries {
        by_dir.entry(dir).or_default().push((name, bytes));
    }
    let req = required_names();
    for (_, files) in by_dir {
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        if req.iter().all(|r| names.contains(&r.as_str())) {
            return Ok(files);
        }
    }
    Err(AppError::Validation(format!(
        "No complete sky found. A sky pack needs these six files in one folder: {}",
        req.join(", ")
    )))
}

fn collect_folder(root: &Path) -> AppResult<Found> {
    let mut entries = vec![];
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        for e in std::fs::read_dir(&dir)?.flatten() {
            let p = e.path();
            let ft = e.file_type()?;
            if ft.is_dir() && depth < 3 {
                stack.push((p, depth + 1));
            } else if ft.is_file() {
                let name = e.file_name().to_string_lossy().to_ascii_lowercase();
                if is_tex(&name) || is_image(&name) {
                    let bytes = read_capped(std::fs::File::open(&p)?, &name)?;
                    entries.push((dir.to_string_lossy().to_string(), name, bytes));
                    if entries.len() > MAX_FILES * 4 {
                        return Err(AppError::Validation("That folder has too many files to be a sky pack".into()));
                    }
                }
            }
        }
    }
    choose(entries)
}

fn collect_zip(path: &Path) -> AppResult<Found> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    let mut entries = vec![];
    for i in 0..zip.len() {
        let f = zip.by_index(i)?;
        if f.is_dir() {
            continue;
        }
        let full = f.name().replace('\\', "/");
        let (dir, name) = match full.rsplit_once('/') {
            Some((d, n)) => (d.to_string(), n.to_ascii_lowercase()),
            None => (String::new(), full.to_ascii_lowercase()),
        };
        if dir.split('/').count() > 4 || !(is_tex(&name) || is_image(&name)) {
            continue;
        }
        let bytes = read_capped(f, &name)?;
        entries.push((dir, name, bytes));
        if entries.len() > MAX_FILES * 4 {
            return Err(AppError::Validation("That zip has too many files to be a sky pack".into()));
        }
    }
    choose(entries)
}

pub struct SkyLibrary {
    pub dir: PathBuf,
}

impl SkyLibrary {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn list(&self) -> Vec<SkyPack> {
        let mut v: Vec<SkyPack> = std::fs::read_dir(&self.dir)
            .map(|rd| {
                rd.flatten()
                    .filter_map(|e| std::fs::read(e.path().join(PACK_FILE)).ok())
                    .filter_map(|b| serde_json::from_slice::<SkyPack>(&b).ok())
                    .collect()
            })
            .unwrap_or_default();
        v.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        v
    }

    pub fn get(&self, id: &str) -> AppResult<SkyPack> {
        let p = safe_join(&self.dir, id).ok_or_else(|| AppError::Validation("invalid sky id".into()))?;
        let b = std::fs::read(p.join(PACK_FILE)).map_err(|_| AppError::NotFound("That sky pack no longer exists".into()))?;
        Ok(serde_json::from_slice(&b)?)
    }

    fn pack_dir(&self, id: &str) -> AppResult<PathBuf> {
        safe_join(&self.dir, id).ok_or_else(|| AppError::Validation("invalid sky id".into()))
    }

    /// Import a sky pack from a folder or a .zip file.
    pub fn import(&self, source: &Path, name: Option<&str>) -> AppResult<SkyPack> {
        let is_zip = source.is_file() && source.extension().map(|e| e.eq_ignore_ascii_case("zip")).unwrap_or(false);
        let found = if is_zip {
            collect_zip(source)?
        } else if source.is_dir() {
            collect_folder(source)?
        } else {
            return Err(AppError::Validation("Choose a folder or a .zip file containing the sky's .tex files".into()));
        };
        let default_name = source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Custom sky".into());
        let name = name.map(str::trim).filter(|n| !n.is_empty()).map(str::to_string).unwrap_or(default_name);
        let name: String = name.chars().take(60).collect();

        let id = uuid::Uuid::new_v4().simple().to_string();
        let dir = self.pack_dir(&id)?;
        std::fs::create_dir_all(&dir)?;
        let mut files = vec![];
        let mut preview = None;
        for (fname, bytes) in found {
            if is_tex(&fname) {
                if files.len() >= MAX_FILES {
                    continue;
                }
                std::fs::write(dir.join(&fname), &bytes)?;
                files.push(fname);
            } else if preview.is_none() && is_image(&fname) {
                let ext = fname.rsplit('.').next().unwrap_or("png").to_string();
                let pname = format!("preview.{ext}");
                std::fs::write(dir.join(&pname), &bytes)?;
                preview = Some(pname);
            }
        }
        files.sort();
        let pack = SkyPack { id, name, files, created_at: chrono::Local::now(), preview };
        store::write_json(&dir.join(PACK_FILE), &pack)?;
        Ok(pack)
    }

    pub fn rename(&self, id: &str, name: &str) -> AppResult<SkyPack> {
        let n = name.trim();
        if n.is_empty() || n.len() > 60 {
            return Err(AppError::Validation("Sky names must be 1-60 characters".into()));
        }
        let mut p = self.get(id)?;
        p.name = n.to_string();
        store::write_json(&self.pack_dir(id)?.join(PACK_FILE), &p)?;
        Ok(p)
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        self.get(id)?;
        std::fs::remove_dir_all(self.pack_dir(id)?)?;
        Ok(())
    }

    /// The pack's preview image as a data URL, if it has one.
    pub fn preview_data_url(&self, id: &str) -> Option<String> {
        let p = self.get(id).ok()?;
        let name = p.preview?;
        let bytes = std::fs::read(self.pack_dir(id).ok()?.join(&name)).ok()?;
        let mime = match name.rsplit('.').next().unwrap_or("") {
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            _ => "image/png",
        };
        use base64::Engine;
        Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
    }
}

pub fn read_overlay(install: &Path) -> Overlay {
    std::fs::read(install.join(OVERLAY_FILE))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Put Roblox's original sky files back and forget the overlay.
pub fn restore(install: &Path) -> AppResult<usize> {
    let ov = read_overlay(install);
    let mut n = 0;
    for rel in &ov.files {
        let (Some(target), Some(orig)) = (safe_join(install, rel), safe_join(&install.join(ORIGINALS_DIR), rel)) else { continue };
        if orig.is_file() {
            std::fs::copy(&orig, &target)?;
            n += 1;
        }
    }
    let _ = std::fs::remove_dir_all(install.join(ORIGINALS_DIR));
    let _ = std::fs::remove_file(install.join(OVERLAY_FILE));
    Ok(n)
}

/// Apply `pack` (or Roblox's default sky when `None`) to an installation.
/// Returns the number of files written and any skipped file names.
pub fn apply(install: &Path, library: &SkyLibrary, pack: Option<&SkyPack>) -> AppResult<(usize, Vec<String>)> {
    if !install.join(crate::installation::manifest::PLAYER_EXE).exists() {
        return Err(AppError::NotFound(format!("{} is not a Roblox installation", install.display())));
    }
    let current = read_overlay(install);
    if pack.map(|p| p.id.as_str()) == current.sky_pack_id.as_deref() && current.files.iter().all(|f| install.join(f).exists()) {
        return Ok((0, vec![])); // already applied
    }
    restore(install)?;
    let Some(pack) = pack else { return Ok((0, vec![])) };

    let sky_dir = install.join(SKY_DIR);
    let src_dir = library.pack_dir(&pack.id)?;
    let mut written = vec![];
    let mut skipped = vec![];
    for f in &pack.files {
        let rel = format!("{SKY_DIR}/{f}");
        let target = sky_dir.join(f);
        // Only replace files Roblox actually has, so we never add unknown files.
        if !target.is_file() {
            skipped.push(f.clone());
            continue;
        }
        let orig = install.join(ORIGINALS_DIR).join(&rel);
        if let Some(parent) = orig.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !orig.exists() {
            std::fs::copy(&target, &orig)?;
        }
        std::fs::copy(src_dir.join(f), &target)?;
        written.push(rel);
    }
    if written.is_empty() {
        let _ = std::fs::remove_dir_all(install.join(ORIGINALS_DIR));
        return Err(AppError::Validation(format!(
            "This Roblox version has no sky files matching '{}'. The pack may be for a different Roblox version.",
            pack.name
        )));
    }
    let n = written.len();
    store::write_json(
        &install.join(OVERLAY_FILE),
        &Overlay { sky_pack_id: Some(pack.id.clone()), sky_pack_name: Some(pack.name.clone()), files: written },
    )?;
    Ok((n, skipped))
}

/// Which sky a launch should use: a profile setting overrides the global one.
pub fn resolve(profile_choice: Option<&str>, global: Option<&str>) -> Option<String> {
    match profile_choice {
        Some(ROBLOX_DEFAULT) => None,
        Some(id) => Some(id.to_string()),
        None => global.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn fake_install(root: &Path) -> PathBuf {
        let inst = root.join("version-abc");
        std::fs::create_dir_all(inst.join(SKY_DIR)).unwrap();
        std::fs::write(inst.join("RobloxPlayerBeta.exe"), b"exe").unwrap();
        for f in required_names() {
            std::fs::write(inst.join(SKY_DIR).join(&f), format!("orig-{f}")).unwrap();
        }
        std::fs::write(inst.join(SKY_DIR).join("indoor512_up.tex"), b"orig-indoor").unwrap();
        inst
    }

    fn pack_folder(root: &Path, nested: bool) -> PathBuf {
        let base = root.join("My Sunset");
        let dir = if nested { base.join("textures") } else { base.clone() };
        std::fs::create_dir_all(&dir).unwrap();
        for f in required_names() {
            std::fs::write(dir.join(f.to_uppercase()), format!("new-{f}")).unwrap();
        }
        std::fs::write(dir.join("unknown_extra.tex"), b"x").unwrap();
        std::fs::write(dir.join("readme.txt"), b"ignored").unwrap();
        std::fs::write(base.join("preview.png"), b"\x89PNG").unwrap();
        base
    }

    #[test]
    fn import_folder_apply_and_restore() {
        let d = tempfile::tempdir().unwrap();
        let lib = SkyLibrary::new(d.path().join("skies"));
        let pack = lib.import(&pack_folder(d.path(), true), None).unwrap();
        assert_eq!(pack.name, "My Sunset");
        assert_eq!(pack.files.len(), 7, "six faces + extra, names lowercased");
        assert!(pack.files.contains(&"sky512_bk.tex".to_string()));
        assert_eq!(lib.list().len(), 1);

        let inst = fake_install(d.path());
        let (n, skipped) = apply(&inst, &lib, Some(&pack)).unwrap();
        assert_eq!(n, 6);
        assert_eq!(skipped, vec!["unknown_extra.tex".to_string()], "files Roblox doesn't have are never added");
        assert!(!inst.join(SKY_DIR).join("unknown_extra.tex").exists());
        assert_eq!(std::fs::read_to_string(inst.join(SKY_DIR).join("sky512_up.tex")).unwrap(), "new-sky512_up.tex");
        assert_eq!(read_overlay(&inst).sky_pack_id.as_deref(), Some(pack.id.as_str()));
        // re-applying the same pack is a no-op
        assert_eq!(apply(&inst, &lib, Some(&pack)).unwrap().0, 0);

        // default sky restores the originals exactly
        apply(&inst, &lib, None).unwrap();
        assert_eq!(std::fs::read_to_string(inst.join(SKY_DIR).join("sky512_up.tex")).unwrap(), "orig-sky512_up.tex");
        assert!(!inst.join(OVERLAY_FILE).exists());
        assert!(!inst.join(ORIGINALS_DIR).exists());
    }

    #[test]
    fn switching_packs_keeps_true_originals() {
        let d = tempfile::tempdir().unwrap();
        let lib = SkyLibrary::new(d.path().join("skies"));
        let a = lib.import(&pack_folder(d.path(), false), Some("A")).unwrap();
        let src_b = d.path().join("b");
        std::fs::create_dir_all(&src_b).unwrap();
        for f in required_names() {
            std::fs::write(src_b.join(&f), format!("b-{f}")).unwrap();
        }
        let b = lib.import(&src_b, Some("B")).unwrap();
        let inst = fake_install(d.path());
        apply(&inst, &lib, Some(&a)).unwrap();
        apply(&inst, &lib, Some(&b)).unwrap();
        assert_eq!(std::fs::read_to_string(inst.join(SKY_DIR).join("sky512_bk.tex")).unwrap(), "b-sky512_bk.tex");
        restore(&inst).unwrap();
        assert_eq!(std::fs::read_to_string(inst.join(SKY_DIR).join("sky512_bk.tex")).unwrap(), "orig-sky512_bk.tex");
    }

    #[test]
    fn import_zip_and_reject_incomplete() {
        let d = tempfile::tempdir().unwrap();
        let lib = SkyLibrary::new(d.path().join("skies"));
        let zp = d.path().join("Night Sky.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&zp).unwrap());
            let o = zip::write::SimpleFileOptions::default();
            for f in required_names() {
                z.start_file(format!("Night Sky/{f}"), o).unwrap();
                z.write_all(b"tex").unwrap();
            }
            z.start_file("../evil.tex", o).unwrap();
            z.write_all(b"x").unwrap();
            z.finish().unwrap();
        }
        let p = lib.import(&zp, None).unwrap();
        assert_eq!(p.name, "Night Sky");
        assert_eq!(p.files.len(), 6);

        let partial = d.path().join("partial");
        std::fs::create_dir_all(&partial).unwrap();
        std::fs::write(partial.join("sky512_bk.tex"), b"x").unwrap();
        let e = lib.import(&partial, None).unwrap_err();
        assert!(e.to_string().contains("No complete sky"));
        assert!(lib.import(&d.path().join("missing"), None).is_err());

        lib.rename(&p.id, "Stars").unwrap();
        assert_eq!(lib.get(&p.id).unwrap().name, "Stars");
        lib.delete(&p.id).unwrap();
        assert!(lib.get(&p.id).is_err());
        assert!(lib.get("../x").is_err());
    }

    #[test]
    fn verify_does_not_flag_applied_sky_as_damaged() {
        use crate::installation::deploy::{verify, FileIndex, FileRecord, FILE_INDEX};
        let d = tempfile::tempdir().unwrap();
        let lib = SkyLibrary::new(d.path().join("skies"));
        let pack = lib.import(&pack_folder(d.path(), false), None).unwrap();
        let inst = fake_install(d.path());
        let files: Vec<FileRecord> = required_names()
            .into_iter()
            .map(|f| {
                let p = inst.join(SKY_DIR).join(&f);
                FileRecord { path: format!("{SKY_DIR}/{f}"), size: std::fs::metadata(&p).unwrap().len(), sha256: crate::installation::deploy::sha256_file(&p).unwrap() }
            })
            .collect();
        let idx = FileIndex { schema_version: 1, version: "1".into(), version_guid: "version-abc".into(), created_at: chrono::Local::now(), packages: vec![], files };
        store::write_json(&inst.join(FILE_INDEX), &idx).unwrap();
        let c = Default::default();
        apply(&inst, &lib, Some(&pack)).unwrap();
        let r = verify(&inst, true, &c, &|_, _| {}).unwrap();
        assert!(r.ok(), "{r:?}");
        assert_eq!(r.overlaid, 6);
        restore(&inst).unwrap();
        let r = verify(&inst, true, &c, &|_, _| {}).unwrap();
        assert!(r.ok() && r.overlaid == 0);
    }

    #[test]
    fn profile_overrides_global() {
        assert_eq!(resolve(None, Some("g")), Some("g".into()));
        assert_eq!(resolve(Some("p"), Some("g")), Some("p".into()));
        assert_eq!(resolve(Some(ROBLOX_DEFAULT), Some("g")), None);
        assert_eq!(resolve(None, None), None);
    }
}
