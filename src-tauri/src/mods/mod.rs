//! Mods: replace Roblox's built-in sounds, cursors and fonts with your own files
//! (or a preset). Every mod targets files that ship with Roblox; games that use
//! their own sounds/cursors/fonts are unaffected.

use crate::configuration::store;
use crate::error::{AppError, AppResult};
use crate::overlay::Planned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Sound,
    Cursor,
    Font,
}

impl Kind {
    fn extensions(self) -> &'static [&'static str] {
        match self {
            Kind::Sound => &["ogg", "mp3", "wav"],
            Kind::Cursor => &["png"],
            Kind::Font => &["ttf", "otf"],
        }
    }
    fn max_size(self) -> u64 {
        match self {
            Kind::Sound => 20 * 1024 * 1024,
            Kind::Cursor => 2 * 1024 * 1024,
            Kind::Font => 40 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub id: &'static str,
    pub kind: Kind,
    pub label: &'static str,
    pub description: &'static str,
    /// Files in the Roblox install this slot replaces.
    pub targets: &'static [&'static str],
    pub presets: &'static [Preset],
}

const CURSOR_PRESETS: &[Preset] = &[
    Preset { id: "dot", label: "Dot" },
    Preset { id: "crosshair", label: "Crosshair" },
    Preset { id: "ring", label: "Ring" },
];

pub const SLOTS: &[Slot] = &[
    Slot { id: "death", kind: Kind::Sound, label: "Death sound", description: "Plays when your character dies.", targets: &["content/sounds/ouch.ogg"], presets: &[Preset { id: "classic-oof", label: "Classic oof" }] },
    Slot { id: "footsteps", kind: Kind::Sound, label: "Footsteps", description: "Walking and running.", targets: &["content/sounds/action_footsteps_plastic.mp3"], presets: &[] },
    Slot { id: "jump", kind: Kind::Sound, label: "Jump", description: "When you jump.", targets: &["content/sounds/action_jump.mp3"], presets: &[] },
    Slot { id: "land", kind: Kind::Sound, label: "Landing", description: "When you land after a jump.", targets: &["content/sounds/action_jump_land.mp3"], presets: &[] },
    Slot { id: "falling", kind: Kind::Sound, label: "Falling", description: "While falling.", targets: &["content/sounds/action_falling.ogg"], presets: &[] },
    Slot { id: "getup", kind: Kind::Sound, label: "Getting up", description: "When your character gets up.", targets: &["content/sounds/action_get_up.mp3"], presets: &[] },
    Slot { id: "swim", kind: Kind::Sound, label: "Swimming", description: "While swimming.", targets: &["content/sounds/action_swim.mp3"], presets: &[] },
    Slot { id: "explosion", kind: Kind::Sound, label: "Explosion", description: "Built-in explosions.", targets: &["content/sounds/impact_explosion_03.mp3"], presets: &[] },
    Slot { id: "splash", kind: Kind::Sound, label: "Water splash", description: "Hitting water.", targets: &["content/sounds/impact_water.mp3"], presets: &[] },
    Slot {
        id: "cursor",
        kind: Kind::Cursor,
        label: "Cursor",
        description: "Roblox's default mouse cursor. PNG, 64×64 recommended.",
        targets: &["content/textures/ArrowCursor.png", "content/textures/Cursors/KeyboardMouse/ArrowCursor.png"],
        presets: CURSOR_PRESETS,
    },
    Slot {
        id: "cursor-far",
        kind: Kind::Cursor,
        label: "Cursor (far)",
        description: "Shown when pointing at something out of reach. PNG, 64×64 recommended.",
        targets: &["content/textures/ArrowFarCursor.png", "content/textures/Cursors/KeyboardMouse/ArrowFarCursor.png"],
        presets: CURSOR_PRESETS,
    },
    Slot {
        id: "font",
        kind: Kind::Font,
        label: "Interface font",
        description: "Replaces Builder Sans, Roblox's main font (menus, chat and most game text).",
        targets: &[
            "content/fonts/BuilderSans-Regular.otf",
            "content/fonts/BuilderSans-Medium.otf",
            "content/fonts/BuilderSans-Bold.otf",
            "content/fonts/BuilderSans-ExtraBold.otf",
        ],
        presets: &[],
    },
    Slot { id: "emoji", kind: Kind::Font, label: "Emoji font", description: "Changes the style of emoji in chat and text.", targets: &["content/fonts/TwemojiMozilla.ttf"], presets: &[] },
];

