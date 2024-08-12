import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte(), cssInjectedByJsPlugin()],
  build: {
    lib: { entry: "src/main.js", formats: ["iife"], name: "app" },
  },
  define: {
    "process.env.NODE_ENV": '"production"',
    __pkg__: '"0.42.0"'
  },
});
