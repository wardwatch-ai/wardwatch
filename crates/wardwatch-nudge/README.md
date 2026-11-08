# wardwatch-nudge

Decides how hard Wardwatch nudges for each update, and when. It is step 4 of
the core loop: significant holes get more aggressive nudges by default, and
the user picks the rest on the Update plan screen.

```rust
let findings = wardwatch_feed::match_inventory(&inventory, &catalog);
let now = jiff::Zoned::now();
let plan = wardwatch_nudge::plan(&findings, &update_plan, &approvals, &history, &now);
// show plan.top_card and plan.nudges, send plan.notify, install plan.install,
history.record(&plan, now.timestamp());
// then call plan() again at plan.next_check, or when the inventory or catalog changes.
```

It is pure: no clock, files, network or OS notifications. The desktop app
stores the `UpdatePlan` and `History` and does the sending.

## Rules

| What | Look | When |
| --- | --- | --- |
| Being exploited right now | `Urgent` (amber act-now) | Same day, reminded every 4 hours until fixed. Locked: the grid can only make it Auto. |
| Exploited, in the OS or a browser, not approved yet | `Urgent` | Waits for the Sunday Tune-Up (see below) |
| Serious (critical or high) | `Quiet`, 3 bars | From the rules grid |
| Moderate (medium, or unknown severity) | `Quiet`, 2 bars | From the rules grid |
| Minor (low, or no security fix) | `Quiet`, 1 bar | From the rules grid |

The grid has a timing per severity row and per column (System, Browsers,
Other apps):

- **Same day**: one notification now, then once a day while it's open.
- **Sunday tune-up**: bundled into one "Sunday Tune-Up" notification at
  10:00 on Sunday. If the device is off then, it can still go out for the
  next 24 hours (outside quiet hours); after that the next Sunday covers it.
- **Monthly**: only in the first tune-up actually handled each month, so a
  missed first Sunday moves them to the next one.
- **Auto** (Install by itself): listed in `install` without a notification.
  The updater still asks before closing an open app, asks before big OS
  upgrades and waits for Wi-Fi and power; those can't be turned off.

**Quiet hours** (default 22:00 to 08:00, may wrap midnight) move every
notification and install to the end of quiet hours. Times are local and go
through jiff's time zone database, so DST changes are handled. History
times in the future (a clock that was ahead) count as never, so they can't
silence a nudge. **Holds**
(by `source` and `id` of any of the finding's installs) remove a program entirely.

Presets `Relaxed`, `Balanced` (default) and `Eager` are in `Preset::rules`;
`Preset::matching` says which preset button to light, if any.

## Top card

`TopCard::UnderAttack` when anything is exploited and not waiting for an
approval (the lighthouse beams on). That includes an exploited update the
grid installs by itself: the beams stay on until it is installed. Otherwise `TopCard::TuneUp` with every update the user
could do now (the "Update all 3" button), its highest meter, and the next
tune-up time; with no keys it is the all-clear card.

## Broad alerts and the approval queue

An exploited hole in the OS or a browser nudges every user of it at once.
Matt's rule (sprint 1, decision 9: hold) is that such an alert waits for his
approval in the alert approval queue (`alerts/` in wardwatch-ai/terraform),
and only the weekly tune-up goes out meanwhile. So these show the truth
(`Urgent`, `awaiting_approval: true`) but don't notify, don't take the top
card and wait for Sunday, unless an `ApprovedAlert` covers one of their
exploited vulnerabilities. An approved alert notifies with Matt's approved
title and body. If the user's grid says Auto, it installs as usual: that is
the user's own choice, not an alert.

Approvals arrive as `alerts.json` (`ApprovedAlerts::from_json`), which the
queue's dispatcher will publish next to `catalog.json`, signed the same way:

```jsonc
{
  "schema_version": 1,
  "expires_at": "2026-11-09T00:00:00Z",       // refuse the file after this
  "alerts": [{
    "id": "…",                                 // the queue's proposal id
    "app_id": "google-chrome",                 // catalog app id
    "vulns": ["CVE-2026-1234"],                // CVE or OSV ids (aliases match too)
    "title": "Update Chrome.", "body": "…",    // as approved, edits included
    "approved_at": "2026-11-08T12:00:00Z",
    "expires_at": "2026-11-08T18:00:00Z"
  }]
}
```

**Not enabled yet.** The queue is off, there is no dispatcher, and there is
no signature check, so the app must pass `ApprovedAlerts::none()` and every
broad alert waits for Sunday. To turn it on: enable the queue (terraform#4
steps), build the producer and dispatcher, sign `alerts.json` with the
catalog keys, and check that signature before `from_json`.

## Tests

```sh
cargo test -p wardwatch-nudge
```

`tests/engine.rs` runs the design spec's sample scenario (Chrome exploited;
Zoom, VLC and macOS waiting for Sunday; Final Cut Pro held) through
reminders, quiet hours, DST, tune-up windows, monthly and Auto.
