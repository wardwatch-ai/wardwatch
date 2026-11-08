import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { createQueryClient } from "../lib/query";
import { TrayApp } from "./TrayApp";

const showMainWindow = vi.hoisted(() => vi.fn(() => Promise.resolve()));
vi.mock("../lib/ipc", async (original) => ({
  ...(await original<typeof import("../lib/ipc")>()),
  showMainWindow,
}));

const renderTray = () =>
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TrayApp />
    </QueryClientProvider>,
  );

describe("TrayApp", () => {
  it("leads with the exploited program", async () => {
    renderTray();
    expect(await screen.findByRole("heading", { name: "One thing needs you" })).toBeInTheDocument();
    expect(screen.getByText("Update Google Chrome")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Update now" }));
    expect(showMainWindow).toHaveBeenLastCalledWith("/fix/mac_app/com.google.Chrome");
  });

  it("shows the score strip and opens screens in the main window", async () => {
    renderTray();
    expect(await screen.findByText("2nd in Ward family")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Score" }));
    expect(showMainWindow).toHaveBeenLastCalledWith("/score");
    await userEvent.click(screen.getByRole("button", { name: "Open Wardwatch" }));
    expect(showMainWindow).toHaveBeenLastCalledWith(undefined);
  });
});
