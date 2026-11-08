//! Microsoft Store (MSIX/AppX) packages, from the AppModel package
//! repository in the current user's registry
//! (`HKCU\Software\Classes\Local Settings\…\AppModel\Repository\Packages`).
//! Reading the registry avoids PowerShell's `Get-AppxPackage`, which takes
//! seconds to start.
//!
//! Like Settings > Installed apps, this drops resource and framework packages
//! (language packs, VCLibs and other runtimes the Store updates with their
//! apps) and the inbox apps under the Windows folder, which Windows Update
//! services.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{Software, Source, version_key};

/// Registry values read from each package's key.
pub const PACKAGE_VALUES: &[&str] = &["DisplayName", "PackageRootFolder"];

/// The parts of a package full name,
/// `Name_Version_Architecture_ResourceId_PublisherId`.
#[derive(Debug, PartialEq, Eq)]
pub struct PackageId<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub arch: &'a str,
    pub resource_id: &'a str,
    pub publisher_id: &'a str,
}

/// Splits a package full name. Package names cannot contain `_`.
pub fn parse_full_name(full: &str) -> Option<PackageId<'_>> {
    let mut p = full.split('_');
    let id = PackageId {
        name: p.next().filter(|n| !n.is_empty())?,
        version: p.next()?,
        arch: p.next()?,
        resource_id: p.next()?,
        publisher_id: p.next()?,
    };
    p.next().is_none().then_some(id)
}

#[derive(Deserialize, Default)]
struct Manifest {
    #[serde(rename = "Properties", default)]
    properties: Properties,
}

#[derive(Deserialize, Default)]
struct Properties {
    #[serde(rename = "DisplayName")]
    display_name: Option<String>,
    #[serde(rename = "PublisherDisplayName")]
    publisher: Option<String>,
    #[serde(rename = "Framework", default)]
    framework: bool,
    #[serde(rename = "ResourcePackage", default)]
    resource: bool,
}

/// A real name, not a `ms-resource:` or `@{…}` reference to a resource table.
fn literal(name: Option<&str>) -> Option<&str> {
    name.map(str::trim)
        .filter(|n| !n.is_empty() && !n.starts_with("ms-resource:") && !n.starts_with("@{"))
}

/// One repository entry: its full name, its registry values, and the text of
/// its `AppxManifest.xml` if it could be read. `windows_dir` is the Windows
/// folder (`C:\Windows`); packages inside it are part of Windows.
pub fn parse_package(
    full_name: &str,
    values: &BTreeMap<String, String>,
    manifest_xml: Option<&str>,
    windows_dir: &str,
) -> Option<Software> {
    let id = parse_full_name(full_name)?;
    if !id.resource_id.is_empty() {
        return None;
    }
    let root = values
        .get("PackageRootFolder")
        .map(|r| r.trim().trim_end_matches('\\'))
        .filter(|r| !r.is_empty());
    let windows_dir = windows_dir.trim_end_matches('\\').to_lowercase();
    if root.is_some_and(|r| r.to_lowercase().starts_with(&format!("{windows_dir}\\"))) {
        return None;
    }
    let manifest = manifest_xml
        .and_then(|x| quick_xml::de::from_str::<Manifest>(x).ok())
        .unwrap_or_default();
    let props = manifest.properties;
    if props.framework || props.resource {
        return None;
    }
    let name = literal(values.get("DisplayName").map(String::as_str))
        .or(literal(props.display_name.as_deref()))
        .unwrap_or(id.name);
    let mut s = Software::new(Source::MsStore, id.name, name);
    s.version = Some(id.version.to_string());
    s.arch = Some(id.arch.to_string());
    s.publisher = literal(props.publisher.as_deref()).map(String::from);
    s.location = root.map(Into::into);
    Some(s)
}

/// Keeps the newest version of each package; an update leaves the old
/// version registered until it is cleaned up.
pub fn newest_only(mut packages: Vec<Software>) -> Vec<Software> {
    packages.sort_by(|a, b| {
        a.id.cmp(&b.id).then_with(|| {
            version_key(b.version.as_deref().unwrap_or(""))
                .cmp(&version_key(a.version.as_deref().unwrap_or("")))
        })
    });
    packages.dedup_by(|later, first| later.id == first.id);
    packages
}

#[cfg(windows)]
mod sys {
    use std::path::Path;

    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

    use super::*;
    use crate::read_capped;

    const REPOSITORY: &str = r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages";

    pub fn scan(warnings: &mut Vec<String>) -> Vec<Software> {
        let Ok(root) =
            RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(REPOSITORY, KEY_READ)
        else {
            warnings.push("store: could not open the package repository".into());
            return Vec::new();
        };
        let windows_dir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let mut out = Vec::new();
        for full_name in root.enum_keys().filter_map(Result::ok) {
            let Ok(key) = root.open_subkey_with_flags(&full_name, KEY_READ) else {
                continue;
            };
            let values: BTreeMap<String, String> = PACKAGE_VALUES
                .iter()
                .filter_map(|&n| Some((n.to_string(), key.get_value::<String, _>(n).ok()?)))
                .collect();
            let manifest = values
                .get("PackageRootFolder")
                .and_then(|r| read_capped(&Path::new(r).join("AppxManifest.xml")).ok())
                .map(|b| String::from_utf8_lossy(&b).into_owned());
            out.extend(parse_package(
                &full_name,
                &values,
                manifest.as_deref(),
                &windows_dir,
            ));
        }
        newest_only(out)
    }
}

