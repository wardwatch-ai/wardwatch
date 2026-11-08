//! Scans this device and prints the inventory.
//!
//! `cargo run --release --example scan` prints the full JSON.
//! `cargo run --release --example scan -- --summary` prints only counts and timing.

use std::collections::BTreeMap;
use std::time::Instant;

fn main() {
    let start = Instant::now();
    let inv = wardwatch_catalog::scan();
    let elapsed = start.elapsed();

    if std::env::args().any(|a| a == "--summary") {
        let mut by_source: BTreeMap<String, usize> = BTreeMap::new();
        for s in &inv.software {
            *by_source.entry(format!("{:?}", s.source)).or_default() += 1;
        }
        println!("os: {} {} ({})", inv.os.name, inv.os.version, inv.os.arch);
        println!("software: {} {by_source:?}", inv.software.len());
        let browsers: Vec<String> = inv
            .software
            .iter()
            .filter_map(|s| {
                Some(format!(
                    "{:?} {}",
                    s.browser?,
                    s.version.as_deref().unwrap_or("?")
                ))
            })
            .collect();
        println!("browsers: {browsers:?}");
        println!(
            "running: {}",
            inv.software.iter().filter(|s| s.running).count()
        );
        println!("warnings: {:?}", inv.warnings);
        println!("scan took {} ms", elapsed.as_millis());
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&inv).expect("inventory serializes")
        );
    }
}
