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
    expect(screen.getByRole("dialog", { name: "Account" })).toHaveTextContent(
      "Family iMacAll clear",
    );
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog", { name: "Account" })).not.toBeInTheDocument();
  });

  it("opens Settings from the account menu", async () => {
    renderAt(<App />);
    await userEvent.click(await screen.findByRole("button", { name: /Matt/ }));
    await userEvent.click(screen.getByRole("link", { name: "Settings" }));
    expect(screen.getByTestId("path")).toHaveTextContent("/settings");
    expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
    expect(screen.queryByRole("dialog", { name: "Account" })).not.toBeInTheDocument();
  });

  it("shows app info and says when data is a sample", async () => {
    renderAt(<App />);
    expect(await screen.findByText(/Wardwatch dev on browser/)).toHaveTextContent("sample data");
  });

  it("goes back to the screen that opened a fix", async () => {
    renderAt(<App />, "/apps");
    const [update] = await screen.findAllByRole("button", { name: /^Update now/ });
    if (!update) throw new Error("no update button on Apps");
    await userEvent.click(update);
    await userEvent.click(await screen.findByRole("button", { name: "Back to Apps" }));
    expect(screen.getByRole("link", { name: "Apps" })).toHaveAttribute("aria-current", "page");
  });

  it("offers Home from a fix opened outside the app, such as the menu bar", async () => {
    renderAt(<App />, "/fix/mac_app/com.google.Chrome");
    const main = screen.getByRole("main");
    expect(await within(main).findByRole("link", { name: "Home" })).toHaveAttribute("href", "/");
    expect(within(main).queryByRole("button", { name: /^Back to/ })).toBeNull();
  });
});
