import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { readFileSync } from "fs";
import { fileURLToPath } from "url";

const file = fileURLToPath(
  new URL("../frontend/package.json", import.meta.url)
);
const json = readFileSync(file, "utf8");
const version = JSON.parse(json);

export default defineConfig({
  plugins: [svelte(), react()],
  server: {
    port: 3000,
    proxy: {
      "^/api/.*": {
        target: process.env.REMOTE ?? "https://app.windmill.dev/",
        changeOrigin: true,
        cookieDomainRewrite: "localhost",
      },
      // Proxying websockets or socket.io: ws://localhost:5173/socket.io -> ws://localhost:5174/socket.io
      "^/ws/.*": {
        target: process.env.REMOTE_LSP ?? "https://app.windmill.dev",
        changeOrigin: true,
        ws: true,
      },
    },
  },
  build: {
    lib: {
      entry: "./lib/main.tsx",
      name: "FileViewer",
      fileName: "file-viewer",
    },
  },
  optimizeDeps: {
    include: ["highlight.js", "highlight.js/lib/core"],
  },
  assetsInclude: ["**/*.wasm"],
  define: {
    __pkg__: {
      version: version,
    },
  },
});
