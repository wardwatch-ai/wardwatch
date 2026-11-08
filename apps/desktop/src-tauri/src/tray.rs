//! The menu bar (macOS) / system tray (Windows, Linux) icon.
//!
//! Left click toggles the popover. Linux tray hosts don't report clicks, so the
//! menu is the way in there.

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Runtime};

use crate::windows;

const MENU_OPEN: &str = "open";
const MENU_QUIT: &str = "quit";

// macOS tints template images to match the menu bar. Elsewhere we use the
// color icon drawn for dark taskbars, the default on Windows 11 and GNOME.
#[cfg(target_os = "macos")]
const ICON: Image<'static> = tauri::include_image!("icons/tray/tray-clear-template-32.png");
#[cfg(not(target_os = "macos"))]
const ICON: Image<'static> = tauri::include_image!("icons/tray/tray-clear-dark-32.png");

pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, MENU_OPEN, "Open Wardwatch", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Wardwatch", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    TrayIconBuilder::with_id("wardwatch")
        .icon(ICON)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Wardwatch")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_OPEN => windows::log_err("show main window", windows::show_main(app)),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Lets the positioner place the popover next to the icon.
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                windows::log_err(
                    "toggle tray popover",
                    windows::toggle_tray_popover(tray.app_handle()),
                );
            }
        })
        .build(app)?;
    Ok(())
}
