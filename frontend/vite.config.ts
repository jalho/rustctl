import * as path from "path";
import * as react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  root: ".",
  base: "./",
  build: {
    outDir: "out",
    emptyOutDir: true
  },
  plugins: [react.default()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src")
    }
  }
});
