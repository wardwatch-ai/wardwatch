# wardwatch-update

Plans and runs one update with the device's own updater. It's the "1-click
fix" behind the desktop app's fix screen. Wardwatch never downloads or
installs anything itself, so the vendor's or distro's signature checks always
apply.

```rust
let plan = wardwatch_update::plan(&finding, inventory.os.family, &Tools::detect())?; // nothing runs
let outcome = wardwatch_update::run(&plan, |progress| { /* step N working / done */ });
// then rescan and rematch to check the hole is closed (step 4)
```

## How each program is updated

First match wins:

| Installed as | Update with | Asks for a password? |
| --- | --- | --- |
| Homebrew cask | `brew upgrade --cask --greedy <token>` | No |
| Homebrew formula | `brew upgrade <formula>` | No |
| Snap | `snap refresh <name>` | snapd shows the system prompt |
| Flatpak | `flatpak update -y --noninteractive <id>` | System installs show the system prompt |
| Debian/Ubuntu package | `pkexec apt-get install --only-upgrade -y <every binary package from that source>` | Yes, the system's own prompt |
| RPM package | `pkexec dnf upgrade -y --refresh <packages>` | Yes, the system's own prompt |
| Windows program winget knows | `winget upgrade --exact --source winget --silent --id <id>` | The installer may show Windows' prompt |
| macOS or Windows itself | Opens Software Update / Windows Update | The OS handles it |
| A distro package with no `pkexec` | Points to the desktop's Software Updater | — |
| Anything else (a Mac app outside Homebrew) | Opens the app and says where its updater is | — |

`--greedy` is there because Homebrew skips casks that update themselves
(Chrome, Firefox) otherwise.

## Trust rules

- **Say it before doing it.** `plan` returns the steps, the exact command
  line, what checks the download, and whether the OS will ask for a password.
  The fix screen shows all of it before the button.
- **Never escalate silently.** Commands run as the signed-in user. apt and dnf
  go through `pkexec`, so the OS shows its own prompt; Wardwatch never sees
  the password. Dismissing the prompt reports "nothing changed".
- **Nothing from the UI reaches a command line.** The core rescans, rematches
  and plans from its own data. Updaters are found at fixed absolute paths
  (never `PATH`), there's no shell, and every package name must be letters,
  digits and `._+-@`, not starting with `-`.
- **Never close an open app.** If it's running, the plan says to restart it.
- One update at a time, and each stops after 30 minutes.

## Not yet

- winget ids are a small built-in table keyed by catalog app id
  (`WINGET_IDS`). They belong in the catalog.
- Mac apps outside Homebrew (Chrome's own installer, App Store apps) are
  guided, not 1-click: Chrome's and Firefox's updaters have no supported
  command-line trigger.
- An activity log of every update Wardwatch ran.
- macOS `softwareupdate` and Windows Update APIs: we open the OS's own screen.

## Try it

```sh
cargo test -p wardwatch-update
# Plan (and with --run, do) a real update against a local catalog:
cargo run -p wardwatch-update --example fix -- catalog.json google-chrome [--run]
```

The example was run on Ubuntu 24.04 against a catalog marking `pcre2`
`10.42-4ubuntu2.1` as vulnerable: apt upgraded all five binary packages to
`10.42-4ubuntu2.2`, progress went through steps 1 to 3, and the rescan
reported the hole closed.
