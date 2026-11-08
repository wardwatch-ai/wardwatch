fn main() {
    // Listing the app's commands makes Tauri generate one permission per
    // command, so each window can only call what its capability grants.
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&["app_info", "show_main_window"]),
        ),
    )
    .expect("failed to run tauri-build");
}
