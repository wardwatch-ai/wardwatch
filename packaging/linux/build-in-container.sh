#!/usr/bin/env bash
# Builds the Linux bundles (.deb, .rpm) inside Ubuntu 22.04.
#
#   packaging/linux/build-in-container.sh [out-dir]
#
# Why a container: a binary only runs on systems whose glibc is at least as
# new as the one it was linked against. Built on Ubuntu 24.04 (glibc 2.39)
# it fails on Debian 12 (2.36) and Ubuntu 22.04 (2.35), and apt installs it
# anyway because the package can't express that. Building on the oldest
# distribution we support (Ubuntu 22.04, glibc 2.35) makes one package work
# on all of them. test-install.sh checks this.
#
# The working tree is copied in (without node_modules/ and target/), so the
# host's build output is untouched. Output: out-dir/{deb,rpm}/ (default
# target/linux-bundle), which build-repo.sh takes as its bundle dir.
# Nothing is signed or uploaded.
#
# Behind a TLS-intercepting proxy, set WARDWATCH_BUILD_CA_BUNDLE to its CA.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
out=${1:-$root/target/linux-bundle}
mkdir -p "$out"
out=$(cd "$out" && pwd)

image=${WARDWATCH_BUILD_IMAGE:-mirror.gcr.io/library/ubuntu:22.04}
# Node major version, matching desktop.yml.
node_version=22

args=(--rm -v "$root:/src:ro" -v "$out:/out" -e "NODE_MAJOR=$node_version")
# The release channel, commit and date stamped into the app (see src-tauri/build.rs).
for var in WARDWATCH_BUILD_CHANNEL WARDWATCH_BUILD_COMMIT WARDWATCH_BUILD_DATE; do
  if [[ -n "${!var:-}" ]]; then args+=(-e "$var"); fi
done
if [[ -n "${WARDWATCH_BUILD_CA_BUNDLE:-}" ]]; then
  args+=(-v "$WARDWATCH_BUILD_CA_BUNDLE:/usr/local/share/ca-certificates/extra.crt:ro")
fi

docker run "${args[@]}" "$image" bash -euo pipefail -c '
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq --no-install-recommends ca-certificates curl xz-utils build-essential file \
  pkg-config libssl-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev >/dev/null
update-ca-certificates >/dev/null 2>&1 || true
export NODE_EXTRA_CA_CERTS=/etc/ssl/certs/ca-certificates.crt

export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt

# Node (official build, latest of the major) and pnpm via corepack. The
# checksum file is fetched once, so a release landing mid-build cannot
# mismatch. It checks integrity over TLS; it is not a signature check.
node_dist="https://nodejs.org/dist/latest-v${NODE_MAJOR}.x"
curl -fsSL "$node_dist/SHASUMS256.txt" -o node-shasums.txt
v=$(grep -o "node-v[0-9.]*-linux-x64.tar.xz" node-shasums.txt | head -1)
curl -fsSLO "$node_dist/$v"
grep " $v\$" node-shasums.txt | sha256sum -c - >/dev/null
tar -xJf "$v" -C /usr/local --strip-components=1
corepack enable

# Rust: latest stable, as in CI (desktop.yml), via the checksummed rustup-init.
rustup_url=https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init
curl -fsSL "$rustup_url" -o rustup-init
echo "$(curl -fsSL "$rustup_url.sha256" | cut -d" " -f1)  rustup-init" | sha256sum -c - >/dev/null
chmod +x rustup-init
./rustup-init -y -q --profile minimal --default-toolchain stable >/dev/null
. "$HOME/.cargo/env"

mkdir /build
# GNU tar excludes match at any depth: this drops every node_modules.
tar -C /src --exclude=node_modules --exclude=./target --exclude=./.git -cf - . | tar -C /build -xf -
cd /build
COREPACK_ENABLE_DOWNLOAD_PROMPT=0 pnpm install --frozen-lockfile >/dev/null
version=$(node -p "require(\"./apps/desktop/package.json\").version")
case "$version" in
  *-*)
    # A nightly (0.2.0-311). The app keeps that semver version, but packages
    # need their own pre-release forms to sort below the 0.2.0 release:
    # .deb Version 0.2.0~311, and .rpm Version 0.2.0 with Release 0.311
    # (RPM versions cannot contain "-"). `tauri bundle` reuses the binary built
    # for the .deb, so the app inside both still reports 0.2.0-311.
    base=${version%%-*} n=${version#*-}
    pnpm --filter @wardwatch/desktop tauri build --bundles deb
    pnpm --filter @wardwatch/desktop tauri bundle --bundles rpm --config \
      "{\"version\":\"$base\",\"bundle\":{\"linux\":{\"rpm\":{\"release\":\"0.$n\"}}}}"
    deb=$(ls target/release/bundle/deb/*.deb)
    rm -rf /tmp/deb && dpkg-deb -R "$deb" /tmp/deb
    sed -i "s/^Version: .*/Version: $base~$n/" /tmp/deb/DEBIAN/control
    rm "$deb"
    dpkg-deb --root-owner-group -b /tmp/deb "target/release/bundle/deb/Wardwatch_${base}~${n}_amd64.deb" >/dev/null
    ;;
  *) pnpm --filter @wardwatch/desktop tauri build --bundles deb,rpm ;;
esac
rm -rf /out/deb /out/rpm
cp -r target/release/bundle/deb target/release/bundle/rpm /out/
chown -R '"$(id -u):$(id -g)"' /out
echo "glibc requirement: $(objdump -T target/release/wardwatch | grep -o "GLIBC_[0-9.]*" | sort -Vu | tail -1)"
'

ls "$out"/deb/*.deb "$out"/rpm/*.rpm
