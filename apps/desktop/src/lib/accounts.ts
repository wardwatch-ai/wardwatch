// Client for the accounts backend (wardwatch-ai/backend accounts/). Nothing is
// deployed yet, so the app uses `sampleAccounts`, which answers with the backend's
// response shapes from the spec's sample scenario. A fetch-based client slots in
// behind the same interface once there's an API host and sign-in.
import type {
  AccountDevice,
  GroupDetail,
  GroupSummary,
  Leaderboard,
  Me,
  Period,
  ScoreResponse,
} from "./model";
import {
  sampleDevices,
  sampleGroupDetail,
  sampleGroups,
  sampleLeaderboard,
  sampleMe,
  sampleScore,
  sampleStakeIdeas,
} from "./sample";

export class AccountsError extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message);
  }
}

export interface AccountsClient {
  /** True while answers are sample data, so screens can say so. */
  readonly preview: boolean;
  me(): Promise<Me>;
  score(): Promise<ScoreResponse>;
  devices(): Promise<AccountDevice[]>;
  groups(): Promise<GroupSummary[]>;
  group(id: string): Promise<GroupDetail>;
  leaderboard(id: string, period: Period): Promise<Leaderboard>;
  stakeIdeas(): Promise<string[]>;
  setStake(id: string, text: string): Promise<void>;
  setStakesEnabled(id: string, enabled: boolean): Promise<void>;
  createGroup(name: string): Promise<GroupSummary>;
  joinGroup(code: string): Promise<GroupSummary>;
}

const STAKE_MAX = 140;

/** In-memory stand-in for the backend. Writes last until the app restarts. */
export function createSampleAccounts(): AccountsClient {
  const groups = structuredClone(sampleGroups);
  const details = structuredClone(sampleGroupDetail);
  const detail = (id: string) => {
    const d = details[id];
    if (!d) throw new AccountsError("not_found", "No such group.");
    return d;
  };
  const owned = (id: string) => {
    const d = detail(id);
    if (d.role !== "owner")
      throw new AccountsError("forbidden", "Only the group's owner can change this.");
    return d;
  };

  return {
    preview: true,
    me: () => Promise.resolve({ ...sampleMe }),
    score: () => Promise.resolve({ ...sampleScore }),
    devices: () => Promise.resolve(structuredClone(sampleDevices)),
    groups: () => Promise.resolve(structuredClone(groups)),
    group: (id) => Promise.resolve().then(() => ({ ...detail(id) })),
    leaderboard: (id, period) =>
      Promise.resolve().then(() => {
        const d = detail(id);
        return { ...sampleLeaderboard(id, period), stake: d.stakes_enabled ? d.stake : null };
      }),
    stakeIdeas: () => Promise.resolve([...sampleStakeIdeas]),
    setStake: (id, text) =>
      Promise.resolve().then(() => {
        const t = text.trim();
        if (!t || t.length > STAKE_MAX)
          throw new AccountsError(
            "invalid_stake",
            `Stakes are 1 to ${String(STAKE_MAX)} characters.`,
          );
        owned(id).stake = { month: "2026-11", text: t };
      }),
    setStakesEnabled: (id, enabled) =>
      Promise.resolve().then(() => {
        owned(id).stakes_enabled = enabled;
      }),
    createGroup: (name) =>
      Promise.resolve().then(() => {
        const n = name.trim();
        if (!n || n.length > 60)
          throw new AccountsError("invalid_name", "Group names are 1 to 60 characters.");
        const id = `g-${String(groups.length + 1)}`;
        const g: GroupSummary = {
          id,
          name: n,
          names: "username",
          visibility: "private",
          role: "owner",
          member_count: 1,
        };
        groups.push(g);
        details[id] = {
          id,
          name: n,
          role: "owner",
          invite_code: "NEW-0000",
          stakes_enabled: true,
          stake: null,
        };
        return { ...g };
      }),
    joinGroup: (code) =>
      Promise.resolve().then(() => {
        const found = Object.values(details).find(
          (d) => d.invite_code === code.trim().toUpperCase(),
        );
        if (!found) throw new AccountsError("invalid_code", "That code doesn't match a group.");
        const g = groups.find((x) => x.id === found.id);
        if (!g) throw new AccountsError("invalid_code", "That code doesn't match a group.");
        return { ...g };
      }),
  };
}
