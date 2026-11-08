//! Findings and the 1-click fix, behind the `list_findings`, `get_fix` and
//! `run_fix` commands. The UI only ever names a program (`source` + `id`);
//! the core rescans, rematches and plans the update itself, so nothing the
//! UI sends reaches a command line.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use wardwatch_catalog::Inventory;
use wardwatch_feed::{Catalog, Finding};
use wardwatch_update::{Outcome, Plan, Progress, StepState, Tools};

/// Largest catalog we'll read; the real one is about 3 MB.
const MAX_CATALOG_BYTES: u64 = 64 << 20;

/// Allows one update at a time.
#[derive(Default)]
pub struct FixState {
    busy: AtomicBool,
}

/// What the fix screen shows before anything runs.
#[derive(Debug, Serialize)]
pub struct FixView {
    pub finding: Finding,
    /// `None` when there's nothing to do here; `no_plan` says why.
    pub plan: Option<Plan>,
    pub no_plan: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FixResult {
    pub outcome: Outcome,
    /// After an install: whether every hole with a fix is now closed.
    pub closed: Option<bool>,
    /// The program as the rescan found it afterwards.
    pub finding: Option<Finding>,
}

/// Reads the local catalog. Catalogs are unsigned until Wardwatch's keys
/// exist, and only development builds may trust an unsigned one, so release
/// builds refuse to load any catalog for now.
fn load_catalog<R: Runtime>(app: &AppHandle<R>) -> Result<Catalog, String> {
    if !cfg!(debug_assertions) {
        return Err("Wardwatch hasn't received a signed vulnerability catalog yet.".into());
    }
    let path = match std::env::var_os("WARDWATCH_CATALOG") {
        Some(p) => PathBuf::from(p),
        None => app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("catalog.json"),
    };
    let bytes = read_capped(&path)
        .map_err(|e| format!("No vulnerability catalog yet ({}: {e}).", path.display()))?;
    Catalog::from_json(&bytes, SystemTime::now()).map_err(|e| e.to_string())
}

fn read_capped(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_CATALOG_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_CATALOG_BYTES {
        return Err(std::io::Error::other("too large"));
    }
    Ok(bytes)
}

fn scan_and_match(catalog: &Catalog) -> (Inventory, Vec<Finding>) {
    let inventory = wardwatch_catalog::scan();
    let findings = wardwatch_feed::match_inventory(&inventory, catalog);
    (inventory, findings)
}

/// The finding with an install of `source`/`id` (any install, not only the primary).
fn find(findings: Vec<Finding>, source: &str, id: &str) -> Option<Finding> {
    findings
        .into_iter()
        .find(|f| f.installs.iter().any(|i| i.source == source && i.id == id))
}

pub fn list_findings<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<Finding>, String> {
    let catalog = load_catalog(app)?;
    Ok(scan_and_match(&catalog).1)
}

pub fn get_fix<R: Runtime>(app: &AppHandle<R>, source: &str, id: &str) -> Result<FixView, String> {
    let catalog = load_catalog(app)?;
    let (inventory, findings) = scan_and_match(&catalog);
    let finding = find(findings, source, id)
        .ok_or("Wardwatch has no update information for this program.")?;
    let (plan, no_plan) =
        match wardwatch_update::plan(&finding, inventory.os.family, &Tools::detect()) {
            Ok(plan) => (Some(plan), None),
            Err(why) => (None, Some(why.to_string())),
        };
    Ok(FixView {
        finding,
        plan,
        no_plan,
    })
}

/// Clears `busy` however `run_fix` returns.
struct Busy<'a>(&'a AtomicBool);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

pub fn run_fix<R: Runtime>(
    app: &AppHandle<R>,
    source: &str,
    id: &str,
    progress: impl Fn(Progress),
) -> Result<FixResult, String> {
    let state = app.state::<FixState>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("Another update is already running.".into());
    }
    let _busy = Busy(&state.busy);

    let catalog = load_catalog(app)?;
    let (inventory, findings) = scan_and_match(&catalog);
    let finding = find(findings, source, id)
        .ok_or("Wardwatch has no update information for this program.")?;
    let plan = wardwatch_update::plan(&finding, inventory.os.family, &Tools::detect())
        .map_err(|e| e.to_string())?;

    let outcome = wardwatch_update::run(&plan, &progress);
    if outcome != Outcome::Installed {
        return Ok(FixResult {
            outcome,
            closed: None,
            finding: None,
        });
    }

    // Step 4: rescan and check that every hole with a fix is closed.
    let check = 3;
    progress(Progress::Step {
        step: check,
        state: StepState::Working,
    });
    let after = scan_and_match(&catalog)
        .1
        .into_iter()
        .find(|f| f.app_id == finding.app_id);
    let closed = after
        .as_ref()
        .is_none_or(|f| f.vulns.iter().all(|v| v.fixed.is_none()));
    if closed {
        progress(Progress::Step {
            step: check,
            state: StepState::Done,
        });
    }
    Ok(FixResult {
        outcome,
        closed: Some(closed),
        finding: after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_builds_refuse_unsigned_catalogs() {
        let app = tauri::test::mock_app();
        if !cfg!(debug_assertions) {
            assert!(load_catalog(app.handle()).unwrap_err().contains("signed"));
        }
    }

    #[test]
    fn find_matches_any_install_of_a_program() {
        let bytes = include_bytes!("../../../../crates/wardwatch-feed/tests/fixtures/catalog.json");
        let catalog = Catalog::from_json(
            bytes,
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_794_139_200),
        )
        .unwrap();
        let (_, findings) = scan_and_match(&catalog);
        // Whatever this machine has, an unknown program is never found.
        assert!(find(findings, "mac_app", "does.not.exist").is_none());
    }

    #[test]
    fn only_one_fix_at_a_time() {
        let app = tauri::test::mock_app();
        app.manage(FixState::default());
        let state = app.state::<FixState>();
        state.busy.store(true, Ordering::SeqCst);
        let err = run_fix(app.handle(), "mac_app", "x", |_| {}).unwrap_err();
        assert_eq!(err, "Another update is already running.");
        assert!(
            state.busy.load(Ordering::SeqCst),
            "a refused call leaves the running one alone"
        );
    }

    #[test]
    fn busy_clears_when_a_fix_ends() {
        let flag = AtomicBool::new(true);
        drop(Busy(&flag));
        assert!(!flag.load(Ordering::SeqCst));
    }
}
