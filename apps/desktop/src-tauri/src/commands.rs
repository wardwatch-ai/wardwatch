//! IPC commands the UI can call. The TypeScript side is `src/lib/ipc.ts`.
//! Each command needs an entry in `build.rs` and a grant in `capabilities/`.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use wardwatch_catalog::Inventory;
use wardwatch_feed::Finding;
use wardwatch_nudge::NudgePlan;
use wardwatch_update::{Method, Progress};

use crate::fix::{self, FixResult, FixView};
use crate::nudge;
use crate::plan::{self, UpdatePlan};
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

/// Event the main window listens for to switch screens.
pub const NAVIGATE_EVENT: &str = "navigate";

/// Routes the tray may open. Anything else is refused, so the tray (or anything
/// that gets to call this) can't steer the main window somewhere unexpected.
fn is_allowed_route(route: &str) -> bool {
    const TABS: &[&str] = &["/", "/score", "/groups", "/apps", "/plan", "/fix/tune-up"];
    if TABS.contains(&route) {
        return true;
    }
    // /fix/<source>/<id>, each part URL-encoded by the UI.
    let Some(rest) = route.strip_prefix("/fix/") else {
        return false;
    };
    let mut parts = rest.split('/');
    let ok = |p: Option<&str>| {
        p.is_some_and(|p| {
            !p.is_empty()
                && p != "."
                && p != ".."
                && p.len() <= 512
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"%-._~".contains(&b))
        })
    };
    ok(parts.next()) && ok(parts.next()) && parts.next().is_none()
}

#[tauri::command]
pub fn show_main_window<R: Runtime>(
    app: AppHandle<R>,
    route: Option<String>,
) -> Result<(), String> {
    if let Some(route) = &route
        && !is_allowed_route(route)
    {
        return Err(format!("not a Wardwatch screen: {route}"));
    }
    windows::hide_tray_popover(&app).map_err(|e| e.to_string())?;
    windows::show_main(&app).map_err(|e| e.to_string())?;
    if let Some(route) = route {
        app.emit_to(windows::MAIN, NAVIGATE_EVENT, route)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The last scan, shared by the main window and the tray. An async lock held for
/// the whole scan means a second window opening mid-scan waits for that scan
/// instead of starting its own.
#[derive(Default)]
pub struct InventoryCache(tauri::async_runtime::Mutex<Option<(Instant, Arc<Inventory>)>>);

/// A scan reads package databases and app bundles; this keeps windows from
/// repeating it every time they open. `refresh` forces a new one.
const INVENTORY_MAX_AGE: Duration = Duration::from_secs(10 * 60);

#[tauri::command]
pub async fn scan_inventory(
    cache: State<'_, InventoryCache>,
    refresh: Option<bool>,
) -> Result<Arc<Inventory>, String> {
    cached_inventory(&cache, refresh.unwrap_or(false)).await
}

async fn cached_inventory(cache: &InventoryCache, refresh: bool) -> Result<Arc<Inventory>, String> {
    let mut last = cache.0.lock().await;
    if !refresh
        && let Some((at, inv)) = last.as_ref()
        && at.elapsed() < INVENTORY_MAX_AGE
    {
        return Ok(Arc::clone(inv));
    }
    // The scan blocks on file reads and, on Linux, `rpm`; keep it off the IPC thread.
    let inventory = Arc::new(
        tauri::async_runtime::spawn_blocking(wardwatch_catalog::scan)
            .await
            .map_err(|e| format!("scan failed: {e}"))?,
    );
    *last = Some((Instant::now(), Arc::clone(&inventory)));
    Ok(inventory)
}

fn plan_path<R: Runtime>(app: &AppHandle<R>) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| plan::path_in(&dir))
        .map_err(|e| format!("no config folder: {e}"))
}

/// Serializes plan writes from both windows.
#[derive(Default)]
pub struct PlanLock(Mutex<()>);

#[tauri::command]
pub fn get_plan<R: Runtime>(
    app: AppHandle<R>,
    lock: State<'_, PlanLock>,
) -> Result<UpdatePlan, String> {
    let _guard = lock.0.lock().map_err(|e| e.to_string())?;
    Ok(plan::load(&plan_path(&app)?))
}

#[tauri::command]
pub fn set_plan<R: Runtime>(
    app: AppHandle<R>,
    lock: State<'_, PlanLock>,
    plan: UpdatePlan,
) -> Result<(), String> {
    let _guard = lock.0.lock().map_err(|e| e.to_string())?;
    plan::save(&plan_path(&app)?, &plan)
}

/// How hard and when to nudge for each update on this device, from the
/// cached scan, the catalog and the saved Update plan. Read-only: nothing is
/// sent from here, so nudge history isn't recorded yet.
#[tauri::command]
pub async fn get_nudges<R: Runtime>(
    app: AppHandle<R>,
    cache: State<'_, InventoryCache>,
    lock: State<'_, PlanLock>,
) -> Result<NudgePlan, String> {
    let stored = {
        let _guard = lock.0.lock().map_err(|e| e.to_string())?;
        plan::load(&plan_path(&app)?)
    };
    let inventory = cached_inventory(&cache, false).await?;
    blocking(move || nudge::plan_for(&app, &inventory, &stored)).await
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
        assert_eq!(show_main_window(app.handle().clone(), None), Ok(()));
        assert_eq!(
            show_main_window(app.handle().clone(), Some("/score".into())),
            Ok(())
        );
    }

    #[test]
    fn show_main_window_refuses_unknown_routes() {
        let app = tauri::test::mock_app();
        assert!(
            show_main_window(app.handle().clone(), Some("https://example.com".into())).is_err()
        );
    }

    #[test]
    fn allowed_routes() {
        for ok in [
            "/",
            "/plan",
            "/fix/tune-up",
            "/fix/mac_app/com.google.Chrome",
            "/fix/os/macos",
            "/fix/dpkg/libssl3%3Aamd64",
            "/fix/windows_registry/Firefox%20145.0.1%20%28x64%20en-US%29",
        ] {
            assert!(is_allowed_route(ok), "{ok}");
        }
        for bad in [
            "",
            "score",
            "/fix/",
            "/fix/a",
            "/fix/a/b/c",
            "/fix/a/b?x=1",
            "/fix/../b",
            "//evil",
            "/fix/a/<b>",
        ] {
            assert!(!is_allowed_route(bad), "{bad}");
        }
    }

    #[test]
    fn scan_inventory_caches_the_last_scan() {
        let app = tauri::test::mock_app();
        app.manage(InventoryCache::default());
        let first = tauri::async_runtime::block_on(scan_inventory(app.state(), None)).unwrap();
        let second = tauri::async_runtime::block_on(scan_inventory(app.state(), None)).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        let fresh =
            tauri::async_runtime::block_on(scan_inventory(app.state(), Some(true))).unwrap();
        assert!(!Arc::ptr_eq(&first, &fresh));
    }
}
