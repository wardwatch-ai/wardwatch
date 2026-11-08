#!/usr/bin/env bash
# Builds the apt and rpm repository trees for downloads.wardwatch.ai from
# Tauri's bundles, laid out exactly as the downloads host serves them
# (wardwatch-ai/terraform: docs/downloads.md).
#
#   packaging/linux/build-repo.sh [bundle-dir] [out-dir]
#
# Defaults: target/release/bundle and target/linux-repo.
#
# The output is UNSIGNED and stays on this machine. Signing (InRelease,
# Release.gpg, repomd.xml.asc, rpm signatures) and uploading are separate,
# manual steps that only Matt runs. Nothing here touches the network.
#
# Needs: dpkg-deb, apt-ftparchive (apt-utils), rpm, createrepo_c.
set -euo pipefail

bundle_dir=${1:-target/release/bundle}
out=${2:-target/linux-repo}
suite=stable
component=main

for tool in dpkg-deb apt-ftparchive rpm createrepo_c; do
  command -v "$tool" >/dev/null || { echo "missing: $tool" >&2; exit 1; }
done

shopt -s nullglob
debs=("$bundle_dir"/deb/*.deb)
rpms=("$bundle_dir"/rpm/*.rpm)
if (( ${#debs[@]} == 0 && ${#rpms[@]} == 0 )); then
  echo "no .deb or .rpm under $bundle_dir (run: pnpm --filter @wardwatch/desktop tauri build --bundles deb,rpm)" >&2
  exit 1
fi

rm -rf "$out"
mkdir -p "$out"

# --- apt ---------------------------------------------------------------------
# pool/ holds each package once under its Debian file name, which is what makes
# those paths immutable on the downloads host. dists/ is the mutable index.
if (( ${#debs[@]} )); then
  archs=()
  for deb in "${debs[@]}"; do
    name=$(dpkg-deb -f "$deb" Package)
    version=$(dpkg-deb -f "$deb" Version)
    arch=$(dpkg-deb -f "$deb" Architecture)
    pool="$out/apt/pool/$component/${name:0:1}/$name"
    mkdir -p "$pool"
    cp "$deb" "$pool/${name}_${version}_${arch}.deb"
    [[ " ${archs[*]} " == *" $arch "* ]] || archs+=("$arch")
  done

  (
    cd "$out/apt"
    for arch in "${archs[@]}"; do
      dir="dists/$suite/$component/binary-$arch"
      mkdir -p "$dir"
      apt-ftparchive --arch "$arch" packages "pool/$component" > "$dir/Packages"
      gzip -9nk "$dir/Packages"
      xz -9k "$dir/Packages"
    done
    apt-ftparchive \
      -o "APT::FTPArchive::Release::Origin=Wardwatch" \
      -o "APT::FTPArchive::Release::Label=Wardwatch" \
      -o "APT::FTPArchive::Release::Suite=$suite" \
      -o "APT::FTPArchive::Release::Codename=$suite" \
      -o "APT::FTPArchive::Release::Architectures=${archs[*]}" \
      -o "APT::FTPArchive::Release::Components=$component" \
      -o "APT::FTPArchive::Release::Description=Wardwatch packages" \
      release "dists/$suite" > "dists/$suite/Release.tmp"
    mv "dists/$suite/Release.tmp" "dists/$suite/Release"
  )
fi

# --- rpm (dnf, zypper) -------------------------------------------------------
# One repository per architecture, matching baseurl=.../rpm/$basearch.
for rpm_file in "${rpms[@]}"; do
  arch=$(rpm -qp --qf '%{ARCH}' "$rpm_file" 2>/dev/null)
  file=$(rpm -qp --qf '%{NAME}-%{VERSION}-%{RELEASE}.%{ARCH}.rpm' "$rpm_file" 2>/dev/null)
  mkdir -p "$out/rpm/$arch"
  cp "$rpm_file" "$out/rpm/$arch/$file"
done
for dir in "$out"/rpm/*/; do
  createrepo_c --quiet --general-compress-type=gz "$dir"
done

echo "Unsigned repo tree in $out:"
(cd "$out" && find . -type f | sort)
