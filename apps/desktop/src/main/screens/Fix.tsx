// The fix screen (design spec: "Fix screen · Chrome"). It explains the hole in
// plain words, shows exactly what Wardwatch will do and who checks the
// download, and only then offers the button. Progress comes from the core.
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ActGlyph, Button, Card, CheckGlyph, Eyebrow, OkGlyph } from "@wardwatch/ui";
import { useState } from "react";
import { useParams } from "react-router";
import {
  type Finding,
  type FixProgress,
  type FixResult,
  type Plan,
  getFix,
  runFix,
} from "../../lib/ipc";
import {
  actionLabel,
  adminLine,
  capitalize,
  crumbFor,
  doneLede,
  foundAgo,
  happening,
  headline,
  knownUsed,
  lede,
  meter,
  risk,
  shellLine,
  worstVuln,
} from "./fixCopy";
import { BackBar } from "../parts";

type StepState = "idle" | "working" | "done";

export function Fix() {
  const { source = "", id = "" } = useParams();
  const queryClient = useQueryClient();
  const fix = useQuery({ queryKey: ["fix", source, id], queryFn: () => getFix(source, id) });
  const [steps, setSteps] = useState<StepState[]>([]);
  const [diy, setDiy] = useState(false);

  const run = useMutation({
    mutationFn: () => {
      setSteps([]);
      if (!fix.data?.plan) throw new Error("There's no update plan to run.");
      return runFix(source, id, fix.data.plan.method, (p: FixProgress) => {
        setSteps((prev) => {
          const next = [...prev];
          next[p.step] = p.state;
          return next;
        });
      });
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["findings"] }),
  });

  if (fix.isPending) return <Shell crumb="Loading">Checking this program…</Shell>;
  if (fix.isError) {
    return (
      <Shell crumb="Fix">
        <h1 className="font-display text-3xl">We couldn't look at this program.</h1>
        <p className="mt-3 text-text-muted">{String(fix.error)}</p>
      </Shell>
    );
  }

  const { finding, plan, no_plan } = fix.data;
  const result = run.data;
  const done =
    result?.outcome.status === "installed" &&
    (result.check === "closed" || result.check === "only_unfixed_left");
  // A step that was working when the run stopped is no longer working.
  const shownSteps = run.isPending ? steps : steps.map((s) => (s === "working" ? "idle" : s));
  const urgent = finding.exploited && !done;

  return (
    <Shell crumb={`${crumbFor(finding)} · ${finding.app_name}`}>
      <div className="flex flex-wrap items-start gap-8">
        <div className="flex min-w-0 flex-[999_1_560px] flex-col gap-7">
          <header className="flex flex-col gap-3.5">
            {done ? (
              <Badge tone="ok">
                <OkGlyph /> Fixed just now
              </Badge>
            ) : (
              finding.exploited && (
                <Badge tone="act">
                  <ActGlyph /> Under attack now{foundAgo(finding)}
                </Badge>
              )
            )}
            <h1 className="font-display text-[38px] leading-tight">
              {done ? `${finding.app_name} is safe again.` : headline(finding)}
            </h1>
            <p className="m-0 max-w-[620px] text-lg">
              {done ? doneLede(finding) : lede(finding, plan)}
            </p>
          </header>

          {!done && finding.vulns.length > 0 && (
            <div className="grid gap-4 sm:grid-cols-2">
              <Card>
                <h2 className="mb-1.5 font-display text-base">What's happening</h2>
                <p className="m-0 text-sm">{happening(finding)}</p>
              </Card>
              <Card>
                <h2 className="mb-1.5 font-display text-base">What could happen to you</h2>
                <p className="m-0 text-sm">{risk(finding)}</p>
              </Card>
            </div>
          )}

          {plan ? (
            <PlanCard plan={plan} steps={shownSteps} />
          ) : (
            <Card>
              <p className="m-0">
                {no_plan?.kind === "unsupported"
                  ? no_plan.why
                  : `${finding.app_name} is up to date.`}
              </p>
            </Card>
          )}

          {plan && !run.isPending && !done && (
            <Actions
              finding={finding}
              plan={plan}
              urgent={urgent}
              retry={
                result !== undefined &&
                ["failed", "cancelled", "nothing_newer", "needs_more_changes"].includes(
                  result.outcome.status,
                )
              }
              onRun={() => {
                run.mutate();
              }}
              diy={diy}
              onDiy={() => {
                setDiy(!diy);
              }}
            />
          )}
          {run.isPending && (
            <p className="m-0 text-text-muted" role="status">
              Working. You can close this window; the update carries on.
            </p>
          )}
          {run.isError && <Problem title="The update couldn't start.">{String(run.error)}</Problem>}
          {result && (
            <ResultCard
              finding={finding}
              plan={plan}
              result={result}
              onCheck={() => {
                void fix.refetch();
              }}
            />
          )}
        </div>

        <aside className="flex min-w-0 flex-[1_1_320px] flex-col gap-4">
          <Curious finding={finding} />
          {plan && <Trust plan={plan} />}
        </aside>
      </div>
    </Shell>
  );
}

