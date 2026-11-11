//! Roblox deployment metadata: client version lookup, package manifest parsing
//! and the package -> directory map used to lay out an installation.
//!
//! Aeolus uses Roblox's public, official distribution endpoints only:
//!   * clientsettingscdn.roblox.com/v2/client-version/WindowsPlayer
//!   * setup.rbxcdn.com/<version>-rbxPkgManifest.txt and the packages it lists

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

pub const CLIENT_VERSION_URL: &str = "https://clientsettingscdn.roblox.com/v2/client-version/WindowsPlayer";
/// Official deployment hosts, tried in order.
pub const DEPLOY_HOSTS: &[&str] = &["https://setup.rbxcdn.com", "https://setup-ak.rbxcdn.com", "https://roblox-setup.cachefly.net"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClientVersion {
    /// Human readable version, e.g. "0.695.0.6950871".
    pub version: String,
    /// Deployment id, e.g. "version-0123456789abcdef".
    pub version_guid: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientVersionResponse {
    version: String,
    client_version_upload: String,
}

pub fn parse_client_version(body: &str) -> AppResult<ClientVersion> {
    let r: ClientVersionResponse = serde_json::from_str(body)
        .map_err(|e| AppError::Network(format!("unexpected response from Roblox version service: {e}")))?;
    if !is_valid_guid(&r.client_version_upload) {
        return Err(AppError::Network(format!("Roblox returned an unexpected version id '{}'", r.client_version_upload)));
    }
    Ok(ClientVersion { version: r.version, version_guid: r.client_version_upload })
}

pub fn is_valid_guid(s: &str) -> bool {
    s.strip_prefix("version-")
        .map(|h| !h.is_empty() && h.len() <= 32 && h.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub name: String,
    pub md5: String,
    pub packed_size: u64,
    pub size: u64,
}

/// Parse `rbxPkgManifest.txt` ("v0" followed by groups of name/md5/packed/unpacked lines).
pub fn parse_package_manifest(text: &str) -> AppResult<Vec<Package>> {
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let bad = |m: &str| AppError::Integrity(format!("package manifest is malformed: {m}"));
    match lines.first() {
        Some(&"v0") => {}
        Some(v) => return Err(bad(&format!("unsupported manifest version '{v}'"))),
        None => return Err(bad("empty manifest")),
    }
    let body = &lines[1..];
    if body.len() % 4 != 0 {
        return Err(bad("unexpected number of lines"));
    }
    let mut out = vec![];
    for c in body.chunks(4) {
        let name = c[0].to_string();
        if name.contains(['/', '\\']) || name.contains("..") {
            return Err(bad(&format!("invalid package name '{name}'")));
        }
        let md5 = c[1].to_ascii_lowercase();
        if md5.len() != 32 || !md5.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(bad(&format!("invalid checksum for {name}")));
        }
        let packed_size = c[2].parse().map_err(|_| bad(&format!("invalid size for {name}")))?;
        let size = c[3].parse().map_err(|_| bad(&format!("invalid size for {name}")))?;
        out.push(Package { name, md5, packed_size, size });
    }
    if out.is_empty() {
        return Err(bad("no packages listed"));
    }
    Ok(out)
}

/// Where each player package is extracted, relative to the version folder.
pub fn package_directory(name: &str) -> Option<&'static str> {
    Some(match name {
        "RobloxApp.zip" => "",
        "Libraries.zip" => "",
        "redist.zip" => "",
        "WebView2.zip" => "",
        "shaders.zip" => "shaders/",
        "ssl.zip" => "ssl/",
        "WebView2RuntimeInstaller.zip" => "WebView2RuntimeInstaller/",
        "content-avatar.zip" => "content/avatar/",
        "content-configs.zip" => "content/configs/",
        "content-fonts.zip" => "content/fonts/",
        "content-sky.zip" => "content/sky/",
        "content-sounds.zip" => "content/sounds/",
        "content-textures2.zip" => "content/textures/",
        "content-models.zip" => "content/models/",
        "content-platform-fonts.zip" => "PlatformContent/pc/fonts/",
        "content-platform-dictionaries.zip" => "PlatformContent/pc/shared_compression_dictionaries/",
        "content-terrain.zip" => "PlatformContent/pc/terrain/",
        "content-textures3.zip" => "PlatformContent/pc/textures/",
        "extracontent-luapackages.zip" => "ExtraContent/LuaPackages/",
        "extracontent-translations.zip" => "ExtraContent/translations/",
        "extracontent-models.zip" => "ExtraContent/models/",
        "extracontent-textures.zip" => "ExtraContent/textures/",
        "extracontent-places.zip" => "ExtraContent/places/",
        _ => return None,
    })
}

pub const APP_SETTINGS_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n<Settings>\r\n\t<ContentFolder>content</ContentFolder>\r\n\t<BaseUrl>http://www.roblox.com</BaseUrl>\r\n</Settings>\r\n";

pub const PLAYER_EXE: &str = "RobloxPlayerBeta.exe";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_client_version() {
        let v = parse_client_version(r#"{"version":"0.695.0.6950871","clientVersionUpload":"version-3fe8a1c2d4b54e1a","bootstrapperVersion":"1, 6, 0, 6950871"}"#).unwrap();
        assert_eq!(v.version_guid, "version-3fe8a1c2d4b54e1a");
        assert!(parse_client_version(r#"{"version":"1","clientVersionUpload":"../../evil"}"#).is_err());
        assert!(parse_client_version("<html>").is_err());
    }

    #[test]
    fn parses_manifest() {
        let m = "v0\nRobloxApp.zip\n0123456789abcdef0123456789ABCDEF\n100\n200\nshaders.zip\nffffffffffffffffffffffffffffffff\n5\n9\n";
        let p = parse_package_manifest(m).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].md5, "0123456789abcdef0123456789abcdef");
        assert_eq!(p[1].packed_size, 5);
        assert_eq!(package_directory(&p[1].name), Some("shaders/"));
    }

    #[test]
    fn rejects_bad_manifests() {
        assert!(parse_package_manifest("").is_err());
        assert!(parse_package_manifest("v1\n").is_err());
        assert!(parse_package_manifest("v0\na.zip\nnothex\n1\n1").is_err());
        assert!(parse_package_manifest("v0\n../a.zip\n0123456789abcdef0123456789abcdef\n1\n1").is_err());
        assert!(parse_package_manifest("v0\na.zip\n0123456789abcdef0123456789abcdef\nx\n1").is_err());
        assert!(parse_package_manifest("v0\na.zip\n0123456789abcdef0123456789abcdef\n1").is_err());
    }
}
