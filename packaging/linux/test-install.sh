#!/usr/bin/env bash
# Installs Wardwatch from a locally built, unsigned repo tree in clean
# containers, the way users will (apt / dnf / zypper against a repo URL),
# and checks that every shared library the binary needs resolves.
#
#   packaging/linux/test-install.sh [repo-dir] [image...]
#
# repo-dir defaults to target/linux-repo (from build-repo.sh). Images
# default to Debian 12, Ubuntu 22.04 and 24.04, Fedora 42 and openSUSE Tumbleweed,
# pulled from registries without Docker Hub's anonymous rate limit.
#
# Signature checks are off ONLY here, because the tree is unsigned on
# purpose. The published repo files (downloads/wardwatch.{sources,repo} in
# wardwatch-ai/terraform) always require signatures.
set -euo pipefail

repo=$(cd "${1:-target/linux-repo}" && pwd)
shift || true
images=("$@")
(( ${#images[@]} )) || images=(
  mirror.gcr.io/library/debian:12
  mirror.gcr.io/library/ubuntu:22.04
  mirror.gcr.io/library/ubuntu:24.04
  quay.io/fedora/fedora:42
  registry.opensuse.org/opensuse/tumbleweed:latest
)

# Serve the tree read-only over HTTP from this machine, like the downloads
# host minus TLS. Containers reach it through the Docker host gateway.
port=${WARDWATCH_REPO_PORT:-18765}
python3 -m http.server "$port" --bind 0.0.0.0 --directory "$repo" >/dev/null 2>&1 &
server_pid=$!
trap 'kill "$server_pid" 2>/dev/null || true' EXIT
for _ in $(seq 50); do
  curl -fs -o /dev/null "http://127.0.0.1:$port/" && break
  sleep 0.1
done
# If the port was taken, the server exited: fail rather than test something else.
kill -0 "$server_pid" 2>/dev/null || { echo "could not serve on port $port" >&2; exit 1; }
url="http://host.docker.internal:$port"

# Behind a TLS-intercepting proxy, pass its CA bundle so the distro's own
# package mirrors still verify: WARDWATCH_TEST_CA_BUNDLE=/path/to/ca.pem
docker_args=(--rm --add-host=host.docker.internal:host-gateway)
dnf_ca=""
if [[ -n "${WARDWATCH_TEST_CA_BUNDLE:-}" ]]; then
  docker_args+=(-v "$WARDWATCH_TEST_CA_BUNDLE:/extra-ca.pem:ro")
  dnf_ca="--setopt=sslcacert=/extra-ca.pem"
fi

# Runs after install in every image: the package is registered, the binary
# and desktop entry exist, and no linked library is missing.
# shellcheck disable=SC2016 # expands inside the container, not here
verify='set -e
test -x /usr/bin/wardwatch
test -f /usr/share/applications/Wardwatch.desktop
if ldd /usr/bin/wardwatch | grep "not found"; then echo "missing libraries" >&2; exit 1; fi
echo "ok: $(. /etc/os-release; echo "$PRETTY_NAME")"'

failed=0
for image in "${images[@]}"; do
  echo "== $image"
  case "$image" in
    *debian*|*ubuntu*)
      script="set -e
export DEBIAN_FRONTEND=noninteractive
echo 'deb [trusted=yes] $url/apt stable main' > /etc/apt/sources.list.d/wardwatch.list
apt-get update -qq
apt-get install -y -qq wardwatch >/dev/null
dpkg -s wardwatch | grep -E '^(Version|Maintainer):'
$verify" ;;
    *fedora*)
      script="set -e
printf '[wardwatch]\nname=Wardwatch (local test)\nbaseurl=$url/rpm/\$basearch\ngpgcheck=0\nrepo_gpgcheck=0\n' > /etc/yum.repos.d/wardwatch.repo
dnf install -y -q $dnf_ca wardwatch >/dev/null
rpm -q wardwatch
$verify" ;;
    *opensuse*)
      script="set -e
zypper --non-interactive --quiet addrepo --no-gpgcheck $url/rpm/\$(uname -m) wardwatch
zypper --non-interactive --quiet refresh
zypper --non-interactive --quiet install --allow-unsigned-rpm wardwatch
rpm -q wardwatch
$verify" ;;
    *) echo "unknown image family: $image" >&2; failed=1; continue ;;
  esac
  if ! docker run "${docker_args[@]}" "$image" bash -c "$script"; then
    echo "FAILED: $image" >&2
    failed=1
  fi
done
exit "$failed"
