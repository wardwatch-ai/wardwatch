import { screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderAt } from "../../test/render";
import { Settings } from "./Settings";

vi.mock("../../lib/ipc", async (original) => ({
  ...(await original<typeof import("../../lib/ipc")>()),
  listAutoInstalls: () =>
    Promise.resolve([
      {
        at: "2026-11-10T14:00:00Z",
        key: "zoom",
        name: "Zoom",
        from: "6.4.2",
        to: "6.5.0",
        result: "skipped",
        why: "it's open right now",
      },
      {
        at: "2026-11-09T14:00:00Z",
        key: "vlc",
        name: "VLC",
        from: "3.0.20",
        to: "3.0.21",
        result: "installed",
        why: "",
      },
    ]),
}));

describe("Settings: installed by itself", () => {
  it("lists every automatic install and why one waits", async () => {
    renderAt(<Settings />);
    expect(await screen.findByText("Zoom")).toBeInTheDocument();
    expect(screen.getByText(/Waiting for you: it's open right now/)).toBeInTheDocument();
    expect(screen.getByText("3.0.20 → 3.0.21")).toBeInTheDocument();
    expect(screen.getByText(/^Installed ·/)).toBeInTheDocument();
  });
});
