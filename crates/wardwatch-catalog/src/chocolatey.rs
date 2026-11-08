//! Chocolatey packages, read from each package's `.nuspec` under
//! `<ChocolateyInstall>\lib`. Reading files avoids starting `choco`, which is
//! a .NET program and slow to start.
//!
//! Packages that wrap an installer (`*.install`, and most plain names) also
//! show up as a [`Source::WindowsRegistry`] entry for the program they
//! installed. Both are kept: the package version is what `choco upgrade`
//! acts on, the registry version is what is actually installed.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{Software, Source, read_capped};

/// Chocolatey's install folder: `%ChocolateyInstall%`, else
/// `%ProgramData%\chocolatey`.
pub fn root() -> PathBuf {
    if let Some(dir) = std::env::var_os("ChocolateyInstall").filter(|d| !d.is_empty()) {
        return dir.into();
    }
    let data = std::env::var_os("ProgramData").unwrap_or_else(|| r"C:\ProgramData".into());
    Path::new(&data).join("chocolatey")
}

#[derive(Deserialize)]
struct Nuspec {
    metadata: Metadata,
}

#[derive(Deserialize)]
struct Metadata {
    id: String,
    version: String,
    title: Option<String>,
    authors: Option<String>,
}

/// Builds an entry from a `.nuspec` file's text.
pub fn parse_nuspec(xml: &str) -> Option<Software> {
    let m = quick_xml::de::from_str::<Nuspec>(xml).ok()?.metadata;
    let id = m.id.trim();
    if id.is_empty() {
        return None;
    }
    let name = m
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or(id);
    let mut s = Software::new(Source::Chocolatey, id, name);
    s.version = Some(m.version.trim().to_string()).filter(|v| !v.is_empty());
    s.publisher = m
        .authors
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty());
    Some(s)
}

/// Lists the packages under one Chocolatey root.
pub fn scan_root(root: &Path) -> std::io::Result<Vec<Software>> {
    let lib = root.join("lib");
    let entries = match fs::read_dir(&lib) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut out = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let Ok(dir_name) = entry.file_name().into_string() else {
            continue;
        };
        // lib-bad and lib-bkp hold failed and backed-up installs.
        if !entry.file_type().is_ok_and(|t| t.is_dir()) || dir_name.starts_with("lib-") {
            continue;
        }
        let dir = entry.path();
        let nuspec = dir.join(format!("{dir_name}.nuspec"));
        let Ok(bytes) = read_capped(&nuspec) else {
            continue;
        };
        if let Some(mut s) = parse_nuspec(&String::from_utf8_lossy(&bytes)) {
            s.location = Some(dir);
            out.push(s);
        }
    }
    Ok(out)
}

pub fn scan(warnings: &mut Vec<String>) -> Vec<Software> {
    scan_root(&root()).unwrap_or_else(|e| {
        warnings.push(format!(
            "chocolatey: could not read packages ({})",
            e.kind()
        ));
        Vec::new()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIT: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://schemas.microsoft.com/packaging/2015/06/nuspec.xsd">
  <metadata>
    <id>git.install</id>
    <version>2.47.0.2</version>
    <title>Git (Install)</title>
    <authors>Johannes Schindelin &amp; contributors</authors>
    <projectUrl>https://gitforwindows.org/</projectUrl>
    <dependencies><dependency id="chocolatey-core.extension" version="1.3.3" /></dependencies>
  </metadata>
</package>"#;

    #[test]
    fn parses_nuspec() {
        let s = parse_nuspec(GIT).unwrap();
        assert_eq!(s.source, Source::Chocolatey);
        assert_eq!(s.id, "git.install");
        assert_eq!(s.name, "Git (Install)");
        assert_eq!(s.version.as_deref(), Some("2.47.0.2"));
        assert_eq!(
            s.publisher.as_deref(),
            Some("Johannes Schindelin & contributors")
        );
        assert!(parse_nuspec("<package><metadata/></package>").is_none());
        assert!(parse_nuspec("not xml").is_none());
    }

    #[test]
    fn scans_lib_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        for d in ["git.install", "7zip", "lib-bad/7zip", "empty"] {
            fs::create_dir_all(lib.join(d)).unwrap();
        }
        fs::write(lib.join("git.install/git.install.nuspec"), GIT).unwrap();
        fs::write(
            lib.join("7zip/7zip.nuspec"),
            "<package><metadata><id>7zip</id><version>24.8.0</version></metadata></package>",
        )
        .unwrap();
        let mut found = scan_root(tmp.path()).unwrap();
        found.sort_by(|a, b| a.id.cmp(&b.id));
        let ids: Vec<(&str, &str)> = found
            .iter()
            .map(|s| (s.id.as_str(), s.name.as_str()))
            .collect();
        assert_eq!(ids, [("7zip", "7zip"), ("git.install", "Git (Install)")]);
        assert_eq!(
            found[0].location.as_deref(),
            Some(lib.join("7zip").as_path())
        );
        assert!(scan_root(&tmp.path().join("missing")).unwrap().is_empty());
    }
}
