import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte(), react()],

  build: {
    lib: {
      entry: "./lib/main.tsx",
      name: "FileViewer",
      fileName: "file-viewer",
    },
  },
  assetsInclude: ["**/*.wasm"],
  define: {
    __pkg__: {
      version: "1.99.0",
    },
  },
});
