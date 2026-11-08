//! Runs the nudge engine (crates/wardwatch-nudge) for Home and the tray.

use tauri::{AppHandle, Runtime};
use wardwatch_catalog::Inventory;
use wardwatch_nudge::{ApprovedAlerts, History, NudgePlan};

use crate::fix;
use crate::plan::UpdatePlan;

/// The engine's view of the saved plan. Both sides use the same JSON shape,
/// so the stored form is converted through it rather than field by field.
fn engine_plan(stored: &UpdatePlan) -> Result<wardwatch_nudge::UpdatePlan, String> {
    let json = serde_json::to_vec(stored).map_err(|e| e.to_string())?;
    wardwatch_nudge::UpdatePlan::from_json(&json)
}

pub fn plan_for<R: Runtime>(
    app: &AppHandle<R>,
    inventory: &Inventory,
    stored: &UpdatePlan,
) -> Result<NudgePlan, String> {
    let catalog = fix::load_catalog(app)?;
    let findings = wardwatch_feed::match_inventory(inventory, &catalog);
    // Broad OS and browser alerts stay held until signed alerts.json exists.
    // History is empty because nothing sends notifications yet.
    Ok(wardwatch_nudge::plan(
        &findings,
        &engine_plan(stored)?,
        &ApprovedAlerts::none(),
        &History::default(),
        &jiff::Zoned::now(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
