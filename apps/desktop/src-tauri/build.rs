fn main() {
    // Listing the app's commands makes Tauri generate one permission per
    // command, so each window can only call what its capability grants.
    let manifest = tauri_build::AppManifest::new().commands(&[
        "app_info",
        "show_main_window",
        "scan_inventory",
        "get_plan",
        "set_plan",
    ]);
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
