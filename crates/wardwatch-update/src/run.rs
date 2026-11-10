//! Runs a [`Plan`]: starts the package manager (or opens the OS's update
//! screen / the app) and reports progress through the plan's first three steps.
//! The fourth step, checking again, needs a rescan, so the caller does it.

use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::{Method, Plan, Tool};

/// Long enough for a big download on a slow line; after this we stop waiting.
const TIMEOUT: Duration = Duration::from_secs(30 * 60);
/// How much of the updater's output to keep for an error message.
const TAIL_LINES: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Working,
    Done,
}

/// Sent while a plan runs. `step` indexes [`Plan::steps`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Progress {
    Step { step: usize, state: StepState },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Outcome {
    /// The updater finished; the caller checks whether the hole is closed.
    Installed,
    /// We opened the OS's update screen or the app; the person finishes it.
    Opened,
    /// The person dismissed the OS's password prompt. Nothing changed.
    Cancelled,
    /// The updater says there's nothing newer for it to install yet.
    NothingNewer {
        tail: Vec<String>,
    },
    /// The upgrade needs other packages removed, which we never do. Nothing changed.
    NeedsMoreChanges {
        tail: Vec<String>,
    },
    Failed {
        message: String,
        tail: Vec<String>,
    },
}

/// Runs the plan, calling `progress` as steps start and finish. Blocks until
/// the updater exits (or [`TIMEOUT`]); call it off the UI thread.
pub fn run(plan: &Plan, mut progress: impl FnMut(Progress)) -> Outcome {
    match &plan.method {
        Method::SystemSettings { target } => open(&[target.as_str()]),
        Method::AppUpdater { open_bundle } => match open_bundle {
            Some(bundle) if cfg!(target_os = "macos") => open(&["-b", bundle.as_str()]),
            // Nothing we can open; the plan's instructions say what to do.
            _ => Outcome::Opened,
        },
        Method::Command { tool, argv } => run_command(*tool, argv, &mut progress),
    }
}

/// Opens a URL, settings page or app with the OS's own opener.
fn open(args: &[&str]) -> Outcome {
    let opener = if cfg!(target_os = "macos") {
        "/usr/bin/open".to_owned()
    } else if cfg!(windows) {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        format!(r"{root}\explorer.exe")
    } else {
        "/usr/bin/xdg-open".to_owned()
    };
    match Command::new(&opener)
        .args(args)
        .stdin(Stdio::null())
        .status()
    {
        // explorer.exe exits 1 even when it opened the page.
        Ok(s) if s.success() || cfg!(windows) => Outcome::Opened,
        Ok(s) => failed(format!("couldn't open it (exit {s})"), vec![]),
        Err(e) => failed(format!("couldn't open it: {e}"), vec![]),
    }
}

