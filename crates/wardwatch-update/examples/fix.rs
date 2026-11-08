//! Plans (and with `--run`, runs) the update for one catalog app on this
//! device, then rescans to check the hole is closed: the same path the app's
//! fix screen takes, from a terminal.
//!
//! cargo run -p wardwatch-update --example fix -- <catalog.json> <app-id> [--run]

use std::time::SystemTime;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(path), Some(app_id)) = (args.first(), args.get(1)) else {
        eprintln!("usage: fix <catalog.json> <app-id> [--run]");
        std::process::exit(2);
    };
    let bytes = std::fs::read(path).expect("read catalog");
    let catalog =
        wardwatch_feed::Catalog::from_json(&bytes, SystemTime::now()).expect("valid catalog");
    let find = || {
        let inventory = wardwatch_catalog::scan();
        let finding = wardwatch_feed::match_inventory(&inventory, &catalog)
            .into_iter()
            .find(|f| f.app_id == *app_id);
        (inventory, finding)
    };
    let (inventory, Some(finding)) = find() else {
        eprintln!("{app_id} isn't installed here (or isn't in the catalog)");
        std::process::exit(1);
    };
    println!(
        "{} {} -> {:?}, {} hole(s), exploited: {}",
        finding.app_name,
        finding.installed,
        finding.fixed_version,
        finding.vulns.len(),
        finding.exploited
    );
    let plan = match wardwatch_update::plan(
        &finding,
        inventory.os.family,
        &wardwatch_update::Tools::detect(),
    ) {
        Ok(p) => p,
        Err(e) => {
            println!("no plan: {e}");
            return;
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&plan).expect("serialize")
    );
    if !args.iter().any(|a| a == "--run") {
        return;
    }
    let outcome = wardwatch_update::run(&plan, |p| println!("progress: {p:?}"));
    println!("outcome: {outcome:?}");
    if outcome == wardwatch_update::Outcome::Installed {
        let after = find().1;
        let closed = after
            .as_ref()
            .is_none_or(|f| f.vulns.iter().all(|v| v.fixed.is_none()));
        println!(
            "after: {:?}, hole closed: {closed}",
            after.map(|f| f.installed)
        );
    }
}