function Shell({ crumb, children }: { crumb: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col">
      <div className="border-b border-line px-8 py-4">
        <BackBar>
          <span className="text-sm text-text-muted">{crumb}</span>
        </BackBar>
      </div>
      <section
        aria-live="polite"
        className="mx-auto box-border w-full max-w-[1160px] px-4 py-9 sm:px-8"
      >
        {children}
      </section>
    </div>
  );
}

function Badge({ tone, children }: { tone: "act" | "ok"; children: React.ReactNode }) {
  const colors = tone === "act" ? "bg-act-tint text-act-ink" : "bg-ok-tint text-ok";
  return (
    <span
      className={`inline-flex items-center gap-2 self-start rounded-full px-3 py-1 text-[13px] font-semibold ${colors}`}
    >
      {children}
    </span>
  );
}

function PlanCard({ plan, steps }: { plan: Plan; steps: StepState[] }) {
  if (!plan.one_click) {
    return (
      <Card>
        <h2 className="font-display text-lg">What to do</h2>
        <p className="mt-2 mb-0">{plan.manual}</p>
        {plan.restart_note && (
          <p className="mt-2 mb-0 text-sm text-text-muted">{plan.restart_note}</p>
        )}
      </Card>
    );
  }
  return (
    <Card>
      <div className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="font-display text-lg">What Wardwatch will do</h2>
        <span className="text-sm text-text-muted">{adminLine(plan)}</span>
      </div>
      <ol className="mt-3 mb-0 list-none p-0">
        {plan.steps.map((s, i) => {
          const state = steps[i] ?? "idle";
          return (
            <li key={s.label} className="flex items-start gap-3 py-2.5">
              <span
                className={`box-border flex size-[26px] flex-none items-center justify-center rounded-full border-2 text-xs font-semibold ${
                  state === "done"
                    ? "border-ok text-ok"
                    : state === "working"
                      ? "border-text text-text"
                      : "border-edge text-text-muted"
                }`}
              >
                {state === "done" ? <CheckGlyph size={14} /> : i + 1}
              </span>
              <div className="min-w-0 flex-1">
                <div className="font-semibold">{s.label}</div>
                <div className="text-[13px] text-text-muted">{s.detail}</div>
              </div>
              <span className="text-[13px] whitespace-nowrap text-text-muted">
                {state === "done" ? "Done" : state === "working" ? "Working…" : ""}
              </span>
            </li>
          );
        })}
      </ol>
      {plan.restart_note && (
        <p className="mt-2 mb-0 text-sm text-text-muted">{plan.restart_note}</p>
      )}
    </Card>
  );
}

function Actions(props: {
  finding: Finding;
  plan: Plan;
  urgent: boolean;
  retry: boolean;
  onRun: () => void;
  diy: boolean;
  onDiy: () => void;
}) {
  const { finding, plan, urgent, retry, onRun, diy, onDiy } = props;
  const label = retry ? "Try again" : actionLabel(finding, plan);
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-3">
        {urgent ? (
          <Button variant="act" onClick={onRun}>
            {label}
          </Button>
        ) : (
          <Button variant="quiet" severity={meter(finding.severity)} onClick={onRun}>
            {label}
          </Button>
        )}
        {plan.method.kind === "command" && (
          <button type="button" className="ww-btn ww-btn-quiet" aria-expanded={diy} onClick={onDiy}>
            I'd rather do it myself
          </button>
        )}
      </div>
      {diy && plan.method.kind === "command" && (
        <Card tone="inset">
          <p className="m-0 text-sm">
            Run this yourself in a terminal. It's exactly what Wardwatch would run:
          </p>
          <pre className="mt-2 mb-0 overflow-x-auto font-mono text-[12.5px] whitespace-pre-wrap">
            {shellLine(plan.method.argv)}
          </pre>
        </Card>
      )}
    </div>
  );
}

