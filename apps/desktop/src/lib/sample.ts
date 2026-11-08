// The design spec's shared sample scenario ("Sample data in the mocks"), so screens,
// tests and screenshots agree. Used in a plain browser (`pnpm dev`) and in tests;
// accounts data uses it everywhere until the backend is deployed.
import type {
  AccountDevice,
  Finding,
  GroupDetail,
  GroupSummary,
  Inventory,
  Leaderboard,
  LeaderboardRow,
  Me,
  Period,
  ScoreResponse,
  Software,
  UpdatePlan,
} from "./model";

const app = (
  id: string,
  name: string,
  version: string,
  publisher: string,
  browser?: string,
): Software => ({
  source: "mac_app",
  id,
  name,
  version,
  publisher,
  running: false,
  ...(browser ? { browser } : {}),
});

export const sampleInventory: Inventory = {
  schema: 1,
  os: {
    family: "macos",
    id: "macos",
    name: "macOS",
    version: "16.1",
    build: "25B78",
    arch: "aarch64",
  },
  software: [
    app("com.google.Chrome", "Google Chrome", "142.0.7444.59", "Google", "chrome"),
    app("org.mozilla.firefox", "Firefox", "145.0.1", "Mozilla", "firefox"),
    app("com.apple.Safari", "Safari", "26.1", "Apple", "safari"),
    app("us.zoom.xos", "Zoom", "6.4.2", "Zoom"),
    app("org.videolan.vlc", "VLC", "3.0.20", "VideoLAN"),
    app("com.apple.FinalCut", "Final Cut Pro", "11.1", "Apple"),
    app("com.tinyspeck.slackmacgap", "Slack", "4.41.97", "Slack"),
    app("com.1password.1password", "1Password", "8.11.16", "AgileBits"),
    app("com.spotify.client", "Spotify", "1.2.74", "Spotify"),
  ],
  warnings: [],
};

const current = (source: string, software_id: string, v: string): Finding => ({
  key: software_id.split(".").at(-1)?.toLowerCase() ?? software_id,
  source,
  software_id,
  installed: v,
  fixed_version: null,
  latest: v,
  update_available: false,
  exploited: false,
  severity: null,
  vulns: [],
});

export const sampleFindings: Finding[] = [
  {
    key: "google-chrome",
    source: "mac_app",
    software_id: "com.google.Chrome",
    installed: "142.0.7444.59",
    fixed_version: "142.0.7444.162",
    latest: "142.0.7444.162",
    update_available: true,
    exploited: true,
    severity: "high",
    vulns: [
      {
        id: "CVE-2026-0001",
        summary: "Memory bug in the JavaScript engine",
        severity: "high",
        exploited: true,
      },
    ],
  },
  {
    key: "zoom",
    source: "mac_app",
    software_id: "us.zoom.xos",
    installed: "6.4.2",
    fixed_version: "6.5.0",
    latest: "6.5.0",
    update_available: true,
    exploited: false,
    severity: "medium",
    vulns: [
      {
        id: "ZSB-26001",
        summary: "Could let a meeting link run a file",
        severity: "medium",
        exploited: false,
      },
      { id: "ZSB-26002", summary: "Leaks your display name", severity: "medium", exploited: false },
    ],
  },
  {
    key: "vlc",
    source: "mac_app",
    software_id: "org.videolan.vlc",
    installed: "3.0.20",
    fixed_version: "3.0.21",
    latest: "3.0.21",
    update_available: true,
    exploited: false,
    severity: "medium",
    vulns: [
      {
        id: "CVE-2026-0002",
        summary: "A booby-trapped video file could crash it",
        severity: "medium",
        exploited: false,
      },
    ],
  },
  {
    key: "macos",
    source: "os",
    software_id: "macos",
    installed: "16.1",
    fixed_version: "16.1.1",
    latest: "16.1.1",
    update_available: true,
    exploited: false,
    severity: "medium",
    vulns: [
      {
        id: "CVE-2026-0003",
        summary: "An app could read files it should not",
        severity: "medium",
        exploited: false,
      },
      { id: "CVE-2026-0004", summary: "Bluetooth bug", severity: "medium", exploited: false },
    ],
  },
  {
    key: "final-cut-pro",
    source: "mac_app",
    software_id: "com.apple.FinalCut",
    installed: "11.1",
    fixed_version: null,
    latest: "11.2",
    update_available: true,
    exploited: false,
    severity: null,
    vulns: [],
  },
  {
    key: "slack",
    source: "mac_app",
    software_id: "com.tinyspeck.slackmacgap",
    installed: "4.41.97",
    fixed_version: null,
    latest: "4.41.105",
    update_available: true,
    exploited: false,
    severity: null,
    vulns: [],
  },
  current("mac_app", "org.mozilla.firefox", "145.0.1"),
  current("mac_app", "com.apple.Safari", "26.1"),
  current("mac_app", "com.1password.1password", "8.11.16"),
  current("mac_app", "com.spotify.client", "1.2.74"),
];

const balanced: UpdatePlan["rules"] = {
  serious: { system: "same_day", browsers: "same_day", apps: "same_day" },
  moderate: { system: "sunday_tuneup", browsers: "sunday_tuneup", apps: "sunday_tuneup" },
  minor: { system: "sunday_tuneup", browsers: "sunday_tuneup", apps: "monthly" },
};

