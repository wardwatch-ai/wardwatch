// Apps (Apps 3): this device's programs as a list and detail. The detail corner
// shows exactly one thing for each state (design spec, Apps).
import { Button, Card, Eyebrow, StatusPill } from "@wardwatch/ui";
import { useDeferredValue, useId, useState } from "react";
import { useDevice, usePlan, useSavePlan } from "../../lib/data";
import type { FeedSeverity, Hold } from "../../lib/model";
import { type Program, meterLevel } from "../../lib/programs";
import { Failed, Loading, UpdateButton } from "../parts";

/** Rows rendered at once; Linux lists thousands of packages, so search narrows the rest. */
const LIST_LIMIT = 150;

const short: Record<Program["state"], string> = {
  urgent: "Now",
  update: "Update",
  held: "Held",
  current: "Current",
  unknown: "",
};

/** Today's local date as YYYY-MM-DD (toISOString would give the UTC date). */
const today = () => {
  const d = new Date();
  return `${String(d.getFullYear())}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
};

function useHolds() {
  const plan = usePlan().data;
  const save = useSavePlan();
  const set = (holds: Hold[]) => {
    if (plan) save.mutate({ ...plan, holds });
  };
  return {
    hold: (p: Program) => {
      const holds = plan?.holds ?? [];
      if (holds.some((h) => h.source === p.source && h.id === p.id)) return;
      set([...holds, { source: p.source, id: p.id, since: today() }]);
    },
    release: (p: Program) => {
      set((plan?.holds ?? []).filter((h) => !(h.source === p.source && h.id === p.id)));
    },
    error: save.error,
  };
}

function Corner({ p, onRelease }: { p: Program; onRelease: () => void }) {
  switch (p.state) {
    case "urgent":
    case "update":
      return <UpdateButton program={p} />;
    case "held":
      return (
        <Button variant="quiet" onClick={onRelease}>
          Release hold
        </Button>
      );
    case "current":
      return <StatusPill tone="ok">Up to date</StatusPill>;
    case "unknown":
      return <span className="text-sm text-text-muted">Not checked yet</span>;
  }
}

const sevWords = { 3: "Serious", 2: "Moderate", 1: "Minor" } as const;
const sevText = (s: FeedSeverity | null, exploited: boolean) => {
  const word = s === null || s === "unknown" ? "Unrated" : sevWords[meterLevel(s)];
  return exploited ? `${word}, exploited` : word;
};

function Detail({ p }: { p: Program }) {
  const holds = useHolds();
  const closes =
    p.state === "current"
      ? "Nothing open"
      : p.vulns.length === 0
        ? "No security fixes"
        : `${String(p.vulns.length)} ${p.vulns.length === 1 ? "hole" : "holes"}`;
  const verdict =
    p.state === "urgent"
      ? "Being exploited right now"
      : p.state === "held"
        ? `Held by you since ${p.heldSince ?? "earlier"}`
        : p.state === "current"
          ? "No update to install"
          : p.vulns.length > 0
            ? "Not known to be exploited"
            : "Bug fixes and features only";

  return (
    <div className="flex min-w-0 flex-[999_1_560px] flex-col gap-5">
      <div className="flex flex-wrap items-center gap-4">
        <div className="min-w-0 flex-1">
          <h1 className="text-3xl">{p.name}</h1>
          <div className="text-sm text-text-muted">
            {p.vendor ? `${p.vendor} · ` : ""}
            <span className="ww-mono">{p.version}</span>
          </div>
        </div>
        <Corner
          p={p}
          onRelease={() => {
            holds.release(p);
          }}
        />
      </div>
      {holds.error && <p role="alert">Couldn't save that: {holds.error.message}</p>}

      {p.state === "unknown" ? (
        <Card>
          <p className="m-0 text-text-muted">
            We haven't checked {p.name} against the update feed yet, so we can't say whether it's up
            to date.
          </p>
        </Card>
      ) : (
        <>
          <Card className="flex flex-col gap-3.5">
            <Eyebrow>How we got here</Eyebrow>
            <div className="flex flex-wrap items-stretch gap-3">
              <Card tone="inset" as="div" className="flex-1 basis-[150px]">
                <Eyebrow>You have</Eyebrow>
                <div className="ww-mono mt-1 text-base">{p.version}</div>
              </Card>
              <span aria-hidden="true" className="self-center text-xl text-text-muted">
                →
              </span>
              <Card tone="inset" as="div" className="flex-1 basis-[150px]">
                <Eyebrow>Newest</Eyebrow>
                <div className="ww-mono mt-1 text-base">{p.latest ?? "Unknown"}</div>
              </Card>
              <span aria-hidden="true" className="self-center text-xl text-text-muted">
                →
              </span>
              <Card
                tone={p.state === "urgent" ? "urgent" : "inset"}
                as="div"
                className="flex-1 basis-[150px]"
              >
                <Eyebrow>It closes</Eyebrow>
                <div className="mt-1 font-semibold">{closes}</div>
                <div
                  className={`text-[13px] ${p.state === "urgent" ? "text-act-ink" : "text-text-muted"}`}
                >
                  {verdict}
                </div>
              </Card>
            </div>
          </Card>

          <Card className="flex flex-col gap-2">
            <Eyebrow>The holes, in detail</Eyebrow>
            {p.vulns.length === 0 ? (
              <p className="m-0 text-sm text-text-muted">
                {p.state === "current"
                  ? "No known holes in the version you have."
                  : "No security fixes in this update."}
              </p>
            ) : (
              <ul className="m-0 flex list-none flex-col p-0">
                {p.vulns.map((v) => (
                  <li
                    key={v.id}
                    className="flex flex-wrap gap-x-4 gap-y-1 border-b border-line py-2.5 last:border-0"
                  >
                    <span className="ww-mono min-w-[150px]">{v.id}</span>
                    <span className="flex-1">{v.summary}</span>
                    <span
                      className={v.exploited ? "font-semibold text-act-ink" : "text-text-muted"}
                    >
                      {sevText(v.severity, v.exploited)}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </Card>

          {p.state === "urgent" && p.heldSince && (
            <Card className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <div className="font-semibold">Held by you since {p.heldSince}</div>
                <div className="text-sm text-text-muted">
                  We still alert you while it's being exploited.
                </div>
              </div>
              <Button
                variant="quiet"
                onClick={() => {
                  holds.release(p);
                }}
              >
                Release hold
              </Button>
            </Card>
          )}
          {(p.state === "update" || p.state === "urgent") && !p.heldSince && (
            <Card className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <div className="font-semibold">Need this version for a while?</div>
                <div className="text-sm text-text-muted">
                  A held program still counts toward your score
                  {p.state === "urgent"
                    ? ", and we still alert you while it's being exploited."
                    : "."}
                </div>
              </div>
              <Button
                variant="quiet"
                onClick={() => {
                  holds.hold(p);
                }}
              >
                Hold
              </Button>
            </Card>
          )}
        </>
      )}
    </div>
  );
}

export function Apps() {
  const { programs, status, inventory, error } = useDevice();
  const [query, setQuery] = useState("");
  const [picked, setPicked] = useState<string>();
  const q = useDeferredValue(query.trim().toLowerCase());
  const searchId = useId();
  if (error) return <Failed error={error} />;
  if (!programs || !status || !inventory) return <Loading />;

  const matches = q ? programs.filter((p) => p.name.toLowerCase().includes(q)) : programs;
  const shown = matches.slice(0, LIST_LIMIT);
  const selected = programs.find((p) => p.key === picked) ?? shown[0];
  // Only when every program was checked: an unknown one is not up to date, just unchecked.
  const allClear = status.checked && status.current === status.total;

  return (
    <div className="flex flex-col gap-5">
      {allClear && (
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h1 className="text-2xl">All {status.total} programs</h1>
            <div className="text-sm text-text-muted">On this device</div>
          </div>
          <StatusPill tone="ok">All up to date</StatusPill>
        </div>
      )}
      {!status.checked && (
        <p className="m-0 text-sm text-text-muted">
          Showing what we found on this device. Update checks start once the update feed is
          connected.
        </p>
      )}
      <div className="flex flex-wrap items-start gap-6">
        <Card className="flex min-w-0 flex-[1_1_300px] flex-col gap-1 sm:max-w-[360px]">
          <label htmlFor={searchId} className="ww-sr-only">
            Search programs
          </label>
          <input
            id={searchId}
            type="search"
            placeholder={`Search ${String(programs.length)} programs`}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
            }}
            className="mb-2 min-h-11 rounded-[10px] border border-edge bg-bg px-3 text-text"
          />
          <ul className="m-0 flex list-none flex-col gap-0.5 p-0">
            {shown.map((p) => (
              <li key={p.key}>
                <button
                  type="button"
                  aria-current={p.key === selected?.key ? "true" : undefined}
                  onClick={() => {
                    setPicked(p.key);
                  }}
                  className={`flex min-h-11 w-full cursor-pointer items-center gap-3 rounded-[10px] border-0 px-2.5 py-1.5 text-left text-text ${
                    p.key === selected?.key ? "bg-sel" : "bg-transparent hover:bg-sel"
                  }`}
                >
                  <span className="flex min-w-0 flex-1 flex-col">
                    <span className="truncate font-semibold">{p.name}</span>
                    <span className="ww-mono truncate text-text-muted">{p.version}</span>
                  </span>
                  <span
                    className={`text-[13px] font-semibold ${
                      p.state === "urgent"
                        ? "text-act-ink"
                        : p.state === "current"
                          ? "text-ok"
                          : "text-text-muted"
                    }`}
                  >
                    {short[p.state]}
                  </span>
                </button>
              </li>
            ))}
          </ul>
          {matches.length > shown.length && (
            <p className="m-0 px-2.5 pt-2 text-sm text-text-muted">
              And {matches.length - shown.length} more. Search to find them.
            </p>
          )}
          {matches.length === 0 && (
            <p className="m-0 px-2.5 text-sm text-text-muted">No programs match.</p>
          )}
          {inventory.warnings.length > 0 && (
            <p className="m-0 px-2.5 pt-2 text-xs text-text-muted">
              Some places couldn't be read: {inventory.warnings.join("; ")}
            </p>
          )}
        </Card>
        {selected && <Detail key={selected.key} p={selected} />}
      </div>
    </div>
  );
}
