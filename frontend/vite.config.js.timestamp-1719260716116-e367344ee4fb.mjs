// vite.config.js
import { sveltekit } from "file:///git/windmill/frontend/node_modules/@sveltejs/kit/src/exports/vite/index.js";
import { readFileSync } from "fs";
import { fileURLToPath } from "url";
import circleDependency from "file:///git/windmill/frontend/node_modules/vite-plugin-circular-dependency/dist/index.mjs";
var __vite_injected_original_import_meta_url = "file:///git/windmill/frontend/vite.config.js";
var file = fileURLToPath(new URL("package.json", __vite_injected_original_import_meta_url));
var json = readFileSync(file, "utf8");
var version = JSON.parse(json);
var config = {
  server: {
    port: 3e3,
    proxy: {
      "^/api/.*": {
        target: process.env.REMOTE ?? "https://app.windmill.dev/",
        changeOrigin: true,
        cookieDomainRewrite: "localhost"
      },
      "^/ws/.*": {
        target: process.env.REMOTE_LSP ?? "https://app.windmill.dev",
        changeOrigin: true,
        ws: true
      },
      "^/ws_mp/.*": {
        target: process.env.REMOTE_MP ?? "https://app.windmill.dev",
        changeOrigin: true,
        ws: true
      }
    }
  },
  preview: {
    port: 3e3
  },
  plugins: [sveltekit(), circleDependency({ circleImportThrowErr: false })],
  define: {
    __pkg__: version
  },
  optimizeDeps: {
    include: ["highlight.js", "highlight.js/lib/core"]
  },
  resolve: {
    alias: {
      path: "path-browserify"
    },
    dedupe: ["monaco-editor", "vscode"]
  },
  assetsInclude: ["**/*.wasm"]
};
var vite_config_default = config;
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcuanMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCIvZ2l0L3dpbmRtaWxsL2Zyb250ZW5kXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ZpbGVuYW1lID0gXCIvZ2l0L3dpbmRtaWxsL2Zyb250ZW5kL3ZpdGUuY29uZmlnLmpzXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ltcG9ydF9tZXRhX3VybCA9IFwiZmlsZTovLy9naXQvd2luZG1pbGwvZnJvbnRlbmQvdml0ZS5jb25maWcuanNcIjtpbXBvcnQgeyBzdmVsdGVraXQgfSBmcm9tICdAc3ZlbHRlanMva2l0L3ZpdGUnXG5pbXBvcnQgeyByZWFkRmlsZVN5bmMgfSBmcm9tICdmcydcbmltcG9ydCB7IGZpbGVVUkxUb1BhdGggfSBmcm9tICd1cmwnXG5pbXBvcnQgY2lyY2xlRGVwZW5kZW5jeSBmcm9tICd2aXRlLXBsdWdpbi1jaXJjdWxhci1kZXBlbmRlbmN5J1xuXG5jb25zdCBmaWxlID0gZmlsZVVSTFRvUGF0aChuZXcgVVJMKCdwYWNrYWdlLmpzb24nLCBpbXBvcnQubWV0YS51cmwpKVxuY29uc3QganNvbiA9IHJlYWRGaWxlU3luYyhmaWxlLCAndXRmOCcpXG5jb25zdCB2ZXJzaW9uID0gSlNPTi5wYXJzZShqc29uKVxuXG4vKiogQHR5cGUge2ltcG9ydCgndml0ZScpLlVzZXJDb25maWd9ICovXG5jb25zdCBjb25maWcgPSB7XG5cdHNlcnZlcjoge1xuXHRcdHBvcnQ6IDMwMDAsXG5cdFx0cHJveHk6IHtcblx0XHRcdCdeL2FwaS8uKic6IHtcblx0XHRcdFx0dGFyZ2V0OiBwcm9jZXNzLmVudi5SRU1PVEUgPz8gJ2h0dHBzOi8vYXBwLndpbmRtaWxsLmRldi8nLFxuXHRcdFx0XHRjaGFuZ2VPcmlnaW46IHRydWUsXG5cdFx0XHRcdGNvb2tpZURvbWFpblJld3JpdGU6ICdsb2NhbGhvc3QnXG5cdFx0XHR9LFxuXHRcdFx0J14vd3MvLionOiB7XG5cdFx0XHRcdHRhcmdldDogcHJvY2Vzcy5lbnYuUkVNT1RFX0xTUCA/PyAnaHR0cHM6Ly9hcHAud2luZG1pbGwuZGV2Jyxcblx0XHRcdFx0Y2hhbmdlT3JpZ2luOiB0cnVlLFxuXHRcdFx0XHR3czogdHJ1ZVxuXHRcdFx0fSxcblx0XHRcdCdeL3dzX21wLy4qJzoge1xuXHRcdFx0XHR0YXJnZXQ6IHByb2Nlc3MuZW52LlJFTU9URV9NUCA/PyAnaHR0cHM6Ly9hcHAud2luZG1pbGwuZGV2Jyxcblx0XHRcdFx0Y2hhbmdlT3JpZ2luOiB0cnVlLFxuXHRcdFx0XHR3czogdHJ1ZVxuXHRcdFx0fVxuXHRcdH1cblx0fSxcblx0cHJldmlldzoge1xuXHRcdHBvcnQ6IDMwMDBcblx0fSxcblx0cGx1Z2luczogW3N2ZWx0ZWtpdCgpLCBjaXJjbGVEZXBlbmRlbmN5KHsgY2lyY2xlSW1wb3J0VGhyb3dFcnI6IGZhbHNlIH0pXSxcblx0ZGVmaW5lOiB7XG5cdFx0X19wa2dfXzogdmVyc2lvblxuXHR9LFxuXHRvcHRpbWl6ZURlcHM6IHtcblx0XHRpbmNsdWRlOiBbJ2hpZ2hsaWdodC5qcycsICdoaWdobGlnaHQuanMvbGliL2NvcmUnXVxuXHR9LFxuXHRyZXNvbHZlOiB7XG5cdFx0YWxpYXM6IHtcblx0XHRcdHBhdGg6ICdwYXRoLWJyb3dzZXJpZnknXG5cdFx0fSxcblx0XHRkZWR1cGU6IFsnbW9uYWNvLWVkaXRvcicsICd2c2NvZGUnXVxuXHR9LFxuXHRhc3NldHNJbmNsdWRlOiBbJyoqLyoud2FzbSddXG59XG5cbmV4cG9ydCBkZWZhdWx0IGNvbmZpZ1xuIl0sCiAgIm1hcHBpbmdzIjogIjtBQUFvUCxTQUFTLGlCQUFpQjtBQUM5USxTQUFTLG9CQUFvQjtBQUM3QixTQUFTLHFCQUFxQjtBQUM5QixPQUFPLHNCQUFzQjtBQUh1SCxJQUFNLDJDQUEyQztBQUtyTSxJQUFNLE9BQU8sY0FBYyxJQUFJLElBQUksZ0JBQWdCLHdDQUFlLENBQUM7QUFDbkUsSUFBTSxPQUFPLGFBQWEsTUFBTSxNQUFNO0FBQ3RDLElBQU0sVUFBVSxLQUFLLE1BQU0sSUFBSTtBQUcvQixJQUFNLFNBQVM7QUFBQSxFQUNkLFFBQVE7QUFBQSxJQUNQLE1BQU07QUFBQSxJQUNOLE9BQU87QUFBQSxNQUNOLFlBQVk7QUFBQSxRQUNYLFFBQVEsUUFBUSxJQUFJLFVBQVU7QUFBQSxRQUM5QixjQUFjO0FBQUEsUUFDZCxxQkFBcUI7QUFBQSxNQUN0QjtBQUFBLE1BQ0EsV0FBVztBQUFBLFFBQ1YsUUFBUSxRQUFRLElBQUksY0FBYztBQUFBLFFBQ2xDLGNBQWM7QUFBQSxRQUNkLElBQUk7QUFBQSxNQUNMO0FBQUEsTUFDQSxjQUFjO0FBQUEsUUFDYixRQUFRLFFBQVEsSUFBSSxhQUFhO0FBQUEsUUFDakMsY0FBYztBQUFBLFFBQ2QsSUFBSTtBQUFBLE1BQ0w7QUFBQSxJQUNEO0FBQUEsRUFDRDtBQUFBLEVBQ0EsU0FBUztBQUFBLElBQ1IsTUFBTTtBQUFBLEVBQ1A7QUFBQSxFQUNBLFNBQVMsQ0FBQyxVQUFVLEdBQUcsaUJBQWlCLEVBQUUsc0JBQXNCLE1BQU0sQ0FBQyxDQUFDO0FBQUEsRUFDeEUsUUFBUTtBQUFBLElBQ1AsU0FBUztBQUFBLEVBQ1Y7QUFBQSxFQUNBLGNBQWM7QUFBQSxJQUNiLFNBQVMsQ0FBQyxnQkFBZ0IsdUJBQXVCO0FBQUEsRUFDbEQ7QUFBQSxFQUNBLFNBQVM7QUFBQSxJQUNSLE9BQU87QUFBQSxNQUNOLE1BQU07QUFBQSxJQUNQO0FBQUEsSUFDQSxRQUFRLENBQUMsaUJBQWlCLFFBQVE7QUFBQSxFQUNuQztBQUFBLEVBQ0EsZUFBZSxDQUFDLFdBQVc7QUFDNUI7QUFFQSxJQUFPLHNCQUFROyIsCiAgIm5hbWVzIjogW10KfQo=
