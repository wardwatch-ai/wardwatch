import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderAt } from "../../test/render";
import { Apps } from "./Apps";

const pick = async (name: RegExp) => {
  await userEvent.click(await screen.findByRole("button", { name }));
};

describe("Apps", () => {
  it("opens on the exploited program with the amber button", async () => {
    renderAt(<Apps />);
    expect(
      await screen.findByRole("heading", { level: 1, name: "Google Chrome" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Update now" })).toHaveClass("ww-btn-act");
    expect(screen.getByText("Being exploited right now")).toBeInTheDocument();
    expect(screen.getByText("CVE-2026-0001")).toBeInTheDocument();
  });

  it("shows one thing in the corner for each state", async () => {
    renderAt(<Apps />);
    await pick(/^Zoom/);
    expect(screen.getByRole("button", { name: "Update now, moderate" })).toHaveClass(
      "ww-btn-quiet",
    );
    await pick(/^Slack/);
    expect(
      screen.getByRole("button", { name: "Update now, minor or no security fix" }),
    ).toBeInTheDocument();
    expect(screen.getByText("No security fixes in this update.")).toBeInTheDocument();
    await pick(/^Firefox/);
    expect(screen.getByText("Up to date")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Update now|Hold/ })).toBeNull();
    await pick(/^Final Cut Pro/);
    expect(screen.getByRole("button", { name: "Release hold" })).toBeInTheDocument();
  });

  it("holds and releases a program", async () => {
    renderAt(<Apps />);
    await pick(/^VLC/);
    await userEvent.click(screen.getByRole("button", { name: "Hold" }));
    expect(await screen.findByRole("button", { name: "Release hold" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Release hold" }));
    expect(await screen.findByRole("button", { name: "Hold" })).toBeInTheDocument();
  });

  it("doesn't hold twice, and shows the hold on an exploited program", async () => {
    renderAt(<Apps />);
    expect(
      await screen.findByRole("heading", { level: 1, name: "Google Chrome" }),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Hold" }));
    expect(await screen.findByText(/Held by you since/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Hold" })).toBeNull();
    expect(screen.getByRole("button", { name: "Update now" })).toHaveClass("ww-btn-act");
    await userEvent.click(screen.getByRole("button", { name: "Release hold" }));
    expect(await screen.findByRole("button", { name: "Hold" })).toBeInTheDocument();
  });

  it("searches the list", async () => {
    renderAt(<Apps />);
    await userEvent.type(await screen.findByRole("searchbox", { name: "Search programs" }), "spot");
    expect(await screen.findByRole("button", { name: /^Spotify/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^Zoom/ })).toBeNull();
  });

  it("lists rows by version, not device", async () => {
    renderAt(<Apps />);
    expect(await screen.findByRole("button", { name: /^Zoom/ })).toHaveTextContent(
      "Zoom6.4.2Update",
    );
  });
});
