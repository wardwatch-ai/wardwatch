import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderAt } from "../../test/render";
import { Home } from "./Home";

describe("Home", () => {
  it("leads with the exploited program and an amber button to fix it", async () => {
    renderAt(<Home />);
    expect(
      await screen.findByRole("heading", { level: 1, name: "Update Google Chrome." }),
    ).toBeInTheDocument();
    expect(screen.getByText("This is being exploited right now.")).toBeInTheDocument();
    const act = screen.getByRole("button", { name: "Update Google Chrome now" });
    expect(act).toHaveClass("ww-btn-act");
    await userEvent.click(act);
    expect(screen.getByTestId("path")).toHaveTextContent("/fix/mac_app/com.google.Chrome");
  });

  it("shows the score, the group glance and the tune-up", async () => {
    renderAt(<Home />);
    expect(await screen.findByText("84")).toBeInTheDocument();
    expect(await screen.findByText("in Ward family")).toBeInTheDocument();
    expect(screen.getByText("Sam leads at 93")).toBeInTheDocument();
    const all = screen.getByRole("button", { name: /Update all 4/ });
    expect(all).toHaveClass("ww-btn-quiet");
    expect(all).toHaveTextContent("moderate");
  });

  it("shows this device and other devices, with status only", async () => {
    renderAt(<Home />);
    expect(await screen.findByText("Google Chrome needs you")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /Apps/ })).toHaveTextContent("2 of 6 current");
    expect(await screen.findByText("iPhone")).toBeInTheDocument();
    expect(screen.queryByText("MacBook Air")).not.toBeInTheDocument();
  });
});
