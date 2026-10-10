import { defineConfig } from "vitest/config";

// One self-contained script (styles included) for web pages and the TEdgeBrowser page.
export default defineConfig({
  publicDir: false,
  build: {
    target: "chrome120",
    lib: {
      entry: "src/index.ts",
      name: "MrTextEdit",
      formats: ["iife"],
      fileName: () => "mrtextedit.js",
    },
  },
  test: {
    environment: "jsdom",
  },
});
