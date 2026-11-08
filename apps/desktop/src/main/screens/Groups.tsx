// Groups (Groups 4): group picker, period switch, mini podium, leaderboard,
// most improved and the group's own monthly stakes. Members see scores, bands
// and fix speed only; never apps, versions or device names.
import { Button, Card, Eyebrow, SegmentedControl } from "@wardwatch/ui";
import { useId, useState, type SyntheticEvent } from "react";
import {
  accounts,
  useGroup,
  useGroupWrite,
  useGroups,
  useLeaderboard,
  useStakeIdeas,
} from "../../lib/data";
import type { GroupDetail, Leaderboard, LeaderboardRow, Period } from "../../lib/model";
import { Failed, Loading } from "../parts";

const periods: readonly { value: Period; label: string }[] = [
  { value: "30d", label: "Last 30 days" },
  { value: "month", label: "This month" },
  { value: "joined", label: "Since joining" },
];

const periodCopy: Record<Period, { title: string; note: string; column: string }> = {
  "30d": {
    title: "Last 30 days",
    note: "Rolling 30 days. This is the score on everyone's home screen.",
    column: "Score",
  },
  month: {
    title: "This month so far",
    note: "Only counts fixes since the 1st, so a fresh start every month.",
    column: "This month",
  },
  joined: {
    title: "Since joining",
    note: "Everything since each person joined the group.",
    column: "All time",
  },
};

const monthName = (month: string) =>
  new Date(`${month}-01T12:00:00Z`).toLocaleString("en-US", { month: "long", timeZone: "UTC" });

const errorText = (e: unknown) => (e instanceof Error ? e.message : "Something went wrong.");

/** A one-field form (Start a group, Join with a code, Change stakes). */
function InlineForm({
  label,
  submit,
  initial = "",
  maxLength,
  onDone,
  run,
}: {
  label: string;
  submit: string;
  initial?: string;
  maxLength: number;
  onDone: () => void;
  run: (value: string) => Promise<unknown>;
}) {
  const id = useId();
  const [value, setValue] = useState(initial);
  const write = useGroupWrite(run);
  const send = (e: SyntheticEvent) => {
    e.preventDefault();
    write.mutate(value, { onSuccess: onDone });
  };
  return (
    <form onSubmit={send} className="flex flex-col gap-2">
      <label htmlFor={id} className="text-sm font-semibold">
        {label}
      </label>
      <input
        id={id}
        value={value}
        onChange={(e) => {
          setValue(e.target.value);
        }}
        className="min-h-11 rounded-[10px] border border-edge bg-surface px-3 text-text"
        maxLength={maxLength}
        required
      />
      {write.isError && (
        <p role="alert" className="m-0 text-sm">
          {errorText(write.error)}
        </p>
      )}
      <div className="flex gap-2">
        <Button variant="brand" type="submit" disabled={write.isPending}>
          {submit}
        </Button>
        <Button variant="quiet" onClick={onDone}>
          Cancel
        </Button>
      </div>
    </form>
  );
}

function Picker({
  selected,
  onSelect,
}: {
  selected: string | undefined;
  onSelect: (id: string) => void;
}) {
  const groups = useGroups().data ?? [];
  const [form, setForm] = useState<"start" | "join" | null>(null);
  return (
    <nav aria-label="Your groups" className="flex flex-[1_1_220px] flex-col gap-1 sm:max-w-[250px]">
      <Eyebrow className="px-3 pb-2">Your groups</Eyebrow>
      {groups.map((g) => (
        <button
          key={g.id}
          type="button"
          aria-current={g.id === selected ? "true" : undefined}
          onClick={() => {
            onSelect(g.id);
          }}
          className={`flex min-h-[52px] w-full cursor-pointer flex-col rounded-[10px] border-0 px-3 py-2 text-left font-semibold text-text ${
            g.id === selected ? "bg-sel" : "bg-transparent hover:bg-sel"
          }`}
        >
          {g.name}
          <span className="text-xs font-normal text-text-muted">
            {g.member_count} {g.member_count === 1 ? "person" : "people"}
          </span>
        </button>
      ))}
      <div className="my-2.5 h-px bg-line" />
      {form === "start" ? (
        <InlineForm
          label="Group name"
          maxLength={60}
          submit="Start"
          onDone={() => {
            setForm(null);
          }}
          run={async (name) => {
            onSelect((await accounts.createGroup(name)).id);
          }}
        />
      ) : form === "join" ? (
        <InlineForm
          label="Invite code"
          maxLength={32}
          submit="Join"
          onDone={() => {
            setForm(null);
          }}
          run={async (code) => {
            onSelect((await accounts.joinGroup(code)).id);
          }}
        />
      ) : (
        <>
          <Button
            variant="quiet"
            onClick={() => {
              setForm("start");
            }}
          >
            Start a group
          </Button>
          <Button
            variant="quiet"
            className="border-transparent"
            onClick={() => {
              setForm("join");
            }}
          >
            Join with a code
          </Button>
        </>
      )}
    </nav>
  );
}

