// vite.config.js
import { sveltekit } from "file:///Users/fatonramadani/Documents/GitHub/windmill/frontend/node_modules/@sveltejs/kit/src/exports/vite/index.js";
import { readFileSync } from "fs";
import { fileURLToPath } from "url";
import monacoEditorPlugin from "file:///Users/fatonramadani/Documents/GitHub/windmill/frontend/node_modules/vite-plugin-monaco-editor/dist/index.js";
import circleDependency from "file:///Users/fatonramadani/Documents/GitHub/windmill/frontend/node_modules/vite-plugin-circular-dependency/dist/index.mjs";
var __vite_injected_original_import_meta_url = "file:///Users/fatonramadani/Documents/GitHub/windmill/frontend/vite.config.js";
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
  plugins: [
    sveltekit(),
    monacoEditorPlugin.default({
      publicPath: "workers",
      languageWorkers: [],
      customWorkers: [
        {
          label: "graphql",
          entry: "monaco-graphql/esm/graphql.worker"
        }
      ]
    }),
    circleDependency({ circleImportThrowErr: false })
  ],
  define: {
    __pkg__: version
  },
  optimizeDeps: {
    include: ["highlight.js", "highlight.js/lib/core", "ag-grid-svelte"]
  },
  resolve: {
    alias: {
      path: "path-browserify"
    }
  },
  assetsInclude: ["**/*.wasm"]
};
var vite_config_default = config;
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcuanMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCIvVXNlcnMvZmF0b25yYW1hZGFuaS9Eb2N1bWVudHMvR2l0SHViL3dpbmRtaWxsL2Zyb250ZW5kXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ZpbGVuYW1lID0gXCIvVXNlcnMvZmF0b25yYW1hZGFuaS9Eb2N1bWVudHMvR2l0SHViL3dpbmRtaWxsL2Zyb250ZW5kL3ZpdGUuY29uZmlnLmpzXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ltcG9ydF9tZXRhX3VybCA9IFwiZmlsZTovLy9Vc2Vycy9mYXRvbnJhbWFkYW5pL0RvY3VtZW50cy9HaXRIdWIvd2luZG1pbGwvZnJvbnRlbmQvdml0ZS5jb25maWcuanNcIjtpbXBvcnQgeyBzdmVsdGVraXQgfSBmcm9tICdAc3ZlbHRlanMva2l0L3ZpdGUnXG5pbXBvcnQgeyByZWFkRmlsZVN5bmMgfSBmcm9tICdmcydcbmltcG9ydCB7IGZpbGVVUkxUb1BhdGggfSBmcm9tICd1cmwnXG5pbXBvcnQgbW9uYWNvRWRpdG9yUGx1Z2luIGZyb20gJ3ZpdGUtcGx1Z2luLW1vbmFjby1lZGl0b3InXG5pbXBvcnQgY2lyY2xlRGVwZW5kZW5jeSBmcm9tICd2aXRlLXBsdWdpbi1jaXJjdWxhci1kZXBlbmRlbmN5J1xuXG5jb25zdCBmaWxlID0gZmlsZVVSTFRvUGF0aChuZXcgVVJMKCdwYWNrYWdlLmpzb24nLCBpbXBvcnQubWV0YS51cmwpKVxuY29uc3QganNvbiA9IHJlYWRGaWxlU3luYyhmaWxlLCAndXRmOCcpXG5jb25zdCB2ZXJzaW9uID0gSlNPTi5wYXJzZShqc29uKVxuXG4vKiogQHR5cGUge2ltcG9ydCgndml0ZScpLlVzZXJDb25maWd9ICovXG5jb25zdCBjb25maWcgPSB7XG5cdHNlcnZlcjoge1xuXHRcdHBvcnQ6IDMwMDAsXG5cdFx0cHJveHk6IHtcblx0XHRcdCdeL2FwaS8uKic6IHtcblx0XHRcdFx0dGFyZ2V0OiBwcm9jZXNzLmVudi5SRU1PVEUgPz8gJ2h0dHBzOi8vYXBwLndpbmRtaWxsLmRldi8nLFxuXHRcdFx0XHRjaGFuZ2VPcmlnaW46IHRydWUsXG5cdFx0XHRcdGNvb2tpZURvbWFpblJld3JpdGU6ICdsb2NhbGhvc3QnXG5cdFx0XHR9LFxuXHRcdFx0J14vd3MvLionOiB7XG5cdFx0XHRcdHRhcmdldDogcHJvY2Vzcy5lbnYuUkVNT1RFX0xTUCA/PyAnaHR0cHM6Ly9hcHAud2luZG1pbGwuZGV2Jyxcblx0XHRcdFx0Y2hhbmdlT3JpZ2luOiB0cnVlLFxuXHRcdFx0XHR3czogdHJ1ZVxuXHRcdFx0fSxcblx0XHRcdCdeL3dzX21wLy4qJzoge1xuXHRcdFx0XHR0YXJnZXQ6IHByb2Nlc3MuZW52LlJFTU9URV9NUCA/PyAnaHR0cHM6Ly9hcHAud2luZG1pbGwuZGV2Jyxcblx0XHRcdFx0Y2hhbmdlT3JpZ2luOiB0cnVlLFxuXHRcdFx0XHR3czogdHJ1ZVxuXHRcdFx0fVxuXHRcdH1cblx0fSxcblx0cHJldmlldzoge1xuXHRcdHBvcnQ6IDMwMDBcblx0fSxcblx0cGx1Z2luczogW1xuXHRcdHN2ZWx0ZWtpdCgpLFxuXHRcdG1vbmFjb0VkaXRvclBsdWdpbi5kZWZhdWx0KHtcblx0XHRcdHB1YmxpY1BhdGg6ICd3b3JrZXJzJyxcblx0XHRcdGxhbmd1YWdlV29ya2VyczogW10sXG5cdFx0XHRjdXN0b21Xb3JrZXJzOiBbXG5cdFx0XHRcdHtcblx0XHRcdFx0XHRsYWJlbDogJ2dyYXBocWwnLFxuXHRcdFx0XHRcdGVudHJ5OiAnbW9uYWNvLWdyYXBocWwvZXNtL2dyYXBocWwud29ya2VyJ1xuXHRcdFx0XHR9XG5cdFx0XHRdXG5cdFx0fSksXG5cdFx0Y2lyY2xlRGVwZW5kZW5jeSh7Y2lyY2xlSW1wb3J0VGhyb3dFcnI6IGZhbHNlfSksXG5cdF0sXG5cdGRlZmluZToge1xuXHRcdF9fcGtnX186IHZlcnNpb25cblx0fSxcblx0b3B0aW1pemVEZXBzOiB7XG5cdFx0aW5jbHVkZTogWydoaWdobGlnaHQuanMnLCAnaGlnaGxpZ2h0LmpzL2xpYi9jb3JlJywgJ2FnLWdyaWQtc3ZlbHRlJ11cblx0fSxcblx0cmVzb2x2ZToge1xuXHRcdGFsaWFzOiB7XG5cdFx0XHRwYXRoOiAncGF0aC1icm93c2VyaWZ5J1xuXHRcdH1cblx0fSxcblx0YXNzZXRzSW5jbHVkZTogWycqKi8qLndhc20nXVxufVxuXG5leHBvcnQgZGVmYXVsdCBjb25maWdcbiJdLAogICJtYXBwaW5ncyI6ICI7QUFBdVYsU0FBUyxpQkFBaUI7QUFDalgsU0FBUyxvQkFBb0I7QUFDN0IsU0FBUyxxQkFBcUI7QUFDOUIsT0FBTyx3QkFBd0I7QUFDL0IsT0FBTyxzQkFBc0I7QUFKeUwsSUFBTSwyQ0FBMkM7QUFNdlEsSUFBTSxPQUFPLGNBQWMsSUFBSSxJQUFJLGdCQUFnQix3Q0FBZSxDQUFDO0FBQ25FLElBQU0sT0FBTyxhQUFhLE1BQU0sTUFBTTtBQUN0QyxJQUFNLFVBQVUsS0FBSyxNQUFNLElBQUk7QUFHL0IsSUFBTSxTQUFTO0FBQUEsRUFDZCxRQUFRO0FBQUEsSUFDUCxNQUFNO0FBQUEsSUFDTixPQUFPO0FBQUEsTUFDTixZQUFZO0FBQUEsUUFDWCxRQUFRLFFBQVEsSUFBSSxVQUFVO0FBQUEsUUFDOUIsY0FBYztBQUFBLFFBQ2QscUJBQXFCO0FBQUEsTUFDdEI7QUFBQSxNQUNBLFdBQVc7QUFBQSxRQUNWLFFBQVEsUUFBUSxJQUFJLGNBQWM7QUFBQSxRQUNsQyxjQUFjO0FBQUEsUUFDZCxJQUFJO0FBQUEsTUFDTDtBQUFBLE1BQ0EsY0FBYztBQUFBLFFBQ2IsUUFBUSxRQUFRLElBQUksYUFBYTtBQUFBLFFBQ2pDLGNBQWM7QUFBQSxRQUNkLElBQUk7QUFBQSxNQUNMO0FBQUEsSUFDRDtBQUFBLEVBQ0Q7QUFBQSxFQUNBLFNBQVM7QUFBQSxJQUNSLE1BQU07QUFBQSxFQUNQO0FBQUEsRUFDQSxTQUFTO0FBQUEsSUFDUixVQUFVO0FBQUEsSUFDVixtQkFBbUIsUUFBUTtBQUFBLE1BQzFCLFlBQVk7QUFBQSxNQUNaLGlCQUFpQixDQUFDO0FBQUEsTUFDbEIsZUFBZTtBQUFBLFFBQ2Q7QUFBQSxVQUNDLE9BQU87QUFBQSxVQUNQLE9BQU87QUFBQSxRQUNSO0FBQUEsTUFDRDtBQUFBLElBQ0QsQ0FBQztBQUFBLElBQ0QsaUJBQWlCLEVBQUMsc0JBQXNCLE1BQUssQ0FBQztBQUFBLEVBQy9DO0FBQUEsRUFDQSxRQUFRO0FBQUEsSUFDUCxTQUFTO0FBQUEsRUFDVjtBQUFBLEVBQ0EsY0FBYztBQUFBLElBQ2IsU0FBUyxDQUFDLGdCQUFnQix5QkFBeUIsZ0JBQWdCO0FBQUEsRUFDcEU7QUFBQSxFQUNBLFNBQVM7QUFBQSxJQUNSLE9BQU87QUFBQSxNQUNOLE1BQU07QUFBQSxJQUNQO0FBQUEsRUFDRDtBQUFBLEVBQ0EsZUFBZSxDQUFDLFdBQVc7QUFDNUI7QUFFQSxJQUFPLHNCQUFROyIsCiAgIm5hbWVzIjogW10KfQo=
