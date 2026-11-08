use std::path::PathBuf;

use wardwatch_catalog::{Browser, OsFamily};
use wardwatch_feed::{AppKind, Finding, Install, Scheme, Severity};

use super::*;

fn install(source: &str, id: &str) -> Install {
    Install {
        source: source.into(),
        id: id.into(),
        name: id.into(),
        version: "1.0".into(),
        running: false,
    }
}

fn finding(app_id: &str, name: &str, kind: AppKind, installs: Vec<Install>) -> Finding {
    let first = installs[0].clone();
    Finding {
        key: format!("{}:{}", first.source, first.id),
        app_id: app_id.into(),
        app_name: name.into(),
        kind,
        source: first.source.clone(),
        software_id: first.id.clone(),
        name: first.name.clone(),
        browser: None,
        version_scheme: Scheme::Dotted,
        installed: "1.0".into(),
        running: false,
        installs,
        fixed_version: Some("1.1".into()),
        latest: Some("1.2".into()),
        update_available: true,
        exploited: true,
        severity: Some(Severity::High),
        vulns: vec![],
    }
}

fn all_tools() -> Tools {
    Tools {
        brew: Some("/opt/homebrew/bin/brew".into()),
        winget: Some(PathBuf::from(
            r"C:\Users\a\AppData\Local\Microsoft\WindowsApps\winget.exe",
        )),
        pkexec: Some("/usr/bin/pkexec".into()),
        apt_get: Some("/usr/bin/apt-get".into()),
        dnf: Some("/usr/bin/dnf".into()),
        snap: Some("/usr/bin/snap".into()),
        flatpak: Some("/usr/bin/flatpak".into()),
    }
}

fn argv_of(plan: &Plan) -> Vec<&str> {
    match &plan.method {
        Method::Command { argv, .. } => argv.iter().map(String::as_str).collect(),
        other => panic!("not a command: {other:?}"),
    }
}

#[test]
fn up_to_date_has_no_plan() {
    let mut f = finding(
        "vlc",
        "VLC",
        AppKind::App,
        vec![install("homebrew_cask", "vlc")],
    );
    f.update_available = false;
    assert_eq!(
        plan(&f, OsFamily::Macos, &all_tools()),
        Err(NoPlan::UpToDate)
    );
}

#[test]
fn homebrew_cask_wins_over_the_app_bundle() {
    let f = finding(
        "google-chrome",
        "Google Chrome",
        AppKind::App,
        vec![
            install("mac_app", "com.google.Chrome"),
            install("homebrew_cask", "google-chrome"),
        ],
    );
    let p = plan(&f, OsFamily::Macos, &all_tools()).unwrap();
    assert_eq!(
        argv_of(&p),
        [
            "/opt/homebrew/bin/brew",
            "upgrade",
            "--cask",
            "--greedy",
            "google-chrome"
        ]
    );
    assert!(p.one_click);
    assert_eq!(p.admin, Admin::No);
    assert_eq!(p.steps.len(), 4);
    assert_eq!(p.steps[0].label, "Download Google Chrome 1.1");
}

#[test]
fn mac_app_without_homebrew_uses_the_apps_own_updater() {
    let mut f = finding(
        "google-chrome",
        "Google Chrome",
        AppKind::App,
        vec![install("mac_app", "com.google.Chrome")],
    );
    f.browser = Some(Browser::Chrome);
    f.running = true;
    let p = plan(&f, OsFamily::Macos, &Tools::default()).unwrap();
    assert_eq!(
        p.method,
        Method::AppUpdater {
            open_bundle: Some("com.google.Chrome".into())
        }
    );
    assert!(!p.one_click);
    assert!(
        p.manual
            .unwrap()
            .starts_with("Wardwatch opens Google Chrome.")
    );
    assert!(p.restart_note.is_some());
}

#[test]
fn debian_packages_go_through_pkexec_apt_together() {
    let f = finding(
        "openssl-linux",
        "OpenSSL",
        AppKind::Package,
        vec![install("dpkg", "libssl3"), install("dpkg", "openssl")],
    );
    let p = plan(&f, OsFamily::Linux, &all_tools()).unwrap();
    let argv = argv_of(&p);
    assert_eq!(
        &argv[..5],
        [
            "/usr/bin/pkexec",
            "/usr/bin/apt-get",
            "install",
            "--only-upgrade",
            "--no-remove"
        ]
    );
    assert_eq!(&argv[argv.len() - 2..], ["libssl3", "openssl"]);
    assert_eq!(p.admin, Admin::Asks);
    assert!(p.steps[2].detail.contains("never sees it"));
}

#[test]
fn rpm_packages_go_through_pkexec_dnf() {
    let f = finding(
        "openssl-linux",
        "OpenSSL",
        AppKind::Package,
        vec![install("rpm", "openssl-libs")],
    );
    let p = plan(&f, OsFamily::Linux, &all_tools()).unwrap();
    assert_eq!(
        argv_of(&p),
        [
            "/usr/bin/pkexec",
            "/usr/bin/dnf",
            "upgrade",
            "-y",
            "--refresh",
            "openssl-libs"
        ]
    );
}

