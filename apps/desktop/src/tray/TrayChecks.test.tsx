import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createQueryClient } from "../lib/query";
import { TrayApp } from "./TrayApp";

// The sample's findings, every one already updated: each row is current.
vi.mock("../lib/ipc", async (original) => {
  const ipc = await original<typeof import("../lib/ipc")>();
  const findings = (await ipc.listFindings()) ?? [];
  return {
    ...ipc,
    listFindings: () =>
      Promise.resolve(
        findings.map((f) => ({ ...f, update_available: false, exploited: false, vulns: [] })),
      ),
  };
});

describe("TrayApp when everything is current", () => {
  it("shows a check beside each current row", async () => {
    const { container } = render(
      <QueryClientProvider client={createQueryClient()}>
        <TrayApp />
      </QueryClientProvider>,
    );
    expect(await screen.findAllByText("Current")).toHaveLength(3);
    const rows = [...container.querySelectorAll("li")].filter((li) =>
      li.textContent.endsWith("Current"),
    );
    expect(rows).toHaveLength(3);
    // CheckGlyph strokes with the ok color; the urgent triangle and blank spacer don't.
    for (const row of rows) expect(row.querySelector('svg[stroke="var(--ww-ok)"]')).not.toBeNull();
  });
});
