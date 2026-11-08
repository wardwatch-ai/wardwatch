//! Reads Wardwatch's update and vulnerability catalog (`catalog.json`,
//! schema_version 1, built by `wardwatch-ai/backend` `catalog/`) and matches
//! it against this device's [`Inventory`], on the device.
//!
//! The result is one [`Finding`] per catalog app found on the device: what's
//! installed, the version that closes its holes, and those holes. Nothing here
//! touches the network or the disk; callers pass the catalog's bytes in.
//!
//! **Signatures.** The catalog is not signed yet. [`Catalog::from_json`]
//! checks shape, schema version and expiry only; verifying `catalog.json.sig`
//! against Wardwatch's keys belongs in front of it once the keys exist.

use std::cmp::Ordering;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use wardwatch_catalog::{Browser, Inventory, OsFamily, OsInfo, Software, Source};

pub mod version;

pub use version::Scheme;

/// The catalog format this crate reads.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Catalog {
    pub schema_version: u32,
    pub generated_at: String,
    pub expires_at: String,
    pub apps: Vec<CatalogApp>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppKind {
    App,
    Os,
    Package,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct CatalogApp {
    pub id: String,
    pub name: String,
    pub kind: AppKind,
    pub version_scheme: Scheme,
    #[serde(rename = "match")]
    pub matches: Vec<MatchRule>,
    #[serde(default)]
    pub latest: Option<Latest>,
    pub vulns: Vec<CatalogVuln>,
}

/// How the device recognises an app. `source` is the scanner's source name,
/// or `os` for the operating system itself (matched on `os.id`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct MatchRule {
    pub source: String,
    #[serde(default)]
    pub id: Option<String>,
    /// `windows_registry` only: a prefix of the Uninstall `DisplayName`.
    #[serde(default)]
    pub name_prefix: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Latest {
    pub version: String,
    /// Where the version came from, e.g. `homebrew_cask:google-chrome`.
    pub source: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    // Declared low to high so `Ord` ranks them.
    Unknown,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kev {
    pub added: String,
    pub due: String,
    pub ransomware: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct CatalogVuln {
    pub id: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub published: String,
    pub severity: Severity,
    #[serde(default)]
    pub cvss: Option<f64>,
    pub exploited: bool,
    #[serde(default)]
    pub kev: Option<Kev>,
    pub ranges: Vec<Range>,
    pub summary: String,
}

/// OSV range semantics: affected when `introduced <= v < fixed`, or
/// `introduced <= v <= last_affected`; neither bound means no fix yet.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Range {
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub introduced: Option<String>,
    #[serde(default)]
    pub fixed: Option<String>,
    #[serde(default)]
    pub last_affected: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CatalogError {
    Parse(String),
    Schema(u32),
    BadExpiry(String),
    Expired { expires_at: String },
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::Parse(e) => write!(f, "the catalog isn't valid: {e}"),
            CatalogError::Schema(v) => write!(f, "catalog schema {v} is not supported"),
            CatalogError::BadExpiry(v) => write!(f, "the catalog's expiry {v:?} isn't a date"),
            CatalogError::Expired { expires_at } => {
                write!(f, "the catalog expired at {expires_at}")
            }
        }
    }
}

impl std::error::Error for CatalogError {}

impl Catalog {
    /// Parses a catalog and refuses one with another schema version or past
    /// its `expires_at` (so a stale or replayed copy can't say you're safe).
    pub fn from_json(bytes: &[u8], now: SystemTime) -> Result<Catalog, CatalogError> {
        #[derive(Deserialize)]
        struct Header {
            schema_version: u32,
        }
        let header: Header =
            serde_json::from_slice(bytes).map_err(|e| CatalogError::Parse(e.to_string()))?;
        if header.schema_version != SCHEMA_VERSION {
            return Err(CatalogError::Schema(header.schema_version));
        }
        let catalog: Catalog =
            serde_json::from_slice(bytes).map_err(|e| CatalogError::Parse(e.to_string()))?;
        let expires = time::OffsetDateTime::parse(
            &catalog.expires_at,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|_| CatalogError::BadExpiry(catalog.expires_at.clone()))?;
        if time::OffsetDateTime::from(now) >= expires {
            return Err(CatalogError::Expired {
                expires_at: catalog.expires_at.clone(),
            });
        }
        Ok(catalog)
    }
}

/// One install of a catalog app on this device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Install {
    /// The scanner's source name (`mac_app`, `dpkg`, ...), or `os`.
    pub source: String,
    /// The id in that source's namespace (bundle ID, package name, ...).
    pub id: String,
    pub name: String,
    pub version: String,
    pub running: bool,
}

