//! Runs the nudge engine (crates/wardwatch-nudge): read-only for Home and the
//! tray, and a background loop that sends the plan's system notifications.
//!
//! The loop re-plans at the plan's `next_check`, or shortly after the Update
//! plan is saved. It checks the clock at least hourly, so a laptop that slept
//! catches up within the hour after waking (tokio's timer pauses during sleep).
//! Quiet hours, the Sunday Tune-Up at 10:00 and holds are all the engine's;
//! this module only sends what it says and remembers that it did.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use jiff::{Timestamp, Zoned};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Notify;
use wardwatch_nudge::{
    ApprovedAlerts, Finding, History, Notification, NotificationKind, NudgePlan,
};

use crate::plan::UpdatePlan;
use crate::{auto, commands, fix, store};

const HISTORY_VERSION: u64 = 1;
const MIN_WAIT: Duration = Duration::from_secs(60);
const MAX_WAIT: Duration = Duration::from_secs(60 * 60);
/// After the plan is saved, wait this long so a run of toggles re-plans once.
const SETTLE: Duration = Duration::from_secs(5);

/// The loop's wake-up call and the lock on the history file.
#[derive(Default)]
pub struct NudgeState {
    wake: Notify,
    history: Mutex<()>,
}

impl NudgeState {
    /// The guard protects no data, so a panic elsewhere mustn't lock it for good.
    fn lock_history(&self) -> MutexGuard<'_, ()> {
        self.history.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Re-plans now, for example after the Update plan changed.
    pub fn wake(&self) {
        self.wake.notify_one();
    }
}

/// The engine's view of the saved plan. Both sides use the same JSON shape,
/// so the stored form is converted through it rather than field by field.
fn engine_plan(stored: &UpdatePlan) -> Result<wardwatch_nudge::UpdatePlan, String> {
    let json = serde_json::to_vec(stored).map_err(|e| e.to_string())?;
    wardwatch_nudge::UpdatePlan::from_json(&json)
}

pub fn plan_for(
    findings: &[Finding],
    stored: &UpdatePlan,
    history: &History,
    now: &Zoned,
) -> Result<NudgePlan, String> {
    // Broad OS and browser alerts stay held until signed alerts.json exists.
    Ok(wardwatch_nudge::plan(
        findings,
        &engine_plan(stored)?,
        &ApprovedAlerts::none(),
        history,
        now,
    ))
}

/// One pass: plan, send each notification, record what went out. A
/// notification that fails to show isn't recorded, so a pass within the hour
/// retries it. Returns the plan, whose `next_check` says when to look again.
pub fn step(
    findings: &[Finding],
    stored: &UpdatePlan,
    history: &mut History,
    now: &Zoned,
    mut send: impl FnMut(&Notification) -> Result<(), String>,
) -> Result<NudgePlan, String> {
    let mut plan = plan_for(findings, stored, history, now)?;
    let before = history.tune_up;
    let mut failed = Vec::new();
    plan.notify.retain(|n| match send(n) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("wardwatch: could not show a notification: {e}");
            failed.push(n.kind);
            false
        }
    });
    history.record(&plan, now.timestamp());
    if failed.contains(&NotificationKind::TuneUp) {
        history.tune_up = before;
    }
    if !failed.is_empty() {
        // Try the failed ones again within the hour. The plan still goes
        // back, so Auto doesn't depend on notifications being allowed.
        plan.next_check = plan
            .next_check
            .min(now.timestamp() + jiff::SignedDuration::from_hours(1));
    }
    Ok(plan)
}

#[derive(Serialize, Deserialize)]
struct StoredHistory {
    v: u64,
    #[serde(flatten)]
    history: History,
}

fn history_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("nudge-history.json"))
        .map_err(|e| format!("no config folder: {e}"))
}

/// A missing or unreadable file means nothing was sent yet. A file from a
/// newer Wardwatch is an error: it can't be read or overwritten, and treating
/// it as empty would re-send everything on every pass.
fn load_history(path: &Path) -> Result<History, String> {
    let Ok(bytes) = std::fs::read(path) else {
        return Ok(History::default());
    };
    match serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|v| v.get("v").and_then(serde_json::Value::as_u64))
    {
        Some(v) if v > HISTORY_VERSION => {
            Err("the nudge history was saved by a newer version of Wardwatch".into())
        }
        _ => Ok(serde_json::from_slice::<StoredHistory>(&bytes)
            .map(|s| s.history)
            .unwrap_or_else(|_| {
                eprintln!("wardwatch: starting a fresh nudge history");
                History::default()
            })),
    }
}

