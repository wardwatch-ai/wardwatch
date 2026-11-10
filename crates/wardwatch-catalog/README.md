# wardwatch-catalog

Finds what is installed on this device: the OS version, browsers, and
installed and running software, on macOS, Windows and Linux. It is step 1 of
Wardwatch's core loop; the app compares this inventory against the signed
update and vulnerability catalog, on the device.

```rust
let inventory = wardwatch_catalog::scan();          // never fails; problems go in inventory.warnings
let json = serde_json::to_string(&inventory)?;      // stable, versioned shape (inventory.schema)
let watch = wardwatch_catalog::watch_paths();       // what to watch (recursively) to rescan on change
```

## What it reads

| Platform | OS version | Software |
| --- | --- | --- |
| macOS | `SystemVersion.plist` (Rapid Security Response aware) | `Info.plist` of apps in `/Applications` and `~/Applications` (one folder deep), Homebrew Cellar and Caskroom, MacPorts (`port -q installed active`) |
| Windows | `HKLM\…\Windows NT\CurrentVersion` (build.UBR, 23H2) | Uninstall registry keys (64-bit, 32-bit, current user), filtered like Settings > Installed apps, with winget portable packages under their winget id; Microsoft Store packages from the AppModel repository in `HKCU`; Chocolatey `.nuspec` files; Scoop `manifest.json` files (user and global) |
| Linux | `/etc/os-release` | dpkg status file, `rpm -qa`, snaps in `/snap` or `/var/lib/snapd/snap`, `flatpak list` (runtimes too, by branch), Linuxbrew, AppImages (below) |

AppImages have no database. Files whose header says they are AppImages are
found in `~/Applications`, `~/AppImages`, `~/.local/bin`, `~/bin`,
`~/Desktop`, `~/Downloads`, `/opt` and `/usr/local/bin` (not recursively),
and wherever a menu entry in `~/.local/share/applications` runs one
(AppImageLauncher and appimaged write these, with `X-AppImage-Version`).
Otherwise the name and version come from the file name
(`Obsidian-1.7.7.AppImage`). Images are never mounted or run.

Services and browser extensions get their own lists:

| | Read from | Left out |
| --- | --- | --- |
| `services` | systemd unit files, the machine's (`systemd`) and the current user's own (`systemd_user`, from `~/.config/systemd/user`, `/etc/systemd/user`, `/usr/lib/systemd/user` and the rest), with running units from `/proc/<pid>/cgroup`; launchd plists in `/Library` and `~/Library`; `HKLM\SYSTEM\CurrentControlSet\Services` | Linux units that are neither enabled nor running; Apple's jobs in `/System`; Windows drivers, per-user service templates, and services under the Windows folder |
| `extensions` | Chromium-based browsers' `Extensions/<id>/<version>/manifest.json` and `Preferences` (enabled state) in every profile; Firefox `extensions.json`; Safari `.appex` bundles inside Mac apps | Themes, dictionaries, and add-ons that ship with the browser (Chrome apps such as Docs Offline are listed as extensions) |

Each service is linked to the software entry whose install folder holds its
program (or a distro package named like the program). Store packages drop
what Settings hides: frameworks, resource packs, and inbox apps under the
Windows folder. A Chocolatey package that ran an installer also has its
registry entry; both are kept.

Browsers (Chrome, Chromium, Edge, Firefox, Safari, Brave, Opera, Vivaldi, Arc)
are tagged on whichever entry installed them. Ubuntu's stub `firefox` deb is
not tagged; its snap is.

`running` is set when a running process's executable lives inside the app's
install folder (or, for distro packages and ports, is named after the
package; for an AppImage, is the file itself). Only the executable path of
each process is read. Flatpak apps run sandboxed, so `flatpak ps` says which
are running instead (apps only, not runtimes).

Each entry's `source` says which `id` namespace it uses: `mac_app` (bundle ID),
`homebrew_formula`, `homebrew_cask`, `windows_registry` (`name` is
DisplayName), `dpkg`, `rpm`, `snap`, `flatpak`, `ms_store` (package name),
`chocolatey`, `scoop`, `macports` (port name; version is `version_revision`),
`appimage` (app name, lowercased), `winget` (winget package id). dpkg and rpm versions keep
their epoch and revision because distro advisories are written against them.

## Privacy and cost

- Nothing leaves the device from this crate. Install paths are kept in memory
  only to match running processes and are never serialized, so usernames in
  paths can't leak into anything built from the inventory. The same goes for
  service programs and browser profile folders. Warnings contain no paths.
- Package databases and plists are read in-process. Only `rpm`, `flatpak`
  and MacPorts' `port` are run, by absolute path, because they have no
  stable on-disk format. An AppImage is opened only to read its first 11
  bytes.
- One pass, no threads, no polling. On an Ubuntu machine with 837 packages a
  release-build scan takes about 4 ms and 13 MB peak memory; 124 rpm packages
  in Fedora 41 take about 70 ms (mostly `rpm` itself).

## Not yet

Marking Flatpak runtimes as running, MacPorts outside `/opt/local`, winget
portables from sources other than winget's own (listed as registry entries),
and user units under a custom `$XDG_CONFIG_HOME` are left out. User units'
descriptions aren't kept, since people write them and they can name folders. Linux services are
linked to distro packages only by program name, not through the package's
file list. Developer tools are in the backlog.

## Try it

```sh
cargo test
cargo run --release --example scan -- --summary   # counts and timing
cargo run --release --example scan                # full JSON
```
