//! Browser extensions, read from browser profiles on disk: Chromium-based
//! browsers (`<profile>/Extensions/<id>/<version>/manifest.json`), Firefox
//! (`<profile>/extensions.json`), and Safari (app extensions inside apps).
//!
//! Extensions bundled with the browser are left out, since browser updates
//! cover them. Profile paths are never serialized; the same extension in
//! several profiles is one entry.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Browser, Software, Source, read_capped, version_key};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Extension {
    /// The browser it is installed in.
    pub browser: Browser,
    /// Chromium extension ID (32 letters, from the Chrome Web Store or Edge
    /// Add-ons), Firefox add-on ID, or the Safari extension's bundle ID.
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    /// Whether it is turned on in at least one profile, when the browser's
    /// settings say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

pub fn scan(software: &[Software], warnings: &mut Vec<String>) -> Vec<Extension> {
    let mut out = Vec::new();
    for (browser, dir) in chromium_dirs() {
        for profile in chromium_profiles(&dir) {
            out.extend(scan_chromium_profile(browser, &profile));
        }
    }
    for profile in firefox_profiles() {
        match scan_firefox_profile(&profile) {
            Some(found) => out.extend(found),
            None => warnings.push("firefox: could not read a profile's extensions".into()),
        }
    }
    out.extend(safari_extensions(software));
    merge(out)
}

/// Folders that change when extensions are added, updated or removed.
pub fn watch_paths() -> Vec<PathBuf> {
    let chromium = chromium_dirs()
        .into_iter()
        .flat_map(|(_, dir)| chromium_profiles(&dir))
        .map(|p| p.join("Extensions"));
    // Firefox keeps installed add-ons as .xpi files in `<profile>/extensions`.
    // (Turning an extension on or off is picked up by the next scheduled scan.)
    let firefox = firefox_profiles().into_iter().map(|p| p.join("extensions"));
    chromium.chain(firefox).collect()
}

/// One entry per browser, ID and version; enabled if any profile has it on.
fn merge(mut list: Vec<Extension>) -> Vec<Extension> {
    list.sort_by(|a, b| {
        (a.browser as u8, &a.id, &a.version).cmp(&(b.browser as u8, &b.id, &b.version))
    });
    list.dedup_by(|later, first| {
        let same = later.browser == first.browser
            && later.id == first.id
            && later.version == first.version;
        if same {
            first.enabled = match (first.enabled, later.enabled) {
                (Some(a), Some(b)) => Some(a || b),
                (a, b) => a.or(b),
            };
        }
        same
    });
    list
}

// ---- Chromium-based browsers ----

/// User data folders of Chromium-based browsers on this OS.
fn chromium_dirs() -> Vec<(Browser, PathBuf)> {
    let mut dirs: Vec<(Browser, PathBuf)> = Vec::new();
    #[cfg(target_os = "macos")]
    if let Some(home) = crate::home_dir() {
        let base = home.join("Library/Application Support");
        for (b, rel) in [
            (Browser::Chrome, "Google/Chrome"),
            (Browser::Chromium, "Chromium"),
            (Browser::Edge, "Microsoft Edge"),
            (Browser::Brave, "BraveSoftware/Brave-Browser"),
            (Browser::Vivaldi, "Vivaldi"),
            (Browser::Opera, "com.operasoftware.Opera"),
            (Browser::Arc, "Arc/User Data"),
        ] {
            dirs.push((b, base.join(rel)));
        }
    }
    #[cfg(target_os = "linux")]
    if let Some(home) = crate::home_dir() {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|c| !c.is_empty())
            .map_or_else(|| home.join(".config"), PathBuf::from);
        for (b, rel) in [
            (Browser::Chrome, "google-chrome"),
            (Browser::Chromium, "chromium"),
            (Browser::Edge, "microsoft-edge"),
            (Browser::Brave, "BraveSoftware/Brave-Browser"),
            (Browser::Vivaldi, "vivaldi"),
            (Browser::Opera, "opera"),
        ] {
            dirs.push((b, config.join(rel)));
        }
        // Snap and Flatpak keep their own copies.
        dirs.push((
            Browser::Chromium,
            home.join("snap/chromium/common/chromium"),
        ));
        for (b, rel) in [
            (Browser::Chrome, "com.google.Chrome/config/google-chrome"),
            (Browser::Chromium, "org.chromium.Chromium/config/chromium"),
            (Browser::Edge, "com.microsoft.Edge/config/microsoft-edge"),
            (
                Browser::Brave,
                "com.brave.Browser/config/BraveSoftware/Brave-Browser",
            ),
        ] {
            dirs.push((b, home.join(".var/app").join(rel)));
        }
    }
    #[cfg(windows)]
    {
        let env = |k: &str| {
            std::env::var_os(k)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        if let Some(local) = env("LOCALAPPDATA") {
            for (b, rel) in [
                (Browser::Chrome, r"Google\Chrome\User Data"),
                (Browser::Chromium, r"Chromium\User Data"),
                (Browser::Edge, r"Microsoft\Edge\User Data"),
                (Browser::Brave, r"BraveSoftware\Brave-Browser\User Data"),
                (Browser::Vivaldi, r"Vivaldi\User Data"),
            ] {
                dirs.push((b, local.join(rel)));
            }
        }
        if let Some(roaming) = env("APPDATA") {
            dirs.push((Browser::Opera, roaming.join(r"Opera Software\Opera Stable")));
        }
    }
    dirs.retain(|(_, d)| d.is_dir());
    dirs
}

