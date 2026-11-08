// Records Matt's verdicts from a review and makes the screens he saw the new
// baselines, so CI holds the app to exactly what he looked at.
//
//   node e2e/visual/approve.ts <verdicts.json>
//
// verdicts.json: {"by": "Matt", "review": "<review page url>",
//                 "verdicts": {"score-light": {"verdict": "ok", "note": ""}, ...}}
// Every screen in it must be in out/report.json from the same review run.
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { type Approval, approvalsFile, dir, outDir, readApprovals } from "./cases.ts";

interface Verdicts {
  by: string;
  review: string;
  verdicts: Record<string, { verdict: "ok" | "fix"; note?: string }>;
}
interface ReportRow {
  id: string;
  app: string;
  mock: string;
}

const file = process.argv[2];
if (!file) throw new Error("usage: node e2e/visual/approve.ts <verdicts.json>");
const input = JSON.parse(readFileSync(file, "utf8")) as Verdicts;
const report = JSON.parse(readFileSync(join(outDir, "report.json"), "utf8")) as ReportRow[];
const approvals = readApprovals();
const at = new Date().toISOString();

mkdirSync(join(dir, "approved"), { recursive: true });
for (const [id, v] of Object.entries(input.verdicts)) {
  const row = report.find((r) => r.id === id);
  if (!row) throw new Error(`${id} is not in out/report.json`);
  if (!["ok", "fix"].includes(v.verdict)) throw new Error(`${id}: verdict must be ok or fix`);
  // A "fix" verdict still becomes the baseline: CI then flags the next change
  // to this screen, which is the fix coming back for review.
  copyFileSync(join(outDir, `${id}.app.png`), join(dir, "approved", `${id}.png`));
  const approval: Approval = {
    verdict: v.verdict,
    note: v.note ?? "",
    mock: row.mock,
    app: row.app,
    by: input.by,
    at,
    review: input.review,
  };
  approvals[id] = approval;
}

const sorted = Object.fromEntries(Object.entries(approvals).sort(([a], [b]) => a.localeCompare(b)));
writeFileSync(approvalsFile, JSON.stringify(sorted, null, 2) + "\n");
const fixes = Object.entries(sorted).filter(([, a]) => a?.verdict === "fix");
console.log(`Recorded ${String(Object.keys(input.verdicts).length)} verdicts.`);
for (const [id, a] of fixes) console.log(`Needs fixing: ${id}: ${a?.note ?? ""}`);
