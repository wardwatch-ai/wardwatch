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
node_version=$(cat "$root/.nvmrc" 2>/dev/null || echo 22)

args=(--rm -v "$root:/src:ro" -v "$out:/out" -e "NODE_MAJOR=$node_version")
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

# Node (official build, latest of the major in .nvmrc) and pnpm via corepack.
v=$(curl -fsSL "https://nodejs.org/dist/latest-v${NODE_MAJOR%%.*}.x/SHASUMS256.txt" | grep -o "node-v[0-9.]*-linux-x64.tar.xz" | head -1)
curl -fsSLO "https://nodejs.org/dist/latest-v${NODE_MAJOR%%.*}.x/$v"
curl -fsSL "https://nodejs.org/dist/latest-v${NODE_MAJOR%%.*}.x/SHASUMS256.txt" | grep " $v\$" | sha256sum -c - >/dev/null
tar -xJf "$v" -C /usr/local --strip-components=1
corepack enable

# Rust: latest stable, as in CI (desktop.yml).
curl -fsSL https://sh.rustup.rs | sh -s -- -y -q --profile minimal --default-toolchain stable >/dev/null
. "$HOME/.cargo/env"

mkdir /build
tar -C /src --exclude=./node_modules --exclude="./*/node_modules" --exclude="./*/*/node_modules" \
  --exclude=./target -cf - . | tar -C /build -xf -
cd /build
COREPACK_ENABLE_DOWNLOAD_PROMPT=0 pnpm install --frozen-lockfile >/dev/null
pnpm --filter @wardwatch/desktop tauri build --bundles deb,rpm
rm -rf /out/deb /out/rpm
cp -r target/release/bundle/deb target/release/bundle/rpm /out/
chown -R '"$(id -u):$(id -g)"' /out
echo "glibc requirement: $(objdump -T target/release/wardwatch | grep -o "GLIBC_[0-9.]*" | sort -Vu | tail -1)"
'

ls "$out"/deb/*.deb "$out"/rpm/*.rpm
