// Update plan presets (Plan 4 board). The nudge engine uses the same three.
import type { Kind, Tier, Timing, UpdatePlan } from "./model";

type Rules = UpdatePlan["rules"];

const all = (t: Timing) => ({ system: t, browsers: t, apps: t });

export const presets = {
  relaxed: {
    label: "Relaxed",
    rules: {
      serious: { system: "sunday_tuneup", browsers: "same_day", apps: "sunday_tuneup" },
      moderate: all("monthly"),
      minor: all("monthly"),
    },
  },
  balanced: {
    label: "Balanced",
    rules: {
      serious: all("same_day"),
      moderate: all("sunday_tuneup"),
      minor: { system: "sunday_tuneup", browsers: "sunday_tuneup", apps: "monthly" },
    },
  },
  eager: {
    label: "Eager",
    rules: {
      serious: { system: "same_day", browsers: "auto", apps: "auto" },
      moderate: all("auto"),
      minor: all("auto"),
    },
  },
} as const satisfies Record<string, { label: string; rules: Rules }>;

export type PresetId = keyof typeof presets;
export const presetIds = Object.keys(presets) as PresetId[];

export const tiers: readonly { id: Tier; label: string; hint: string; level: 1 | 2 | 3 }[] = [
  { id: "serious", label: "Serious", hint: "Could let someone take over", level: 3 },
  { id: "moderate", label: "Moderate", hint: "Harder to use, or limited harm", level: 2 },
  { id: "minor", label: "Minor, or no security fix", hint: "Bug fixes and features", level: 1 },
];

export const kinds: readonly { id: Kind; label: string; examples: string }[] = [
  { id: "system", label: "System", examples: "macOS, Windows, Linux" },
  { id: "browsers", label: "Browsers", examples: "Chrome, Safari, Firefox" },
  { id: "apps", label: "Other apps", examples: "Zoom, VLC and the rest" },
];

export const timings: readonly { id: Timing; label: string }[] = [
  { id: "same_day", label: "Same day" },
  { id: "sunday_tuneup", label: "Sunday tune-up" },
  { id: "monthly", label: "Monthly" },
  { id: "auto", label: "Install by itself" },
];

/** The preset the rules match exactly, or null once any box differs. */
export function matchingPreset(rules: Rules): PresetId | null {
  return (
    presetIds.find((id) =>
      tiers.every((t) => kinds.every((k) => presets[id].rules[t.id][k.id] === rules[t.id][k.id])),
    ) ?? null
  );
}

export function withPreset(plan: UpdatePlan, id: PresetId): UpdatePlan {
  return { ...plan, rules: structuredClone(presets[id].rules) };
}

export function withRule(plan: UpdatePlan, tier: Tier, kind: Kind, timing: Timing): UpdatePlan {
  return { ...plan, rules: { ...plan.rules, [tier]: { ...plan.rules[tier], [kind]: timing } } };
}

/** "22:00" → "10 pm" */
export function hourLabel(hhmm: string): string {
  const h = Number(hhmm.slice(0, 2));
  if (h === 0) return "Midnight";
  if (h === 12) return "Noon";
  return h < 12 ? `${String(h)} am` : `${String(h - 12)} pm`;
}

export const hours = Array.from({ length: 24 }, (_, h) => `${String(h).padStart(2, "0")}:00`);
