// Score (Score 6): score card, "Raise your score" under it, devices on the right.
import { ActGlyph, BandBar, Button, Card, Eyebrow, OkGlyph, SubscoreBar } from "@wardwatch/ui";
import { useDevice, useDevices, useScore } from "../../lib/data";
import { daysToSunday, pendingFixes, pointsForFixingNow } from "../../lib/estimate";
import { type Program, listNames } from "../../lib/programs";
import { worstSeverity } from "../../lib/format";
import { Failed, Loading, UpdateButton } from "../parts";
import { useOpen } from "../useOpen";

function ScoreCard() {
  const { data: score, error } = useScore();
  const devices = useDevices().data;
  if (error) return <Failed error={error} />;
  if (!score) return <Loading />;
  const count = devices
    ? ` · ${String(devices.length)} ${devices.length === 1 ? "device" : "devices"}`
    : "";
  if (score.score === null) {
    return (
      <Card className="flex flex-col gap-2 p-7">
        <Eyebrow>Your score{count}</Eyebrow>
        <h1 className="text-3xl">No score yet</h1>
        <p className="m-0 text-text-muted">
          Your score starts after your first fix. Fixing things fast raises it.
        </p>
      </Card>
    );
  }
  return (
    <Card className="flex flex-col gap-6 p-7">
      <div className="flex flex-wrap items-end justify-between gap-5">
        <div>
          <Eyebrow>Your score{count}</Eyebrow>
          <h1 className="mt-1 flex items-baseline gap-3.5">
            <span className="text-[88px] leading-none">{score.score}</span>
            <span className="text-[28px] text-ok">{score.band}</span>
          </h1>
        </div>
        {score.next_band && (
          <p className="m-0 max-w-[280px] text-sm text-text-muted">
            {score.points_to_next_band} points to{" "}
            <strong className="text-text">{score.next_band}</strong>.
          </p>
        )}
      </div>
      <BandBar score={score.score} />
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <Subscore
          label="How up to date you are"
          value={score.up_to_date}
          note="The share of updates you installed in time. Serious holes weigh more."
        />
        <Subscore
          label="How fast you fix"
          value={score.fix_speed}
          note="Fixing well inside each hole's deadline raises this."
        />
      </div>
    </Card>
  );
}

function Subscore({ label, value, note }: { label: string; value: number; note: string }) {
  return (
    <Card tone="inset" as="div" className="flex flex-col gap-2 p-4">
      <div className="flex items-baseline justify-between">
        <span className="font-semibold">{label}</span>
        <span className="ww-display text-[22px]">{value}</span>
      </div>
      <SubscoreBar value={value} label={label} />
      <div className="text-[13px] text-text-muted">{note}</div>
    </Card>
  );
}

/** Card title with the estimated points, as on the approved Score design. */
function MoveTitle({ title, points }: { title: string; points: number | null }) {
  return (
    <div className="flex items-baseline justify-between gap-2">
      <span className="text-base font-semibold">{title}</span>
      {points !== null && points > 0 && (
        <span className="ww-display text-xl text-ok" title="Estimated points">
          <span className="sr-only">About </span>+{points}
          <span className="sr-only"> points</span>
        </span>
      )}
    </div>
  );
}

function RaiseYourScore() {
  const open = useOpen();
  const { status, inventory } = useDevice();
  const score = useScore().data;
  const estimate = (programs: Program[], laterDays: number) =>
    score
      ? pointsForFixingNow(score.sums, score.algorithm, pendingFixes(programs), laterDays)
      : null;
  const devices = useDevices().data ?? [];
  const remote = devices.filter((d) => !d.current && d.remote_fix);
  const here = devices.find((d) => d.current)?.name ?? inventory?.os.name ?? "This device";
  const moves = [
    ...(status?.urgent ?? []).map((p) => (
      <Card key={p.key} tone="urgent" className="flex h-full flex-col gap-2.5">
        {/* Today rather than tomorrow: an exploited hole is only "in time" for a day. */}
        <MoveTitle title={`Fix ${p.name}`} points={estimate([p], 1)} />
        <div className="text-sm">
          This is being exploited right now. Fixing it today stops your score sliding.
        </div>
        <div className="text-[13px] text-text-muted">{here} · 1 minute</div>
        <UpdateButton program={p} className="mt-auto w-full">
          Fix now
        </UpdateButton>
      </Card>
    )),
    ...(status && status.waiting.length > 0
      ? [
          <Card key="tune-up" className="flex h-full flex-col gap-2.5">
            <MoveTitle
              title="Do the tune-up early"
              // Counts everything waiting as due Sunday; monthly and automatic
              // slots would gain more. TODO: use each program's plan slot.
              points={estimate(status.waiting, daysToSunday(new Date()))}
            />
            <div className="text-sm">
              {listNames(status.waiting)} {status.waiting.length === 1 ? "is" : "are"} waiting for
              Sunday. Doing them now counts as a faster fix.
            </div>
            <div className="text-[13px] text-text-muted">{here}</div>
            <Button
              variant="quiet"
              severity={worstSeverity(status.waiting)}
              className="mt-auto w-full"
              onClick={() => {
                open("/fix/tune-up");
              }}
            >
              Do it now
            </Button>
          </Card>,
        ]
      : []),
    // Fixes on another device: dashed, no button, the steps to do it there.
    ...remote.map((d) => (
      <Card key={d.id} tone="remote" className="flex h-full flex-col gap-2.5">
        <span className="text-base font-semibold">{d.remote_fix?.title}</span>
        <div className="text-sm">{d.remote_fix?.steps}</div>
        <div className="text-[13px] text-text-muted">{d.name} · do this on that device</div>
      </Card>
    )),
  ];
  if (moves.length === 0) return null;
  return (
    <section className="flex flex-col gap-2.5">
      <h2 className="ww-eyebrow font-sans">Raise your score</h2>
      <div className="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] items-stretch gap-3">
        {moves}
      </div>
    </section>
  );
}

function Devices() {
  const devices = useDevices().data;
  if (!devices) return null;
  return (
    <section className="flex flex-col gap-2.5">
      <Eyebrow>By device</Eyebrow>
      {devices.map((d) => (
        <Card key={d.id} as="div" className="flex flex-col gap-1.5 p-4">
          <div className="flex items-center gap-2">
            {d.status === "needs_you" ? <ActGlyph /> : <OkGlyph />}
            <span className="flex-1 font-semibold">{d.name}</span>
            <span className="ww-display text-xl">{d.score}</span>
          </div>
          <div
            className={`text-[13px] font-semibold ${
              d.status === "needs_you"
                ? "text-act-ink"
                : d.status === "clear"
                  ? "text-ok"
                  : "text-text-muted"
            }`}
          >
            {d.summary}
          </div>
          <div className="text-[13px] text-text-muted">{d.detail}</div>
        </Card>
      ))}
    </section>
  );
}

export function Score() {
  return (
    <div className="flex flex-wrap items-start gap-6">
      <div className="flex min-w-0 flex-[999_1_600px] flex-col gap-5">
        <ScoreCard />
        <RaiseYourScore />
      </div>
      <aside className="min-w-0 flex-[1_1_280px]">
        <Devices />
      </aside>
    </div>
  );
}
