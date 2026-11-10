//! MacPorts: active ports, from `port -q installed active`. The registry is a
//! SQLite database with no stable schema, so MacPorts' own tool is run, by
//! absolute path. Ports share one prefix (`/opt/local`), so like distro
//! packages they are marked running by executable name.

use crate::{Software, Source};

/// MacPorts' default prefix. Ports built for another prefix aren't looked for.
pub const PREFIX: &str = "/opt/local/";

/// The `port` tool in [`PREFIX`].
pub const PORT: &str = "/opt/local/bin/port";

/// The registry folder, which changes when ports are installed or removed.
pub const REGISTRY: &str = "/opt/local/var/macports/registry";

/// Parses `port -q installed active` lines such as
/// `  curl @8.11.0_0+ssl (active)`. The version keeps its `_revision`
/// (MacPorts' own rebuild counter) and drops the `+variants`.
pub fn parse_installed(text: &str) -> Vec<Software> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut cols = line.split_whitespace();
        let (Some(name), Some(version)) = (cols.next(), cols.next()) else {
            continue;
        };
        let Some(version) = version.strip_prefix('@') else {
            continue; // a header line, if -q didn't drop it
        };
        if cols.next() != Some("(active)") {
            continue;
        }
        let version = version.split('+').next().unwrap_or_default();
        let mut s = Software::new(Source::Macports, name, name);
        s.version = (!version.is_empty()).then(|| version.to_string());
        out.push(s);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out.dedup_by(|a, b| a.id == b.id);
    out
}

/// Active ports, if MacPorts is installed.
pub fn scan(warnings: &mut Vec<String>) -> Vec<Software> {
    if !std::path::Path::new(PORT).exists() {
        return Vec::new();
    }
    match crate::run(PORT, &["-q", "installed", "active"]) {
        Ok(text) => parse_installed(&text),
        Err(e) => {
            warnings.push(format!("macports: {e}"));
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_active_ports() {
        let text = "The following ports are currently installed:\n  \
            curl @8.11.0_0+ssl (active)\n  \
            openssl3 @3.4.0_1+universal (active)\n  \
            python312 @3.12.7_0 (active)\n  \
            python312 @3.12.6_0\n  \
            zlib @1.3.1_0 (active)\n";
        let got: Vec<(String, Option<String>)> = parse_installed(text)
            .into_iter()
            .map(|s| (s.id, s.version))
            .collect();
        let want = [
            ("curl", "8.11.0_0"),
            ("openssl3", "3.4.0_1"),
            ("python312", "3.12.7_0"),
            ("zlib", "1.3.1_0"),
        ]
        .map(|(id, v)| (id.to_string(), Some(v.to_string())));
        assert_eq!(got, want);
    }
}
