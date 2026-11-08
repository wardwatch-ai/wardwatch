import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Local-only gallery of the components: `pnpm dev`. Never deployed.
export default defineConfig({
  root: new URL(".", import.meta.url).pathname,
  plugins: [react()],
  build: { outDir: "../dist/preview", emptyOutDir: true },
});
