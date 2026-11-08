import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderAt } from "../../test/render";
import { Score } from "./Score";

describe("Score", () => {
  it("shows the score card with both subscores", async () => {
    renderAt(<Score />);
    expect(await screen.findByRole("heading", { level: 1, name: /84\s*Good/ })).toBeInTheDocument();
    expect(screen.getByRole("meter", { name: "How up to date you are" })).toHaveAttribute(
      "aria-valuenow",
      "90",
    );
    expect(screen.getByRole("meter", { name: "How fast you fix" })).toHaveAttribute(
      "aria-valuenow",
      "78",
    );
    expect(screen.getByText(/6 points to/)).toHaveTextContent("Excellent");
  });

  it("offers buttons only for moves on this device", async () => {
    renderAt(<Score />);
    const moves = (await screen.findByRole("heading", { name: "Raise your score" })).closest(
      "section",
    );
    if (!moves) throw new Error("no moves section");
    expect(await within(moves).findByRole("button", { name: "Fix now" })).toHaveClass("ww-btn-act");
    expect(within(moves).getByRole("button", { name: /Do it now/ })).toHaveClass("ww-btn-quiet");
    const phone = within(moves).getByText("Update the iPhone").closest("section");
    if (!phone) throw new Error("no iPhone card");
    expect(phone).toHaveClass("ww-card-remote");
    expect(within(phone).queryByRole("button")).toBeNull();
  });

  it("stacks the account's devices", async () => {
    renderAt(<Score />);
    expect(await screen.findByText("1 needs you now · 3 for Sunday")).toBeInTheDocument();
    expect(screen.getByText("Family iMac")).toBeInTheDocument();
  });
});
