// The menu bar / tray popover (board D), the main day-to-day surface.
import { useQueryClient } from "@tanstack/react-query";
import { ActGlyph, Button, Card, Eyebrow, Lighthouse } from "@wardwatch/ui";
import { useEffect, useState } from "react";
import { useDevice, useGroupGlance, useScore } from "../lib/data";
import { showMainWindow } from "../lib/ipc";
import type { DeviceStatus } from "../lib/programs";
import { fixPath, kindLine, listNames } from "../lib/programs";
import { KindMark } from "../main/parts";
import { ordinal, worstSeverity } from "../lib/format";

const open = (route?: string) => () => {
  showMainWindow(route).catch((e: unknown) => {
    console.error("wardwatch: could not open the main window", e);
  });
};

/**
 * The popover is created once and only hidden and shown, so nothing remounts.
 * Refetch whenever it gets focus (the main window may have changed the plan or
 * holds), and re-render every 30 seconds so "Checked N minutes ago" moves.
 */
function useFreshOnFocus() {
  const client = useQueryClient();
  const [, tick] = useState(0);
  useEffect(() => {
    const refresh = () => void client.invalidateQueries();
    window.addEventListener("focus", refresh);
    const timer = setInterval(() => {
      tick((n) => n + 1);
    }, 30_000);
    return () => {
      window.removeEventListener("focus", refresh);
      clearInterval(timer);
    };
  }, [client]);
}

function minutesAgo(at: number) {
  const m = Math.floor((Date.now() - at) / 60_000);
  return m < 1 ? "Checked just now" : `Checked ${String(m)} ${m === 1 ? "minute" : "minutes"} ago`;
}

function TopCard({ status }: { status: DeviceStatus }) {
  const urgent = status.attack[0];
  if (urgent) {
    return (
      <Card tone="urgent" className="mx-3 flex flex-col gap-2.5 p-3.5">
        <Eyebrow className="flex items-center gap-2">
          <ActGlyph size={12} />
          Under attack now
        </Eyebrow>
        <div className="text-[15px] font-semibold">Update {urgent.name}</div>
        <div className="text-[13px]">
          <div>This is being exploited right now.</div>
          <div className="text-text-muted">1 minute to update. Tabs will come back.</div>
        </div>
        <div className="flex gap-2">
          <Button variant="act" className="flex-1" onClick={open(fixPath(urgent))}>
            Update now
          </Button>
          <Button variant="quiet" className="bg-surface" onClick={open(fixPath(urgent))}>
            Why?
          </Button>
        </div>
      </Card>
    );
  }
  if (status.tuneUp.length > 0) {
    return (
      <Card tone="inset" className="mx-3 flex flex-col gap-2.5 p-3.5">
        <Eyebrow>Sunday tune-up</Eyebrow>
        <div className="text-[15px] font-semibold">{listNames(status.tuneUp)}</div>
        <div className="text-[13px] text-text-muted">
          {status.quietAttack.length > 0
            ? "Attackers are using a hole in one of these, so do it soon."
            : "Routine updates, no rush."}{" "}
          We'll remind you Sunday morning.
        </div>
        <div className="flex gap-2">
          <Button
            variant="quiet"
            severity={worstSeverity(status.tuneUp)}
            className="flex-1 bg-surface"
            onClick={open("/fix/tune-up")}
          >
            Update now
          </Button>
          <Button variant="quiet" className="bg-surface" onClick={open("/apps")}>
            Details
          </Button>
        </div>
      </Card>
    );
  }
  return null;
}

function Checklist({ status }: { status: DeviceStatus }) {
  const rows = [
    ["System", "system"],
    ["Browsers", "browsers"],
    ["Apps", "apps"],
  ] as const;
  return (
    <ul className="m-0 mt-2 flex list-none flex-col border-t border-line px-4 py-1.5">
      {rows.map(([label, kind]) => {
        const line = kindLine(status, kind);
        return (
          <li key={label} className="flex items-center gap-2.5 py-1.5 text-[13px]">
            {/* A check only when the whole row is current, as on Home. */}
            <KindMark mark={line.mark} />
            <span className="flex-1">{label}</span>
            <span className="text-text-muted">{line.text}</span>
          </li>
        );
      })}
    </ul>
  );
}

function ScoreStrip() {
  const score = useScore().data;
  const glance = useGroupGlance();
  if (!score || score.score === null) return null;
  return (
    <Card tone="ok" as="div" className="mx-3 flex items-center gap-3 p-3">
      <span className="ww-display text-[26px]">{score.score}</span>
      <div className="flex-1 text-[13px]">
        <div className="font-semibold text-ok">{score.band}</div>
        {glance?.rank != null && (
          <div className="text-text-muted">
            {ordinal(glance.rank)} in {glance.group.name}
          </div>
        )}
      </div>
      <button
        type="button"
        className="min-h-11 cursor-pointer border-0 bg-transparent text-[13px] font-semibold text-link underline"
        onClick={open("/score")}
      >
        Score
      </button>
    </Card>
  );
}

export function TrayApp() {
  useFreshOnFocus();
  const { status, error, checkedAt } = useDevice();
  const headline = !status
    ? "Wardwatch"
    : status.attack.length > 0
      ? status.attack.length === 1
        ? "One thing needs you"
        : `${String(status.attack.length)} things need you`
      : status.quietAttack.length > 0
        ? "Worth doing soon"
        : status.checked
          ? "All clear"
          : "Not checked yet";
  const item =
    "flex min-h-11 w-full cursor-pointer items-center rounded-lg border-0 bg-transparent px-2 text-left text-sm text-text hover:bg-sel";

  return (
    <div className="flex min-h-screen flex-col bg-surface">
      <div className="flex items-center gap-3 px-4 pb-3 pt-4">
        <Lighthouse beams={(status?.attack.length ?? 0) > 0} size={40} />
        <div className="flex-1">
          <h1 className="text-lg">{headline}</h1>
          <div className="text-xs text-text-muted">
            {error
              ? `Couldn't check this device: ${error.message}`
              : status
                ? minutesAgo(checkedAt)
                : "Checking"}
          </div>
        </div>
      </div>
      {status && (
        <>
          <TopCard status={status} />
          <Checklist status={status} />
        </>
      )}
      <ScoreStrip />
      <div className="mt-2 border-t border-line p-2">
        <button type="button" className={item} onClick={open()}>
          Open Wardwatch
        </button>
        <button type="button" className={item} onClick={open("/plan")}>
          Update plan
        </button>
      </div>
    </div>
  );
}
