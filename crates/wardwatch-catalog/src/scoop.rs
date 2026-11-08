//! Scoop apps, read from `apps\<app>\current\manifest.json` in the user's
//! and the global Scoop folders. `current` links to the installed version.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{Software, Source, read_capped};

/// Scoop folders: the user's (`%SCOOP%`, else `~\scoop`) and the global one
/// (`%SCOOP_GLOBAL%`, else `%ProgramData%\scoop`).
pub fn roots() -> Vec<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty());
    let user = env("SCOOP")
        .map(PathBuf::from)
        .or_else(|| crate::home_dir().map(|h| h.join("scoop")));
    let global = env("SCOOP_GLOBAL")
        .map(PathBuf::from)
        .or_else(|| env("ProgramData").map(|d| Path::new(&d).join("scoop")));
    user.into_iter().chain(global).collect()
}

#[derive(Deserialize)]
struct Manifest {
    version: Option<String>,
}

#[derive(Deserialize)]
struct InstallInfo {
    architecture: Option<String>,
}

/// Lists the apps under one Scoop folder.
pub fn scan_root(root: &Path) -> std::io::Result<Vec<Software>> {
    let apps = root.join("apps");
    let entries = match fs::read_dir(&apps) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut out = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let dir = entry.path();
        let current = dir.join("current");
        // Scoop's own folder is a git checkout with no manifest.
        let Some(manifest) = read_capped(&current.join("manifest.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Manifest>(&b).ok())
        else {
            continue;
        };
        let mut s = Software::new(Source::Scoop, &name, &name);
        s.version = manifest.version.filter(|v| !v.trim().is_empty());
        s.arch = read_capped(&current.join("install.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<InstallInfo>(&b).ok())
            .and_then(|i| i.architecture);
        // The app folder, not `current`: processes may run from either path.
        s.location = Some(dir);
        out.push(s);
    }
    Ok(out)
}

pub fn scan(warnings: &mut Vec<String>) -> Vec<Software> {
    let mut out = Vec::new();
    for root in roots() {
        match scan_root(&root) {
            // An app installed both per-user and globally is listed twice;
            // each copy is updated separately.
            Ok(found) => out.extend(found),
            Err(e) => warnings.push(format!("scoop: could not read apps ({})", e.kind())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_apps_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let apps = tmp.path().join("apps");
        for d in [
            "git/2.47.0/",
            "git/current",
            "scoop/current",
            "7zip/current",
        ] {
            fs::create_dir_all(apps.join(d)).unwrap();
        }
        fs::write(
            apps.join("git/current/manifest.json"),
            r#"{"version":"2.47.0","description":"Git","homepage":"https://git-scm.com"}"#,
        )
        .unwrap();
        fs::write(
            apps.join("git/current/install.json"),
            r#"{"bucket":"main","architecture":"64bit"}"#,
        )
        .unwrap();
        fs::write(apps.join("7zip/current/manifest.json"), r#"{"version":""}"#).unwrap();
        fs::write(apps.join("scoop/current/README.md"), "scoop").unwrap();

        let mut found = scan_root(tmp.path()).unwrap();
        found.sort_by(|a, b| a.id.cmp(&b.id));
        let summary: Vec<(&str, Option<&str>, Option<&str>)> = found
            .iter()
            .map(|s| (s.id.as_str(), s.version.as_deref(), s.arch.as_deref()))
            .collect();
        assert_eq!(
            summary,
            [("7zip", None, None), ("git", Some("2.47.0"), Some("64bit"))]
        );
        assert_eq!(
            found[1].location.as_deref(),
            Some(apps.join("git").as_path())
        );
        assert!(scan_root(&tmp.path().join("missing")).unwrap().is_empty());
    }
}
