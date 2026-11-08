import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderAt } from "../../test/render";
import { Plan } from "./Plan";

describe("Update plan", () => {
  it("starts on Balanced with the exploited row locked", async () => {
    renderAt(<Plan />);
    expect(await screen.findByRole("button", { name: "Balanced" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByText("Always same day. Can't be changed.")).toBeInTheDocument();
    expect(
      screen.getAllByRole("combobox", { name: /, (System|Browsers|Other apps)$/ }),
    ).toHaveLength(9);
  });

  it("clears the presets when a box changes and lights one that matches", async () => {
    renderAt(<Plan />);
    const cell = await screen.findByRole("combobox", { name: "Moderate, Browsers" });
    await userEvent.selectOptions(cell, "auto");
    for (const name of ["Relaxed", "Balanced", "Eager"]) {
      expect(screen.getByRole("button", { name })).toHaveAttribute("aria-pressed", "false");
    }
    await userEvent.click(screen.getByRole("button", { name: "Eager" }));
    expect(screen.getByRole("button", { name: "Eager" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("combobox", { name: "Serious, Other apps" })).toHaveValue("auto");
  });

  it("sets quiet hours", async () => {
    renderAt(<Plan />);
    const start = await screen.findByRole("combobox", { name: "Quiet hours start" });
    expect(start).toHaveValue("22:00");
    await userEvent.selectOptions(start, "21:00");
    expect(screen.getByRole("combobox", { name: "Quiet hours start" })).toHaveValue("21:00");
    expect(screen.getByText(/Held apps: 1/)).toBeInTheDocument();
  });
});
