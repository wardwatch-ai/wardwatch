import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderAt } from "../test/render";
import { App } from "./App";

describe("App", () => {
  it("shows the five tabs from the design", () => {
    renderAt(<App />);
    expect(screen.getByRole("navigation", { name: "Main" })).toHaveTextContent(
      "HomeScoreGroupsAppsUpdate plan",
    );
  });

  it("navigates between tabs", async () => {
    renderAt(<App />);
    await userEvent.click(screen.getByRole("link", { name: "Update plan" }));
    expect(
      await screen.findByRole("heading", { level: 1, name: "Your update rules" }),
    ).toBeInTheDocument();
  });

  it("sends unknown paths to Home", async () => {
    renderAt(<App />, "/nowhere");
    expect(screen.getByRole("link", { name: "Home" })).toHaveAttribute("aria-current", "page");
    expect(
      await screen.findByRole("heading", { level: 1, name: "Update Google Chrome." }),
    ).toBeInTheDocument();
  });

  it("lights the beams while something is exploited", async () => {
    renderAt(<App />);
    const header = screen.getByRole("banner");
    expect(
      await within(header).findByRole("img", { name: "Something needs you" }),
    ).toBeInTheDocument();
  });

  it("lists the account's devices in the account menu", async () => {
    renderAt(<App />);
    await userEvent.click(await screen.findByRole("button", { name: /Matt/ }));
    expect(screen.getByRole("group", { name: "Account" })).toHaveTextContent(
      "Family iMacAll clear",
    );
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("group", { name: "Account" })).not.toBeInTheDocument();
  });

  it("shows app info and says when data is a sample", async () => {
    renderAt(<App />);
    expect(await screen.findByText(/Wardwatch dev on browser/)).toHaveTextContent("sample data");
  });
});
