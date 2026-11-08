// Update plan (Plan 4): presets, the rules grid, and quiet hours.
import { ActGlyph, Card, Chip, Eyebrow, SeverityMeter } from "@wardwatch/ui";
import { Link } from "react-router";
import { usePlan, useSavePlan } from "../../lib/data";
import type { Kind, Timing, UpdatePlan } from "../../lib/model";
import {
  hourLabel,
  hourOptions,
  kinds,
  matchingPreset,
  presetIds,
  presets,
  tiers,
  timings,
  withPreset,
  withRule,
} from "../../lib/plan";
import { Failed, Loading, Select } from "../parts";

// As in the approved design (Plan 4): each timing has its own tint and icon, so the
// grid reads at a glance; the label carries the meaning. Same day uses the amber
// tint there, though amber otherwise means "exploited now" (brand rule 6).
const cellTone: Record<Timing, string> = {
  same_day: "bg-act-tint",
  sunday_tuneup: "bg-sel",
  monthly: "bg-bg",
  auto: "bg-ok-tint",
};

const svg = { width: 18, height: 18, viewBox: "0 0 24 24", "aria-hidden": true } as const;

function TimingIcon({ timing }: { timing: Timing }) {
  switch (timing) {
    case "same_day":
      return (
        <svg {...svg} className="text-act-ink">
          <path d="M13 2 L4 14 H11 L10 22 L20 9 H13 Z" fill="currentColor" />
        </svg>
      );
    case "sunday_tuneup":
      return (
        <svg {...svg}>
          <circle cx="12" cy="12" r="4.5" fill="currentColor" />
          <path
            d="M12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M5 19l2-2M17 7l2-2"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
          />
        </svg>
      );
    case "monthly":
      return (
        <svg {...svg} className="text-text-muted">
          <rect
            x="3"
            y="5"
            width="18"
            height="16"
            rx="2"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          />
          <path d="M3 10h18M8 3v4M16 3v4" stroke="currentColor" strokeWidth="2" />
        </svg>
      );
    case "auto":
      return (
        <svg {...svg} className="text-ok">
          <path
            d="M20 12a8 8 0 1 1-2.3-5.6M20 4v5h-5"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.4"
            strokeLinecap="round"
          />
        </svg>
      );
  }
}

const kindIcon: Record<Kind, React.ReactNode> = {
  system: (
    <svg {...svg}>
      <rect
        x="3"
        y="4"
        width="18"
        height="12"
        rx="2"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      />
      <path d="M8 20h8M12 16v4" stroke="currentColor" strokeWidth="2" />
    </svg>
  ),
  browsers: (
    <svg {...svg}>
      <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth="2" />
      <path
        d="M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      />
    </svg>
  ),
  apps: (
    <svg {...svg}>
      <rect x="4" y="4" width="7" height="7" rx="2" fill="currentColor" />
      <rect x="13" y="4" width="7" height="7" rx="2" fill="currentColor" />
      <rect x="4" y="13" width="7" height="7" rx="2" fill="currentColor" />
      <rect x="13" y="13" width="7" height="7" rx="2" fill="currentColor" />
    </svg>
  ),
};

const timingOptions = timings.map((o) => ({ value: o.id, label: o.label }));

