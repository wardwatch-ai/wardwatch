import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Local-only gallery of the components: `pnpm dev`. Never deployed.
export default defineConfig({
  root: import.meta.dirname,
  plugins: [react()],
  build: { outDir: "../dist/preview", emptyOutDir: true },
});