/// Profiles in a user data folder: its subfolders (`Default`, `Profile 1`)
/// and, for Opera, the folder itself, wherever an `Extensions` folder exists.
pub fn chromium_profiles(user_data: &Path) -> Vec<PathBuf> {
    let mut profiles = vec![user_data.to_path_buf()];
    if let Ok(entries) = fs::read_dir(user_data) {
        profiles.extend(entries.filter_map(|e| Some(e.ok()?.path())));
    }
    profiles.retain(|p| p.join("Extensions").is_dir());
    profiles.sort();
    profiles
}

/// A Chrome extension ID: 32 letters from `a` to `p`.
fn is_extension_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| (b'a'..=b'p').contains(&b))
}

/// Resolves a `__MSG_name__` placeholder from `_locales/<locale>/messages.json`.
fn localize(ext_dir: &Path, text: &str, default_locale: Option<&str>) -> Option<String> {
    let Some(key) = text
        .strip_prefix("__MSG_")
        .and_then(|t| t.strip_suffix("__"))
    else {
        return Some(text.to_string());
    };
    // The locale comes from the manifest; don't let it climb out of `_locales`.
    let locale = default_locale.filter(|l| {
        !l.is_empty()
            && l.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    })?;
    let file = ext_dir.join("_locales").join(locale).join("messages.json");
    let messages: serde_json::Map<String, Value> =
        serde_json::from_slice(&read_capped(&file).ok()?).ok()?;
    messages
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .and_then(|(_, v)| v.get("message")?.as_str())
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
}

/// The part of a profile's `Preferences` or `Secure Preferences` that says
/// which extensions are on. Everything else in those files is skipped.
#[derive(Deserialize, Default)]
pub struct ChromiumPrefs {
    #[serde(default)]
    extensions: PrefsExtensions,
}

#[derive(Deserialize, Default)]
struct PrefsExtensions {
    #[serde(default)]
    settings: HashMap<String, ExtensionSetting>,
}

#[derive(Deserialize)]
struct ExtensionSetting {
    disable_reasons: Option<Value>,
    state: Option<Value>,
}

/// Whether an extension is on: off when it has disable reasons.
pub fn chromium_enabled(prefs: &ChromiumPrefs, id: &str) -> Option<bool> {
    let settings = prefs.extensions.settings.get(id)?;
    let disabled_by_reasons = match &settings.disable_reasons {
        Some(Value::Number(n)) => n.as_u64() != Some(0),
        Some(Value::Array(a)) => !a.is_empty(),
        _ => false,
    };
    // Older versions use `state`: 1 on, 0 off.
    let disabled_by_state = settings.state.as_ref().and_then(Value::as_u64) == Some(0);
    Some(!(disabled_by_reasons || disabled_by_state))
}

