import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { FixProgress, FixResult, FixView } from "../../lib/ipc";
import { createQueryClient } from "../../lib/query";
import { Fix } from "./Fix";
import { meter, shellLine, worstVuln } from "./fixCopy";

const ipc = vi.hoisted(() => ({ getFix: vi.fn(), runFix: vi.fn() }));
vi.mock("../../lib/ipc", () => ipc);

function chrome(overrides: Partial<FixView["finding"]> = {}): FixView {
  return {
    finding: {
      key: "google-chrome",
      app_id: "google-chrome",
      app_name: "Google Chrome",
      kind: "app",
      source: "mac_app",
      software_id: "com.google.Chrome",
      name: "Google Chrome",
      browser: "chrome",
      version_scheme: "dotted",
      installed: "142.0.7444.59",
      running: false,
      installs: [],
      fixed_version: "142.0.7444.162",
      latest: null,
      update_available: true,
      exploited: true,
      severity: "high",
      vulns: [
        {
          id: "CVE-2026-13224",
          aliases: [],
          published: "2026-11-05",
          severity: "high",
          cvss: 8.8,
          exploited: true,
          kev: { added: "2026-11-06", due: "2026-11-27", ransomware: false },
          summary: "Type confusion in V8.",
          fixed: "142.0.7444.162",
        },
      ],
      ...overrides,
    },
    plan: {
      method: {
        kind: "command",
        tool: "homebrew",
        argv: ["/opt/homebrew/bin/brew", "upgrade", "--cask", "google-chrome"],
      },
      by: "Homebrew",
      one_click: true,
      admin: "no",
      steps: [
        { label: "Download Google Chrome 142.0.7444.162", detail: "a" },
        { label: "Make sure it's the real thing", detail: "b" },
        { label: "Install it", detail: "c" },
        { label: "Check again that the hole is closed", detail: "d" },
      ],
      verifies: "Homebrew checks the file's SHA-256 fingerprint.",
      manual: null,
      restart_note: null,
    },
    no_plan: null,
  };
}

