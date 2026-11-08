//! Window labels and show/hide helpers. Labels match `tauri.conf.json`.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, Runtime, WebviewWindow};
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

/// When the popover loses focus because the tray icon itself was clicked,
/// the blur arrives before the click. Remembering when the blur hid the
/// popover lets that click close it instead of reopening it.
#[derive(Default)]
pub struct PopoverState {
    hidden_by_blur_at: Mutex<Option<Instant>>,
}

const BLUR_CLICK_WINDOW: Duration = Duration::from_millis(300);

pub fn hide_popover_on_blur<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    if let Ok(mut at) = window.state::<PopoverState>().hidden_by_blur_at.lock() {
        *at = Some(Instant::now());
    }
    window.hide()
}

pub fn toggle_tray_popover<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(TRAY) else {
        return Ok(());
    };
    if window.is_visible()? {
        return window.hide();
    }
    let just_hidden = app
        .state::<PopoverState>()
        .hidden_by_blur_at
        .lock()
        .ok()
        .and_then(|mut at| at.take())
        .is_some_and(|at| at.elapsed() < BLUR_CLICK_WINDOW);
    if just_hidden {
        return Ok(());
    }
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