fn run_command(tool: Tool, argv: &[String], progress: &mut impl FnMut(Progress)) -> Outcome {
    let Some((bin, args)) = argv.split_first() else {
        return failed("empty command".into(), vec![]);
    };
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Plain, untranslated output so the messages we look for match.
        .env("LC_ALL", "C")
        .env("HOMEBREW_NO_ENV_HINTS", "1")
        .env("HOMEBREW_NO_EMOJI", "1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return failed(format!("couldn't start {}: {e}", tool.label()), vec![]),
    };

    let (tx, rx) = mpsc::channel::<String>();
    let readers: Vec<_> = [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    .map(|stream| {
        let tx = tx.clone();
        std::thread::spawn(move || read_lines(stream, &tx))
    })
    .collect();
    drop(tx);

    // The updaters don't report their phases reliably (some print "Upgrading"
    // before downloading), so the three steps they cover run together and
    // only finish when the updater says it succeeded.
    for step in 0..3 {
        progress(Progress::Step {
            step,
            state: StepState::Working,
        });
    }
    let mut tail: Vec<String> = Vec::new();
    let deadline = Instant::now() + TIMEOUT;
    loop {
        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(line) => {
                if tail.len() == TAIL_LINES {
                    tail.remove(0);
                }
                tail.push(line);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => return timed_out(tool, &mut child, tail),
        }
    }
    for r in readers {
        let _ = r.join();
    }
    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => return failed(format!("lost track of {}: {e}", tool.label()), tail),
    };
    if status.success() {
        // apt exits 0 when its package lists don't know the new version yet.
        if tool == Tool::Apt && says(&tail, &["is already the newest version"]) {
            return Outcome::NothingNewer { tail };
        }
        if tool == Tool::AppImageUpdate && says(&tail, &["update not required"]) {
            return Outcome::NothingNewer { tail };
        }
        for step in 0..3 {
            progress(Progress::Step {
                step,
                state: StepState::Done,
            });
        }
        return Outcome::Installed;
    }
    match (tool, status.code()) {
        // pkexec: 126 = the person dismissed the prompt, 127 = not authorized.
        (Tool::Apt | Tool::Dnf, Some(126)) => Outcome::Cancelled,
        (Tool::Apt | Tool::Dnf, Some(127)) => failed(
            "your system didn't allow the update. You may need an administrator".into(),
            tail,
        ),
        // `--no-remove`: the upgrade would have removed other packages.
        (Tool::Apt, _) if says(&tail, &["remove is disabled"]) => {
            Outcome::NeedsMoreChanges { tail }
        }
        // snapd and the Flatpak system helper ask through polkit themselves.
        (Tool::Snap | Tool::Flatpak, _)
            if says(
                &tail,
                &["access denied", "not authorized", "authorization failed"],
            ) =>
        {
            Outcome::Cancelled
        }
        // macOS's administrator prompt was dismissed: "User canceled. (-128)".
        (Tool::MacPorts, _) if says(&tail, &["(-128)"]) => Outcome::Cancelled,
        // The new file wasn't signed by the same key; AppImageUpdate put the old one back.
        (Tool::AppImageUpdate, _) if says(&tail, &["validation error"]) => failed(
            "the new version isn't signed by the same maker as your copy, so AppImageUpdate kept your copy".into(),
            tail,
        ),
        // APPINSTALLER_CLI_ERROR_UPDATE_NOT_APPLICABLE: winget has nothing newer yet.
        (Tool::Winget, Some(-1_978_335_189)) => Outcome::NothingNewer { tail },
        (_, code) => failed(
            format!(
                "{} stopped with an error{}",
                tool.label(),
                code.map(|c| format!(" (exit {c})")).unwrap_or_default()
            ),
            tail,
        ),
    }
}

/// After [`TIMEOUT`]: stop waiting, and say honestly what happened. An
/// updater running as root (through pkexec or macOS's prompt) can't be stopped by us, and an
/// updater's own child processes may outlive it, so the reader threads are
/// left to finish on their own rather than joined.
fn timed_out(tool: Tool, child: &mut std::process::Child, tail: Vec<String>) -> Outcome {
    let message = match tool {
        Tool::Apt | Tool::Dnf | Tool::MacPorts => format!(
            "{} is still running after 30 minutes. We stopped waiting; it will finish on its own",
            tool.label()
        ),
        _ => {
            let _ = child.kill();
            format!(
                "{} took longer than 30 minutes, so we asked it to stop",
                tool.label()
            )
        }
    };
    let _ = child.try_wait();
    failed(message, tail)
}

fn says(tail: &[String], phrases: &[&str]) -> bool {
    tail.iter().any(|l| {
        let l = l.to_ascii_lowercase();
        phrases.iter().any(|p| l.contains(p))
    })
}

fn failed(message: String, tail: Vec<String>) -> Outcome {
    Outcome::Failed { message, tail }
}