function Grid({ plan, save }: { plan: UpdatePlan; save: (p: UpdatePlan) => void }) {
  const th = "border-b border-line p-3 text-left align-bottom";
  return (
    <Card className="overflow-x-auto p-0">
      <table className="w-full min-w-[640px] border-collapse text-sm">
        <thead>
          <tr>
            <th
              scope="col"
              className={`${th} text-xs font-semibold uppercase tracking-[0.04em] text-text-muted`}
            >
              How serious
            </th>
            {kinds.map((k) => (
              <th
                key={k.id}
                scope="col"
                className={`${th} text-xs font-semibold uppercase tracking-[0.04em] text-text-muted`}
              >
                <span className="flex items-center gap-2">
                  {kindIcon[k.id]}
                  {k.label}
                  <span className="font-normal normal-case tracking-normal">{k.examples}</span>
                </span>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          <tr>
            <th scope="row" className="border-b border-line p-3 text-left font-normal">
              <div className="flex items-center gap-2.5">
                <ActGlyph size={16} />
                <div>
                  <div className="font-semibold">Being exploited right now</div>
                  <div className="text-xs text-text-muted">Always same day. Can't be changed.</div>
                </div>
              </div>
            </th>
            {kinds.map((k) => (
              <td key={k.id} className="border-b border-line p-2">
                <div className="flex min-h-10 items-center gap-2 rounded-lg bg-act-tint px-2.5 text-[13px] font-semibold text-act-ink">
                  <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
                    <rect x="5" y="11" width="14" height="10" rx="2" fill="currentColor" />
                    <path
                      d="M8 11V8a4 4 0 0 1 8 0v3"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                    />
                  </svg>
                  Same day
                </div>
              </td>
            ))}
          </tr>
          {tiers.map((t) => (
            <tr key={t.id}>
              <th scope="row" className="border-b border-line p-3 text-left font-normal">
                <div className="flex items-center gap-2.5">
                  <SeverityMeter level={t.level} />
                  <div>
                    <div className="font-semibold">{t.label}</div>
                    <div className="text-xs text-text-muted">{t.hint}</div>
                  </div>
                </div>
              </th>
              {kinds.map((k) => {
                const value = plan.rules[t.id][k.id];
                return (
                  <td key={k.id} className="border-b border-line p-2">
                    <div
                      className={`flex items-center gap-2 rounded-[10px] px-2 py-1.5 ${cellTone[value]}`}
                    >
                      <span className="flex size-[22px] flex-none items-center justify-center">
                        <TimingIcon timing={value} />
                      </span>
                      <Select
                        label={`${t.label}, ${k.label}`}
                        value={value}
                        options={timingOptions}
                        onChange={(v) => {
                          save(withRule(plan, t.id, k.id, v));
                        }}
                        className="w-full"
                      />
                    </div>
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </Card>
  );
}

export function Plan() {
  const { data: plan, error } = usePlan();
  const save = useSavePlan();
  if (error) return <Failed error={error} />;
  if (!plan) return <Loading />;
  const current = matchingPreset(plan.rules);
  const setHours = (part: "start" | "end", value: string) => {
    save.mutate({ ...plan, quiet_hours: { ...plan.quiet_hours, [part]: value } });
  };

  return (
    <div className="flex flex-col gap-5">
      <div>
        <h1 className="text-3xl">Your update rules</h1>
        <p className="m-0 mt-1.5 max-w-[720px] text-text-muted">
          Decide when we nudge you, by how serious the hole is and what kind of program it's in.
          Start from a preset and change any box.
        </p>
      </div>
      {save.isError && <p role="alert">Couldn't save your plan: {save.error.message}</p>}

      <div role="group" aria-label="Presets" className="flex flex-wrap items-center gap-2">
        <Eyebrow className="mr-1">Start from</Eyebrow>
        {presetIds.map((id) => (
          <Chip
            key={id}
            on={current === id}
            onClick={() => {
              save.mutate(withPreset(plan, id));
            }}
          >
            {presets[id].label}
          </Chip>
        ))}
      </div>

      <Grid
        plan={plan}
        save={(p) => {
          save.mutate(p);
        }}
      />

      <Card className="flex flex-col gap-3">
        <Eyebrow>Quiet hours</Eyebrow>
        <div className="flex flex-wrap items-center gap-2.5">
          <span>Quiet hours from</span>
          <Select
            label="Quiet hours start"
            value={plan.quiet_hours.start}
            options={hourOptions(plan.quiet_hours.start).map((h) => ({
              value: h,
              label: hourLabel(h),
            }))}
            onChange={(v) => {
              setHours("start", v);
            }}
            className="min-w-24"
          />
          <span>to</span>
          <Select
            label="Quiet hours end"
            value={plan.quiet_hours.end}
            options={hourOptions(plan.quiet_hours.end).map((h) => ({
              value: h,
              label: hourLabel(h),
            }))}
            onChange={(v) => {
              setHours("end", v);
            }}
            className="min-w-24"
          />
        </div>
        <p className="m-0 text-sm text-text-muted">
          We always ask before closing an app that's open, ask before big upgrades like macOS 16 to
          17, and wait for Wi-Fi and power before large downloads.
        </p>
        <p className="m-0 text-sm text-text-muted">
          On iPhone we can only remind you, because Apple installs the update. Held apps:{" "}
          {plan.holds.length}, <Link to="/apps">manage in Apps</Link>.
        </p>
      </Card>
    </div>
  );
}
