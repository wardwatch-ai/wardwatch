import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderAt } from "../../test/render";
import { Settings } from "./Settings";

describe("Settings", () => {
  it("shows the version and starts on mainline", async () => {
    renderAt(<Settings />);
    expect(await screen.findByText("Wardwatch dev")).toBeInTheDocument();
    expect(screen.getByText("Local build")).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /Mainline/ })).toBeChecked();
  });

  it("asks before switching to nightly, and can back out", async () => {
    renderAt(<Settings />);
    await userEvent.click(await screen.findByRole("radio", { name: /Nightly/ }));
    expect(screen.getByRole("alertdialog", { name: "Switch to nightly?" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Stay on mainline" }));
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /Mainline/ })).toBeChecked();

    await userEvent.click(screen.getByRole("radio", { name: /Nightly/ }));
    await userEvent.click(screen.getByRole("button", { name: "Switch to nightly" }));
    expect(await screen.findByRole("radio", { name: /Nightly/ })).toBeChecked();
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();

    // Going back to mainline needs no confirmation.
    await userEvent.click(screen.getByRole("radio", { name: /Mainline/ }));
    expect(await screen.findByRole("radio", { name: /Mainline/ })).toBeChecked();
  });
});
