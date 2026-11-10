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
| Flatpak | `flatpak update --noninteractive <id>` | System installs show the system prompt |
| MacPorts port | `osascript` with a fixed script that runs `port -N sync && port -N upgrade <ports>` `with administrator privileges` | Yes, macOS's own prompt |
| AppImage that its maker signed, with update information, in the home folder, when AppImageUpdate is installed | `appimageupdatetool --remove-old <file>` | No |
| Debian/Ubuntu package | `pkexec apt-get install --only-upgrade --no-remove -y -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold <every binary package from that source>` | Yes, the system's own prompt |
| RPM package | `pkexec dnf upgrade -y --refresh <packages>` | Yes, the system's own prompt |
| Windows program winget knows | `winget upgrade --exact --source winget --silent --disable-interactivity --accept-source-agreements --accept-package-agreements --id <id>` | The installer may show Windows' prompt |
| macOS or Windows itself | Opens Software Update / Windows Update | The OS handles it |
| A distro package with no `pkexec` | Points to the desktop's Software Updater | — |
| Anything else (a Mac app outside Homebrew, an unsigned AppImage) | Opens the app and says where its updater is | — |

`--greedy` is there because Homebrew skips casks that update themselves
(Chrome, Firefox) otherwise.

**MacPorts** runs as root, and a desktop app has no terminal for `sudo`, so
the update goes through macOS's administrator prompt (`do shell script ...
with administrator privileges`). That prompt takes a shell command, so the
AppleScript is a constant (`MACPORTS_SCRIPT`) and port names arrive as
`osascript` arguments, each quoted with AppleScript's `quoted form of`, after
the usual name check. `port sync` first refreshes MacPorts' list of ports
(MacPorts checks its signature), so `upgrade` sees the fixed version. It never
runs `selfupdate`, which would also upgrade MacPorts itself.

**AppImages** have no package manager. AppImageUpdate reads the update
information the maker embedded (`.upd_info`, usually their GitHub releases)
and downloads the new version with zsync. That source is only as trustworthy
as the maker's server, so Wardwatch offers it only for AppImages whose maker
signed them (`.sha256_sig` and `.sig_key` filled in): AppImageUpdate then
keeps the new file only if the same key signed it, and otherwise deletes it
and puts the old one back. Unsigned AppImages (most of them, including
AppImageUpdate's own) get instructions instead. `--remove-old` deletes the old
copy once the new one passes, since it still has the hole; the new file lands
in the same folder. Only AppImages in the home folder qualify (system folders
need root), and only an `appimageupdatetool` in `/usr/bin`, `/usr/local/bin`
or the person's `~/.local/bin` or `~/Applications` that nobody else can write
to is used. Wardwatch never downloads AppImageUpdate itself.

## Trust rules

- **Say it before doing it.** `plan` returns the steps, the exact command
  line, what checks the download, and whether the OS will ask for a password.
  The fix screen shows all of it before the button.
- **Never escalate silently.** Commands run as the signed-in user. apt and dnf
  go through `pkexec`, so the OS shows its own prompt; Wardwatch never sees
  the password. Dismissing the prompt reports "nothing changed".
- **Nothing from the UI reaches a command line.** The core rescans, rematches
  and plans from its own data. Updaters are found at fixed absolute paths
  (never `PATH`), there's no shell (except the fixed MacPorts script inside
  macOS's admin prompt, see above), and every package name must be letters,
  digits and `._+-@`, not starting with `-`.
- **Never close an open app.** If it's running, the plan says to restart it.
- **Never remove anything.** apt runs with `--no-remove`; an upgrade that
  needs removals stops and points to Software Updater. The one exception is
  an updated AppImage's old file, removed only after the new one passes.
- **Only run what was shown.** The UI sends back the plan method it showed,
  and the core refuses to run a fresh plan that differs.
- **Only call it fixed on evidence.** After an update the core rescans:
  `closed`, `only_unfixed_left`, `still_open`, or `not_found` (never read as fixed).
- One update at a time. After 30 minutes we stop waiting and say so; an
  updater running as root (apt, dnf, MacPorts) can't be stopped by us and finishes on its own.

## Not yet

- winget ids are a small built-in table keyed by catalog app id
  (`WINGET_IDS`). They belong in the catalog.
- Store, Chocolatey and Scoop installs get instructions, not 1-click, yet.
- MacPorts and AppImageUpdate are covered by unit tests with stand-in
  commands; the MacPorts path still needs a run on a real Mac.
- Mac apps outside Homebrew (Chrome's own installer, App Store apps) are
  guided, not 1-click: Chrome's and Firefox's updaters have no supported
  command-line trigger.
- An activity log of every update Wardwatch ran.
- macOS `softwareupdate` and Windows Update APIs: we open the OS's own screen.
- apt uses the package lists as last refreshed (Ubuntu refreshes them daily);
  if they're stale we say so instead of asking for a second password.
- Per-phase progress: updaters don't report phases reliably, so download,
  check and install show as one working state until the updater succeeds.

## Try it

```sh
cargo test -p wardwatch-update
# Plan (and with --run, do) a real update against a local catalog:
cargo run -p wardwatch-update --example fix -- catalog.json google-chrome [--run]
```

The example was run on Ubuntu 24.04 against a catalog marking `pcre2`
`10.42-4ubuntu2.1` as vulnerable: apt upgraded all five binary packages to
`10.42-4ubuntu2.2`, progress went through steps 1 to 3, and the rescan
reported the hole closed. It was rerun after review against `sqlite3` with
`--no-remove`.
