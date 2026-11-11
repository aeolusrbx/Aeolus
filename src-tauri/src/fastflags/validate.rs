//! Fast Flag validation and classification.
//!
//! Important distinction: a flag can be *valid JSON with the right type* and still
//! be ignored by Roblox. Since 30 Sept 2025 the Roblox client only honours local
//! overrides for flags on its published allowlist; everything else is silently
//! ignored. Aeolus ships the baseline allowlist published by Roblox and marks other
//! flags as "not allowlisted" rather than claiming they work.

use serde::Serialize;
use serde_json::Value;

/// Baseline allowlist announced by Roblox on 29 Sept 2025 (devforum post
/// "Allowlist for local client configuration via Fast Flags"). Roblox may change
/// it without notice; the effective list is delivered to the client per release.
pub const ALLOWLIST: &[(&str, Category)] = &[
    ("DFIntCSGLevelOfDetailSwitchingDistance", Category::Graphics),
    ("DFIntCSGLevelOfDetailSwitchingDistanceL12", Category::Graphics),
    ("DFIntCSGLevelOfDetailSwitchingDistanceL23", Category::Graphics),
    ("DFIntCSGLevelOfDetailSwitchingDistanceL34", Category::Graphics),
    ("FFlagHandleAltEnterFullscreenManually", Category::Interface),
    ("DFFlagTextureQualityOverrideEnabled", Category::Graphics),
    ("DFIntTextureQualityOverride", Category::Graphics),
    ("FIntDebugForceMSAASamples", Category::Rendering),
    ("DFFlagDisableDPIScale", Category::Interface),
    ("FFlagDebugGraphicsPreferD3D11", Category::Rendering),
    ("FFlagDebugSkyGray", Category::Rendering),
    ("DFFlagDebugPauseVoxelizer", Category::Performance),
    ("DFIntDebugFRMQualityLevelOverride", Category::Graphics),
    ("FIntFRMMaxGrassDistance", Category::Graphics),
    ("FIntFRMMinGrassDistance", Category::Graphics),
    ("FFlagDebugGraphicsPreferVulkan", Category::Rendering),
    ("FFlagDebugGraphicsPreferOpenGL", Category::Rendering),
    ("FIntGrassMovementReducedMotionFactor", Category::Interface),
];

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Rendering,
    Graphics,
    Performance,
    Networking,
    Interface,
    Experimental,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FlagType {
    Boolean,
    Integer,
    String,
    Log,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FlagStatus {
    /// On the known allowlist and the value has the right type.
    Allowlisted,
    /// Valid name and value, but Roblox will most likely ignore it.
    NotAllowlisted,
    /// Name or value is wrong.
    Invalid,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagCheck {
    pub name: String,
    pub flag_type: FlagType,
    pub category: Category,
    pub status: FlagStatus,
    pub messages: Vec<String>,
}

const PREFIXES: &[(&str, FlagType)] = &[
    ("DFFlag", FlagType::Boolean),
    ("SFFlag", FlagType::Boolean),
    ("FFlag", FlagType::Boolean),
    ("DFInt", FlagType::Integer),
    ("SFInt", FlagType::Integer),
    ("FInt", FlagType::Integer),
    ("DFString", FlagType::String),
    ("SFString", FlagType::String),
    ("FString", FlagType::String),
    ("DFLog", FlagType::Log),
    ("FLog", FlagType::Log),
];

pub fn flag_type(name: &str) -> FlagType {
    PREFIXES
        .iter()
        .find(|(p, _)| name.starts_with(p) && name.len() > p.len())
        .map(|(_, t)| *t)
        .unwrap_or(FlagType::Unknown)
}

pub fn is_allowlisted(name: &str) -> bool {
    ALLOWLIST.iter().any(|(n, _)| *n == name)
}

pub fn category(name: &str) -> Category {
    if let Some((_, c)) = ALLOWLIST.iter().find(|(n, _)| *n == name) {
        return *c;
    }
    let l = name.to_ascii_lowercase();
    let has = |ks: &[&str]| ks.iter().any(|k| l.contains(k));
    if has(&["d3d11", "vulkan", "opengl", "msaa", "shadow", "render", "lighting"]) {
        Category::Rendering
    } else if has(&["texture", "grass", "frm", "quality", "lod"]) {
        Category::Graphics
    } else if has(&["taskscheduler", "fps", "perf"]) {
        Category::Performance
    } else if has(&["network", "http", "ping", "bandwidth"]) {
        Category::Networking
    } else if has(&["dpi", "fullscreen", "gui", "menu", "topbar"]) {
        Category::Interface
    } else if l.contains("debug") {
        Category::Experimental
    } else {
        Category::Other
    }
}

/// Normalize a value typed into the editor (e.g. "true", "4") into a JSON value
/// of the type implied by the flag name.
pub fn coerce(name: &str, raw: &Value) -> Result<Value, String> {
    let t = flag_type(name);
    match (t, raw) {
        (FlagType::Boolean, Value::Bool(b)) => Ok(Value::Bool(*b)),
        (FlagType::Boolean, Value::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(format!("{name} expects true or false")),
        },
        (FlagType::Integer | FlagType::Log, Value::Number(n)) if n.is_i64() => Ok(raw.clone()),
        (FlagType::Integer | FlagType::Log, Value::Number(_)) => Err(format!("{name} expects a whole number")),
        (FlagType::Integer | FlagType::Log, Value::String(s)) => s
            .trim()
            .parse::<i64>()
            .map(Value::from)
            .map_err(|_| format!("{name} expects a whole number")),
        (FlagType::String, Value::String(_)) => Ok(raw.clone()),
        (FlagType::String, Value::Number(n)) => Ok(Value::String(n.to_string())),
        (FlagType::String, Value::Bool(b)) => Ok(Value::String(b.to_string())),
        (FlagType::Unknown, v @ (Value::Bool(_) | Value::Number(_) | Value::String(_))) => Ok(v.clone()),
        (_, Value::Null | Value::Array(_) | Value::Object(_)) => Err(format!("{name} must be a boolean, number or string")),
        (t, _) => Err(format!("{name} has the wrong type (expected {t:?})").to_lowercase()),
    }
}

pub fn valid_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Flag name is empty".into());
    }
    if name.len() > 200 {
        return Err("Flag name is too long".into());
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Flag names may only contain letters, digits and underscores".into());
    }
    Ok(())
}

