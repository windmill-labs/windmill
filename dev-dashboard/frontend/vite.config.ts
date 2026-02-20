import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  server: {
    port: 5112,
    proxy: {
      "/api": "http://localhost:5111",
      "/ws": {
        target: "ws://localhost:5111",
        ws: true,
      },
    },
  },
});
