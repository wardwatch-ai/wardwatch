//! Homebrew formulae and casks, read from the Cellar and Caskroom folders.
//! Listing folders is far cheaper than running `brew`, which starts Ruby.

use std::fs;
use std::path::Path;

use crate::{Software, Source};

/// Default Homebrew prefixes: Apple silicon, Intel macOS, and Linux.
pub const PREFIXES: [&str; 3] = ["/opt/homebrew", "/usr/local", "/home/linuxbrew/.linuxbrew"];

pub fn scan(warnings: &mut Vec<String>) -> Vec<Software> {
    let mut out = Vec::new();
    for prefix in PREFIXES {
        let prefix = Path::new(prefix);
        if !prefix.join("Cellar").is_dir() && !prefix.join("Caskroom").is_dir() {
            continue;
        }
        match scan_prefix(prefix) {
            Ok(found) => out.extend(found),
            Err(e) => warnings.push(format!("homebrew: could not read a prefix ({})", e.kind())),
        }
    }
    out
}

/// Lists formulae and casks under one Homebrew prefix.
pub fn scan_prefix(prefix: &Path) -> std::io::Result<Vec<Software>> {
    let mut out = Vec::new();
    for name in subdirs(&prefix.join("Cellar"))? {
        let keg_dir = prefix.join("Cellar").join(&name);
        // `opt/<name>` links to the version in use; older kegs may linger.
        let linked = fs::read_link(prefix.join("opt").join(&name))
            .ok()
            .and_then(|t| t.file_name()?.to_str().map(String::from));
        let versions = match linked {
            Some(v) if keg_dir.join(&v).is_dir() => vec![v],
            _ => subdirs(&keg_dir)?,
        };
        for v in versions {
            let mut s = Software::new(Source::HomebrewFormula, &name, &name);
            s.location = Some(keg_dir.join(&v));
            s.version = Some(v);
            out.push(s);
        }
    }
    for token in subdirs(&prefix.join("Caskroom"))? {
        for v in subdirs(&prefix.join("Caskroom").join(&token))? {
            let mut s = Software::new(Source::HomebrewCask, &token, &token);
            // "latest" casks have no meaningful version.
            s.version = (v != "latest").then_some(v);
            out.push(s);
        }
    }
    Ok(out)
}

/// Names of visible subfolders, or nothing if `dir` doesn't exist.
fn subdirs(dir: &Path) -> std::io::Result<Vec<String>> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_linked_formula_versions_and_casks() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path();
        for d in [
            "Cellar/openssl@3/3.3.2",
            "Cellar/openssl@3/3.4.0",
            "Cellar/jq/1.7.1",
            "Caskroom/firefox/132.0.1",
            "Caskroom/firefox/.metadata",
            "Caskroom/obsidian/latest",
            "opt",
        ] {
            fs::create_dir_all(p.join(d)).unwrap();
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink("../Cellar/openssl@3/3.4.0", p.join("opt/openssl@3")).unwrap();

        let found = scan_prefix(p).unwrap();
        let summary: Vec<(Source, &str, Option<&str>)> = found
            .iter()
            .map(|s| (s.source, s.id.as_str(), s.version.as_deref()))
            .collect();
        let mut want = vec![(Source::HomebrewFormula, "jq", Some("1.7.1"))];
        if cfg!(unix) {
            want.push((Source::HomebrewFormula, "openssl@3", Some("3.4.0")));
        } else {
            want.push((Source::HomebrewFormula, "openssl@3", Some("3.3.2")));
            want.push((Source::HomebrewFormula, "openssl@3", Some("3.4.0")));
        }
        want.push((Source::HomebrewCask, "firefox", Some("132.0.1")));
        want.push((Source::HomebrewCask, "obsidian", None));
        assert_eq!(summary, want);
    }

    #[test]
    fn missing_prefix_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(scan_prefix(tmp.path()).unwrap().is_empty());
    }
}
