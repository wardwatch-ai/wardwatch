import { QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../index.css";
import { createQueryClient } from "../lib/query";
import { TrayApp } from "./TrayApp";

const root = document.getElementById("root");
if (!root) throw new Error("missing #root");

createRoot(root).render(
  <StrictMode>
    <QueryClientProvider client={createQueryClient()}>
      <TrayApp />
    </QueryClientProvider>
  </StrictMode>,
);
