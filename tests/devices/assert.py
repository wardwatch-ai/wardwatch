"""Checks what the scanner + matcher found inside one fixture device.

python3 assert.py <image> <dir with before.json and after.json>
"""

import json
import sys
from pathlib import Path

image, out = sys.argv[1], Path(sys.argv[2])
family = "rpm" if image.startswith("fedora") else "deb"
app = "fixture-vuln-rpm" if family == "rpm" else "fixture-vuln"
old, new = ("1.0-1", "1.1-1")
problems = []


def expect(cond, what):
    if not cond:
        problems.append(what)


def load(name):
    findings = json.loads((out / f"{name}.json").read_text())
    return findings, {f["app_id"]: f for f in findings}


before, by_id = load("before")
f = by_id.get(app)
expect(f is not None, f"before: {app} not found")
if f:
    expect(f["installed"] == old, f"before: installed {f['installed']!r}, want {old!r}")
    expect(f["fixed_version"] == new, f"before: fixed_version {f['fixed_version']!r}, want {new!r}")
    expect(f["exploited"] is True, "before: not marked exploited")
    expect(f["severity"] == "critical", f"before: severity {f['severity']!r}")
    expect([v["id"] for v in f["vulns"]] == ["CVE-2099-9001"], f"before: vulns {[v['id'] for v in f['vulns']]}")
    expect(before[0]["app_id"] == app, "before: the exploited package isn't listed first")
expect("fixture-not-installed" not in by_id, "before: a package that isn't installed was reported")

# The OS row proves the scanner read os-release: its version is the image tag.
osf = by_id.get("fixture-os")
expect(osf is not None, "before: OS not found")
if osf:
    want = image.split(":", 1)[1]
    expect(osf["installed"] == want, f"before: OS version {osf['installed']!r}, want {want!r}")

# The 1-click fix: the plan the fix screen shows, then the real update.
fix = (out / "fix.txt").read_text()
tool, admin = ("dnf", "/usr/bin/dnf") if family == "rpm" else ("apt", "/usr/bin/apt-get")
try:  # the example prints the plan as pretty JSON between a summary and progress lines
    plan = json.loads(fix[fix.index("\n{\n") : fix.index("\n}\n") + 2])
except ValueError:
    plan = {}
    problems.append(f"fix: no plan printed:\n{fix}")
method = plan.get("method", {})
expect(method.get("tool") == tool, f"fix: plan uses {method.get('tool')!r}, want {tool!r}")
expect(method.get("argv", [])[:2] == ["/usr/bin/pkexec", admin], f"fix: plan runs {method.get('argv')}")
expect(plan.get("one_click") is True, "fix: not a one-click fix")
expect("outcome: Installed" in fix, "fix: the update didn't report Installed")
expect(f'after: Some("{new}"), check: Closed' in fix, "fix: the updater's own recheck didn't say Closed")

after, by_id = load("after")
f = by_id.get(app)
expect(f is not None, f"after: {app} not found")
if f:
    expect(f["installed"] == new, f"after: installed {f['installed']!r}, want {new!r}")
    expect(f["vulns"] == [] and f["severity"] is None and not f["exploited"], "after: still vulnerable after the fix")

for p in problems:
    print(f"  {image}: {p}")
sys.exit(1 if problems else 0)