#[cfg(windows)]
pub use sys::scan;

#[cfg(test)]
mod tests {
    use super::*;

    const TERMINAL: &str = "Microsoft.WindowsTerminal_1.21.2911.0_x64__8wekyb3d8bbwe";

    fn values(display: &str, root: &str) -> BTreeMap<String, String> {
        [("DisplayName", display), ("PackageRootFolder", root)]
            .into_iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    const APP_MANIFEST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10">
  <Identity Name="Microsoft.WindowsTerminal" Publisher="CN=Microsoft Corporation" Version="1.21.2911.0" ProcessorArchitecture="x64"/>
  <Properties>
    <DisplayName>Windows Terminal</DisplayName>
    <PublisherDisplayName>Microsoft Corporation</PublisherDisplayName>
    <Logo>Images\StoreLogo.png</Logo>
  </Properties>
  <Applications><Application Id="App"><uap:VisualElements DisplayName="ms-resource:AppName"/></Application></Applications>
</Package>"#;

    const FRAMEWORK_MANIFEST: &str = r#"<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10">
  <Identity Name="Microsoft.VCLibs.140.00" Version="14.0.33519.0"/>
  <Properties><DisplayName>Microsoft Visual C++ 2015 UWP Runtime Package</DisplayName><PublisherDisplayName>Microsoft Platform Extensions</PublisherDisplayName><Framework>true</Framework></Properties>
</Package>"#;

    #[test]
    fn splits_full_names() {
        assert_eq!(
            parse_full_name(TERMINAL),
            Some(PackageId {
                name: "Microsoft.WindowsTerminal",
                version: "1.21.2911.0",
                arch: "x64",
                resource_id: "",
                publisher_id: "8wekyb3d8bbwe",
            })
        );
        assert_eq!(parse_full_name("Foo_1.0_x64"), None);
        assert_eq!(parse_full_name("a_1_x64__p_extra"), None);
    }

    #[test]
    fn parses_store_apps_and_drops_what_settings_hides() {
        let root = r"C:\Program Files\WindowsApps\Microsoft.WindowsTerminal_1.21.2911.0_x64__8wekyb3d8bbwe";
        let s = parse_package(
            TERMINAL,
            &values("@{Microsoft.WindowsTerminal_1.21.2911.0_x64__8wekyb3d8bbwe?ms-resource://Microsoft.WindowsTerminal/Resources/AppName}", root),
            Some(APP_MANIFEST),
            r"C:\Windows",
        )
        .unwrap();
        assert_eq!(s.source, Source::MsStore);
        assert_eq!(s.id, "Microsoft.WindowsTerminal");
        assert_eq!(s.name, "Windows Terminal");
        assert_eq!(s.version.as_deref(), Some("1.21.2911.0"));
        assert_eq!(s.arch.as_deref(), Some("x64"));
        assert_eq!(s.publisher.as_deref(), Some("Microsoft Corporation"));
        assert_eq!(s.location.as_deref(), Some(std::path::Path::new(root)));

        // No manifest and a resource-table name: falls back to the package name.
        let s = parse_package(
            TERMINAL,
            &values("ms-resource:AppName", ""),
            None,
            r"C:\Windows",
        )
        .unwrap();
        assert_eq!(s.name, "Microsoft.WindowsTerminal");

        // Framework, resource pack, and inbox app are dropped.
        let vclibs = "Microsoft.VCLibs.140.00_14.0.33519.0_x64__8wekyb3d8bbwe";
        assert!(
            parse_package(
                vclibs,
                &values("", r"C:\Program Files\WindowsApps\x"),
                Some(FRAMEWORK_MANIFEST),
                r"C:\Windows"
            )
            .is_none()
        );
        let lang = "Microsoft.WindowsTerminal_1.21.2911.0_neutral_split.language-de_8wekyb3d8bbwe";
        assert!(parse_package(lang, &values("", ""), None, r"C:\Windows").is_none());
        let inbox =
            "Microsoft.Windows.ShellExperienceHost_10.0.22621.1_neutral_neutral_cw5n1h2txyewy";
        assert!(
            parse_package(
                inbox,
                &values(
                    "",
                    r"c:\windows\SystemApps\ShellExperienceHost_cw5n1h2txyewy"
                ),
                None,
                r"C:\Windows\"
            )
            .is_none()
        );
    }

    #[test]
    fn keeps_newest_version() {
        let pkg = |v: &str| {
            let mut s = Software::new(Source::MsStore, "App", "App");
            s.version = Some(v.into());
            s
        };
        let kept = newest_only(vec![pkg("1.9.0.0"), pkg("1.10.0.0"), pkg("1.2.0.0")]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].version.as_deref(), Some("1.10.0.0"));
    }
}
