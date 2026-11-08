/* eslint-disable react-refresh/only-export-components -- test helper, never hot-reloaded */
import { QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router";
import { createQueryClient } from "../lib/query";

/** Shows the current path, so tests can check where a button went. */
function Where() {
  const { pathname } = useLocation();
  return <output data-testid="path">{pathname}</output>;
}

export function renderAt(ui: ReactNode, path = "/") {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path="*" element={ui} />
        </Routes>
        <Where />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
