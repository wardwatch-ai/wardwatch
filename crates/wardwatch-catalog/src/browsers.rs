//! Recognizes web browsers among installed software.

use serde::{Deserialize, Serialize};

use crate::{Software, Source};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Browser {
    Chrome,
    Chromium,
    Edge,
    Firefox,
    Safari,
    Brave,
    Opera,
    Vivaldi,
    Arc,
}

/// macOS bundle IDs and Flatpak app IDs (both reverse-DNS).
const BY_APP_ID: &[(&str, Browser)] = &[
    ("com.google.chrome", Browser::Chrome),
    ("org.chromium.chromium", Browser::Chromium),
    ("com.microsoft.edgemac", Browser::Edge),
    ("com.microsoft.edge", Browser::Edge),
    ("org.mozilla.firefox", Browser::Firefox),
    ("com.apple.safari", Browser::Safari),
    ("com.brave.browser", Browser::Brave),
    ("com.operasoftware.opera", Browser::Opera),
    ("com.opera.opera", Browser::Opera),
    ("com.vivaldi.vivaldi", Browser::Vivaldi),
    ("company.thebrowser.browser", Browser::Arc),
];

/// Linux package and snap names.
const BY_PACKAGE: &[(&str, Browser)] = &[
    ("google-chrome-stable", Browser::Chrome),
    ("chromium", Browser::Chromium),
    ("chromium-browser", Browser::Chromium),
    ("microsoft-edge-stable", Browser::Edge),
    ("firefox", Browser::Firefox),
    ("firefox-esr", Browser::Firefox),
    ("mozillafirefox", Browser::Firefox),
    ("brave-browser", Browser::Brave),
    ("brave", Browser::Brave),
    ("opera-stable", Browser::Opera),
    ("opera", Browser::Opera),
    ("vivaldi-stable", Browser::Vivaldi),
];

/// Windows `DisplayName`s. Exact names, except where the vendor appends a
/// version or locale (marked `true`), so "Microsoft Edge WebView2 Runtime"
/// is not mistaken for Edge.
const BY_WINDOWS_NAME: &[(&str, bool, Browser)] = &[
    ("Google Chrome", false, Browser::Chrome),
    ("Chromium", false, Browser::Chromium),
    ("Microsoft Edge", false, Browser::Edge),
    ("Mozilla Firefox", true, Browser::Firefox),
    ("Brave", false, Browser::Brave),
    ("Opera Stable", true, Browser::Opera),
    ("Vivaldi", false, Browser::Vivaldi),
];

fn lookup(table: &[(&str, Browser)], key: &str) -> Option<Browser> {
    table
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|&(_, b)| b)
}

pub(crate) fn identify(s: &Software) -> Option<Browser> {
    match s.source {
        Source::MacApp | Source::Flatpak => lookup(BY_APP_ID, &s.id),
        Source::Snap => lookup(BY_PACKAGE, &s.id),
        Source::Dpkg | Source::Rpm => {
            // Ubuntu's "firefox" and "chromium-browser" debs are stubs that
            // install the snap; the snap entry carries the real version.
            if s.version.as_deref().is_some_and(|v| v.contains("snap1")) {
                return None;
            }
            lookup(BY_PACKAGE, &s.id)
        }
        Source::WindowsRegistry => BY_WINDOWS_NAME
            .iter()
            .find(|(name, prefix, _)| {
                s.name == *name || (*prefix && s.name.starts_with(&format!("{name} ")))
            })
            .map(|&(_, _, b)| b),
        // Casks also install an app bundle, which carries the tag.
        Source::HomebrewCask | Source::HomebrewFormula => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sw(source: Source, id: &str, name: &str, version: &str) -> Software {
        let mut s = Software::new(source, id, name);
        s.version = Some(version.into());
        s
    }

    #[test]
    fn identifies_browsers_per_source() {
        let cases = [
            (
                sw(Source::MacApp, "com.apple.Safari", "Safari", "18.1"),
                Some(Browser::Safari),
            ),
            (
                sw(
                    Source::MacApp,
                    "com.google.Chrome",
                    "Google Chrome",
                    "130.0",
                ),
                Some(Browser::Chrome),
            ),
            (sw(Source::MacApp, "com.apple.Notes", "Notes", "4.11"), None),
            (
                sw(Source::Flatpak, "org.mozilla.firefox", "Firefox", "132.0"),
                Some(Browser::Firefox),
            ),
            (
                sw(Source::Snap, "firefox", "firefox", "132.0-1"),
                Some(Browser::Firefox),
            ),
            (
                sw(Source::Dpkg, "firefox", "firefox", "1:1snap1-0ubuntu5"),
                None,
            ),
            (
                sw(
                    Source::Dpkg,
                    "google-chrome-stable",
                    "google-chrome-stable",
                    "130.0.6723.116-1",
                ),
                Some(Browser::Chrome),
            ),
            (
                sw(Source::Rpm, "firefox", "firefox", "132.0-1.fc41"),
                Some(Browser::Firefox),
            ),
            (
                sw(
                    Source::WindowsRegistry,
                    "Google Chrome",
                    "Google Chrome",
                    "130.0",
                ),
                Some(Browser::Chrome),
            ),
            (
                sw(
                    Source::WindowsRegistry,
                    "Mozilla Firefox 132.0 (x64 en-US)",
                    "Mozilla Firefox (x64 en-US)",
                    "132.0",
                ),
                Some(Browser::Firefox),
            ),
            (
                sw(
                    Source::WindowsRegistry,
                    "x",
                    "Microsoft Edge WebView2 Runtime",
                    "130.0",
                ),
                None,
            ),
            (
                sw(Source::WindowsRegistry, "x", "Microsoft Edge", "130.0"),
                Some(Browser::Edge),
            ),
            (
                sw(Source::Flatpak, "com.opera.Opera", "Opera", "114.0"),
                Some(Browser::Opera),
            ),
            (
                sw(Source::Rpm, "MozillaFirefox", "MozillaFirefox", "132.0-1.1"),
                Some(Browser::Firefox),
            ),
            (
                sw(
                    Source::Dpkg,
                    "chromium-browser",
                    "chromium-browser",
                    "2:1snap1-0ubuntu2",
                ),
                None,
            ),
            (
                sw(Source::HomebrewCask, "firefox", "firefox", "132.0"),
                None,
            ),
        ];
        for (s, want) in cases {
            assert_eq!(identify(&s), want, "{:?} {}", s.source, s.name);
        }
    }
}
