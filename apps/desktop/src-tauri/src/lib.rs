//! Wardwatch desktop shell: the main window, the menu bar / tray popover,
//! and the IPC commands the UI calls.
//!
//! Scanning, the update feed and scoring live in their own crates under
//! `crates/` and are exposed to the UI only through `commands`.

mod commands;
mod plan;
mod tray;
mod windows;

use tauri::{Manager, WindowEvent};

pub fn run() {
    let app = tauri::Builder::default()
        // A second launch focuses the running copy instead of starting another.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::log_err("show main window", windows::show_main(app));
        }))
        .plugin(tauri_plugin_positioner::init())
        .manage(windows::PopoverState::default())
        .manage(commands::InventoryCache::default())
        .manage(commands::PlanLock::default())
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::show_main_window,
            commands::scan_inventory,
            commands::get_plan,
            commands::set_plan
        ])
        .setup(|app| {
            tray::create(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            // On macOS and Windows, closing the main window keeps Wardwatch in the
            // menu bar / tray, where Quit lives. Many Linux desktops (stock GNOME)
            // show no tray at all, so there closing the window quits the app.
            #[cfg(not(target_os = "linux"))]
            (windows::MAIN, WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                windows::log_err("hide main window", window.hide());
            }
            // The hidden popover window would keep the app alive, so exit explicitly.
            #[cfg(target_os = "linux")]
            (windows::MAIN, WindowEvent::CloseRequested { .. }) => {
                window.app_handle().exit(0);
            }
            // The popover behaves like a menu: it goes away when you click elsewhere.
            (windows::TRAY, WindowEvent::Focused(false)) => {
                if let Some(popover) = window.app_handle().get_webview_window(windows::TRAY) {
                    windows::log_err("hide tray popover", windows::hide_popover_on_blur(&popover));
                }
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
