//! Scans this device and prints what a catalog says needs updating, as the
//! JSON array of findings the app works from.
//!
//! cargo run -p wardwatch-feed --example check -- <catalog.json>
//!
//! The fixture-device tests (`tests/devices/`) run this inside containers.

use std::time::SystemTime;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: check <catalog.json>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("can't read {path}: {e}");
        std::process::exit(2);
    });
    let catalog =
        wardwatch_feed::Catalog::from_json(&bytes, SystemTime::now()).unwrap_or_else(|e| {
            eprintln!("bad catalog: {e}");
            std::process::exit(2);
        });
    let findings = wardwatch_feed::match_inventory(&wardwatch_catalog::scan(), &catalog);
    println!(
        "{}",
        serde_json::to_string_pretty(&findings).expect("findings serialize")
    );
}
