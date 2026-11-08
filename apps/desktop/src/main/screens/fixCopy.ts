// The fix screen's words, built only from what the catalog says, so nothing
// claims more than we know. Kept apart from the components so it's easy to test.
import type { Severity as Meter } from "@wardwatch/ui";
import type { Finding, Plan, Severity, VulnHit } from "../../lib/ipc";

const crumbs: Record<Finding["kind"], string> = { app: "Apps", os: "System", package: "System" };

export function crumbFor(f: Finding): string {
  return f.browser ? "Browsers" : crumbs[f.kind];
}

export function headline(f: Finding): string {
  if (f.exploited) return `${f.app_name} has a hole attackers are using right now.`;
  switch (f.severity) {
    case "critical":
    case "high":
      return `${f.app_name} has a serious security hole.`;
    case "medium":
    case "low":
    case "unknown":
      return `${f.app_name} has a security hole.`;
    default:
      return f.update_available
        ? `An update for ${f.app_name} is ready.`
        : `${f.app_name} is up to date.`;
  }
}

export function lede(f: Finding, plan: Plan | null): string {
  if (!f.update_available) return "There's nothing to do.";
  if (f.vulns.length === 0) return "It has no security fixes, but it keeps you current.";
  if (!plan?.one_click) return "Updating closes it. Here's how.";
  return f.browser
    ? "1 minute to update. Tabs will come back."
    : "Updating closes it, and Wardwatch can do it for you.";
}

export function doneLede(f: Finding): string {
  return `The update installed and we checked again: ${f.app_name} no longer has the hole.`;
}

export function happening(f: Finding): string {
  const n = f.vulns.length;
  const fixed = f.fixed_version
    ? ` Its maker fixed ${n === 1 ? "it" : "them"} in version ${f.fixed_version}.`
    : "";
  const used = f.exploited ? " Security agencies have seen it used against real people." : "";
  return `${f.app_name} ${f.installed} has ${n === 1 ? "a known security hole" : `${String(n)} known security holes`}.${fixed}${used}`;
}

export function risk(f: Finding): string {
  if (f.exploited)
    return `Someone could use it to get into ${f.app_name}, and through it your device, until you update.`;
  if (f.severity === "critical" || f.severity === "high")
    return "Attackers often start using holes like this soon after they're published.";
  return "It's unlikely to be used against you, but updating removes the chance.";
}

export function adminLine(plan: Plan): string {
  if (plan.admin === "asks")
    return "Your system will ask for your password. Wardwatch never sees it.";
  if (plan.admin === "may_ask") return "Your system may ask for permission.";
  return "About a minute · nothing to type";
}

export function actionLabel(f: Finding, plan: Plan): string {
  switch (plan.method.kind) {
    case "command":
      return `Update ${f.app_name} now`;
    case "system_settings":
      return `Open ${plan.by}`;
    case "app_updater":
      return plan.method.open_bundle ? `Open ${f.app_name}` : "Got it";
  }
}

export function knownUsed(f: Finding): string {
  const kev = f.vulns.find((v) => v.exploited)?.kev;
  if (!kev) return f.exploited ? "Yes" : "No";
  return `Yes · CISA list, ${formatDate(kev.added)}${kev.ransomware ? " · ransomware" : ""}`;
}

export function foundAgo(f: Finding): string {
  const added = f.vulns.find((v) => v.exploited)?.kev?.added;
  if (!added) return "";
  const days = Math.floor((Date.now() - Date.parse(added)) / 86_400_000);
  if (Number.isNaN(days) || days < 0) return "";
  return days === 0 ? " · found today" : ` · found ${String(days)} day${days === 1 ? "" : "s"} ago`;
}

export function formatDate(iso: string): string {
  const d = new Date(`${iso}T00:00:00Z`);
  return Number.isNaN(d.getTime())
    ? iso
    : d.toLocaleDateString("en-US", { month: "short", day: "numeric", timeZone: "UTC" });
}

/** critical/high = 3 bars, medium = 2, anything else (or no security fix) = 1. */
export function meter(s: Severity | null): Meter {
  if (s === "critical" || s === "high") return 3;
  if (s === "medium") return 2;
  return 1;
}

/** For display and copy-paste: quotes arguments a shell would split. */
export function shellLine(argv: string[]): string {
  return argv
    .map((a) => (/^[\w./:@=+-]+$/.test(a) ? a : `'${a.replaceAll("'", `'\\''`)}'`))
    .join(" ");
}

export function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

const rank: Record<Severity, number> = { critical: 4, high: 3, medium: 2, low: 1, unknown: 0 };

/** The hole to describe first: exploited before not, then by severity. Vulns arrive newest first. */
export function worstVuln(f: Finding): VulnHit | undefined {
  return f.vulns.reduce<VulnHit | undefined>((best, v) => {
    if (!best) return v;
    const key = (x: VulnHit) => (x.exploited ? 10 : 0) + rank[x.severity];
    return key(v) > key(best) ? v : best;
  }, undefined);
}
