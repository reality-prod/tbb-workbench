import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "src") },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks(id) {
          // Only pull out the one dependency group that's actually worth
          // isolating (CodeMirror, used only by the lazy-loaded Editor
          // route). Everything else is left to Rollup's default chunking
          // so we don't risk circular chunk graphs between vendor groups
          // that import each other (e.g. react-router importing react).
          if (id.includes("node_modules") && (id.includes("@codemirror") || id.includes("/codemirror/") || id.includes("@lezer"))) {
            return "vendor-codemirror";
          }
          return undefined;
        },
      },
    },
  },
});