function renderFix() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <MemoryRouter initialEntries={["/fix/mac_app/com.google.Chrome"]}>
        <Routes>
          <Route path="/fix/:source/:id" element={<Fix />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("Fix screen", () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it("explains an exploited hole and what will run before offering the act-now button", async () => {
    ipc.getFix.mockResolvedValue(chrome());
    renderFix();
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent(
      "Google Chrome has a hole attackers are using right now.",
    );
    expect(ipc.getFix).toHaveBeenCalledWith("mac_app", "com.google.Chrome");
    expect(screen.getByText("1 minute to update. Tabs will come back.")).toBeInTheDocument();
    expect(
      screen.getAllByText("/opt/homebrew/bin/brew upgrade --cask google-chrome"),
    ).not.toHaveLength(0);
    expect(screen.getByText("Homebrew checks the file's SHA-256 fingerprint.")).toBeInTheDocument();
    const button = screen.getByRole("button", { name: "Update Google Chrome now" });
    expect(button).toHaveClass("ww-btn-act");
    expect(ipc.runFix).not.toHaveBeenCalled();
  });

  it("shows live progress and the result", async () => {
    ipc.getFix.mockResolvedValue(chrome());
    let finish: (r: FixResult) => void = () => undefined;
    ipc.runFix.mockImplementation(
      (_s: string, _i: string, _shown: unknown, onProgress: (p: FixProgress) => void) => {
        onProgress({ event: "step", step: 0, state: "done" });
        onProgress({ event: "step", step: 1, state: "working" });
        return new Promise<FixResult>((resolve) => (finish = resolve));
      },
    );
    renderFix();
    await userEvent.click(await screen.findByRole("button", { name: "Update Google Chrome now" }));
    expect(ipc.runFix).toHaveBeenCalledWith(
      "mac_app",
      "com.google.Chrome",
      {
        kind: "command",
        tool: "homebrew",
        argv: ["/opt/homebrew/bin/brew", "upgrade", "--cask", "google-chrome"],
      },
      expect.any(Function),
    );
    expect(await screen.findByRole("status")).toHaveTextContent("Working.");
    expect(screen.getByText("Working…")).toBeInTheDocument();
    finish({ outcome: { status: "installed" }, check: "closed", finding: null });
    expect(await screen.findByText("Google Chrome is safe again.")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Done" })).toHaveClass("ww-btn-brand");
  });

  it("says nothing changed when the password prompt is dismissed", async () => {
    ipc.getFix.mockResolvedValue(chrome());
    ipc.runFix.mockResolvedValue({ outcome: { status: "cancelled" }, check: null, finding: null });
    renderFix();
    await userEvent.click(await screen.findByRole("button", { name: "Update Google Chrome now" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Nothing changed.");
    expect(screen.getByRole("button", { name: "Try again" })).toBeInTheDocument();
  });

  it("uses a quiet button with the severity meter when nothing is exploited", async () => {
    const view = chrome({ exploited: false, severity: "medium", browser: null });
    view.finding.vulns = view.finding.vulns.map((v) => ({
      ...v,
      exploited: false,
      kev: null,
      severity: "medium",
    }));
    ipc.getFix.mockResolvedValue(view);
    renderFix();
    const button = await screen.findByRole("button", { name: /Update Google Chrome now/ });
    expect(button).toHaveClass("ww-btn-quiet");
    expect(button).toHaveTextContent("moderate");
    expect(screen.queryByText(/Under attack now/)).not.toBeInTheDocument();
  });

  it("shows instructions when the app's own updater has to do it", async () => {
    const view = chrome();
    if (!view.plan) throw new Error("sample has a plan");
    view.plan = {
      ...view.plan,
      method: { kind: "app_updater", open_bundle: "com.google.Chrome" },
      by: "Google Chrome",
      one_click: false,
      steps: [],
      manual: "Wardwatch opens Google Chrome. Open its menu and let it update.",
    };
    ipc.getFix.mockResolvedValue(view);
    ipc.runFix.mockResolvedValue({ outcome: { status: "opened" }, check: null, finding: null });
    renderFix();
    expect(await screen.findByRole("heading", { name: "What to do" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Open Google Chrome" }));
    expect(
      await screen.findByRole("button", { name: "I've updated it, check again" }),
    ).toBeInTheDocument();
  });

  it("shows the core's error when it can't look the program up", async () => {
    ipc.getFix.mockRejectedValue("Wardwatch has no update information for this program.");
    renderFix();
    expect(
      await screen.findByText("Wardwatch has no update information for this program."),
    ).toBeInTheDocument();
  });
});

describe("fix copy", () => {
  it("maps severities to meter bars", () => {
    expect([meter("critical"), meter("high"), meter("medium"), meter("low"), meter(null)]).toEqual([
      3, 3, 2, 1, 1,
    ]);
  });

  it("quotes only arguments a shell would split", () => {
    expect(
      shellLine([
        "/usr/bin/pkexec",
        "apt-get",
        "-o",
        "Dpkg::Options::=--force-confold",
        "it's here",
      ]),
    ).toBe("/usr/bin/pkexec apt-get -o Dpkg::Options::=--force-confold 'it'\\''s here'");
  });

  it("describes the exploited or most severe hole first", () => {
    const f = chrome().finding;
    const newest = {
      id: "CVE-NEW",
      aliases: [],
      published: "2026-11-07",
      severity: "medium" as const,
      cvss: 5.3,
      exploited: false,
      kev: null,
      summary: "",
      fixed: null,
    };
    expect(worstVuln({ ...f, vulns: [newest, ...f.vulns] })?.id).toBe("CVE-2026-13224");
  });
});

it("says the hole is still open when the recheck can't find the program", async () => {
  ipc.getFix.mockResolvedValue(chrome());
  ipc.runFix.mockResolvedValue({
    outcome: { status: "installed" },
    check: "not_found",
    finding: null,
  });
  renderFix();
  await userEvent.click(await screen.findByRole("button", { name: "Update Google Chrome now" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("couldn't find Google Chrome again");
  expect(screen.queryByText("Google Chrome is safe again.")).not.toBeInTheDocument();
});
