import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { describe, expect, it } from "vitest";
import { createQueryClient } from "../lib/query";
import { App } from "./App";

function renderApp(path = "/") {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <MemoryRouter initialEntries={[path]}>
        <App />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("App", () => {
  it("shows the five tabs from the design", () => {
    renderApp();
    const nav = screen.getByRole("navigation", { name: "Main" });
    expect(nav).toHaveTextContent("HomeScoreGroupsAppsUpdate plan");
  });

  it("navigates between tabs", async () => {
    renderApp();
    await userEvent.click(screen.getByRole("link", { name: "Score" }));
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Score");
  });

  it("sends unknown paths to Home", () => {
    renderApp("/nowhere");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Home");
    expect(screen.getByRole("link", { name: "Home" })).toHaveAttribute("aria-current", "page");
  });

  it("shows app info from the core, falling back outside Tauri", async () => {
    renderApp();
    expect(await screen.findByText("Wardwatch dev on browser")).toBeInTheDocument();
  });
});
