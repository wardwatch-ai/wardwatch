fn main() {
    // Listing the app's commands makes Tauri generate one permission per
    // command, so each window can only call what its capability grants.
    let manifest = tauri_build::AppManifest::new().commands(&[
        "app_info",
        "show_main_window",
        "scan_inventory",
        "get_plan",
        "set_plan",
        "list_findings",
        "get_fix",
        "run_fix",
        "get_nudges",
        "get_settings",
        "set_settings",
    ]);
    stamp_build();

    let mut attributes = tauri_build::Attributes::new().app_manifest(manifest);

    // Tauri normally embeds the Windows app manifest only into the app binary,
    // so `cargo test` executables fail to start (STATUS_ENTRYPOINT_NOT_FOUND)
    // without Common Controls v6. Embed it via the linker for every target
    // instead, as the tauri-build docs for `new_without_app_manifest` describe.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", path.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", path.display());
    }

    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}

/// Stamps the release channel, commit and build date into the binary, for the
/// Settings page. The release workflow sets all three; a local build is "dev".
fn stamp_build() {
    for var in [
        "WARDWATCH_BUILD_CHANNEL",
        "WARDWATCH_BUILD_COMMIT",
        "WARDWATCH_BUILD_DATE",
    ] {
        println!("cargo:rerun-if-env-changed={var}");
    }
    let channel = std::env::var("WARDWATCH_BUILD_CHANNEL").unwrap_or_else(|_| "dev".into());
    assert!(
        matches!(channel.as_str(), "mainline" | "nightly" | "dev"),
        "WARDWATCH_BUILD_CHANNEL must be mainline, nightly or dev, not {channel:?}"
    );
    let commit = std::env::var("WARDWATCH_BUILD_COMMIT").unwrap_or_default();
    assert!(
        commit.len() <= 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()),
        "WARDWATCH_BUILD_COMMIT must be a git commit id"
    );
    let date = std::env::var("WARDWATCH_BUILD_DATE").unwrap_or_default();
    let b = date.as_bytes();
    assert!(
        date.is_empty()
            || (b.len() == 10
                && b[4] == b'-'
                && b[7] == b'-'
                && b.iter()
                    .enumerate()
                    .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())),
        "WARDWATCH_BUILD_DATE must be YYYY-MM-DD"
    );
    println!("cargo:rustc-env=WARDWATCH_BUILD_CHANNEL={channel}");
    println!(
        "cargo:rustc-env=WARDWATCH_BUILD_COMMIT={}",
        &commit[..commit.len().min(12)]
    );
    println!("cargo:rustc-env=WARDWATCH_BUILD_DATE={date}");
}