// Fixed per person, so an avatar keeps its color when the ranking changes.
const avatarColors = ["#2F6F62", "#A23B2C", "#6B6358", "#8A5300"];
const avatarColor = (id: string) => {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) >>> 0;
  return avatarColors[h % avatarColors.length];
};

function Podium({ rows }: { rows: LeaderboardRow[] }) {
  const ranked = rows.filter((r) => r.rank !== null).slice(0, 3);
  if (ranked.length < 2) return null;
  // Second, first, third, like a real podium.
  const order = [ranked[1], ranked[0], ranked[2]];
  // Each step has an edge so it stands out from the card (3:1 in both themes), and
  // its label colour is picked for its own fill: band-3 is light in dark mode too.
  const steps = [
    "h-[76px] bg-band-2 text-text",
    "h-[96px] bg-band-3 text-on-act",
    "h-[62px] bg-band-1 text-text",
  ];
  return (
    <ol
      aria-label="Top three"
      className="m-0 grid list-none grid-cols-[repeat(3,84px)] items-end gap-1.5 p-0"
    >
      {order.map((r, i) =>
        r ? (
          <li
            key={r.user_id}
            className={`flex flex-col items-center justify-end gap-0.5 rounded-t-[10px] border border-b-0 border-edge px-1 py-2 ${steps[i] ?? ""}`}
          >
            <span
              aria-hidden="true"
              className="flex size-[30px] items-center justify-center rounded-full text-[13px] font-semibold text-white"
              style={{ background: avatarColor(r.user_id) }}
            >
              {r.name.charAt(0).toUpperCase()}
            </span>
            <span className="text-xs font-semibold">
              {r.rank} · {r.you ? "You" : r.name}
            </span>
          </li>
        ) : (
          <li key="empty" />
        ),
      )}
    </ol>
  );
}

