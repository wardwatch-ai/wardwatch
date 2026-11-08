//! Plans and runs one update, using only the device's own updaters:
//! Homebrew, winget, apt, dnf, snap and flatpak, or the OS's own settings
//! screen for system updates. Wardwatch never downloads or installs anything
//! itself, so the vendor or distro's signature checks always apply.
//!
//! The trust rules this crate follows:
//!
//! - **Say it before doing it.** [`plan`] returns every step, the exact
//!   command line, who checks the download, and whether the OS will ask for a
//!   password. The UI shows all of it before the person presses anything.
//! - **Never escalate silently.** Commands run as the signed-in user. Where a
//!   package manager needs admin rights (apt, dnf), the command goes through
//!   `pkexec`, so the OS shows its own password prompt; Wardwatch never sees the
//!   password. winget installers may show Windows' own permission prompt.
//! - **Nothing from the UI reaches a command line.** Commands are built here
//!   from a [`Finding`] the core matched itself, binaries are called by
//!   absolute path with no shell, and every package name is checked first.
//! - **Never close an open app.** If the app is running, the plan says to
//!   restart it afterwards; we don't kill it.

use std::path::{Path, PathBuf};

use serde::Serialize;
use wardwatch_catalog::OsFamily;
use wardwatch_feed::{AppKind, Finding};

mod run;

pub use run::{Outcome, Progress, StepState, run};

/// The updaters found on this device, by absolute path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tools {
    pub brew: Option<PathBuf>,
    pub winget: Option<PathBuf>,
    pub pkexec: Option<PathBuf>,
    pub apt_get: Option<PathBuf>,
    pub dnf: Option<PathBuf>,
    pub snap: Option<PathBuf>,
    pub flatpak: Option<PathBuf>,
}

impl Tools {
    /// Looks for each updater in its standard location only (never `PATH`,
    /// which a user-writable folder could shadow).
    pub fn detect() -> Tools {
        let first = |paths: &[PathBuf]| paths.iter().find(|p| p.is_file()).cloned();
        let fixed = |paths: &[&str]| first(&paths.iter().map(PathBuf::from).collect::<Vec<_>>());
        let brew = first(
            &wardwatch_catalog::homebrew::PREFIXES
                .iter()
                .map(|p| Path::new(p).join("bin/brew"))
                .collect::<Vec<_>>(),
        );
        // winget is an app execution alias in the user's WindowsApps folder.
        let winget = std::env::var_os("LOCALAPPDATA")
            .map(|d| PathBuf::from(d).join(r"Microsoft\WindowsApps\winget.exe"))
            .filter(|p| p.is_file());
        Tools {
            brew,
            winget,
            pkexec: fixed(&["/usr/bin/pkexec"]),
            apt_get: fixed(&["/usr/bin/apt-get"]),
            dnf: fixed(&["/usr/bin/dnf"]),
            snap: fixed(&["/usr/bin/snap"]),
            flatpak: fixed(&["/usr/bin/flatpak"]),
        }
    }
}

/// How the update happens.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Method {
    /// Wardwatch runs the package manager; one click does it all.
    Command { tool: Tool, argv: Vec<String> },
    /// The OS installs system updates itself; we open its update screen.
    SystemSettings { target: String },
    /// The app updates itself; we open it (when we can) and say where to click.
    AppUpdater { open_bundle: Option<String> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    Homebrew,
    Winget,
    Apt,
    Dnf,
    Snap,
    Flatpak,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Homebrew => "Homebrew",
            Tool::Winget => "winget",
            Tool::Apt => "apt",
            Tool::Dnf => "dnf",
            Tool::Snap => "Snap",
            Tool::Flatpak => "Flatpak",
        }
    }
}

/// Whether the OS will ask for permission. Wardwatch never asks for or sees a password.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Admin {
    No,
    /// The OS shows its own password prompt (pkexec, snapd, flatpak system installs).
    Asks,
    /// The installer may show the OS's permission prompt (winget machine-wide installs).
    MayAsk,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Step {
    pub label: String,
    pub detail: String,
}

/// Everything the person sees before they press Update.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Plan {
    pub method: Method,
    /// Who does the work, in words: "Homebrew", "Software Update", "Chrome".
    pub by: String,
    /// `true` when pressing the button finishes the job.
    pub one_click: bool,
    pub admin: Admin,
    /// Four steps for a command (get, check, install, check again).
    pub steps: Vec<Step>,
    /// What checks the download is genuine, honestly per updater.
    pub verifies: String,
    /// Instructions when the person finishes it (system settings, app updater).
    pub manual: Option<String>,
    /// Set when the app is open: the new version starts after a restart.
    pub restart_note: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum NoPlan {
    UpToDate,
    /// We can't update this here; the string says what to do instead.
    Unsupported(String),
}

impl std::fmt::Display for NoPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoPlan::UpToDate => f.write_str("already up to date"),
            NoPlan::Unsupported(why) => f.write_str(why),
        }
    }
}

const MACOS_SOFTWARE_UPDATE: &str =
    "x-apple.systempreferences:com.apple.Software-Update-Settings.extension";
