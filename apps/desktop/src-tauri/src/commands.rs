//! IPC commands the UI can call. The TypeScript side is `src/lib/ipc.ts`.
//! Each command needs an entry in `build.rs` and a grant in `capabilities/`.

use serde::Serialize;
use tauri::{AppHandle, Runtime};

use crate::windows;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub os: &'static str,
}

#[tauri::command]
pub fn app_info<R: Runtime>(app: AppHandle<R>) -> AppInfo {
    let package = app.package_info();
    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        os: std::env::consts::OS,
    }
}

#[tauri::command]
pub fn show_main_window<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    windows::hide_tray_popover(&app).map_err(|e| e.to_string())?;
    windows::show_main(&app).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_reports_package_and_os() {
        let app = tauri::test::mock_app();
        let info = app_info(app.handle().clone());
        assert_eq!(info.version, app.package_info().version.to_string());
        assert_eq!(info.os, std::env::consts::OS);
    }

    #[test]
    fn show_main_window_without_windows_is_ok() {
        let app = tauri::test::mock_app();
        assert_eq!(show_main_window(app.handle().clone()), Ok(()));
    }
}