/// Extensions in one Chromium profile.
pub fn scan_chromium_profile(browser: Browser, profile: &Path) -> Vec<Extension> {
    // A file over the read cap fails to parse, leaving `enabled` unknown.
    let prefs: Vec<ChromiumPrefs> = ["Secure Preferences", "Preferences"]
        .iter()
        .filter_map(|f| serde_json::from_slice(&read_capped(&profile.join(f)).ok()?).ok())
        .collect();
    let Ok(entries) = fs::read_dir(profile.join("Extensions")) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let Ok(id) = entry.file_name().into_string() else {
            continue;
        };
        if !is_extension_id(&id) {
            continue;
        }
        // Old version folders can linger until the browser restarts.
        let Some(dir) = fs::read_dir(entry.path()).ok().and_then(|versions| {
            versions
                .filter_map(|v| Some(v.ok()?.path()))
                .filter(|p| p.join("manifest.json").is_file())
                .max_by_key(|p| version_key(&p.file_name().unwrap_or_default().to_string_lossy()))
        }) else {
            continue;
        };
        let Some(manifest) = read_capped(&dir.join("manifest.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        else {
            continue;
        };
        if manifest.get("theme").is_some() {
            continue;
        }
        let text = |k: &str| manifest.get(k).and_then(Value::as_str).map(str::trim);
        let name = text("name")
            .and_then(|n| localize(&dir, n, text("default_locale")))
            .unwrap_or_else(|| id.clone());
        out.push(Extension {
            browser,
            version: text("version").map(String::from),
            enabled: prefs.iter().find_map(|p| chromium_enabled(p, &id)),
            id,
            name,
        });
    }
    out
}

// ---- Firefox ----

/// Firefox profile folders that have an `extensions.json`.
fn firefox_profiles() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "macos")]
    if let Some(home) = crate::home_dir() {
        roots.push(home.join("Library/Application Support/Firefox/Profiles"));
    }
    #[cfg(target_os = "linux")]
    if let Some(home) = crate::home_dir() {
        for rel in [
            ".mozilla/firefox",
            ".config/mozilla/firefox",
            "snap/firefox/common/.mozilla/firefox",
            ".var/app/org.mozilla.firefox/.mozilla/firefox",
        ] {
            roots.push(home.join(rel));
        }
    }
    #[cfg(windows)]
    if let Some(roaming) = std::env::var_os("APPDATA").filter(|v| !v.is_empty()) {
        roots.push(Path::new(&roaming).join(r"Mozilla\Firefox\Profiles"));
    }
    let mut profiles: Vec<PathBuf> = roots
        .iter()
        .filter_map(|r| fs::read_dir(r).ok())
        .flatten()
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.join("extensions.json").is_file())
        .collect();
    profiles.sort();
    profiles
}

#[derive(Deserialize)]
struct AddonsDb {
    #[serde(default)]
    addons: Vec<Addon>,
}

#[derive(Deserialize)]
struct Addon {
    id: String,
    version: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    location: Option<String>,
    active: Option<bool>,
    #[serde(rename = "defaultLocale")]
    default_locale: Option<AddonLocale>,
}

#[derive(Deserialize)]
struct AddonLocale {
    name: Option<String>,
}

/// Install locations of add-ons that ship with Firefox, or are loaded
/// temporarily by a developer.
const FIREFOX_BUILTIN: &[&str] = &[
    "app-builtin",
    "app-builtin-addons",
    "app-system-defaults",
    "app-system-addons",
    "app-temporary",
];

/// Parses a profile's `extensions.json`.
pub fn parse_firefox_extensions(bytes: &[u8]) -> Option<Vec<Extension>> {
    let db: AddonsDb = serde_json::from_slice(bytes).ok()?;
    Some(
        db.addons
            .into_iter()
            .filter(|a| a.kind.as_deref() == Some("extension"))
            .filter(|a| {
                !a.location
                    .as_deref()
                    .is_some_and(|l| FIREFOX_BUILTIN.contains(&l))
            })
            .map(|a| Extension {
                browser: Browser::Firefox,
                name: a
                    .default_locale
                    .and_then(|l| l.name)
                    .map(|n| n.trim().to_string())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| a.id.clone()),
                id: a.id,
                version: a.version,
                enabled: a.active,
            })
            .collect(),
    )
}

fn scan_firefox_profile(profile: &Path) -> Option<Vec<Extension>> {
    parse_firefox_extensions(&read_capped(&profile.join("extensions.json")).ok()?)
}

