//! The design spec's sample scenario: Chrome is being exploited, Zoom, VLC
//! and macOS wait for Sunday, Final Cut Pro is held.

use jiff::{Timestamp, Zoned};
use wardwatch_nudge::*;

fn finding(key: &str, app_id: &str, kind: &str, sev: Option<&str>, exploited: bool) -> Finding {
    let (source, id) = key.split_once(':').unwrap();
    serde_json::from_value(serde_json::json!({
        "key": app_id, "app_id": app_id, "app_name": app_id, "kind": kind,
        "source": source, "software_id": id, "name": app_id,
        "browser": if app_id == "google-chrome" { Some("chrome") } else { None },
        "version_scheme": "dotted", "installed": "1.0", "running": false,
        "installs": [{"source": source, "id": id, "name": app_id, "version": "1.0", "running": false}],
        "fixed_version": "2.0", "latest": "2.0", "update_available": true,
        "exploited": exploited, "severity": sev,
        "vulns": [{"id": format!("CVE-{app_id}"), "aliases": [], "published": "2026-11-01",
                   "severity": sev.unwrap_or("low"), "cvss": null, "exploited": exploited,
                   "kev": null, "summary": "", "fixed": "2.0"}],
    }))
    .unwrap()
}

fn sample() -> Vec<Finding> {
    vec![
        finding(
            "mac_app:com.videolan.vlc",
            "vlc",
            "app",
            Some("medium"),
            false,
        ),
        finding("mac_app:us.zoom.xos", "zoom", "app", Some("medium"), false),
        finding("os:macos", "macos", "os", Some("medium"), false),
        finding(
            "mac_app:com.google.Chrome",
            "google-chrome",
            "app",
            Some("high"),
            true,
        ),
        finding(
            "mac_app:com.apple.FinalCut",
            "final-cut",
            "app",
            None,
            false,
        ),
    ]
}

fn held_plan() -> UpdatePlan {
    let mut p = UpdatePlan::default();
    p.holds.push(Hold {
        source: "mac_app".into(),
        id: "com.apple.FinalCut".into(),
        since: "2026-11-02".into(),
    });
    p
}

fn ny(s: &str) -> Zoned {
    format!("{s}[America/New_York]").parse().unwrap()
}

fn ts(s: &str) -> Timestamp {
    ny(s).timestamp()
}

fn chrome_approved() -> ApprovedAlerts {
    let mut a = ApprovedAlerts::none();
    a.alerts.push(ApprovedAlert {
        id: "p1".into(),
        app_id: "google-chrome".into(),
        vulns: vec!["CVE-google-chrome".into()],
        title: "Update Chrome now.".into(),
        body: "Approved copy.".into(),
        approved_at: ts("2026-11-11T09:00"),
        expires_at: ts("2026-11-12T09:00"),
    });
    a
}

const WEDNESDAY: &str = "2026-11-11T14:00";

#[test]
fn approved_exploited_browser_is_urgent_today() {
    let now = ny(WEDNESDAY);
    let p = plan(
        &sample(),
        &held_plan(),
        &chrome_approved(),
        &History::default(),
        &now,
    );

    let chrome = &p.nudges[0];
    assert_eq!(chrome.key, "google-chrome");
    assert_eq!(
        (chrome.strength, chrome.timing),
        (Strength::Urgent, Timing::SameDay)
    );
    assert!(!chrome.awaiting_approval);
    assert_eq!(
        p.top_card,
        TopCard::UnderAttack {
            keys: vec![chrome.key.clone()]
        }
    );

    // One notification, with Matt's approved copy.
    assert_eq!(p.notify.len(), 1);
    assert_eq!(p.notify[0].kind, NotificationKind::Urgent);
    assert_eq!(p.notify[0].title, "Update Chrome now.");
    assert_eq!(p.notify[0].alert_id.as_deref(), Some("p1"));

    // Held Final Cut gets nothing; the rest wait quietly for Sunday.
    assert!(p.nudges.iter().all(|n| n.app_id != "final-cut"));
    for n in &p.nudges[1..] {
        assert_eq!(
            (n.strength, n.timing, n.due_at),
            (Strength::Quiet { bars: 2 }, Timing::SundayTuneup, None)
        );
    }
    assert_eq!(p.nudges[1].category, Category::System);
}

