#!/bin/sh
# Builds the planted fixture package at 1.0 (vulnerable in catalog.json) and
# 1.1 (fixed), as .deb and .rpm, plus a local apt and dnf repository holding
# 1.1, into the directory given. Runs on the host so the containers need no
# network. Needs dpkg-deb, apt-ftparchive, rpmbuild and createrepo_c
# (on Ubuntu: apt-get install apt-utils rpm createrepo-c).
set -eu

pkg=wardwatch-fixture
dest=$1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$dest/apt" "$dest/rpm"

for v in 1.0 1.1; do
  d="$work/deb-$v"
  mkdir -p "$d/DEBIAN" "$d/usr/share/$pkg"
  echo "$v" > "$d/usr/share/$pkg/version"
  cat > "$d/DEBIAN/control" <<CONTROL
Package: $pkg
Version: $v-1
Architecture: all
Maintainer: Wardwatch tests <tests@wardwatch.ai>
Description: Wardwatch test fixture, safe to remove
CONTROL
  dpkg-deb --root-owner-group --build "$d" "$dest/$pkg-$v.deb" >/dev/null

  cat > "$work/$pkg-$v.spec" <<SPEC
Name: $pkg
Version: $v
Release: 1
Summary: Wardwatch test fixture, safe to remove
License: GPL-3.0-or-later
BuildArch: noarch
%description
Wardwatch test fixture.
%install
mkdir -p %{buildroot}/usr/share/$pkg
echo $v > %{buildroot}/usr/share/$pkg/version
%files
/usr/share/$pkg/version
SPEC
  rpmbuild --quiet --define "_topdir $work/rpm" --define "dist %{nil}" -bb "$work/$pkg-$v.spec"
  cp "$work/rpm/RPMS/noarch/$pkg-$v-1.noarch.rpm" "$dest/$pkg-$v.rpm"
done

# The repositories offer only the fixed version, as a distro mirror would.
cp "$dest/$pkg-1.1.deb" "$dest/apt/"
(cd "$dest/apt" && apt-ftparchive packages . > Packages)
cp "$dest/$pkg-1.1.rpm" "$dest/rpm/"
createrepo_c --quiet "$dest/rpm"
