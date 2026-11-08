// Home (board E): status beside score, then the tune-up, this device and other devices.
import { ActGlyph, BandBar, Button, Card, Eyebrow, Lighthouse, OkGlyph } from "@wardwatch/ui";
import { Link, useNavigate } from "react-router";
import { useDevice, useDevices, useGroupGlance, useScore } from "../../lib/data";
import type { Kind } from "../../lib/model";
import { type DeviceStatus, type Program, fixPath, kindLine, listNames } from "../../lib/programs";
import { ordinal, versionStep, worstSeverity } from "../../lib/format";
import { Failed, KindMark, Loading, UpdateButton } from "../parts";

function StatusCard({ status }: { status: DeviceStatus }) {
  const navigate = useNavigate();
  const top = status.attack[0];
  const quiet = status.quietAttack;
  return (
    <Card
      tone={top ? "urgent" : "default"}
      className="flex flex-1 basis-[420px] items-center gap-6 p-7"
    >
      <Lighthouse beams={Boolean(top)} size={96} />
      <div className="flex min-w-0 flex-1 flex-col gap-3">
        {top ? (
          <>
            <Eyebrow className="flex items-center gap-2">
              <ActGlyph size={12} />
              Needs you today
            </Eyebrow>
            <h1 className="text-[32px] leading-tight">Update {top.name}.</h1>
            <div>
              <div>This is being exploited right now.</div>
              <div className="text-text-muted">1 minute to update. Tabs will come back.</div>
            </div>
            <div className="flex flex-wrap gap-2.5">
              <UpdateButton program={top}>Update {top.name} now</UpdateButton>
              <Button variant="quiet" onClick={() => void navigate(fixPath(top))}>
                What's going on?
              </Button>
            </div>
            {status.attack.length > 1 && (
              <p className="m-0 text-sm text-text-muted">
                {status.attack.length - 1} more {status.attack.length === 2 ? "needs" : "need"} you
                too. See <Link to="/apps">Apps</Link>.
              </p>
            )}
          </>
        ) : quiet.length > 0 ? (
          <>
            <Eyebrow>Worth doing soon</Eyebrow>
            <h1 className="text-[32px] leading-tight">Update {listNames(quiet)} soon.</h1>
            <p className="m-0 text-text-muted">
              Attackers are using a hole in {quiet.length === 1 ? "it" : "them"}. Update from{" "}
              <Link to="/apps">Apps</Link> when you can.
            </p>
          </>
        ) : status.checked ? (
          <>
            <Eyebrow>All clear</Eyebrow>
            <h1 className="text-[32px] leading-tight">Nothing needs you.</h1>
            <p className="m-0 text-text-muted">
              {status.waiting.length > 0
                ? "Nothing is under attack. The routine updates below can wait for Sunday."
                : status.current === status.total
                  ? "This device's system, browsers and apps are up to date. We'll keep watch."
                  : `${String(status.current)} of ${String(status.total)} programs are checked and up to date; the rest aren't in the update feed yet. We'll keep watch.`}
            </p>
          </>
        ) : (
          <>
            <Eyebrow>Not checked yet</Eyebrow>
            <h1 className="text-[32px] leading-tight">We haven't checked for updates yet.</h1>
            <p className="m-0 text-text-muted">
              We found {status.total} programs on this device. Checks start once the update feed is
              connected.
            </p>
          </>
        )}
      </div>
    </Card>
  );
}