#[test]
fn unapproved_broad_alert_waits_for_the_tune_up() {
    let now = ny(WEDNESDAY);
    let p = plan(
        &sample(),
        &held_plan(),
        &ApprovedAlerts::none(),
        &History::default(),
        &now,
    );

    let chrome = p
        .nudges
        .iter()
        .find(|n| n.app_id == "google-chrome")
        .unwrap();
    assert!(chrome.awaiting_approval);
    assert_eq!(
        (chrome.strength, chrome.timing),
        (Strength::Urgent, Timing::SundayTuneup)
    );
    assert!(p.notify.is_empty());
    match &p.top_card {
        TopCard::TuneUp { keys, bars, at } => {
            assert_eq!(keys.len(), 4);
            assert_eq!(*bars, 3);
            assert_eq!(*at, ts("2026-11-15T10:00"));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(p.next_check, ts("2026-11-15T10:00"));
}

#[test]
fn exploited_ordinary_app_needs_no_approval() {
    let f = vec![finding(
        "mac_app:us.zoom.xos",
        "zoom",
        "app",
        Some("low"),
        true,
    )];
    let p = plan(
        &f,
        &UpdatePlan::default(),
        &ApprovedAlerts::none(),
        &History::default(),
        &ny(WEDNESDAY),
    );
    assert_eq!(p.nudges[0].timing, Timing::SameDay);
    assert_eq!(p.notify[0].title, "Update zoom.");
    assert_eq!(p.notify[0].body, "This is being exploited right now.");
    assert!(matches!(p.top_card, TopCard::UnderAttack { .. }));
}

#[test]
fn urgent_reminders_repeat_every_four_hours_outside_quiet_hours() {
    let f = sample();
    let plan_ = held_plan();
    let approved = chrome_approved();
    let mut history = History::default();

    let first = plan(&f, &plan_, &approved, &history, &ny(WEDNESDAY));
    history.record(&first, ts(WEDNESDAY));
    assert_eq!(first.next_check, ts("2026-11-11T18:00"));

    let soon = plan(&f, &plan_, &approved, &history, &ny("2026-11-11T17:59"));
    assert!(soon.notify.is_empty());

    let later = plan(&f, &plan_, &approved, &history, &ny("2026-11-11T18:00"));
    assert_eq!(later.notify.len(), 1);
    history.record(&later, ts("2026-11-11T18:00"));

    // 22:00 is quiet: the next reminder waits for 08:00, when the approval
    // has still not expired (09:00).
    let night = plan(&f, &plan_, &approved, &history, &ny("2026-11-11T22:30"));
    assert!(night.notify.is_empty());
    assert_eq!(night.next_check, ts("2026-11-12T08:00"));
}

#[test]
fn same_day_serious_nudges_once_a_day() {
    let f = vec![finding("os:macos", "macos", "os", Some("critical"), false)];
    let mut history = History::default();
    let p = plan(
        &f,
        &UpdatePlan::default(),
        &ApprovedAlerts::none(),
        &history,
        &ny(WEDNESDAY),
    );
    assert_eq!(p.nudges[0].strength, Strength::Quiet { bars: 3 });
    assert_eq!(p.notify[0].kind, NotificationKind::SameDay);
    assert_eq!(p.notify[0].body, "It fixes a serious security hole.");
    history.record(&p, ts(WEDNESDAY));
    let again = plan(
        &f,
        &UpdatePlan::default(),
        &ApprovedAlerts::none(),
        &history,
        &ny("2026-11-12T13:59"),
    );
    assert!(again.notify.is_empty());
    assert_eq!(again.next_check, ts("2026-11-12T14:00"));
}

#[test]
fn sunday_tune_up_bundles_waiting_updates_once() {
    let f = sample();
    let mut history = History::default();
    let sunday = "2026-11-15T10:05";
    let p = plan(
        &f,
        &held_plan(),
        &ApprovedAlerts::none(),
        &history,
        &ny(sunday),
    );
    assert!(p.tune_up_due);
    assert_eq!(p.notify.len(), 1);
    assert_eq!(p.notify[0].kind, NotificationKind::TuneUp);
    assert_eq!(
        p.notify[0].keys.len(),
        4,
        "includes the held-back broad alert"
    );
    assert_eq!(p.notify[0].body, "4 updates are ready.");

    history.record(&p, ts(sunday));
    let again = plan(
        &f,
        &held_plan(),
        &ApprovedAlerts::none(),
        &history,
        &ny("2026-11-15T15:00"),
    );
    assert!(!again.tune_up_due && again.notify.is_empty());
    assert_eq!(again.next_check, ts("2026-11-22T10:00"));
}

#[test]
fn tune_up_respects_quiet_hours_and_lapses_after_a_day() {
    let f = sample();
    let mut plan_ = held_plan();
    plan_.quiet_hours = QuietHours {
        start: "21:00".parse().unwrap(),
        end: "11:00".parse().unwrap(),
    };
    let none = ApprovedAlerts::none();

    let early = plan(
        &f,
        &plan_,
        &none,
        &History::default(),
        &ny("2026-11-15T10:30"),
    );
    assert!(early.notify.is_empty());
    assert_eq!(early.next_check, ts("2026-11-15T11:00"));
    assert!(
        plan(
            &f,
            &plan_,
            &none,
            &History::default(),
            &ny("2026-11-15T11:00")
        )
        .tune_up_due
    );

    // Opened Monday afternoon after the window: wait for next Sunday.
    let missed = plan(
        &f,
        &plan_,
        &none,
        &History::default(),
        &ny("2026-11-16T11:00"),
    );
    assert!(!missed.tune_up_due);
    match missed.top_card {
        TopCard::TuneUp { at, .. } => assert_eq!(at, ts("2026-11-22T11:00")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn monthly_updates_join_only_the_first_tune_up_of_the_month() {
    let f = vec![finding(
        "mac_app:org.example",
        "example",
        "app",
        Some("low"),
        false,
    )];
    let none = ApprovedAlerts::none();
    let plan_ = UpdatePlan::default(); // Balanced: minor apps are monthly.
    let second = plan(
        &f,
        &plan_,
        &none,
        &History::default(),
        &ny("2026-11-08T10:00"),
    );
    assert!(second.tune_up_due && second.notify.is_empty());
    let first = plan(
        &f,
        &plan_,
        &none,
        &History::default(),
        &ny("2026-12-06T10:00"),
    );
    assert_eq!(first.notify.len(), 1);
}

#[test]
fn auto_installs_outside_quiet_hours_without_notifying() {
    let mut plan_ = held_plan();
    plan_.rules = Preset::Eager.rules();
    let f = sample();
    let none = ApprovedAlerts::none();

    let day = plan(&f, &plan_, &none, &History::default(), &ny(WEDNESDAY));
    let mut keys = day.install.clone();
    keys.sort();
    // Chrome (exploited, browsers auto), Zoom and VLC; macOS moderate is
    // auto too under Eager. Final Cut is held.
    assert_eq!(keys.len(), 4);
    assert!(day.notify.is_empty());
    assert!(day.nudges.iter().all(|n| !n.awaiting_approval));

    let night = plan(
        &f,
        &plan_,
        &none,
        &History::default(),
        &ny("2026-11-11T23:00"),
    );
    assert!(night.install.is_empty());
    assert_eq!(night.next_check, ts("2026-11-12T08:00"));
}

#[test]
fn up_to_date_device_shows_an_empty_tune_up() {
    let p = plan(
        &[],
        &UpdatePlan::default(),
        &ApprovedAlerts::none(),
        &History::default(),
        &ny(WEDNESDAY),
    );
    assert_eq!(
        p.top_card,
        TopCard::TuneUp {
            keys: vec![],
            bars: 0,
            at: ts("2026-11-15T10:00")
        }
    );
}

#[test]
fn plan_serializes_for_the_ui() {
    let p = plan(
        &sample(),
        &held_plan(),
        &chrome_approved(),
        &History::default(),
        &ny(WEDNESDAY),
    );
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["top_card"]["kind"], "under_attack");
    assert_eq!(v["nudges"][0]["strength"]["kind"], "urgent");
    assert_eq!(
        v["nudges"][1]["strength"],
        serde_json::json!({"kind": "quiet", "bars": 2})
    );
    assert_eq!(v["nudges"][0]["timing"], "same_day");
}
