import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderAt } from "../../test/render";
import { Groups } from "./Groups";

describe("Groups", () => {
  it("ranks the first group over the last 30 days", async () => {
    renderAt(<Groups />);
    expect(
      await screen.findByRole("heading", { level: 1, name: "Ward family" }),
    ).toBeInTheDocument();
    expect(await screen.findByText("Sam leads with 93")).toBeInTheDocument();
    const rows = screen.getAllByRole("row").slice(1);
    expect(rows.map((r) => within(r).getAllByRole("cell")[1]?.textContent)).toEqual([
      "Sam",
      "Matt (you)",
      "Jo",
      "Alex",
    ]);
    expect(screen.getByText("Alex, up 9")).toBeInTheDocument();
  });

  it("switches periods, each with its own ranking", async () => {
    renderAt(<Groups />);
    await userEvent.click(await screen.findByRole("button", { name: "This month" }));
    expect(await screen.findByText("You lead with 91")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "This month" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("lets the owner change the stakes and see ideas", async () => {
    renderAt(<Groups />);
    expect(
      await screen.findByText("Last place on Nov 30 cooks Sunday dinner."),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Get ideas" }));
    expect(await screen.findByText("Last place buys the first round.")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Change stakes" }));
    const field = screen.getByLabelText("This month's stakes");
    await userEvent.clear(field);
    await userEvent.type(field, "Loser washes the car.");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Loser washes the car.")).toBeInTheDocument();
  });

  it("switches groups and hides owner controls from members", async () => {
    renderAt(<Groups />);
    await userEvent.click(await screen.findByRole("button", { name: /Pine Street Library/ }));
    expect(
      await screen.findByRole("heading", { level: 1, name: "Pine Street Library" }),
    ).toBeInTheDocument();
    expect(await screen.findByText("This group plays without stakes.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Turn stakes on" })).toBeNull();
  });

  it("joins with a code and reports a bad one", async () => {
    renderAt(<Groups />);
    await userEvent.click(await screen.findByRole("button", { name: "Join with a code" }));
    await userEvent.type(screen.getByLabelText("Invite code"), "nope");
    await userEvent.click(screen.getByRole("button", { name: "Join" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("That code doesn't match a group.");
  });
});