// ---- Safari ----

/// Extension points of Safari extensions (web extensions and the older kind).
const SAFARI_EXTENSION_POINTS: &[&str] = &[
    "com.apple.Safari.web-extension",
    "com.apple.Safari.extension",
];

/// Builds an entry from a `.appex` bundle's `Info.plist`, if it is a Safari extension.
pub fn parse_safari_appex(bytes: &[u8]) -> Option<Extension> {
    let info: plist::Dictionary = plist::from_bytes(bytes).ok()?;
    let point = info
        .get("NSExtension")?
        .as_dictionary()?
        .get("NSExtensionPointIdentifier")?
        .as_string()?;
    if !SAFARI_EXTENSION_POINTS.contains(&point) {
        return None;
    }
    let text = |k: &str| {
        info.get(k)
            .and_then(plist::Value::as_string)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let id = text("CFBundleIdentifier")?.to_string();
    Some(Extension {
        browser: Browser::Safari,
        name: text("CFBundleDisplayName")
            .or(text("CFBundleName"))
            .unwrap_or(&id)
            .to_string(),
        version: text("CFBundleShortVersionString")
            .or(text("CFBundleVersion"))
            .map(String::from),
        enabled: None,
        id,
    })
}

/// Safari extensions ship inside Mac apps, in `Contents/PlugIns/*.appex`.
fn safari_extensions(software: &[Software]) -> Vec<Extension> {
    software
        .iter()
        .filter(|s| s.source == Source::MacApp)
        .filter_map(|s| fs::read_dir(s.location.as_ref()?.join("Contents/PlugIns")).ok())
        .flatten()
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "appex"))
        .filter_map(|p| parse_safari_appex(&read_capped(&p.join("Contents/Info.plist")).ok()?))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const UBLOCK: &str = "cjpalhdlnbpafiamejdnhcphjbkeiagm";
    const DARK: &str = "eimadpbcbfnmbkopoojfekhnkhdbieeh";

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn scans_chromium_profiles() {
        let tmp = tempfile::tempdir().unwrap();
        let user_data = tmp.path();
        let default = user_data.join("Default");
        let ext = default.join("Extensions");
        // Two versions left on disk; the newer one counts.
        write(
            &ext.join(UBLOCK).join("1.9.0_0/manifest.json"),
            r#"{"name":"uBlock Origin","version":"1.9.0"}"#,
        );
        write(
            &ext.join(UBLOCK).join("1.10.0_0/manifest.json"),
            r#"{"name":"__MSG_extName__","version":"1.10.0","default_locale":"en"}"#,
        );
        write(
            &ext.join(UBLOCK).join("1.10.0_0/_locales/en/messages.json"),
            r#"{"extName":{"message":"uBlock Origin","description":"x"}}"#,
        );
        write(
            &ext.join(DARK).join("4.9.96_0/manifest.json"),
            r#"{"name":"Dark Reader","version":"4.9.96"}"#,
        );
        write(
            &ext.join("theme_id_aaaaaaaaaaaaaaaaaaaaaaaaaa/1/manifest.json"),
            r#"{"name":"x"}"#,
        );
        write(&ext.join("Temp/x/manifest.json"), r#"{"name":"x"}"#);
        write(
            &default.join("Secure Preferences"),
            &format!(
                r#"{{"extensions":{{"settings":{{"{UBLOCK}":{{"disable_reasons":[]}},"{DARK}":{{"disable_reasons":1}}}}}}}}"#
            ),
        );
        fs::create_dir_all(user_data.join("System Profile")).unwrap();

        let profiles = chromium_profiles(user_data);
        assert_eq!(profiles, std::slice::from_ref(&default));
        let mut found = scan_chromium_profile(Browser::Chrome, &default);
        found.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(
            found,
            [
                Extension {
                    browser: Browser::Chrome,
                    id: DARK.into(),
                    name: "Dark Reader".into(),
                    version: Some("4.9.96".into()),
                    enabled: Some(false),
                },
                Extension {
                    browser: Browser::Chrome,
                    id: UBLOCK.into(),
                    name: "uBlock Origin".into(),
                    version: Some("1.10.0".into()),
                    enabled: Some(true),
                },
            ]
        );
    }

    #[test]
    fn locale_cannot_leave_the_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let ext = tmp.path().join("ext");
        write(
            &tmp.path().join("messages.json"),
            r#"{"n":{"message":"outside"}}"#,
        );
        write(
            &ext.join("_locales/en/messages.json"),
            r#"{"n":{"message":"inside"}}"#,
        );
        assert_eq!(
            localize(&ext, "__MSG_n__", Some("en")).as_deref(),
            Some("inside")
        );
        assert_eq!(localize(&ext, "__MSG_n__", Some("../..")), None);
        assert_eq!(localize(&ext, "plain", None).as_deref(), Some("plain"));
    }

    #[test]
    fn reads_enabled_state_from_preferences() {
        let prefs: ChromiumPrefs = serde_json::from_str(&format!(
            r#"{{"extensions":{{"settings":{{"{UBLOCK}":{{"state":0}},"{DARK}":{{"state":1,"disable_reasons":0}}}}}}}}"#
        ))
        .unwrap();
        assert_eq!(chromium_enabled(&prefs, UBLOCK), Some(false));
        assert_eq!(chromium_enabled(&prefs, DARK), Some(true));
        assert_eq!(chromium_enabled(&prefs, "missing"), None);
    }

    #[test]
    fn parses_firefox_extensions() {
        let json = br#"{"schemaVersion":36,"addons":[
          {"id":"uBlock0@raymondhill.net","version":"1.61.0","type":"extension","location":"app-profile","active":true,"defaultLocale":{"name":"uBlock Origin"}},
          {"id":"addon@darkreader.org","version":"4.9.96","type":"extension","location":"app-profile","active":false,"defaultLocale":{}},
          {"id":"formautofill@mozilla.org","version":"1.0.1","type":"extension","location":"app-builtin","active":true},
          {"id":"default-theme@mozilla.org","version":"1.4.1","type":"theme","location":"app-builtin","active":true},
          {"id":"de@dictionaries.addons.mozilla.org","version":"2.0","type":"dictionary","location":"app-profile","active":true}
        ]}"#;
        let found = parse_firefox_extensions(json).unwrap();
        let summary: Vec<(&str, &str, Option<bool>)> = found
            .iter()
            .map(|e| (e.id.as_str(), e.name.as_str(), e.enabled))
            .collect();
        assert_eq!(
            summary,
            [
                ("uBlock0@raymondhill.net", "uBlock Origin", Some(true)),
                ("addon@darkreader.org", "addon@darkreader.org", Some(false)),
            ]
        );
        assert!(parse_firefox_extensions(b"{").is_none());
    }

    #[test]
    fn parses_safari_appex() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
  <key>CFBundleIdentifier</key><string>com.agilebits.onepassword7.safari</string>
  <key>CFBundleDisplayName</key><string>1Password</string>
  <key>CFBundleShortVersionString</key><string>8.10.50</string>
  <key>NSExtension</key><dict><key>NSExtensionPointIdentifier</key><string>com.apple.Safari.web-extension</string></dict>
</dict></plist>"#;
        let e = parse_safari_appex(xml).unwrap();
        assert_eq!(
            (e.browser, e.id.as_str(), e.name.as_str()),
            (
                Browser::Safari,
                "com.agilebits.onepassword7.safari",
                "1Password"
            )
        );
        assert_eq!(e.version.as_deref(), Some("8.10.50"));

        let share = br#"<plist version="1.0"><dict><key>CFBundleIdentifier</key><string>x</string><key>NSExtension</key><dict><key>NSExtensionPointIdentifier</key><string>com.apple.share-services</string></dict></dict></plist>"#;
        assert!(parse_safari_appex(share).is_none());
    }

    #[test]
    fn merges_profiles() {
        let e = |enabled: Option<bool>| Extension {
            browser: Browser::Chrome,
            id: UBLOCK.into(),
            name: "uBlock Origin".into(),
            version: Some("1.10.0".into()),
            enabled,
        };
        assert_eq!(
            merge(vec![e(Some(false)), e(None), e(Some(true))]),
            [e(Some(true))]
        );
        assert_eq!(merge(vec![e(None), e(Some(false))]), [e(Some(false))]);
    }
}
