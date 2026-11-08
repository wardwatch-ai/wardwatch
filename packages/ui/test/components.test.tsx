import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import {
  AccountPill,
  BandBar,
  Button,
  Card,
  Chip,
  Lighthouse,
  NavTab,
  SegmentedControl,
  SeverityMeter,
  StatusPill,
  SubscoreBar,
  TopBar,
  bandFor,
  setTheme,
} from "../src";

async function expectNoAxeViolations(container: HTMLElement) {
  // color-contrast needs real layout; contrast.test.ts covers it from the tokens.
  const result = await axe.run(container, { rules: { "color-contrast": { enabled: false } } });
  expect(result.violations.map((v) => `${v.id}: ${v.help}`)).toEqual([]);
}

describe("Button", () => {
  it("renders the act-now variant as a button by default", () => {
    render(<Button variant="act">Update now</Button>);
    const btn = screen.getByRole("button", { name: "Update now" });
    expect(btn).toHaveClass("ww-btn", "ww-btn-act");
    expect(btn).toHaveAttribute("type", "button");
  });

  it("renders a link when given href, keeping the act-now class", () => {
    render(
      <Button variant="act" href="#fix">
        Fix now
      </Button>,
    );
    expect(screen.getByRole("link", { name: "Fix now" })).toHaveClass("ww-btn-act");
  });

  it("gives quiet update buttons the meter and an accessible severity", () => {
    const { container } = render(
      <Button variant="quiet" severity={2}>
        Update all 3
      </Button>,
    );
    expect(screen.getByRole("button", { name: "Update all 3, moderate" })).toHaveClass("ww-btn-quiet");
    expect(container.querySelectorAll(".ww-meter-on")).toHaveLength(2);
  });

  it("forwards clicks and disabled", async () => {
    let clicks = 0;
    render(
      <>
        <Button variant="brand" onClick={() => clicks++}>
          Invite
        </Button>
        <Button variant="brand" disabled onClick={() => clicks++}>
          Done
        </Button>
      </>,
    );
    await userEvent.click(screen.getByRole("button", { name: "Invite" }));
    await userEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(clicks).toBe(1);
  });
});

describe("SeverityMeter", () => {
  it.each([1, 2, 3] as const)("fills %s of 3 bars", (level) => {
    const { container } = render(<SeverityMeter level={level} />);
    expect(container.querySelectorAll("i")).toHaveLength(3);
    expect(container.querySelectorAll(".ww-meter-on")).toHaveLength(level);
  });
});

describe("Lighthouse", () => {
  it("changes shape, not just color: beams only when something needs you", () => {
    const on = render(<Lighthouse beams />);
    expect(on.getByRole("img", { name: "Something needs you" }).querySelectorAll("path")).toHaveLength(5);
    on.unmount();
    const off = render(<Lighthouse beams={false} />);
    expect(off.getByRole("img", { name: "All clear" }).querySelectorAll("path")).toHaveLength(3);
  });
});

describe("Score", () => {
  it.each([
    [0, "At risk"],
    [59, "At risk"],
    [60, "Fair"],
    [74, "Fair"],
    [75, "Good"],
    [84, "Good"],
    [90, "Excellent"],
    [100, "Excellent"],
    [140, "Excellent"],
    [-5, "At risk"],
  ] as const)("score %s is %s", (score, band) => {
    expect(bandFor(score)).toBe(band);
  });

  it("labels the band bar and subscore meters", () => {
    render(
      <>
        <BandBar score={84} />
        <SubscoreBar value={90} label="How up to date you are" />
      </>,
    );
    expect(screen.getByRole("img", { name: "Score 84, Good band" })).toBeInTheDocument();
    expect(screen.getByRole("meter", { name: "How up to date you are" })).toHaveAttribute("aria-valuenow", "90");
  });
});

describe("toggles", () => {
  it("segmented control marks exactly one option pressed", async () => {
    function Periods() {
      const [v, setV] = useState<"30d" | "month" | "all">("30d");
      return (
        <SegmentedControl
          label="Period"
          value={v}
          onChange={setV}
          options={[
            { value: "30d", label: "Last 30 days" },
            { value: "month", label: "This month" },
            { value: "all", label: "Since joining" },
          ]}
        />
      );
    }
    render(<Periods />);
    await userEvent.click(screen.getByRole("button", { name: "This month" }));
    expect(screen.getByRole("button", { name: "This month" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "Last 30 days" })).toHaveAttribute("aria-pressed", "false");
  });

  it("chip reflects on", () => {
    render(<Chip on={false}>Balanced</Chip>);
    expect(screen.getByRole("button", { name: "Balanced" })).toHaveAttribute("aria-pressed", "false");
  });
});

describe("setTheme", () => {
  it("pins light or dark, and clears the pin for system", () => {
    setTheme("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    setTheme("light");
    expect(document.documentElement.dataset.theme).toBe("light");
    setTheme("system");
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });
});

describe("accessibility", () => {
  it("a full screen of components has no axe violations", async () => {
    const { container } = render(
      <div>
        <TopBar
          beams
          nav={
            <>
              <NavTab href="#home" current>
                Home
              </NavTab>
              <NavTab href="#score">Score</NavTab>
            </>
          }
          account={<AccountPill name="Matt" score={84} />}
        />
        <main>
          <Card tone="urgent">
            <h1>Update Chrome.</h1>
            <p>This is being exploited right now.</p>
            <Button variant="act">Update now</Button>
          </Card>
          <Card>
            <Button variant="quiet" severity={1}>
              Update now
            </Button>
            <StatusPill tone="ok">Up to date</StatusPill>
          </Card>
          <Card tone="remote">
            <StatusPill tone="remote">Update on the iPhone</StatusPill>
          </Card>
          <BandBar score={84} />
        </main>
      </div>,
    );
    expect(screen.getByRole("link", { name: "Home" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "Matt, score 84" })).toBeInTheDocument();
    await expectNoAxeViolations(container);
  });
});
