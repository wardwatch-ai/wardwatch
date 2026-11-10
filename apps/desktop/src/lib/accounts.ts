// Client for the accounts backend (wardwatch-ai/backend accounts/). Two clients
// share one interface: `createSampleAccounts` answers with the backend's response
// shapes from the spec's sample scenario (the default, and what screens show until
// the person signs in), and `createHttpAccounts` calls the real API through the
// core, which holds the session token.
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
  /** 3 to 20 letters, numbers, _ or -. Changeable once every 30 days after the first pick. */
  setUsername(username: string): Promise<void>;
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

type Method = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
/** One API call: the HTTP status and parsed JSON body (ipc.ts accountsRequest). */
export type Call = (
  method: Method,
  path: string,
  body?: unknown,
) => Promise<{ status: number; body: unknown }>;

/** The real backend, through `call`. Errors come back as AccountsError with the API's code. */
export function createHttpAccounts(call: Call): AccountsClient {
  const api = async <T>(method: Method, path: string, body?: unknown): Promise<T> => {
    const res = await call(method, path, body);
    if (res.status < 200 || res.status > 299) {
      const err = (res.body as { error?: { code?: string; message?: string } } | null)?.error;
      throw new AccountsError(
        err?.code ?? (res.status === 401 ? "unauthorized" : "server_error"),
        err?.message ?? "Something went wrong on our side.",
      );
    }
    return res.body as T;
  };
  const id = encodeURIComponent;
  type GroupFull = Omit<GroupSummary, "member_count"> & { members: unknown[] };
  const summary = (g: GroupFull): GroupSummary => ({
    id: g.id,
    name: g.name,
    names: g.names,
    visibility: g.visibility,
    role: g.role,
    member_count: g.members.length,
  });
  return {
    preview: false,
    me: async () => {
      const me = await api<Me>("GET", "/v1/me");
      return { username: me.username, real_name: me.real_name };
    },
    setUsername: async (username) => {
      await api("PATCH", "/v1/me", { username });
    },
    score: () => api<ScoreResponse>("GET", "/v1/score"),
    // Scores are per account; the backend has no per-device list yet.
    devices: () => Promise.resolve([]),
    groups: async () => (await api<{ groups: GroupSummary[] }>("GET", "/v1/groups")).groups,
    group: async (g) => {
      const d = await api<GroupDetail>("GET", `/v1/groups/${id(g)}`);
      return {
        id: d.id,
        name: d.name,
        role: d.role,
        invite_code: d.invite_code,
        stakes_enabled: d.stakes_enabled,
        stake: d.stake,
      };
    },
    leaderboard: (g, period) =>
      api<Leaderboard>("GET", `/v1/groups/${id(g)}/leaderboard?period=${period}`),
    stakeIdeas: async () => (await api<{ ideas: string[] }>("GET", "/v1/groups/stake-ideas")).ideas,
    setStake: async (g, text) => {
      await api("PUT", `/v1/groups/${id(g)}/stake`, { text });
    },
    setStakesEnabled: async (g, enabled) => {
      await api("PATCH", `/v1/groups/${id(g)}`, { stakes_enabled: enabled });
    },
    createGroup: async (name) => summary(await api<GroupFull>("POST", "/v1/groups", { name })),
    joinGroup: async (code) => summary(await api<GroupFull>("POST", "/v1/groups/join", { code })),
  };
}

/** In-memory stand-in for the backend. Writes last until the app restarts. */
export function createSampleAccounts(): AccountsClient {
  const me = { ...sampleMe };
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
    me: () => Promise.resolve({ ...me }),
    setUsername: (username) =>
      Promise.resolve().then(() => {
        if (!/^[A-Za-z0-9_-]{3,20}$/.test(username))
          throw new AccountsError(
            "invalid_username",
            "Usernames are 3 to 20 letters, numbers, _ or -.",
          );
        me.username = username;
      }),
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