/// One vulnerability that affects the installed version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VulnHit {
    pub id: String,
    pub aliases: Vec<String>,
    pub published: String,
    pub severity: Severity,
    pub cvss: Option<f64>,
    pub exploited: bool,
    pub kev: Option<Kev>,
    pub summary: String,
    /// The version that fixes it here, or `None` when there's no fix yet.
    pub fixed: Option<String>,
}

/// A catalog app found on this device, and what updating it would close.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// `source:id` of the primary install; stable across scans.
    pub key: String,
    pub app_id: String,
    pub app_name: String,
    pub kind: AppKind,
    pub source: String,
    pub software_id: String,
    pub name: String,
    pub browser: Option<Browser>,
    pub version_scheme: Scheme,
    /// The installed version (the oldest one, if it's installed more than once).
    pub installed: String,
    pub running: bool,
    /// Every install of this app, primary first. Distro packages built from
    /// one source package (`openssl`, `libssl3t64`) are separate installs.
    pub installs: Vec<Install>,
    /// The lowest version that closes every hole here that has a fix.
    pub fixed_version: Option<String>,
    pub latest: Option<String>,
    /// A newer version exists (a fix, or `latest` beyond what's installed).
    pub update_available: bool,
    pub exploited: bool,
    /// The worst affecting hole; `None` when no known hole affects this version.
    pub severity: Option<Severity>,
    /// Newest first, as in the catalog.
    pub vulns: Vec<VulnHit>,
}

/// Matches this device's inventory against the catalog. Entries whose version
/// can't be read in the app's scheme are skipped rather than guessed at.
/// Results are sorted most urgent first: exploited, then severity, then name.
pub fn match_inventory(inventory: &Inventory, catalog: &Catalog) -> Vec<Finding> {
    let mut findings: Vec<Finding> = catalog
        .apps
        .iter()
        .filter_map(|app| match_app(app, inventory))
        .collect();
    findings.sort_by(|a, b| {
        b.exploited
            .cmp(&a.exploited)
            .then_with(|| b.severity.cmp(&a.severity))
            .then_with(|| b.update_available.cmp(&a.update_available))
            .then_with(|| a.app_name.cmp(&b.app_name))
    });
    findings
}

fn match_app(app: &CatalogApp, inventory: &Inventory) -> Option<Finding> {
    let scheme = app.version_scheme;
    let mut installs: Vec<(Install, Option<Browser>)> = Vec::new();
    for rule in &app.matches {
        if rule.source == "os" {
            let os = &inventory.os;
            if rule.id.as_deref() == Some(os.id.as_str()) && !os.version.is_empty() {
                let install = Install {
                    source: "os".into(),
                    id: os.id.clone(),
                    name: os.name.clone(),
                    version: os.version.clone(),
                    running: false,
                };
                installs.push((install, None));
            }
            continue;
        }
        for s in inventory.software.iter().filter(|s| rule_matches(rule, s)) {
            let Some(version) = s.version.clone() else {
                continue;
            };
            let source = source_name(s.source);
            if installs
                .iter()
                .any(|(i, _)| i.source == source && i.id == s.id)
            {
                continue;
            }
            let install = Install {
                source: source.into(),
                id: s.id.clone(),
                name: s.name.clone(),
                version,
                running: s.running,
            };
            installs.push((install, s.browser));
        }
    }
    // Skip versions this scheme can't read rather than guess at them.
    installs.retain(|(i, _)| version::compare(scheme, &i.version, &i.version).is_some());
    // The oldest install decides what's affected; ties keep catalog match order,
    // which lists the app's own identity (bundle ID) before package managers.
    let primary = (0..installs.len()).min_by(|&a, &b| {
        version::compare(scheme, &installs[a].0.version, &installs[b].0.version)
            .unwrap_or(Ordering::Equal)
    })?;
    let first = installs.remove(primary);
    let browser = first.1.or_else(|| installs.iter().find_map(|(_, b)| *b));
    let mut all = vec![first.0];
    all.extend(installs.into_iter().map(|(i, _)| i));
    let installed = all[0].version.clone();
    let scopes = device_scopes(&inventory.os);

    let mut vulns = Vec::new();
    for v in &app.vulns {
        let mut hit = false;
        let mut fixed: Option<&str> = None;
        let mut unfixed = false;
        for r in v.ranges.iter().filter(|r| applies(r, &scopes)) {
            if !in_range(scheme, &installed, r) {
                continue;
            }
            hit = true;
            match r.fixed.as_deref() {
                Some(f) => fixed = Some(max_version(scheme, fixed, f)),
                None => unfixed = true,
            }
        }
        if hit {
            vulns.push(VulnHit {
                id: v.id.clone(),
                aliases: v.aliases.clone(),
                published: v.published.clone(),
                severity: v.severity,
                cvss: v.cvss,
                exploited: v.exploited,
                kev: v.kev.clone(),
                summary: v.summary.clone(),
                // A range with no fix means some affected path stays open.
                fixed: if unfixed {
                    None
                } else {
                    fixed.map(str::to_owned)
                },
            });
        }
    }

    let fixed_version = vulns
        .iter()
        .filter_map(|v| v.fixed.as_deref())
        .fold(None, |acc, f| Some(max_version(scheme, acc, f)))
        .map(str::to_owned);
    let latest = app.latest.as_ref().map(|l| l.version.clone());
    let newer = |v: &Option<String>| {
        v.as_deref()
            .and_then(|v| version::compare(scheme, v, &installed))
            .is_some_and(Ordering::is_gt)
    };
    let update_available = newer(&fixed_version) || newer(&latest);
    let first = &all[0];
    Some(Finding {
        key: format!("{}:{}", first.source, first.id),
        app_id: app.id.clone(),
        app_name: app.name.clone(),
        kind: app.kind,
        source: first.source.clone(),
        software_id: first.id.clone(),
        name: first.name.clone(),
        browser,
        version_scheme: scheme,
        installed: installed.clone(),
        running: all.iter().any(|i| i.running),
        exploited: vulns.iter().any(|v| v.exploited),
        severity: vulns.iter().map(|v| v.severity).max(),
        installs: all,
        fixed_version,
        latest,
        update_available,
        vulns,
    })
}

