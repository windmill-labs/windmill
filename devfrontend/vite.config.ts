import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 5173,
    proxy: {
      "^/api/.*": {
        target: process.env.REMOTE ?? "http://localhost:3000/",
        changeOrigin: true,
        cookieDomainRewrite: "localhost",
      },
    },
  },
});