fn save_history(path: &Path, history: History) -> Result<(), String> {
    let stored = StoredHistory {
        v: HISTORY_VERSION,
        history,
    };
    store::save(path, &stored, HISTORY_VERSION, "nudge history")
}

/// The nudge plan as of now, for Home and the tray. Sends nothing.
pub fn current<R: Runtime>(app: &AppHandle<R>) -> Result<NudgePlan, String> {
    let stored = commands::load_plan(app)?;
    let findings = fix::list_findings(app)?;
    let history = {
        let state = app.state::<NudgeState>();
        let _guard = state.lock_history();
        load_history(&history_path(app)?)?
    };
    plan_for(&findings, &stored, &history, &Zoned::now())
}

fn run_once<R: Runtime>(app: &AppHandle<R>) -> Result<Timestamp, String> {
    let stored = commands::load_plan(app)?;
    let findings = fix::list_findings(app)?;
    let state = app.state::<NudgeState>();
    let path = history_path(app)?;
    let mut history = {
        let _guard = state.lock_history();
        let history = load_history(&path)?;
        // Make sure what we send can be recorded; otherwise every pass would
        // send it again.
        save_history(&path, history.clone())?;
        history
    };
    // Only this loop writes the history, so sending needs no lock.
    let result = step(&findings, &stored, &mut history, &Zoned::now(), |n| {
        let (what, value) = click_target(n);
        app.notification()
            .builder()
            .title(&n.title)
            .body(&n.body)
            .extra(what, value)
            .show()
            .map_err(|e| e.to_string())
    });
    {
        let _guard = state.lock_history();
        save_history(&path, history)?;
    }
    let plan = result?;
    // Auto: what the Update plan installs by itself (outside quiet hours,
    // never held), under the fix screen's rules. See `auto`.
    auto::run(app, &plan.install, &findings, |title, body, key| {
        if let Err(e) = app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .extra(KEY, key)
            .show()
        {
            eprintln!("wardwatch: could not show a notification: {e}");
        }
    })?;
    Ok(plan.next_check)
}

/// The `extra` keys that say what a notification is about: one program's
/// catalog key, or a fixed screen for a batch.
const KEY: &str = "key";
const ROUTE: &str = "route";

/// What a click on `n` should open. A single program is named by its stable
/// catalog key and looked up at click time, since its install id can change
/// once it's updated (Windows registry ids carry the version).
fn click_target(n: &Notification) -> (&'static str, String) {
    match n.keys.as_slice() {
        [key] if n.kind != NotificationKind::TuneUp => (KEY, key.clone()),
        _ => (ROUTE, "/fix/tune-up".into()),
    }
}

/// The fix screen for the program `key` as it's installed now, or Home if
/// it's gone. Routes use the same encoding as the UI's `fixPath`.
fn route_for_key(key: &str, findings: &[Finding]) -> String {
    findings
        .iter()
        .find(|f| f.key == key)
        .map(|f| fix_route(&f.source, &f.software_id))
        .filter(|r| commands::is_allowed_route(r))
        .unwrap_or_else(|| "/".into())
}

/// The fix screen for one program.
pub fn fix_route(source: &str, id: &str) -> String {
    format!("/fix/{}/{}", path_part(source), path_part(id))
}

