// Data shapes the screens read. Three sources feed them:
//  - the Rust core over IPC (inventory from wardwatch-catalog, the update plan),
//  - findings (inventory matched against the update feed by the core's
//    `list_findings`; the full shape is in ipc.ts, these are the fields screens read),
//  - the accounts backend (score and groups, wardwatch-ai/backend accounts/), behind
//    the client in accounts.ts.

// ---- wardwatch-catalog Inventory (crates/wardwatch-catalog/src/lib.rs) ----

export type Source =
  | "mac_app"
  | "homebrew_formula"
  | "homebrew_cask"
  | "windows_registry"
  | "dpkg"
  | "rpm"
  | "snap"
  | "flatpak";

export interface Software {
  source: Source;
  id: string;
  name: string;
  version: string | null;
  publisher?: string;
  arch?: string;
  source_package?: string;
  browser?: string;
  running: boolean;
}

export interface OsInfo {
  family: "macos" | "windows" | "linux";
  id: string;
  name: string;
  version: string;
  build?: string | null;
  release?: string | null;
  kernel?: string | null;
  arch: string;
}

export interface Inventory {
  schema: number;
  os: OsInfo;
  software: Software[];
  warnings: string[];
}

// ---- Findings (inventory vs the update feed) ----

export type FeedSeverity = "critical" | "high" | "medium" | "low" | "unknown";

export interface Vuln {
  id: string;
  summary: string;
  severity: FeedSeverity | null;
  exploited: boolean;
}

/**
 * One catalog app found on this device (crates/wardwatch-feed; fields the
 * screens don't use are left out, the full shape is ipc.ts Finding).
 * `key` is the catalog's app id ("google-chrome").
 * `source` and `software_id` name the primary install ("os" and the OS id for the
 * OS); one finding can cover several installs, such as all dpkg packages built
 * from one source package.
 */
export interface Finding {
  key: string;
  source: string;
  software_id: string;
  installs?: { source: string; id: string }[];
  installed: string;
  fixed_version: string | null;
  latest: string | null;
  update_available: boolean;
  exploited: boolean;
  severity: FeedSeverity | null;
  vulns: Vuln[];
}

// ---- Update plan (stored by the core; the nudge engine reads the same JSON) ----

export type Timing = "same_day" | "sunday_tuneup" | "monthly" | "auto";
export type Tier = "serious" | "moderate" | "minor";
export type Kind = "system" | "browsers" | "apps";

export interface Hold {
  source: Source | "os";
  id: string;
  /** YYYY-MM-DD */
  since: string;
}

export interface UpdatePlan {
  v: 1;
  rules: Record<Tier, Record<Kind, Timing>>;
  quiet_hours: { start: string; end: string };
  holds: Hold[];
}

// ---- Accounts backend responses (backend accounts/src) ----

export type BandName = "At risk" | "Fair" | "Good" | "Excellent";

/** GET /v1/score */
export type ScoreResponse =
  | { score: null; patches_scored: 0; sums?: ScoreSums; algorithm: number }
  | {
      score: number;
      band: BandName;
      next_band: BandName | null;
      points_to_next_band: number;
      up_to_date: number;
      fix_speed: number;
      percentile: number | null;
      patches_scored: number;
      /** Sums behind the subscores, for on-device estimates (backend#3). */
      sums?: ScoreSums;
      algorithm: number;
    };

export interface ScoreSums {
  total: number;
  in_time: number;
  speed: number;
}

/** GET /v1/groups */
export interface GroupSummary {
  id: string;
  name: string;
  names: "username" | "real";
  visibility: "private" | "public";
  role: "owner" | "member";
  member_count: number;
}

export type Period = "30d" | "month" | "joined";

export interface Stake {
  month: string;
  text: string;
}

export interface LeaderboardRow {
  user_id: string;
  name: string;
  score: number | null;
  band: BandName | null;
  /** The 0-100 "How fast you fix" subscore, not days. */
  fix_speed: number | null;
  rank: number | null;
  you: boolean;
}

/** GET /v1/groups/:id/leaderboard */
export interface Leaderboard {
  period: Period;
  members: LeaderboardRow[];
  most_improved: {
    this_month: { month: string; winner: { user_id: string; name: string; change: number } | null };
    last_month: { month: string; winner: { user_id: string; name: string; change: number } | null };
  };
  stake: Stake | null;
}

/** GET /v1/groups/:id (member view, trimmed to what the screens use) */
export interface GroupDetail {
  id: string;
  name: string;
  role: "owner" | "member";
  invite_code: string;
  stakes_enabled: boolean;
  stake: Stake | null;
}

/** GET /v1/me */
export interface Me {
  username: string | null;
  real_name: string | null;
}

/**
 * The account's other devices and what waits on each. The backend has no
 * per-device endpoint yet (scores are per account), so this exists only in the
 * client stub for now.
 */
export interface AccountDevice {
  id: string;
  name: string;
  /** This device. Used only to leave it out of "other devices"; never shown as a badge. */
  current: boolean;
  score: number;
  status: "needs_you" | "waiting" | "clear";
  /** What waits there, e.g. "1 for Sunday". */
  summary: string;
  /** "139 programs", "iOS 26.1". */
  detail: string;
  checked: string;
  /** A fix that has to happen on that device: no button, just where and how. */
  remote_fix?: {
    title: string;
    steps: string;
    /** The known hole the update closes, as the server matched it; absent when it closes none. */
    fix?: { exploited: boolean; severity: FeedSeverity | null };
  };
}

// ---- Nudge plan (crates/wardwatch-nudge NudgePlan, from the core's get_nudges) ----

export interface Nudge {
  /** The finding's key (catalog app id). */
  key: string;
  app_id: string;
  name: string;
  category: Kind;
  level: Tier;
  exploited: boolean;
  strength: { kind: "urgent" } | { kind: "quiet"; bars: number };
  timing: Timing;
  /** An exploited OS or browser hole whose broad alert isn't approved yet. */
  awaiting_approval: boolean;
  due_at: string | null;
}

/** What leads Home and the tray. Timestamps are RFC 3339. */
export type TopCard =
  | { kind: "under_attack"; keys: string[] }
  | { kind: "tune_up"; keys: string[]; bars: number; at: string };

export interface NudgePlan {
  nudges: Nudge[];
  top_card: TopCard;
  notify: {
    kind: "urgent" | "same_day" | "tune_up";
    keys: string[];
    title: string;
    body: string;
  }[];
  install: string[];
  tune_up_due: boolean;
  next_check: string;
}
