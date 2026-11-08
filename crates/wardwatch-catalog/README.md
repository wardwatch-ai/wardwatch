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
| macOS | `SystemVersion.plist` (Rapid Security Response aware) | `Info.plist` of apps in `/Applications` and `~/Applications` (one folder deep), Homebrew Cellar and Caskroom |
| Windows | `HKLM\…\Windows NT\CurrentVersion` (build.UBR, 23H2) | Uninstall registry keys (64-bit, 32-bit, current user), filtered like Settings > Installed apps; Microsoft Store packages from the AppModel repository in `HKCU`; Chocolatey `.nuspec` files; Scoop `manifest.json` files (user and global) |
| Linux | `/etc/os-release` | dpkg status file, `rpm -qa`, snaps in `/snap` or `/var/lib/snapd/snap`, `flatpak list` (runtimes too, by branch), Linuxbrew |

Services and browser extensions get their own lists:

| | Read from | Left out |
| --- | --- | --- |
| `services` | systemd unit files (running units from `/proc/<pid>/cgroup`); launchd plists in `/Library` and `~/Library`; `HKLM\SYSTEM\CurrentControlSet\Services` | Linux units that are neither enabled nor running; Apple's jobs in `/System`; Windows drivers, per-user service templates, and services under the Windows folder |
| `extensions` | Chromium-based browsers' `Extensions/<id>/<version>/manifest.json` and `Preferences` (enabled state) in every profile; Firefox `extensions.json`; Safari `.appex` bundles inside Mac apps | Themes, dictionaries, and add-ons that ship with the browser |

Each service is linked to the software entry whose install folder holds its
program (or a distro package named like the program). Store packages drop
what Settings hides: frameworks, resource packs, and inbox apps under the
Windows folder. A Chocolatey package that ran an installer also has its
registry entry; both are kept.

Browsers (Chrome, Chromium, Edge, Firefox, Safari, Brave, Opera, Vivaldi, Arc)
are tagged on whichever entry installed them. Ubuntu's stub `firefox` deb is
not tagged; its snap is.

`running` is set when a running process's executable lives inside the app's
install folder (or, for distro packages, is named after the package). Only
the executable path of each process is read.

Each entry's `source` says which `id` namespace it uses: `mac_app` (bundle ID),
`homebrew_formula`, `homebrew_cask`, `windows_registry` (`name` is
DisplayName), `dpkg`, `rpm`, `snap`, `flatpak`, `ms_store` (package name),
`chocolatey`, `scoop`. dpkg and rpm versions keep
their epoch and revision because distro advisories are written against them.

## Privacy and cost

- Nothing leaves the device from this crate. Install paths are kept in memory
  only to match running processes and are never serialized, so usernames in
  paths can't leak into anything built from the inventory. The same goes for
  service programs and browser profile folders. Warnings contain no paths.
- Package databases and plists are read in-process. Only `rpm` and `flatpak`
  are run, by absolute path, because they have no stable on-disk format.
- One pass, no threads, no polling. On an Ubuntu machine with 837 packages a
  release-build scan takes about 4 ms and 13 MB peak memory; 124 rpm packages
  in Fedora 41 take about 70 ms (mostly `rpm` itself).

## Not yet

MacPorts, AppImage, winget-only installs, systemd user units, and marking
Flatpak apps as running (`flatpak ps`) are follow-ups. Linux services are
linked to distro packages only by program name, not through the package's
file list. Developer tools are in the backlog.

## Try it

```sh
cargo test
cargo run --release --example scan -- --summary   # counts and timing
cargo run --release --example scan                # full JSON
```