/// Percent-encodes everything but letters, digits and `-._~`.
fn path_part(s: &str) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// Opens the notification's screen when it's clicked. Registered at launch,
/// before anything is sent, so the plugin reports clicks. What's reported
/// depends on the OS: Windows reports a click only while the toast is on
/// screen, not from the Action Center.
pub fn on_click<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();
    // Never fails on desktop.
    let _ = app.notification().on_action(move |action| {
        if action.action_id() != "tap" {
            return;
        }
        let extra = action.notification().map(|n| n.extra());
        let get = |k: &str| {
            extra
                .and_then(|e| e.get(k))
                .and_then(serde_json::Value::as_str)
        };
        // This runs on the plugin's own thread, so a rescan here is fine.
        let route = match (get(KEY), get(ROUTE)) {
            (Some(key), _) => fix::list_findings(&handle)
                .map(|findings| route_for_key(key, &findings))
                .unwrap_or_else(|_| "/".into()),
            (None, Some(route)) => route.to_owned(),
            (None, None) => "/".into(),
        };
        // open_screen re-checks the route against the allowed screens; if it
        // refuses, still open Wardwatch.
        if commands::open_screen(&handle, Some(&route)).is_err()
            && let Err(e) = commands::open_screen(&handle, Some("/"))
        {
            eprintln!("wardwatch: could not open the notification's screen: {e}");
        }
    });
}

fn wait_until(next: Timestamp) -> Duration {
    let ms = next.duration_since(Timestamp::now()).as_millis();
    Duration::from_millis(u64::try_from(ms).unwrap_or(0)).clamp(MIN_WAIT, MAX_WAIT)
}

