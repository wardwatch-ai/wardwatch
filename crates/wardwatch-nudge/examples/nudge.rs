//! Scans this device, matches it against a catalog and prints the nudge plan
//! with the default Update plan, no approved broad alerts and no history:
//! what a fresh install would show and send right now.
//!
//! cargo run -p wardwatch-nudge --example nudge -- <catalog.json>
//!
//! The fixture-device tests (`tests/devices/`) run this inside containers.

use std::time::SystemTime;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: nudge <catalog.json>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path).expect("read catalog");
    let catalog =
        wardwatch_feed::Catalog::from_json(&bytes, SystemTime::now()).expect("valid catalog");
    let findings = wardwatch_feed::match_inventory(&wardwatch_catalog::scan(), &catalog);
    let plan = wardwatch_nudge::plan(
        &findings,
        &wardwatch_nudge::UpdatePlan::default(),
        &wardwatch_nudge::ApprovedAlerts::none(),
        &wardwatch_nudge::History::default(),
        &jiff::Zoned::now(),
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&plan).expect("plan serializes")
    );
}