const WINDOWS_UPDATE: &str = "ms-settings:windowsupdate";

/// winget package ids for catalog apps. The scanner reads DisplayName from
/// the registry, which winget can't upgrade by; this maps the catalog's app
/// id to winget's. Belongs in the catalog itself once it carries winget ids.
const WINGET_IDS: &[(&str, &str)] = &[
    ("google-chrome", "Google.Chrome"),
    ("mozilla-firefox", "Mozilla.Firefox"),
    ("vlc", "VideoLAN.VLC"),
    ("7-zip", "7zip.7zip"),
];

/// Works out how to update `finding` on this device, without running anything.
pub fn plan(finding: &Finding, os: OsFamily, tools: &Tools) -> Result<Plan, NoPlan> {
    if !finding.update_available {
        return Err(NoPlan::UpToDate);
    }
    let name = &finding.app_name;
    let target = finding
        .fixed_version
        .as_deref()
        .or(finding.latest.as_deref())
        .map(|v| format!("{name} {v}"))
        .unwrap_or_else(|| format!("the newest {name}"));
    let restart_note = (finding.running && finding.kind != AppKind::Os).then(|| {
        format!(
            "{name} is open. It keeps running until you restart it, then the new version starts."
        )
    });

    if finding.kind == AppKind::Os {
        let (target_url, manual) = match os {
            OsFamily::Macos => (
                MACOS_SOFTWARE_UPDATE,
                "Software Update opens. Press Update Now (or Restart Now) and follow the steps. Your Mac may restart.",
            ),
            OsFamily::Windows => (
                WINDOWS_UPDATE,
                "Windows Update opens. Press Check for updates, then Download & install. Windows may restart.",
            ),
            OsFamily::Linux => {
                return Err(NoPlan::Unsupported(
                    "System updates on Linux come through your distribution's packages.".into(),
                ));
            }
        };
        return Ok(Plan {
            method: Method::SystemSettings {
                target: target_url.into(),
            },
            by: if os == OsFamily::Macos { "Software Update" } else { "Windows Update" }.into(),
            one_click: false,
            admin: Admin::Asks,
            steps: vec![],
            verifies: if os == OsFamily::Macos {
                "Apple signs every macOS update, and your Mac checks the signature before installing."
            } else {
                "Microsoft signs every Windows update, and Windows checks the signature before installing."
            }
            .into(),
            manual: Some(manual.into()),
            restart_note: None,
        });
    }

    let installs = |source: &str| -> Vec<&str> {
        finding
            .installs
            .iter()
            .filter(|i| i.source == source)
            .map(|i| i.id.as_str())
            .collect()
    };
    let one = |source: &str| installs(source).first().copied();
    let path = |p: &Option<PathBuf>| p.as_ref().map(|p| p.to_string_lossy().into_owned());

    // Package managers first: they verify downloads and need no clicks.
    let command: Option<(Tool, Vec<String>, Admin)> =
        if let (Some(token), Some(brew)) = (one("homebrew_cask"), path(&tools.brew)) {
            // --greedy: casks that update themselves (Chrome, Firefox) are skipped otherwise.
            Some((
                Tool::Homebrew,
                argv(&[&brew, "upgrade", "--cask", "--greedy"], &[token])?,
                Admin::No,
            ))
        } else if let (Some(formula), Some(brew)) = (one("homebrew_formula"), path(&tools.brew)) {
            Some((
                Tool::Homebrew,
                argv(&[&brew, "upgrade"], &[formula])?,
                Admin::No,
            ))
        } else if let (Some(snap), Some(bin)) = (one("snap"), path(&tools.snap)) {
            Some((Tool::Snap, argv(&[&bin, "refresh"], &[snap])?, Admin::Asks))
        } else if let (Some(id), Some(bin)) = (one("flatpak"), path(&tools.flatpak)) {
            Some((
                Tool::Flatpak,
                argv(&[&bin, "update", "-y", "--noninteractive"], &[id])?,
                Admin::MayAsk,
            ))
        } else if let (false, Some(pkexec), Some(apt)) = (
            installs("dpkg").is_empty(),
            path(&tools.pkexec),
            path(&tools.apt_get),
        ) {
            let base = [
                pkexec.as_str(),
                apt.as_str(),
                "install",
                "--only-upgrade",
                "-y",
                // Keep the person's edited config files rather than stopping to ask.
                "-o",
                "Dpkg::Options::=--force-confdef",
                "-o",
                "Dpkg::Options::=--force-confold",
            ];
            Some((Tool::Apt, argv(&base, &installs("dpkg"))?, Admin::Asks))
        } else if let (false, Some(pkexec), Some(dnf)) = (
            installs("rpm").is_empty(),
            path(&tools.pkexec),
            path(&tools.dnf),
        ) {
            let base = [pkexec.as_str(), dnf.as_str(), "upgrade", "-y", "--refresh"];
            Some((Tool::Dnf, argv(&base, &installs("rpm"))?, Admin::Asks))
        } else if let (Some(_), Some(winget), Some(id)) = (
            one("windows_registry"),
            path(&tools.winget),
            WINGET_IDS
                .iter()
                .find(|(app, _)| *app == finding.app_id)
                .map(|(_, id)| *id),
        ) {
            let base = [
                winget.as_str(),
                "upgrade",
                "--exact",
                "--source",
                "winget",
                "--silent",
                "--disable-interactivity",
                "--accept-source-agreements",
                "--accept-package-agreements",
                "--id",
            ];
            Some((Tool::Winget, argv(&base, &[id])?, Admin::MayAsk))
        } else {
            None
        };

    if let Some((tool, argv, admin)) = command {
        let by = tool.label();
        let (get, check) = match tool {
            Tool::Homebrew => (
                format!("From the vendor's own download, through {by}"),
                "Homebrew checks the file's fingerprint, and macOS checks the app's signature"
                    .to_owned(),
            ),
            Tool::Winget => (
                format!("From the vendor's own download, through {by}"),
                "winget checks the file's fingerprint against Microsoft's catalog".to_owned(),
            ),
            Tool::Apt | Tool::Dnf => (
                "From your distribution's own servers".to_owned(),
                "Checks your distribution's digital signature".to_owned(),
            ),
            Tool::Snap | Tool::Flatpak => (
                format!("From the {by} store"),
                format!("{by} checks the publisher's signature"),
            ),
        };
        let verifies = match tool {
            Tool::Homebrew => {
                "Homebrew downloads from the vendor and checks the file's SHA-256 fingerprint; macOS then checks the vendor's signature when the app opens."
            }
            Tool::Winget => {
                "winget downloads from the vendor and checks the installer's SHA-256 fingerprint against Microsoft's winget catalog before running it."
            }
            Tool::Apt => "apt only installs packages signed by your distribution's keys.",
            Tool::Dnf => "dnf only installs packages signed by your distribution's keys.",
            Tool::Snap => "snapd only installs snaps signed by the Snap Store.",
            Tool::Flatpak => "Flatpak only installs updates signed by the remote's key.",
        };
        let install_detail = match admin {
            Admin::Asks => "Your system asks for your password. Wardwatch never sees it",
            Admin::MayAsk => "Your system may ask for permission",
            Admin::No => "Nothing to type",
        };
        let install = if restart_note.is_some() {
            format!("Install it, then restart {name}")
        } else {
            "Install it".to_owned()
        };
        return Ok(Plan {
            method: Method::Command { tool, argv },
            by: by.into(),
            one_click: true,
            admin,
            steps: vec![
                step(&format!("Download {target}"), &get),
                step("Make sure it's the real thing", &check),
                step(&install, install_detail),
                step(
                    "Check again that the hole is closed",
                    "Wardwatch rescans this device",
                ),
            ],
            verifies: verifies.into(),
            manual: None,
            restart_note,
        });
    }

    // Distro packages without pkexec: the desktop's own updater can do it.
    if !installs("dpkg").is_empty() || !installs("rpm").is_empty() {
        return Ok(Plan {
            method: Method::AppUpdater { open_bundle: None },
            by: "Software Updater".into(),
            one_click: false,
            admin: Admin::Asks,
            steps: vec![],
            verifies: "Your distribution signs its packages, and your package manager checks the signature before installing.".into(),
            manual: Some(format!(
                "Open your system's Software Updater (or Software, then Updates) and install the updates it lists. That includes {target}."
            )),
            restart_note,
        });
    }

    // No package manager owns it: the app's own updater does the job.
    let open_bundle = one("mac_app").filter(|id| safe_arg(id)).map(str::to_owned);
    let manual = match finding.browser {
        Some(_) => format!(
            "Open {name}'s menu, choose Help or About {name}, and let it update. Then restart {name}."
        ),
        None => format!(
            "Open {name} and use its Check for Updates menu item, or download {target} from the maker's website."
        ),
    };
    let os_can_open = os == OsFamily::Macos && open_bundle.is_some();
    Ok(Plan {
        method: Method::AppUpdater { open_bundle },
        by: name.clone(),
        one_click: false,
        admin: Admin::No,
        steps: vec![],
        verifies: format!(
            "{name}'s own updater downloads from its maker and checks the maker's signature."
        ),
        manual: Some(if os_can_open {
            format!("Wardwatch opens {name}. {manual}")
        } else {
            manual
        }),
        restart_note,
    })
}

fn step(label: &str, detail: &str) -> Step {
    Step {
        label: label.into(),
        detail: detail.into(),
    }
}

/// `base` followed by checked package names.
fn argv(base: &[&str], names: &[&str]) -> Result<Vec<String>, NoPlan> {
    if let Some(bad) = names.iter().find(|n| !safe_arg(n)) {
        return Err(NoPlan::Unsupported(format!(
            "Wardwatch won't pass the name {bad:?} to an updater."
        )));
    }
    Ok(base.iter().chain(names).map(|s| (*s).to_owned()).collect())
}

/// Package names, cask tokens, snap names, Flatpak and winget ids, bundle ids:
/// letters, digits and `._+-@`, not starting with `-` (so never an option).
fn safe_arg(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 200
        && !s.starts_with('-')
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'+' | b'-' | b'@'))
}

#[cfg(test)]
mod tests;
