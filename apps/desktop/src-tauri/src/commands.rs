//! IPC commands the UI can call. The TypeScript side is `src/lib/ipc.ts`.
//! Each command needs an entry in `build.rs` and a grant in `capabilities/`.

use serde::Serialize;
use tauri::{AppHandle, Runtime};

use tauri::ipc::Channel;
use wardwatch_feed::Finding;
use wardwatch_update::{Method, Progress};

use crate::fix::{self, FixResult, FixView};
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

// Scans and updates block, so they run off the async runtime's threads.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

/// This device's catalog apps, most urgent first.
#[tauri::command]
pub async fn list_findings<R: Runtime>(app: AppHandle<R>) -> Result<Vec<Finding>, String> {
    blocking(move || fix::list_findings(&app)).await
}

/// One program's finding and the update plan, before anything runs.
#[tauri::command]
pub async fn get_fix<R: Runtime>(
    app: AppHandle<R>,
    source: String,
    id: String,
) -> Result<FixView, String> {
    blocking(move || fix::get_fix(&app, &source, &id)).await
}

/// Runs the update for one program, streaming step progress to `on_progress`.
/// `shown` is the plan method the screen showed; a different plan won't run.
#[tauri::command]
pub async fn run_fix<R: Runtime>(
    app: AppHandle<R>,
    source: String,
    id: String,
    shown: Method,
    on_progress: Channel<Progress>,
) -> Result<FixResult, String> {
    blocking(move || {
        fix::run_fix(&app, &source, &id, &shown, |p| {
            // The window may have closed; the update carries on regardless.
            let _ = on_progress.send(p);
        })
    })
    .await
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