pub fn slot(id: &str) -> AppResult<&'static Slot> {
    SLOTS.iter().find(|s| s.id == id).ok_or_else(|| AppError::Validation(format!("Unknown mod '{id}'")))
}

fn preset_bytes(id: &str) -> Option<&'static [u8]> {
    match id {
        "dot" => Some(include_bytes!("../../resources/mods/cursor-dot.png")),
        "crosshair" => Some(include_bytes!("../../resources/mods/cursor-crosshair.png")),
        "ring" => Some(include_bytes!("../../resources/mods/cursor-ring.png")),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModChoice {
    pub enabled: bool,
    /// File the user added (stored in the mods folder), or…
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub original_name: Option<String>,
    /// …a preset id.
    #[serde(default)]
    pub preset: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ModStoreFile {
    pub schema_version: u32,
    pub slots: BTreeMap<String, ModChoice>,
}

pub struct ModStore {
    dir: PathBuf,
    path: PathBuf,
}

/// Check the first bytes so obviously wrong files are rejected.
fn looks_valid(kind: Kind, ext: &str, b: &[u8]) -> bool {
    match kind {
        Kind::Cursor => b.starts_with(b"\x89PNG\r\n\x1a\n"),
        Kind::Font => b.starts_with(&[0, 1, 0, 0]) || b.starts_with(b"OTTO") || b.starts_with(b"true") || b.starts_with(b"ttcf"),
        Kind::Sound => match ext {
            "ogg" => b.starts_with(b"OggS"),
            "wav" => b.starts_with(b"RIFF"),
            _ => b.starts_with(b"ID3") || (b.len() > 1 && b[0] == 0xFF && (b[1] & 0xE0) == 0xE0),
        },
    }
}

impl ModStore {
    pub fn new(dir: PathBuf) -> Self {
        let path = dir.join("mods.json");
        Self { dir, path }
    }

    pub fn load(&self) -> ModStoreFile {
        store::load_json_or_default::<ModStoreFile>(&self.path).map(|l| l.value).unwrap_or_default()
    }

    fn save(&self, f: &ModStoreFile) -> AppResult<()> {
        store::write_json(&self.path, f)
    }

    fn files_dir(&self) -> PathBuf {
        self.dir.join("files")
    }

    /// Store the user's file for a slot.
    pub fn set_file(&self, slot_id: &str, source: &Path) -> AppResult<ModChoice> {
        let slot = slot(slot_id)?;
        let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if !slot.kind.extensions().contains(&ext.as_str()) {
            return Err(AppError::Validation(format!("{} needs a {} file", slot.label, slot.kind.extensions().join(" / ").to_uppercase())));
        }
        let meta = std::fs::metadata(source).map_err(|_| AppError::NotFound("That file can't be opened".into()))?;
        if meta.len() > slot.kind.max_size() {
            return Err(AppError::Validation(format!("That file is too large for {}", slot.label)));
        }
        let bytes = std::fs::read(source)?;
        if !looks_valid(slot.kind, &ext, &bytes) {
            return Err(AppError::Validation(format!("That doesn't look like a real .{ext} file")));
        }
        std::fs::create_dir_all(self.files_dir())?;
        // remove a previous file for this slot
        let mut f = self.load();
        if let Some(old) = f.slots.get(slot_id).and_then(|c| c.file.clone()) {
            let _ = std::fs::remove_file(self.files_dir().join(old));
        }
        let stored = format!("{slot_id}-{}.{ext}", uuid::Uuid::new_v4().simple());
        store::atomic_write(&self.files_dir().join(&stored), &bytes)?;
        let choice = ModChoice {
            enabled: true,
            file: Some(stored),
            original_name: source.file_name().map(|n| n.to_string_lossy().to_string()),
            preset: None,
            updated_at: chrono::Local::now(),
        };
        f.slots.insert(slot_id.into(), choice.clone());
        self.save(&f)?;
        Ok(choice)
    }

    pub fn set_preset(&self, slot_id: &str, preset: &str) -> AppResult<ModChoice> {
        let slot = slot(slot_id)?;
        if !slot.presets.iter().any(|p| p.id == preset) {
            return Err(AppError::Validation(format!("Unknown preset for {}", slot.label)));
        }
        let mut f = self.load();
        if let Some(old) = f.slots.get(slot_id).and_then(|c| c.file.clone()) {
            let _ = std::fs::remove_file(self.files_dir().join(old));
        }
        let choice = ModChoice { enabled: true, file: None, original_name: None, preset: Some(preset.into()), updated_at: chrono::Local::now() };
        f.slots.insert(slot_id.into(), choice.clone());
        self.save(&f)?;
        Ok(choice)
    }

    pub fn set_enabled(&self, slot_id: &str, enabled: bool) -> AppResult<()> {
        slot(slot_id)?;
        let mut f = self.load();
        let c = f.slots.get_mut(slot_id).ok_or_else(|| AppError::NotFound("Add a file or preset first".into()))?;
        c.enabled = enabled;
        c.updated_at = chrono::Local::now();
        self.save(&f)
    }

    pub fn clear(&self, slot_id: &str) -> AppResult<()> {
        slot(slot_id)?;
        let mut f = self.load();
        if let Some(c) = f.slots.remove(slot_id) {
            if let Some(file) = c.file {
                let _ = std::fs::remove_file(self.files_dir().join(file));
            }
        }
        self.save(&f)
    }

    pub fn clear_all(&self) -> AppResult<()> {
        let ids: Vec<String> = self.load().slots.keys().cloned().collect();
        for id in ids {
            self.clear(&id)?;
        }
        Ok(())
    }

    /// Where a choice's bytes come from, for an install (presets may live in the install itself).
    fn source_for(&self, choice: &ModChoice, install: &Path) -> AppResult<Option<PathBuf>> {
        if let Some(file) = &choice.file {
            return Ok(Some(self.files_dir().join(file)));
        }
        match choice.preset.as_deref() {
            // Roblox still ships its classic death sound; use the install's own copy.
            Some("classic-oof") => {
                let orig = install.join(crate::overlay::ORIGINALS_DIR).join("content/sounds/oof.ogg");
                let p = if orig.is_file() { orig } else { install.join("content/sounds/oof.ogg") };
                Ok(p.is_file().then_some(p))
            }
            Some(id) => match preset_bytes(id) {
                Some(bytes) => {
                    let dir = self.dir.join("presets");
                    std::fs::create_dir_all(&dir)?;
                    let p = dir.join(format!("{id}.png"));
                    if std::fs::read(&p).map(|b| b != bytes).unwrap_or(true) {
                        store::atomic_write(&p, bytes)?;
                    }
                    Ok(Some(p))
                }
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    /// Planned overlay entries for all enabled mods, and the slot ids they cover.
    pub fn plan(&self, install: &Path) -> AppResult<(Vec<Planned>, Vec<String>)> {
        let f = self.load();
        let mut planned = vec![];
        let mut used = vec![];
        for s in SLOTS {
            let Some(choice) = f.slots.get(s.id).filter(|c| c.enabled) else { continue };
            let Some(source) = self.source_for(choice, install)? else { continue };
            for t in s.targets {
                planned.push(Planned { rel: (*t).to_string(), source: source.clone() });
            }
            used.push(s.id.to_string());
        }
        Ok((planned, used))
    }

    /// Data URL for an image mod (cursor) so the UI can preview it.
    pub fn preview(&self, slot_id: &str) -> Option<String> {
        let s = slot(slot_id).ok()?;
        if s.kind != Kind::Cursor {
            return None;
        }
        let c = self.load().slots.get(slot_id)?.clone();
        let bytes = match (&c.file, &c.preset) {
            (Some(f), _) => std::fs::read(self.files_dir().join(f)).ok()?,
            (None, Some(p)) => preset_bytes(p)?.to_vec(),
            _ => return None,
        };
        use base64::Engine;
        Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
    }

    pub fn preset_preview(id: &str) -> Option<String> {
        use base64::Engine;
        preset_bytes(id).map(|b| format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(b)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install(root: &Path) -> PathBuf {
        let i = root.join("inst");
        for s in SLOTS {
            for t in s.targets {
                let p = i.join(t);
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(&p, format!("orig:{t}")).unwrap();
            }
        }
        std::fs::write(i.join("content/sounds/oof.ogg"), b"OggS classic").unwrap();
        std::fs::write(i.join("RobloxPlayerBeta.exe"), b"exe").unwrap();
        i
    }

    #[test]
    fn files_presets_and_overlay() {
        let d = tempfile::tempdir().unwrap();
        let m = ModStore::new(d.path().join("mods"));
        let inst = install(d.path());

        // validation
        let bad = d.path().join("x.ogg");
        std::fs::write(&bad, b"not ogg").unwrap();
        assert!(m.set_file("death", &bad).is_err());
        let wrong = d.path().join("x.png");
        std::fs::write(&wrong, b"\x89PNG\r\n\x1a\nxx").unwrap();
        assert!(m.set_file("death", &wrong).is_err(), "png can't be a sound");
        assert!(m.set_file("nope", &wrong).is_err());

        let jump = d.path().join("boing.mp3");
        std::fs::write(&jump, b"ID3 boing").unwrap();
        m.set_file("jump", &jump).unwrap();
        m.set_preset("death", "classic-oof").unwrap();
        m.set_preset("cursor", "dot").unwrap();
        assert!(m.set_preset("death", "dot").is_err());
        assert!(m.preview("cursor").unwrap().starts_with("data:image/png;base64,"));

        let (plan, used) = m.plan(&inst).unwrap();
        assert_eq!(used, vec!["death", "jump", "cursor"]);
        assert_eq!(plan.len(), 4, "cursor covers two files");
        crate::overlay::sync(&inst, &plan, None, &used).unwrap();
        assert_eq!(std::fs::read(inst.join("content/sounds/ouch.ogg")).unwrap(), b"OggS classic");
        assert_eq!(std::fs::read(inst.join("content/sounds/action_jump.mp3")).unwrap(), b"ID3 boing");
        assert!(std::fs::read(inst.join("content/textures/Cursors/KeyboardMouse/ArrowCursor.png")).unwrap().starts_with(b"\x89PNG"));

        // disabling restores the original
        m.set_enabled("jump", false).unwrap();
        let (plan, used) = m.plan(&inst).unwrap();
        crate::overlay::sync(&inst, &plan, None, &used).unwrap();
        assert_eq!(std::fs::read_to_string(inst.join("content/sounds/action_jump.mp3")).unwrap(), "orig:content/sounds/action_jump.mp3");
        // classic oof still works after a re-sync (reads the original from the backup)
        assert_eq!(std::fs::read(inst.join("content/sounds/ouch.ogg")).unwrap(), b"OggS classic");

        m.clear_all().unwrap();
        let (plan, used) = m.plan(&inst).unwrap();
        assert!(plan.is_empty());
        crate::overlay::sync(&inst, &plan, None, &used).unwrap();
        assert_eq!(std::fs::read_to_string(inst.join("content/sounds/ouch.ogg")).unwrap(), "orig:content/sounds/ouch.ogg");
    }
}