function ScoreCard() {
  const navigate = useNavigate();
  const score = useScore().data;
  const glance = useGroupGlance();
  if (!score) return null;
  return (
    <Card className="flex flex-1 basis-[300px] flex-col gap-3.5 p-6">
      <Eyebrow>Your score</Eyebrow>
      {score.score === null ? (
        <p className="m-0">No score yet. It starts after your first fix.</p>
      ) : (
        <>
          <div className="flex items-baseline gap-3">
            <span className="ww-display text-[64px] leading-none">{score.score}</span>
            <span className="ww-display text-2xl text-ok">{score.band}</span>
          </div>
          <BandBar score={score.score} />
          {score.next_band && (
            <p className="m-0 text-sm text-text-muted">
              {score.points_to_next_band} points to {score.next_band}.
            </p>
          )}
        </>
      )}
      {glance?.rank != null && (
        <div className="flex flex-wrap items-baseline gap-x-2 text-sm">
          <span className="ww-display text-lg">{ordinal(glance.rank)}</span>
          <span>in {glance.group.name}</span>
          {glance.leader && (
            <span className="text-text-muted">
              {glance.leader.name} leads at {glance.leader.score}
            </span>
          )}
        </div>
      )}
      <Button variant="quiet" className="mt-auto" onClick={() => void navigate("/score")}>
        See your score
      </Button>
    </Card>
  );
}

function TuneUp({ waiting }: { waiting: Program[] }) {
  const navigate = useNavigate();
  return (
    <Card className="flex flex-col gap-4 p-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 className="text-xl">Sunday tune-up</h2>
          <p className="m-0 mt-1 text-text-muted">
            Routine updates, no rush. Doing them before Sunday counts as a faster fix.
          </p>
        </div>
        <Button
          variant="quiet"
          severity={worstSeverity(waiting)}
          onClick={() => void navigate("/fix/tune-up")}
        >
          Update all {waiting.length}
        </Button>
      </div>
      <ul className="m-0 grid list-none grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3 p-0">
        {waiting.map((p) => (
          <li key={p.key} className="ww-card ww-card-inset px-4 py-3">
            <div className="font-semibold">{p.name}</div>
            <div className="ww-mono text-text-muted">{versionStep(p)}</div>
          </li>
        ))}
      </ul>
    </Card>
  );
}

function ThisDevice({ status }: { status: DeviceStatus }) {
  const row = (label: string, kind: Kind, to?: string) => {
    const line = kindLine(status, kind);
    const body = (
      <>
        <KindMark mark={line.mark} />
        <span className="flex-1 font-semibold">{label}</span>
        <span className={line.mark === "urgent" ? "font-semibold text-act-ink" : "text-text-muted"}>
          {line.text}
        </span>
      </>
    );
    const cls =
      "ww-card ww-card-default flex min-h-11 items-center gap-2.5 px-4 py-3 text-text no-underline";
    return to ? (
      <Link to={to} className={cls}>
        {body}
      </Link>
    ) : (
      <div className={cls}>{body}</div>
    );
  };
  return (
    <section className="flex flex-1 basis-[320px] flex-col gap-2.5">
      <Eyebrow>On this device</Eyebrow>
      {row("System", "system")}
      {row("Browsers", "browsers")}
      {row("Apps", "apps", "/apps")}
    </section>
  );
}

function OtherDevices() {
  const devices = useDevices().data?.filter((d) => !d.current);
  if (!devices) return null;
  return (
    <section className="flex flex-1 basis-[320px] flex-col gap-2.5">
      <Eyebrow>Your other devices</Eyebrow>
      {devices.length === 0 && <p className="m-0 text-text-muted">Only this one so far.</p>}
      {devices.map((d) => (
        <div
          key={d.id}
          className="ww-card ww-card-default flex min-h-11 items-center gap-2.5 px-4 py-3"
        >
          {d.status === "needs_you" ? <ActGlyph size={16} /> : <OkGlyph size={16} />}
          <span className="flex-1 font-semibold">{d.name}</span>
          <span className="text-sm text-text-muted">
            {d.summary} · {d.checked}
          </span>
        </div>
      ))}
    </section>
  );
}

export function Home() {
  const { status, error } = useDevice();
  if (error) return <Failed error={error} />;
  if (!status) return <Loading />;
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap gap-6">
        <StatusCard status={status} />
        <ScoreCard />
      </div>
      {status.tuneUp.length > 0 && <TuneUp waiting={status.tuneUp} />}
      <div className="flex flex-wrap gap-6">
        <ThisDevice status={status} />
        <OtherDevices />
      </div>
    </div>
  );
}