function Table({ board, period }: { board: Leaderboard; period: Period }) {
  const th =
    "border-b border-line px-3 py-2.5 text-left text-xs font-semibold uppercase tracking-[0.04em] text-text-muted";
  const td = "border-b border-line p-3";
  return (
    <Card className="overflow-x-auto p-0">
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr>
            <th scope="col" className={th}>
              #
            </th>
            <th scope="col" className={th}>
              Member
            </th>
            <th scope="col" className={th}>
              {periodCopy[period].column}
            </th>
            <th scope="col" className={th}>
              Band
            </th>
            <th scope="col" className={th}>
              Fix speed
            </th>
          </tr>
        </thead>
        <tbody>
          {board.members.map((m) => (
            <tr key={m.user_id} className={m.you ? "bg-ok-tint" : undefined}>
              <td className={`${td} ww-display`}>{m.rank ?? "–"}</td>
              <td className={`${td} font-semibold`}>{m.you ? `${m.name} (you)` : m.name}</td>
              <td className={`${td} ww-display text-lg`}>{m.score ?? "No score yet"}</td>
              <td
                className={`${td} font-semibold ${m.score !== null && m.score >= 75 ? "text-ok" : "text-text-muted"}`}
              >
                {m.band ?? "–"}
              </td>
              <td className={td}>{m.fix_speed ?? "–"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </Card>
  );
}

function Stakes({ group, board }: { group: GroupDetail; board: Leaderboard }) {
  const [editing, setEditing] = useState(false);
  const [ideasOpen, setIdeasOpen] = useState(false);
  const ideas = useStakeIdeas(ideasOpen).data;
  const toggle = useGroupWrite((on: boolean) => accounts.setStakesEnabled(group.id, on));
  const owner = group.role === "owner";
  const month = board.stake ? monthName(board.stake.month) : "This month";

  if (!group.stakes_enabled) {
    return (
      <Card className="flex flex-col gap-2">
        <Eyebrow>Stakes</Eyebrow>
        <p className="m-0 text-text-muted">This group plays without stakes.</p>
        {owner && (
          <div>
            <Button
              variant="quiet"
              onClick={() => {
                toggle.mutate(true);
              }}
            >
              Turn stakes on
            </Button>
          </div>
        )}
      </Card>
    );
  }
  return (
    <Card className="flex flex-col gap-1">
      <Eyebrow>{month} stakes · set by your group</Eyebrow>
      {editing ? (
        <InlineForm
          label="This month's stakes"
          maxLength={140}
          submit="Save"
          initial={board.stake?.text ?? ""}
          onDone={() => {
            setEditing(false);
          }}
          run={(text) => accounts.setStake(group.id, text)}
        />
      ) : (
        <div className="ww-display text-xl">
          {board.stake?.text ?? "No stakes set for this month yet."}
        </div>
      )}
      {ideasOpen && ideas && (
        <ul className="m-0 mt-2 flex flex-col gap-1 pl-5 text-sm">
          {ideas.map((idea) => (
            <li key={idea}>{idea}</li>
          ))}
        </ul>
      )}
      <div className="mt-1.5 flex flex-wrap gap-x-4 text-sm">
        {owner && !editing && (
          <button
            type="button"
            className="min-h-11 cursor-pointer border-0 bg-transparent p-0 text-link underline"
            onClick={() => {
              setEditing(true);
            }}
          >
            Change stakes
          </button>
        )}
        <button
          type="button"
          className="min-h-11 cursor-pointer border-0 bg-transparent p-0 text-link underline"
          aria-expanded={ideasOpen}
          onClick={() => {
            setIdeasOpen(!ideasOpen);
          }}
        >
          {ideasOpen ? "Hide ideas" : "Get ideas"}
        </button>
        {owner && (
          <button
            type="button"
            className="min-h-11 cursor-pointer border-0 bg-transparent p-0 text-link underline"
            onClick={() => {
              toggle.mutate(false);
            }}
          >
            Turn off for this group
          </button>
        )}
      </div>
    </Card>
  );
}

function GroupView({ id }: { id: string }) {
  const [period, setPeriod] = useState<Period>("30d");
  const [showCode, setShowCode] = useState(false);
  const group = useGroup(id);
  const board = useLeaderboard(id, period);
  if (group.error) return <Failed error={group.error} />;
  if (board.error) return <Failed error={board.error} />;
  if (!group.data || !board.data) return <Loading />;
  const g = group.data;
  const b = board.data;
  const leader = b.members[0];
  const improved = b.most_improved.this_month;
  const you = b.members.find((m) => m.you);

  return (
    <div className="flex min-w-0 flex-[999_1_600px] flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <h1 className="text-3xl">{g.name}</h1>
        <Button
          variant="brand"
          aria-expanded={showCode}
          onClick={() => {
            setShowCode(!showCode);
          }}
        >
          Invite
        </Button>
      </div>
      {showCode && (
        <Card tone="inset" as="div" className="text-sm">
          Share this code. Anyone who enters it in Groups, Join with a code, joins {g.name}:{" "}
          <strong className="ww-mono text-base">{g.invite_code}</strong>
        </Card>
      )}
      <div className="self-start">
        <SegmentedControl label="Period" options={periods} value={period} onChange={setPeriod} />
      </div>
      <Card className="flex flex-wrap items-end gap-6">
        <Podium rows={b.members} />
        <div className="min-w-0 flex-[1_1_260px]">
          <Eyebrow>{periodCopy[period].title}</Eyebrow>
          <div className="mt-0.5 text-base font-semibold">
            {leader?.score != null
              ? `${leader.you ? "You lead" : `${leader.name} leads`} with ${String(leader.score)}`
              : "No scores yet"}
          </div>
          <div className="text-[13px] text-text-muted">{periodCopy[period].note}</div>
        </div>
      </Card>
      <Table board={b} period={period} />
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <Card className="flex flex-col gap-1">
          <Eyebrow>Most improved · {monthName(improved.month)}</Eyebrow>
          <div className="ww-display text-xl">
            {improved.winner
              ? `${improved.winner.user_id === you?.user_id ? "You" : improved.winner.name}, up ${String(improved.winner.change)}`
              : "Nobody yet"}
          </div>
          <div className="text-[13px] text-text-muted">Crowned on the 1st of next month.</div>
        </Card>
        <Stakes group={g} board={b} />
      </div>
    </div>
  );
}

export function Groups() {
  const groups = useGroups();
  const [picked, setPicked] = useState<string>();
  if (groups.error) return <Failed error={groups.error} />;
  if (!groups.data) return <Loading />;
  const selected = picked ?? groups.data[0]?.id;
  return (
    <div className="flex flex-wrap items-start gap-6">
      <Picker selected={selected} onSelect={setPicked} />
      {selected ? (
        <GroupView key={selected} id={selected} />
      ) : (
        <div className="flex-[999_1_600px]">
          <h1 className="text-3xl">Groups</h1>
          <p className="text-text-muted">
            Start a group with family, friends or coworkers, or join one with a code. You'll see
            each other's scores and fix speed, never your apps or devices.
          </p>
        </div>
      )}
    </div>
  );
}
