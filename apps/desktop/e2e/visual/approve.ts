// Records Matt's verdicts from a review and makes the screens he saw the new
// baselines, so CI holds the app to exactly what he looked at.
//
//   node e2e/visual/approve.ts <verdicts.json>
//
// verdicts.json: {"by": "Matt", "review": "<review page url>",
//                 "verdicts": {"score-light": {"verdict": "ok", "note": ""}, ...}}
// Every screen in it must have an out/<id>.json from a review of the current commit.
import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { type Approval, approvalsFile, dir, outDir, readApprovals } from "./cases.ts";

interface Verdicts {
  by?: string;
  review?: string;
  verdicts?: Record<string, { verdict?: string; note?: string }>;
}
interface ReportRow {
  id: string;
  commit: string;
  app: string;
  mock: string;
}

const file = process.argv[2];
if (!file) throw new Error("usage: node e2e/visual/approve.ts <verdicts.json>");
const input = JSON.parse(readFileSync(file, "utf8")) as Verdicts;
const { by, review, verdicts } = input;
if (!by || !review || !verdicts) throw new Error("verdicts.json needs by, review and verdicts");
const head = execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();

// Check everything before touching approved/ or approvals.json.
const rows = Object.entries(verdicts).map(
  ([id, v]): Pick<Approval, "verdict" | "note"> & { row: ReportRow } => {
    const rowFile = join(outDir, `${id}.json`);
    if (!existsSync(rowFile)) throw new Error(`${id} is not in out/; run pnpm visual:review`);
    const row = JSON.parse(readFileSync(rowFile, "utf8")) as ReportRow;
    if (row.id !== id) throw new Error(`out/${id}.json describes ${row.id}`);
    if (row.commit !== head) {
      throw new Error(`out/${id}.json was rendered at ${row.commit}, not HEAD; review again`);
    }
    const verdict = v.verdict;
    if (verdict !== "ok" && verdict !== "fix") throw new Error(`${id}: verdict must be ok or fix`);
    return { row, verdict, note: v.note ?? "" };
  },
);

const approvals = readApprovals();
const at = new Date().toISOString();
mkdirSync(join(dir, "approved"), { recursive: true });
for (const { row, verdict, note } of rows) {
  // A "fix" verdict still becomes the baseline: CI then flags the next change
  // to this screen, which is the fix coming back for review.
  copyFileSync(join(outDir, `${row.id}.app.png`), join(dir, "approved", `${row.id}.png`));
  const approval: Approval = { verdict, note, mock: row.mock, app: row.app, by, at, review };
  approvals[row.id] = approval;
}

const sorted = Object.fromEntries(Object.entries(approvals).sort(([a], [b]) => a.localeCompare(b)));
writeFileSync(approvalsFile, JSON.stringify(sorted, null, 2) + "\n");
console.log(`Recorded ${String(rows.length)} verdicts.`);
for (const { row, verdict, note } of rows) {
  if (verdict === "fix") console.log(`Needs fixing: ${row.id}: ${note}`);
}
