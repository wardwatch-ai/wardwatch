//! Wardwatch desktop shell: the main window, the menu bar / tray popover,
//! and the IPC commands the UI calls.
//!
//! Scanning, the update feed and scoring live in their own crates under
//! `crates/` and are exposed to the UI only through `commands`.

mod commands;
mod tray;
mod windows;

use tauri::WindowEvent;

pub fn run() {
    let app = tauri::Builder::default()
        // A second launch focuses the running copy instead of starting another.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::log_err("show main window", windows::show_main(app));
        }))
        .plugin(tauri_plugin_positioner::init())
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::show_main_window
        ])
        .setup(|app| {
            tray::create(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            // Closing the main window keeps Wardwatch watching from the tray.
            // Quit lives in the tray menu.
            (windows::MAIN, WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                windows::log_err("hide main window", window.hide());
            }
            // The popover behaves like a menu: it goes away when you click elsewhere.
            (windows::TRAY, WindowEvent::Focused(false)) => {
                windows::log_err("hide tray popover", window.hide());
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("failed to build the Wardwatch app");

    app.run(|app, event| {
        // macOS: clicking the Dock icon brings the main window back.
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            windows::log_err("show main window", windows::show_main(app));
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (app, event);
    });
}
