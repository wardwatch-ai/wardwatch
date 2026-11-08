// Chrome is exploited, but its broad alert isn't approved, so the nudge engine
// keeps it out of the lead: no amber card, no beams, first in the tune-up.
import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createQueryClient } from "../../lib/query";
import { renderAt } from "../../test/render";
import { TrayApp } from "../../tray/TrayApp";
import { Home } from "./Home";

vi.mock("../../lib/ipc", async (original) => {
  const ipc = await original<typeof import("../../lib/ipc")>();
  const { sampleNudges } = await import("../../lib/sample");
  return {
    ...ipc,
    getNudges: () =>
      Promise.resolve({
        ...sampleNudges,
        top_card: {
          kind: "tune_up",
          keys: ["google-chrome", "zoom", "vlc", "macos", "slack"],
          bars: 3,
          at: "2026-11-15T10:00:00Z",
        },
      }),
  };
});

describe("an exploited hole awaiting alert approval", () => {
  it("doesn't take Home's lead but still says so plainly", async () => {
    renderAt(<Home />);
    expect(
      await screen.findByRole("heading", { level: 1, name: "Update Google Chrome soon." }),
    ).toBeInTheDocument();
    expect(screen.queryByText("This is being exploited right now.")).not.toBeInTheDocument();
    expect(document.querySelector(".ww-btn-act")).toBeNull();
    expect(screen.getByRole("button", { name: /Update all 5/ })).toHaveClass("ww-btn-quiet");
  });

  it("leads the tray with the tune-up instead of an attack card", async () => {
    render(
      <QueryClientProvider client={createQueryClient()}>
        <TrayApp />
      </QueryClientProvider>,
    );
    expect(await screen.findByRole("heading", { name: "Worth doing soon" })).toBeInTheDocument();
    expect(screen.getByText("Sunday tune-up")).toBeInTheDocument();
    expect(screen.queryByText("Under attack now")).not.toBeInTheDocument();
    expect(document.querySelector(".ww-btn-act")).toBeNull();
  });
});
