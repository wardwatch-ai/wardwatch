#!/bin/sh
# Fixture-device test: one throwaway container per distro image, the real
# scanner, matcher, nudge engine and 1-click updater inside it, and assertions on what
# they found and fixed.
#
#   tests/devices/run.sh [image ...]     (default: every image CI runs)
#
# Needs Docker, Python 3, the packaging tools in packages.sh and three static
# binaries, built with:
#   cargo build --release --target x86_64-unknown-linux-musl -p wardwatch-feed \
#     --example check -p wardwatch-update --example fix -p wardwatch-nudge --example nudge
set -eu

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
bin=${EXAMPLES_DIR:-$root/target/x86_64-unknown-linux-musl/release/examples}
for b in check fix nudge; do
  [ -x "$bin/$b" ] || { echo "missing $bin/$b; see the comment at the top of $0" >&2; exit 2; }
done
[ $# -gt 0 ] || set -- ubuntu:22.04 ubuntu:24.04 debian:12 fedora:41

pkgs=$(mktemp -d)
trap 'rm -rf "$pkgs"' EXIT
chmod 755 "$pkgs" # apt reads its repo as the unprivileged _apt user
"$here/packages.sh" "$pkgs"

status=0
for image in "$@"; do
  out=$(mktemp -d)
  echo "== $image"
  if docker run --rm --network none \
      -v "$bin/check:/wardwatch/check:ro" -v "$bin/fix:/wardwatch/fix:ro" \
      -v "$bin/nudge:/wardwatch/nudge:ro" -v "$pkgs:/pkgs:ro" -v "$here/catalog.json:/catalog.json:ro" \
      -v "$here/device.sh:/device.sh:ro" -v "$out:/out" \
      "$image" /device.sh \
    && python3 "$here/assert.py" "$image" "$out"; then
    echo "ok   $image"; rm -rf "$out"
  else
    echo "FAIL $image (output kept in $out)"; status=1
  fi
done
exit $status
