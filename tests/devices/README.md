# Fixture devices

The whole core loop on real operating systems, without touching a real
computer. Each run starts a throwaway container per distro, with no network,
and plants a package that [`catalog.json`](catalog.json) calls vulnerable at
1.0 and fixed in 1.1. A local package repository inside the container offers
1.1, the way a distro mirror would. Then, with the real code:

1. scan and match (`wardwatch-feed --example check`): `before.json`
2. the 1-click fix (`wardwatch-update --example fix --run`): the plan the fix
   screen shows, the real `apt-get` or `dnf` upgrade, and the updater's own
   recheck: `fix.txt`
3. scan and match again: `after.json`

[`assert.py`](assert.py) checks that the planted package is found, marked
exploited and critical, ranked first, with the right fixed version; that a
package that isn't installed never shows up; that the OS version was read;
that the fix goes through `pkexec` and the distro's own updater and reports
the hole closed; and that afterwards nothing affects the device.

There's no polkit in a container, so `device.sh` puts a stand-in `pkexec` in
place that runs the command as if the person had typed their password.

Distros: Ubuntu 22.04 and 24.04, Debian 12, Fedora 41.

```sh
sudo apt-get install apt-utils rpm createrepo-c docker.io   # on Ubuntu
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl \
  -p wardwatch-feed --example check -p wardwatch-update --example fix
tests/devices/run.sh                 # or: tests/devices/run.sh fedora:41
```

A failing run keeps its `before.json`, `fix.txt` and `after.json` and prints
where. Next: check the nudge level too once the nudge engine lands.