export const samplePlan: UpdatePlan = {
  v: 1,
  rules: balanced,
  quiet_hours: { start: "22:00", end: "08:00" },
  holds: [{ source: "mac_app", id: "com.apple.FinalCut", since: "2026-11-02" }],
};

export const sampleMe: Me = { username: "matt", real_name: "Matt" };

export const sampleScore: ScoreResponse = {
  score: 84,
  band: "Good",
  next_band: "Excellent",
  points_to_next_band: 6,
  up_to_date: 90,
  fix_speed: 78,
  percentile: 71,
  patches_scored: 42,
  // Consistent with 90 up to date and 78 fix speed.
  sums: { total: 40, in_time: 36, speed: 31.2 },
  algorithm: 1,
};

export const sampleDevices: AccountDevice[] = [
  {
    id: "d-macbook",
    name: "MacBook Air",
    current: true,
    score: 78,
    status: "needs_you",
    summary: "1 needs you now · 3 for Sunday",
    detail: "139 programs",
    checked: "checked 4 minutes ago",
  },
  {
    id: "d-imac",
    name: "Family iMac",
    current: false,
    score: 90,
    status: "clear",
    summary: "All clear",
    detail: "8 programs",
    checked: "checked 1 hour ago",
  },
  {
    id: "d-iphone",
    name: "iPhone",
    current: false,
    score: 93,
    status: "waiting",
    summary: "1 for Sunday",
    detail: "iOS 26.1",
    checked: "checked 3 hours ago",
    remote_fix: {
      title: "Update the iPhone",
      steps:
        "iOS 26.1.1 is ready. Open Settings on the iPhone, then General, then Software Update.",
      fix: { exploited: false, severity: "high" },
    },
  },
];

export const sampleGroups: GroupSummary[] = [
  {
    id: "g-ward",
    name: "Ward family",
    names: "real",
    visibility: "private",
    role: "owner",
    member_count: 4,
  },
  {
    id: "g-pine",
    name: "Pine Street Library",
    names: "username",
    visibility: "private",
    role: "member",
    member_count: 12,
  },
];

export const sampleGroupDetail: Record<string, GroupDetail> = {
  "g-ward": {
    id: "g-ward",
    name: "Ward family",
    role: "owner",
    invite_code: "WARD-7K2Q",
    stakes_enabled: true,
    stake: { month: "2026-11", text: "Last place on Nov 30 cooks Sunday dinner." },
  },
  "g-pine": {
    id: "g-pine",
    name: "Pine Street Library",
    role: "member",
    invite_code: "PINE-4M8D",
    stakes_enabled: false,
    stake: null,
  },
};

const row = (
  user_id: string,
  name: string,
  score: number,
  fix_speed: number,
  you = false,
): LeaderboardRow => ({
  user_id,
  name,
  score,
  band: score >= 90 ? "Excellent" : score >= 75 ? "Good" : score >= 60 ? "Fair" : "At risk",
  fix_speed,
  rank: null,
  you,
});

const ranked = (rows: LeaderboardRow[]) =>
  rows.map((r, i) => ({ ...r, rank: i > 0 && rows[i - 1]?.score === r.score ? i : i + 1 }));

const wardBoards: Record<Period, LeaderboardRow[]> = {
  "30d": ranked([
    row("u-sam", "Sam", 93, 95),
    row("u-matt", "Matt", 84, 78, true),
    row("u-jo", "Jo", 79, 70),
    row("u-alex", "Alex", 71, 62),
  ]),
  month: ranked([
    row("u-matt", "Matt", 91, 88, true),
    row("u-sam", "Sam", 90, 92),
    row("u-alex", "Alex", 82, 75),
    row("u-jo", "Jo", 77, 68),
  ]),
  joined: ranked([
    row("u-sam", "Sam", 92, 90),
    row("u-jo", "Jo", 81, 72),
    row("u-matt", "Matt", 80, 74, true),
    row("u-alex", "Alex", 64, 55),
  ]),
};

export function sampleLeaderboard(groupId: string, period: Period): Leaderboard {
  const members =
    groupId === "g-ward"
      ? wardBoards[period]
      : ranked([
          row("u-lib1", "circdesk", 88, 85),
          row("u-matt", "matt", 84, 78, true),
          row("u-lib2", "stacks", 76, 70),
        ]);
  return {
    period,
    members,
    most_improved: {
      this_month: {
        month: "2026-11",
        winner: groupId === "g-ward" ? { user_id: "u-alex", name: "Alex", change: 9 } : null,
      },
      last_month: { month: "2026-10", winner: null },
    },
    stake: sampleGroupDetail[groupId]?.stake ?? null,
  };
}

/** GET /v1/groups/stake-ideas, as the backend returns it today. */
export const sampleStakeIdeas = [
  "Last place cooks Sunday dinner.",
  "Last place gives $10 to a charity the winner picks.",
  "The winner picks the next movie night film.",
  "Last place buys the first round.",
  "Last place does the dishes for a week.",
  "Everyone gives $5 to the winner's favorite charity.",
];
