#!/bin/sh
# Runs INSIDE a throwaway container as root, with no network (see run.sh).
# 1. Points the package manager at the local fixture repo only (1.1 is there).
# 2. Installs the fixture package at 1.0 and scans: before.json.
# 3. Runs the real 1-click update (wardwatch-update's plan, then its run):
#    fix.txt.
# 4. Scans again: after.json.
set -eu

# The updater asks for admin rights through pkexec, which shows the OS's
# password prompt. There's no polkit in a container, so this stand-in plays
# the person who typed their password: it runs the command as given.
cat > /usr/bin/pkexec <<'SHIM'
#!/bin/sh
exec "$@"
SHIM
chmod +x /usr/bin/pkexec

if command -v dpkg >/dev/null; then
  rm -f /etc/apt/sources.list /etc/apt/sources.list.d/*
  echo "deb [trusted=yes] file:/pkgs/apt ./" > /etc/apt/sources.list.d/fixture.list
  apt-get -qq update
  dpkg -i /pkgs/wardwatch-fixture-1.0.deb >/dev/null
  app=fixture-vuln
else
  mkdir /etc/yum.repos.d/off && mv /etc/yum.repos.d/*.repo /etc/yum.repos.d/off/
  printf '[fixture]\nname=fixture\nbaseurl=file:///pkgs/rpm\ngpgcheck=0\n' > /etc/yum.repos.d/fixture.repo
  rpm -i /pkgs/wardwatch-fixture-1.0.rpm
  app=fixture-vuln-rpm
fi

/wardwatch/check /catalog.json > /out/before.json
/wardwatch/fix /catalog.json "$app" --run > /out/fix.txt 2>&1 || true
/wardwatch/check /catalog.json > /out/after.json
