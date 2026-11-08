import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TrayApp } from "./TrayApp";

const showMainWindow = vi.hoisted(() => vi.fn(() => Promise.resolve()));
vi.mock("../lib/ipc", () => ({ showMainWindow }));

describe("TrayApp", () => {
  it("opens the main window", async () => {
    render(<TrayApp />);
    await userEvent.click(screen.getByRole("button", { name: "Open Wardwatch" }));
    expect(showMainWindow).toHaveBeenCalledOnce();
  });
});
