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
        // Plain, untranslated output so progress matching works.
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

    progress(Progress::Step {
        step: 0,
        state: StepState::Working,
    });
    let mut at = 0;
    let mut tail: Vec<String> = Vec::new();
    let deadline = Instant::now() + TIMEOUT;
    let mut timed_out = false;
    loop {
        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(line) => {
                if let Some(step) = step_for(tool, &line).filter(|&s| s > at) {
                    for done in at..step {
                        progress(Progress::Step {
                            step: done,
                            state: StepState::Done,
                        });
                    }
                    at = step;
                    progress(Progress::Step {
                        step,
                        state: StepState::Working,
                    });
                }
                if tail.len() == TAIL_LINES {
                    tail.remove(0);
                }
                tail.push(line);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                timed_out = true;
                let _ = child.kill();
                break;
            }
        }
    }
    for r in readers {
        let _ = r.join();
    }
    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => return failed(format!("lost track of {}: {e}", tool.label()), tail),
    };
    if timed_out {
        return failed(
            format!(
                "{} took longer than 30 minutes, so we stopped it",
                tool.label()
            ),
            tail,
        );
    }
    if status.success() {
        for step in at..3 {
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
            "your system didn't allow the update. You may need an administrator.".into(),
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

/// Which plan step an output line shows the updater has reached:
/// 0 = downloading, 1 = checking, 2 = installing.
fn step_for(tool: Tool, line: &str) -> Option<usize> {
    let l = line.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| l.contains(w));
    let (check, install): (&[&str], &[&str]) = match tool {
        Tool::Homebrew => (
            &["verifying", "sha256"],
            &[
                "==> upgrading",
                "==> installing",
                "==> moving",
                "==> purging",
            ],
        ),
        Tool::Winget => (
            &["successfully verified installer hash"],
            &["starting package install"],
        ),
        Tool::Apt => (&["preparing to unpack"], &["unpacking", "setting up"]),
        Tool::Dnf => (
            &["running transaction check", "verifying"],
            &["upgrading", "installing", "running transaction"],
        ),
        Tool::Snap => (
            &["check snap", "verify snap"],
            &["mount snap", "copy snap", "refreshed"],
        ),
        Tool::Flatpak => (
            &["verifying"],
            &["installing", "updating", "changes complete"],
        ),
    };
    if has(install) {
        Some(2)
    } else if has(check) {
        Some(1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_lines_advance_steps() {
        assert_eq!(
            step_for(Tool::Winget, "Successfully verified installer hash"),
            Some(1)
        );
        assert_eq!(
            step_for(Tool::Winget, "Starting package install..."),
            Some(2)
        );
        assert_eq!(
            step_for(Tool::Homebrew, "==> Upgrading 1 outdated package:"),
            Some(2)
        );
        assert_eq!(
            step_for(
                Tool::Apt,
                "Unpacking openssl (3.0.22-1) over (3.0.20-1) ..."
            ),
            Some(2)
        );
        assert_eq!(
            step_for(Tool::Apt, "Get:1 http://deb.debian.org openssl"),
            None
        );
    }

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
                (0, Done),
                (1, Working),
                (1, Done),
                (2, Working),
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