pub fn check(name: &str, value: &Value, siblings: &[&str]) -> FlagCheck {
    let mut messages = vec![];
    let t = flag_type(name);
    let mut invalid = false;
    if let Err(e) = valid_name(name) {
        messages.push(e);
        invalid = true;
    }
    if t == FlagType::Unknown {
        messages.push("Unrecognized prefix; Roblox flags start with FFlag, DFFlag, FInt, DFInt, FString, DFString, FLog or DFLog".into());
        invalid = true;
    }
    if let Err(e) = coerce(name, value) {
        messages.push(e);
        invalid = true;
    }
    let dup = siblings.iter().filter(|s| s.eq_ignore_ascii_case(name)).count();
    if dup > 1 {
        messages.push("Duplicate flag name (names differ only by letter case)".into());
        invalid = true;
    }
    let status = if invalid {
        FlagStatus::Invalid
    } else if is_allowlisted(name) {
        FlagStatus::Allowlisted
    } else {
        messages.push("Not on Roblox's known allowlist; the client will most likely ignore it".into());
        FlagStatus::NotAllowlisted
    };
    FlagCheck { name: name.into(), flag_type: t, category: category(name), status, messages }
}

/// Convert a stored value into the string form written to ClientAppSettings.json
/// (the format used by Roblox's own configuration and established bootstrappers).
pub fn to_client_value(v: &Value) -> Value {
    match v {
        Value::Bool(true) => Value::String("True".into()),
        Value::Bool(false) => Value::String("False".into()),
        Value::Number(n) => Value::String(n.to_string()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn types_and_coercion() {
        assert_eq!(flag_type("FFlagX"), FlagType::Boolean);
        assert_eq!(flag_type("DFIntX"), FlagType::Integer);
        assert_eq!(flag_type("FStringX"), FlagType::String);
        assert_eq!(flag_type("Garbage"), FlagType::Unknown);
        assert_eq!(flag_type("FFlag"), FlagType::Unknown);
        assert_eq!(coerce("FFlagX", &json!("True")).unwrap(), json!(true));
        assert_eq!(coerce("FIntX", &json!("42")).unwrap(), json!(42));
        assert!(coerce("FIntX", &json!(4.5)).is_err());
        assert!(coerce("FIntX", &json!("abc")).is_err());
        assert!(coerce("FFlagX", &json!(1)).is_err());
        assert!(coerce("FFlagX", &json!([1])).is_err());
    }

    #[test]
    fn statuses() {
        let c = check("DFIntTextureQualityOverride", &json!(3), &["DFIntTextureQualityOverride"]);
        assert_eq!(c.status, FlagStatus::Allowlisted);
        assert_eq!(c.category, Category::Graphics);
        let c = check("DFIntTaskSchedulerTargetFps", &json!(240), &[]);
        assert_eq!(c.status, FlagStatus::NotAllowlisted);
        assert_eq!(c.category, Category::Performance);
        let c = check("FFlagA", &json!("yes"), &[]);
        assert_eq!(c.status, FlagStatus::Invalid);
        let c = check("FFlagA", &json!(true), &["FFlagA", "fflaga"]);
        assert_eq!(c.status, FlagStatus::Invalid);
        let c = check("FFlag A", &json!(true), &[]);
        assert_eq!(c.status, FlagStatus::Invalid);
    }

    #[test]
    fn client_values() {
        assert_eq!(to_client_value(&json!(true)), json!("True"));
        assert_eq!(to_client_value(&json!(8)), json!("8"));
        assert_eq!(to_client_value(&json!("x")), json!("x"));
    }
}
