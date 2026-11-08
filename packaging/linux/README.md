# Linux packages

Wardwatch ships for Linux as a `.deb` (Debian, Ubuntu and derivatives) and an `.rpm` (Fedora, RHEL, openSUSE), from its own apt and dnf/zypper repositories on `downloads.wardwatch.ai`. There is no Flatpak, because its sandbox stops the app from cataloging the device. The hosting side (bucket layout, caching, the signing key and the install commands users run) is in [`wardwatch-ai/terraform`: docs/downloads.md](https://github.com/wardwatch-ai/terraform/blob/main/docs/downloads.md).

Tauri's bundler builds the packages. Package metadata lives in `apps/desktop/src-tauri/tauri.conf.json` (`bundle`, `bundle.linux`), and the maintainer comes from `authors` in `apps/desktop/src-tauri/Cargo.toml`. The scripts here only build on top of that.

Everything here runs locally. **Nothing signs or uploads anything.** Signing and publishing are manual steps that Matt runs.

## Build, assemble, test

```bash
# 1. Bundles, built on Ubuntu 22.04 so the binary runs on every supported distro
packaging/linux/build-in-container.sh                  # -> target/linux-bundle/{deb,rpm}

# 2. apt + rpm repository trees, laid out as downloads.wardwatch.ai serves them
packaging/linux/build-repo.sh target/linux-bundle      # -> target/linux-repo/{apt,rpm}

# 3. Install from that tree in clean containers and check every library resolves
packaging/linux/test-install.sh target/linux-repo
```

Behind a TLS-intercepting proxy, set `WARDWATCH_BUILD_CA_BUNDLE` (step 1) and `WARDWATCH_TEST_CA_BUNDLE` (step 3) to its CA file.

`build-repo.sh` needs `dpkg-deb`, `apt-ftparchive` (apt-utils), `rpm` and `createrepo_c`. On Ubuntu: `sudo apt-get install apt-utils rpm createrepo-c`. Steps 1 and 3 need Docker.

### Which distributions

`test-install.sh` covers Debian 12, Ubuntu 22.04 and 24.04, Fedora 42 and openSUSE Tumbleweed by default (all pass). Pass image names to test others. The binary needs glibc 2.34 or newer plus WebKitGTK 4.1, which rules out RHEL/Alma/Rocky 8 and Debian 11.

**Why Ubuntu 22.04 for the build:** a binary built on Ubuntu 24.04 links against glibc 2.39. On Debian 12 apt installs it without complaint and it then fails to start (`GLIBC_2.39 not found`). `test-install.sh` caught exactly that, which is why release bundles always come from `build-in-container.sh`. CI's own Linux bundle is a throwaway smoke test.

## Not done yet

- **Signing.** The trees are unsigned on purpose. Once Matt has the repo key, the release step adds `InRelease` and `Release.gpg` (apt), `repomd.xml.asc` (rpm metadata) and per-package rpm signatures, then uploads the packages first and the metadata last.
- **Shipping the repo config inside the packages,** so that installing a downloaded `.deb` or `.rpm` also subscribes the machine to updates. That waits for the public key, because a source with no key makes `apt update` fail. The plan is to add `bundle.linux.deb.files` and `bundle.linux.rpm.files` entries for `/etc/apt/sources.list.d/wardwatch.sources`, `/etc/apt/keyrings/wardwatch.asc` and `/etc/yum.repos.d/wardwatch.repo`, using the same files the downloads host serves.
- **arm64 / aarch64** builds. The scripts already handle more than one architecture.
- **Keeping old versions.** `build-repo.sh` builds a tree from scratch. Publishing will merge the new packages into the existing pool, so older versions stay installable.
