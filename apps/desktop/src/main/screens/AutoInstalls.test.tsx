import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderAt } from "../../test/render";
import { Settings } from "./Settings";

// Outside Tauri the log is the shared sample: one of each result.
describe("Settings: installed by itself", () => {
  it("lists every automatic install and why one waits", async () => {
    renderAt(<Settings />);
    expect(await screen.findByText("Zoom")).toBeInTheDocument();
    expect(screen.getByText(/Waiting for you: it's open right now/)).toBeInTheDocument();
    expect(screen.getByText("144.0.2 → 145.0.1")).toBeInTheDocument();
    expect(screen.getByText(/^Installed ·/)).toBeInTheDocument();
    expect(screen.getByText(/Didn't work: the updater has nothing newer yet/)).toBeInTheDocument();
  });
});