/// Splits output on `\n` and `\r` (progress bars redraw with `\r`).
fn read_lines(stream: impl Read, tx: &mpsc::Sender<String>) {
    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                for part in String::from_utf8_lossy(&buf).split('\r') {
                    let line = part.trim();
                    if !line.is_empty() && tx.send(line.to_owned()).is_err() {
                        return;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn plan_running(script: &str) -> Plan {
        Plan {
            method: Method::Command {
                tool: Tool::Apt,
                argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
            },
            by: "apt".into(),
            one_click: true,
            admin: crate::Admin::No,
            steps: vec![],
            verifies: String::new(),
            manual: None,
            restart_note: None,
        }
    }

    #[cfg(unix)]
    #[test]
    fn runs_reports_steps_in_order_and_succeeds() {
        let plan = plan_running(
            "echo 'Preparing to unpack x'; printf 'Unpacking x\\r'; echo 'Setting up x'",
        );
        let mut seen = vec![];
        assert_eq!(run(&plan, |p| seen.push(p)), Outcome::Installed);
        let steps: Vec<_> = seen
            .iter()
            .map(|Progress::Step { step, state }| (*step, *state))
            .collect();
        use StepState::*;
        assert_eq!(
            steps,
            [
                (0, Working),
                (1, Working),
                (2, Working),
                (0, Done),
                (1, Done),
                (2, Done)
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn pkexec_dismissal_is_a_cancel_and_errors_keep_the_tail() {
        assert_eq!(run(&plan_running("exit 126"), |_| {}), Outcome::Cancelled);
        match run(
            &plan_running("echo 'E: Unable to locate package x' >&2; exit 100"),
            |_| {},
        ) {
            Outcome::Failed { message, tail } => {
                assert!(message.contains("exit 100"), "{message}");
                assert_eq!(tail, ["E: Unable to locate package x"]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn apt_messages_map_to_honest_outcomes() {
        let stale =
            plan_running("echo 'openssl is already the newest version (3.0.13-0ubuntu3.4).'");
        assert!(matches!(run(&stale, |_| {}), Outcome::NothingNewer { .. }));
        let removes = plan_running(
            "echo 'E: Packages need to be removed but remove is disabled.' >&2; exit 100",
        );
        assert!(matches!(
            run(&removes, |_| {}),
            Outcome::NeedsMoreChanges { .. }
        ));
    }

    #[cfg(unix)]
    fn as_tool(tool: Tool, script: &str) -> Plan {
        let mut plan = plan_running(script);
        plan.method = Method::Command {
            tool,
            argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        };
        plan
    }

    #[cfg(unix)]
    #[test]
    fn macports_and_appimageupdate_messages_map_to_honest_outcomes() {
        let dismissed = as_tool(
            Tool::MacPorts,
            "echo '0:93: execution error: User canceled. (-128)' >&2; exit 1",
        );
        assert_eq!(run(&dismissed, |_| {}), Outcome::Cancelled);
        let current = as_tool(
            Tool::AppImageUpdate,
            "echo 'Checking for updates...'; echo '... done!'; echo 'Update not required, exiting.'",
        );
        assert!(matches!(
            run(&current, |_| {}),
            Outcome::NothingNewer { .. }
        ));
        let other_key = as_tool(
            Tool::AppImageUpdate,
            "echo 'Validation error: Key changed for signing AppImages' >&2; echo 'Restoring original file' >&2; exit 1",
        );
        match run(&other_key, |_| {}) {
            Outcome::Failed { message, .. } => assert!(message.contains("same maker"), "{message}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_missing_binary_fails_cleanly() {
        let plan = Plan {
            method: Method::Command {
                tool: Tool::Dnf,
                argv: vec!["/nonexistent/dnf".into()],
            },
            by: "dnf".into(),
            one_click: true,
            admin: crate::Admin::Asks,
            steps: vec![],
            verifies: String::new(),
            manual: None,
            restart_note: None,
        };
        assert!(matches!(run(&plan, |_| {}), Outcome::Failed { .. }));
    }
}
