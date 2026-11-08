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
| Windows | `HKLM\…\Windows NT\CurrentVersion` (build.UBR, 23H2) | Uninstall registry keys (64-bit, 32-bit, current user), filtered like Settings > Installed apps |
| Linux | `/etc/os-release` | dpkg status file, `rpm -qa`, snaps in `/snap` or `/var/lib/snapd/snap`, `flatpak list` (runtimes too, by branch), Linuxbrew |

Browsers (Chrome, Chromium, Edge, Firefox, Safari, Brave, Opera, Vivaldi, Arc)
are tagged on whichever entry installed them. Ubuntu's stub `firefox` deb is
not tagged; its snap is.

`running` is set when a running process's executable lives inside the app's
install folder (or, for distro packages, is named after the package). Only
the executable path of each process is read.

Each entry's `source` says which `id` namespace it uses: `mac_app` (bundle ID),
`homebrew_formula`, `homebrew_cask`, `windows_registry` (`name` is
DisplayName), `dpkg`, `rpm`, `snap`, `flatpak`. dpkg and rpm versions keep
their epoch and revision because distro advisories are written against them.

## Privacy and cost

- Nothing leaves the device from this crate. Install paths are kept in memory
  only to match running processes and are never serialized, so usernames in
  paths can't leak into anything built from the inventory. Warnings contain no paths.
- Package databases and plists are read in-process. Only `rpm` and `flatpak`
  are run, by absolute path, because they have no stable on-disk format.
- One pass, no threads, no polling. On an Ubuntu machine with 837 packages a
  release-build scan takes about 4 ms and 13 MB peak memory; 124 rpm packages
  in Fedora 41 take about 70 ms (mostly `rpm` itself).

## Not yet

Microsoft Store apps, Chocolatey, Scoop, MacPorts, AppImage, background
services (launchd, systemd, Windows services) and browser extensions are
follow-ups, as is marking Flatpak apps as running (`flatpak ps`). Developer tools are in the backlog.

## Try it

```sh
cargo test
cargo run --release --example scan -- --summary   # counts and timing
cargo run --release --example scan                # full JSON
```