#[test]
fn no_pkexec_means_no_package_manager_plan() {
    let f = finding(
        "openssl-linux",
        "OpenSSL",
        AppKind::Package,
        vec![install("dpkg", "openssl")],
    );
    let tools = Tools {
        pkexec: None,
        ..all_tools()
    };
    let p = plan(&f, OsFamily::Linux, &tools).unwrap();
    assert!(matches!(p.method, Method::AppUpdater { open_bundle: None }));
    assert_eq!(p.by, "Software Updater");
    assert!(!p.one_click);
}

#[test]
fn snap_and_flatpak() {
    let f = finding(
        "mozilla-firefox",
        "Firefox",
        AppKind::App,
        vec![install("snap", "firefox")],
    );
    assert_eq!(
        argv_of(&plan(&f, OsFamily::Linux, &all_tools()).unwrap()),
        ["/usr/bin/snap", "refresh", "firefox"]
    );
    let f = finding(
        "vlc",
        "VLC",
        AppKind::App,
        vec![install("flatpak", "org.videolan.VLC")],
    );
    assert_eq!(
        argv_of(&plan(&f, OsFamily::Linux, &all_tools()).unwrap()),
        [
            "/usr/bin/flatpak",
            "update",
            "--noninteractive",
            "org.videolan.VLC"
        ]
    );
}

#[test]
fn windows_registry_maps_to_a_winget_id() {
    let f = finding(
        "google-chrome",
        "Google Chrome",
        AppKind::App,
        vec![install(
            "windows_registry",
            "{8A69D345-D564-463C-AFF1-A69D9E530F96}",
        )],
    );
    let p = plan(&f, OsFamily::Windows, &all_tools()).unwrap();
    let argv = argv_of(&p);
    assert_eq!(argv[1], "upgrade");
    assert_eq!(&argv[argv.len() - 2..], ["--id", "Google.Chrome"]);
    assert_eq!(p.admin, Admin::MayAsk);

    // An app winget doesn't know about falls back to instructions.
    let f = finding(
        "other",
        "Other",
        AppKind::App,
        vec![install("windows_registry", "x")],
    );
    assert!(!plan(&f, OsFamily::Windows, &all_tools()).unwrap().one_click);
}

#[test]
fn system_updates_open_the_os_update_screen() {
    let f = finding("macos", "macOS", AppKind::Os, vec![install("os", "macos")]);
    let p = plan(&f, OsFamily::Macos, &all_tools()).unwrap();
    assert!(
        matches!(&p.method, Method::SystemSettings { target } if target.starts_with("x-apple.systempreferences:"))
    );
    assert!(!p.one_click);
    let f = finding(
        "windows",
        "Windows",
        AppKind::Os,
        vec![install("os", "windows")],
    );
    let p = plan(&f, OsFamily::Windows, &all_tools()).unwrap();
    assert_eq!(
        p.method,
        Method::SystemSettings {
            target: "ms-settings:windowsupdate".into()
        }
    );
}

#[test]
fn unsafe_names_never_reach_a_command() {
    for bad in [
        "--config=/tmp/x",
        "a b",
        "a;rm",
        "",
        "$(x)",
        "a/b",
        "openssl-",
        "openssl+",
    ] {
        let f = finding(
            "vlc",
            "VLC",
            AppKind::App,
            vec![install("homebrew_cask", bad)],
        );
        assert!(
            matches!(
                plan(&f, OsFamily::Macos, &all_tools()),
                Err(NoPlan::Unsupported(_))
            ),
            "{bad:?}"
        );
    }
}

#[test]
fn plans_serialize_for_the_ui() {
    let f = finding(
        "vlc",
        "VLC",
        AppKind::App,
        vec![install("homebrew_cask", "vlc")],
    );
    let json = serde_json::to_value(plan(&f, OsFamily::Macos, &all_tools()).unwrap()).unwrap();
    assert_eq!(json["method"]["kind"], "command");
    assert_eq!(json["method"]["tool"], "homebrew");
    assert_eq!(json["admin"], "no");
    assert_eq!(
        json["steps"][3]["label"],
        "Check again that the hole is closed"
    );
}

#[test]
fn the_recheck_only_says_closed_on_evidence() {
    use wardwatch_feed::VulnHit;
    let vuln = |fixed: Option<&str>| VulnHit {
        id: "CVE-1".into(),
        aliases: vec![],
        published: "2026-11-01".into(),
        severity: Severity::High,
        cvss: None,
        exploited: false,
        kev: None,
        summary: String::new(),
        fixed: fixed.map(Into::into),
    };
    let mut f = finding(
        "vlc",
        "VLC",
        AppKind::App,
        vec![install("homebrew_cask", "vlc")],
    );
    assert_eq!(check(None), Check::NotFound);
    f.vulns = vec![];
    assert_eq!(check(Some(&f)), Check::Closed);
    f.vulns = vec![vuln(None)];
    assert_eq!(check(Some(&f)), Check::OnlyUnfixedLeft);
    f.vulns = vec![vuln(None), vuln(Some("2.0"))];
    assert_eq!(check(Some(&f)), Check::StillOpen);
}
