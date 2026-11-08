import { screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderAt } from "../../test/render";
import { Apps } from "./Apps";
import { Home } from "./Home";

// Inside Tauri, until the matcher lands, nothing has been checked against the feed.
vi.mock("../../lib/ipc", async (original) => ({
  ...(await original<typeof import("../../lib/ipc")>()),
  listFindings: () => Promise.resolve(null),
}));

describe("before anything is checked", () => {
  it("Home says so instead of claiming all clear", async () => {
    renderAt(<Home />);
    expect(
      await screen.findByRole("heading", { level: 1, name: "We haven't checked for updates yet." }),
    ).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "All clear" })).toBeInTheDocument();
    expect(screen.queryByText("Current")).toBeNull();
  });

  it("Apps lists versions and makes no up-to-date claim", async () => {
    renderAt(<Apps />);
    expect(await screen.findByRole("heading", { level: 1, name: "1Password" })).toBeInTheDocument();
    expect(screen.getByText("Not checked yet")).toBeInTheDocument();
    expect(screen.queryByText("Up to date")).toBeNull();
    expect(screen.queryByRole("button", { name: /Update now|Hold/ })).toBeNull();
  });
});
