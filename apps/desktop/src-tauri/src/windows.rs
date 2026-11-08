//! Window labels and show/hide helpers. Labels match `tauri.conf.json`.

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_positioner::{Position, WindowExt};

pub const MAIN: &str = "main";
pub const TRAY: &str = "tray";

/// macOS puts the menu bar at the top of the screen; Windows and most Linux
/// desktops put the tray at the bottom, so the popover opens upward there.
#[cfg(target_os = "macos")]
const POPOVER_POSITION: Position = Position::TrayCenter;
#[cfg(not(target_os = "macos"))]
const POPOVER_POSITION: Position = Position::TrayBottomCenter;

pub fn show_main<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(MAIN) else {
        return Ok(());
    };
    window.unminimize()?;
    window.show()?;
    window.set_focus()
}

pub fn hide_tray_popover<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    match app.get_webview_window(TRAY) {
        Some(window) => window.hide(),
        None => Ok(()),
    }
}

pub fn show_tray_popover<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(TRAY) else {
        return Ok(());
    };
    window.move_window(POPOVER_POSITION)?;
    window.show()?;
    window.set_focus()
}

/// Window calls fail only in rare platform states; they should never crash the app.
pub fn log_err(what: &str, result: tauri::Result<()>) {
    if let Err(err) = result {
        eprintln!("wardwatch: could not {what}: {err}");
    }
}