fn rule_matches(rule: &MatchRule, s: &Software) -> bool {
    if rule.source != source_name(s.source) {
        return false;
    }
    match s.source {
        Source::WindowsRegistry => rule
            .name_prefix
            .as_deref()
            .is_some_and(|p| s.name.starts_with(p)),
        // The catalog names distro packages by source package, which covers
        // every binary package built from it.
        Source::Dpkg | Source::Rpm => {
            rule.id.as_deref() == Some(s.source_package.as_deref().unwrap_or(&s.id))
        }
        _ => rule.id.as_deref() == Some(s.id.as_str()),
    }
}

/// The scanner's serialized name for a source (the catalog uses the same names).
pub fn source_name(source: Source) -> &'static str {
    match source {
        Source::MacApp => "mac_app",
        Source::HomebrewFormula => "homebrew_formula",
        Source::HomebrewCask => "homebrew_cask",
        Source::WindowsRegistry => "windows_registry",
        Source::Dpkg => "dpkg",
        Source::Rpm => "rpm",
        Source::Snap => "snap",
        Source::Flatpak => "flatpak",
    }
}

/// Range scopes that apply here: the OS family, and the OSV ecosystem of
/// this distro release (`Debian:12`, `Ubuntu:24.04:LTS`).
fn device_scopes(os: &OsInfo) -> Vec<String> {
    let family = match os.family {
        OsFamily::Macos => "os:macos",
        OsFamily::Windows => "os:windows",
        OsFamily::Linux => "os:linux",
    };
    let mut scopes = vec![family.to_owned()];
    if let Some(eco) = osv_ecosystem(&os.id, &os.version) {
        scopes.push(eco);
    }
    scopes
}

fn osv_ecosystem(id: &str, version: &str) -> Option<String> {
    match id {
        // VERSION_ID is the major release ("12"); OSV uses the same.
        "debian" => Some(format!("Debian:{}", version.split('.').next()?)),
        // Ubuntu LTS releases are even years in April.
        "ubuntu" => {
            let (year, month) = version.split_once('.')?;
            let lts = month == "04" && year.parse::<u32>().ok()? % 2 == 0;
            Some(if lts {
                format!("Ubuntu:{version}:LTS")
            } else {
                format!("Ubuntu:{version}")
            })
        }
        _ => None,
    }
}

fn applies(r: &Range, scopes: &[String]) -> bool {
    r.scope.as_ref().is_none_or(|s| scopes.contains(s))
}

fn in_range(scheme: Scheme, v: &str, r: &Range) -> bool {
    let cmp = |b: &str| version::compare(scheme, v, b);
    if let Some(intro) = r.introduced.as_deref() {
        // OSV writes "0" for "from the start".
        if intro != "0" && !cmp(intro).is_some_and(Ordering::is_ge) {
            return false;
        }
    }
    match (r.fixed.as_deref(), r.last_affected.as_deref()) {
        (Some(f), _) => cmp(f).is_some_and(Ordering::is_lt),
        (None, Some(l)) => cmp(l).is_some_and(Ordering::is_le),
        (None, None) => true,
    }
}

fn max_version<'a>(scheme: Scheme, acc: Option<&'a str>, v: &'a str) -> &'a str {
    match acc {
        Some(a) if version::compare(scheme, a, v).is_some_and(Ordering::is_ge) => a,
        _ => v,
    }
}