function ResultCard(props: {
  finding: Finding;
  plan: Plan | null;
  result: FixResult;
  onCheck: () => void;
}) {
  const { finding, plan, result, onCheck } = props;
  const name = finding.app_name;
  const o = result.outcome;
  const done = (r: FixResult) => r.check === "closed" || r.check === "only_unfixed_left";
  switch (o.status) {
    case "installed":
      return done(result) ? (
        <Card tone="ok" className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <div className="font-semibold text-ok">
              {name} is up to date. Your lighthouse is dark again.
            </div>
            {plan?.restart_note && (
              <div className="text-sm">Restart {name} to start the new version.</div>
            )}
          </div>
          <Button variant="brand" href="#/">
            Done
          </Button>
        </Card>
      ) : (
        <Problem
          title={
            result.check === "not_found"
              ? `The update ran, but we couldn't find ${name} again to check it.`
              : "The update ran, but the hole is still open."
          }
        >
          {result.check === "not_found"
            ? "Open it once, then come back here and we'll check again."
            : `The fixed version (${finding.fixed_version ?? "newer"}) may not be out for your system yet. We'll keep checking and let you know.`}
        </Problem>
      );
    case "opened":
      return (
        <Card>
          <p className="m-0">{plan?.manual}</p>
          <div className="mt-3">
            <Button variant="quiet" onClick={onCheck}>
              I've updated it, check again
            </Button>
          </div>
        </Card>
      );
    case "cancelled":
      return (
        <Problem title="You closed the password prompt.">
          Nothing changed. Press Try again when you're ready.
        </Problem>
      );
    case "nothing_newer":
      return (
        <Problem
          title={`${plan?.by ?? "The updater"} doesn't have the fixed version yet.`}
          tail={o.tail}
        >
          It usually arrives within a day. You can also use {name}'s own updater.
        </Problem>
      );
    case "needs_more_changes":
      return (
        <Problem title="This update needs bigger changes than Wardwatch makes." tail={o.tail}>
          It would remove other packages, so we stopped. Nothing changed. Use your system's Software
          Updater, which shows every change before making it.
        </Problem>
      );
    case "failed":
      return (
        <Problem title="The update didn't finish." tail={o.tail}>
          {capitalize(o.message)}. Nothing else on your device was changed.
        </Problem>
      );
  }
}

function Problem({
  title,
  tail,
  children,
}: {
  title: string;
  tail?: string[];
  children: React.ReactNode;
}) {
  return (
    <Card role="alert">
      <div className="font-semibold">{title}</div>
      <p className="mt-1 mb-0 text-sm">{children}</p>
      {tail && tail.length > 0 && (
        <details className="mt-2 text-sm">
          <summary className="cursor-pointer">Details</summary>
          <pre className="mt-2 mb-0 overflow-x-auto font-mono text-xs whitespace-pre-wrap">
            {tail.join("\n")}
          </pre>
        </details>
      )}
    </Card>
  );
}

function Curious({ finding }: { finding: Finding }) {
  const worst = worstVuln(finding);
  const rows: [string, React.ReactNode][] = [
    ["Your version", <Mono key="v">{finding.installed}</Mono>],
    ...(finding.fixed_version
      ? [["Fixed in", <Mono key="f">{finding.fixed_version}</Mono>] as [string, React.ReactNode]]
      : []),
    ...(finding.latest && finding.latest !== finding.fixed_version
      ? [["Newest", <Mono key="l">{finding.latest}</Mono>] as [string, React.ReactNode]]
      : []),
    ...(finding.vulns.length > 0
      ? [
          [
            finding.vulns.length === 1 ? "Issue" : "Issues",
            <Mono key="i">{finding.vulns.map((v) => v.id).join(", ")}</Mono>,
          ] as [string, React.ReactNode],
        ]
      : []),
    ...(worst
      ? [
          [
            "Severity",
            `${capitalize(finding.severity ?? "unknown")}${worst.cvss != null ? ` · ${String(worst.cvss)} of 10` : ""}`,
          ] as [string, React.ReactNode],
        ]
      : []),
    ...(finding.vulns.length > 0
      ? [["Known used in attacks", knownUsed(finding)] as [string, React.ReactNode]]
      : []),
  ];
  return (
    <Card>
      <Eyebrow className="mb-2">For the curious</Eyebrow>
      <dl className="m-0">
        {rows.map(([k, v]) => (
          <div key={k} className="flex justify-between gap-3 border-t border-line py-2 text-[13px]">
            <dt className="text-text-muted">{k}</dt>
            <dd className="m-0 text-right">{v}</dd>
          </div>
        ))}
      </dl>
      {worst && <p className="mt-3 mb-0 text-[13px]">{worst.summary}</p>}
    </Card>
  );
}

function Trust({ plan }: { plan: Plan }) {
  return (
    <Card>
      <Eyebrow className="mb-2">Why you can trust this fix</Eyebrow>
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-[18px] text-[13px]">
        <li>{plan.verifies}</li>
        <li>Wardwatch never downloads or installs anything itself. {plan.by} does.</li>
        {plan.admin !== "no" && (
          <li>Anything that needs admin rights goes through your system's own prompt.</li>
        )}
        <li>We don't touch your settings, bookmarks, passwords or files.</li>
      </ul>
      {plan.method.kind === "command" && (
        <div className="mt-3 rounded-sm bg-code px-3 py-2.5">
          <div className="text-[12px] text-text-muted">Exactly what will run</div>
          <code className="font-mono text-[12.5px] break-words">{shellLine(plan.method.argv)}</code>
        </div>
      )}
    </Card>
  );
}

function Mono({ children }: { children: React.ReactNode }) {
  return <span className="font-mono text-[12.5px]">{children}</span>;
}
