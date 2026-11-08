// Update plan (Plan 4): presets, the rules grid, and quiet hours.
import { ActGlyph, Card, Chip, Eyebrow, SeverityMeter } from "@wardwatch/ui";
import { Link } from "react-router";
import { usePlan, useSavePlan } from "../../lib/data";
import type { Timing, UpdatePlan } from "../../lib/model";
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
import { Failed, Loading } from "../parts";

// Each timing has its own tint so the grid reads at a glance; the label carries the
// meaning. No amber here: amber is only for a hole being exploited right now.
const cellTone: Record<Timing, string> = {
  same_day: "bg-surface",
  sunday_tuneup: "bg-ok-tint",
  monthly: "bg-sel",
  auto: "bg-bg",
};

const select =
  "min-h-11 cursor-pointer rounded-[10px] border border-edge px-2.5 font-semibold text-text";

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
              <th key={k.id} scope="col" className={th}>
                <span className="block font-semibold">{k.label}</span>
                <span className="block text-xs font-normal text-text-muted">{k.examples}</span>
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
                <div className="flex min-h-11 items-center rounded-[10px] bg-act-tint px-3 font-semibold text-act-ink">
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
                    <select
                      aria-label={`${t.label}, ${k.label}`}
                      value={value}
                      onChange={(e) => {
                        save(withRule(plan, t.id, k.id, e.target.value as Timing));
                      }}
                      className={`${select} w-full ${cellTone[value]}`}
                    >
                      {timings.map((o) => (
                        <option key={o.id} value={o.id}>
                          {o.label}
                        </option>
                      ))}
                    </select>
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
          <select
            aria-label="Quiet hours start"
            value={plan.quiet_hours.start}
            onChange={(e) => {
              setHours("start", e.target.value);
            }}
            className={`${select} bg-surface`}
          >
            {hourOptions(plan.quiet_hours.start).map((h) => (
              <option key={h} value={h}>
                {hourLabel(h)}
              </option>
            ))}
          </select>
          <span>to</span>
          <select
            aria-label="Quiet hours end"
            value={plan.quiet_hours.end}
            onChange={(e) => {
              setHours("end", e.target.value);
            }}
            className={`${select} bg-surface`}
          >
            {hourOptions(plan.quiet_hours.end).map((h) => (
              <option key={h} value={h}>
                {hourLabel(h)}
              </option>
            ))}
          </select>
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