/// Starts the background loop. Without a catalog yet (every release build for
/// now) each pass fails quietly and the loop tries again within the hour.
pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        loop {
            let handle = app.clone();
            let next = match tauri::async_runtime::spawn_blocking(move || run_once(&handle)).await {
                Ok(Ok(next)) => next,
                Ok(Err(e)) => {
                    eprintln!("wardwatch: nudges not checked: {e}");
                    Timestamp::now() + jiff::SignedDuration::from_hours(1)
                }
                Err(e) => {
                    eprintln!("wardwatch: nudge check stopped: {e}");
                    Timestamp::now() + jiff::SignedDuration::from_hours(1)
                }
            };
            // Sleep until `next`, checking the clock at least hourly; a saved
            // plan cuts it short.
            let state = app.state::<NudgeState>();
            while Timestamp::now() < next {
                tokio::select! {
                    () = tokio::time::sleep(wait_until(next)) => {}
                    () = state.wake.notified() => {
                        tokio::time::sleep(SETTLE).await;
                        break;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exploited_app() -> Finding {
        serde_json::from_value(serde_json::json!({
            "key": "zoom", "app_id": "zoom", "app_name": "Zoom", "kind": "app",
            "source": "mac_app", "software_id": "us.zoom.xos", "name": "Zoom",
            "browser": null, "version_scheme": "dotted", "installed": "1.0", "running": false,
            "installs": [{"source": "mac_app", "id": "us.zoom.xos", "name": "Zoom",
                          "version": "1.0", "running": false}],
            "fixed_version": "2.0", "latest": "2.0", "update_available": true,
            "exploited": true, "severity": "high",
            "vulns": [{"id": "CVE-1", "aliases": [], "published": "2026-11-01",
                       "severity": "high", "cvss": null, "exploited": true,
                       "kev": null, "summary": "", "fixed": "2.0"}],
        }))
        .unwrap()
    }

    fn at(s: &str) -> Zoned {
        format!("{s}[America/New_York]").parse().unwrap()
    }

    #[test]
    fn the_saved_plan_reads_as_the_engines_plan() {
        let mut stored = UpdatePlan::default();
        stored.holds.push(crate::plan::Hold {
            source: "mac_app".into(),
            id: "com.apple.FinalCut".into(),
            since: "2026-11-02".into(),
        });
        let engine = engine_plan(&stored).unwrap();
        assert_eq!(
            wardwatch_nudge::Preset::matching(&engine.rules),
            Some(wardwatch_nudge::Preset::Balanced)
        );
        assert!(engine.is_held("mac_app", "com.apple.FinalCut"));
    }

    #[test]
    fn sends_once_then_remembers_it() {
        let findings = [exploited_app()];
        let plan = UpdatePlan::default();
        let mut history = History::default();
        let mut sent = Vec::new();
        // Tuesday 14:00, outside quiet hours.
        step(
            &findings,
            &plan,
            &mut history,
            &at("2026-11-10T14:00"),
            |n| {
                sent.push(n.title.clone());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(sent.len(), 1);
        step(
            &findings,
            &plan,
            &mut history,
            &at("2026-11-10T14:05"),
            |n| {
                sent.push(n.title.clone());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(sent.len(), 1, "no repeat within the reminder interval");
    }

    #[test]
    fn quiet_hours_and_holds_send_nothing() {
        let findings = [exploited_app()];
        let mut history = History::default();
        let mut sent = 0;
        step(
            &findings,
            &UpdatePlan::default(),
            &mut history,
            &at("2026-11-10T23:30"),
            |_| {
                sent += 1;
                Ok(())
            },
        )
        .unwrap();
        let mut held = UpdatePlan::default();
        held.holds.push(crate::plan::Hold {
            source: "mac_app".into(),
            id: "us.zoom.xos".into(),
            since: "2026-11-02".into(),
        });
        step(
            &findings,
            &held,
            &mut history,
            &at("2026-11-10T14:00"),
            |_| {
                sent += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(sent, 0);
    }

    #[test]
    fn a_failed_notification_is_retried_next_pass() {
        let findings = [exploited_app()];
        let plan = UpdatePlan::default();
        let mut history = History::default();
        let now = at("2026-11-10T14:00");
        let first = step(&findings, &plan, &mut history, &now, |_| {
            Err("no daemon".into())
        })
        .unwrap();
        assert!(history.notified.is_empty());
        assert!(first.next_check <= now.timestamp() + jiff::SignedDuration::from_hours(1));
        let mut sent = 0;
        step(
            &findings,
            &plan,
            &mut history,
            &at("2026-11-10T14:05"),
            |_| {
                sent += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(sent, 1);
    }

    #[test]
    fn history_round_trips_and_bad_files_start_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nudge-history.json");
        assert_eq!(load_history(&path).unwrap(), History::default());
        let mut history = History::default();
        history
            .notified
            .insert("zoom".into(), "2026-11-10T19:00:00Z".parse().unwrap());
        save_history(&path, history.clone()).unwrap();
        assert_eq!(load_history(&path).unwrap(), history);
        std::fs::write(&path, b"not json").unwrap();
        assert_eq!(load_history(&path).unwrap(), History::default());
        // A newer file can't be read or replaced, so nothing gets sent.
        std::fs::write(&path, br#"{"v":2}"#).unwrap();
        assert!(load_history(&path).is_err());
    }

    fn notification(kind: NotificationKind, keys: &[&str]) -> Notification {
        Notification {
            kind,
            keys: keys.iter().map(|k| (*k).to_string()).collect(),
            title: String::new(),
            body: String::new(),
            alert_id: None,
        }
    }

    #[test]
    fn clicks_open_the_programs_fix_screen_or_the_tune_up() {
        let mut windows_app = exploited_app();
        windows_app.key = "firefox".into();
        windows_app.source = "windows_registry".into();
        windows_app.software_id = "Mozilla Firefox 145.0.1 (x64 en-US)".into();
        let findings = [exploited_app(), windows_app];
        assert_eq!(
            click_target(&notification(NotificationKind::Urgent, &["zoom"])),
            (KEY, "zoom".to_owned())
        );
        let batch = notification(NotificationKind::SameDay, &["zoom", "firefox"]);
        assert_eq!(click_target(&batch), (ROUTE, "/fix/tune-up".to_owned()));
        let tune_up = notification(NotificationKind::TuneUp, &["zoom"]);
        assert_eq!(click_target(&tune_up), (ROUTE, "/fix/tune-up".to_owned()));

        assert_eq!(route_for_key("zoom", &findings), "/fix/mac_app/us.zoom.xos");
        assert_eq!(
            route_for_key("firefox", &findings),
            "/fix/windows_registry/Mozilla%20Firefox%20145.0.1%20%28x64%20en-US%29"
        );
        assert_eq!(route_for_key("gone", &findings), "/");
        // A route the allow-list would refuse opens Home instead.
        let mut huge = exploited_app();
        huge.key = "huge".into();
        huge.software_id = "x".repeat(600);
        assert_eq!(route_for_key("huge", &[huge]), "/");
    }

    #[test]
    fn waits_between_a_minute_and_an_hour() {
        assert_eq!(wait_until(Timestamp::now()), MIN_WAIT);
        let far = Timestamp::now() + jiff::SignedDuration::from_hours(30);
        assert_eq!(wait_until(far), MAX_WAIT);
    }
}
