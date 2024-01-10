// vite.config.js
import { sveltekit } from "file:///git/windmill/frontend/node_modules/@sveltejs/kit/src/exports/vite/index.js";
import { readFileSync } from "fs";
import { fileURLToPath } from "url";
import monacoEditorPlugin from "file:///git/windmill/frontend/node_modules/vite-plugin-monaco-editor/dist/index.js";
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
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcuanMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCIvZ2l0L3dpbmRtaWxsL2Zyb250ZW5kXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ZpbGVuYW1lID0gXCIvZ2l0L3dpbmRtaWxsL2Zyb250ZW5kL3ZpdGUuY29uZmlnLmpzXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ltcG9ydF9tZXRhX3VybCA9IFwiZmlsZTovLy9naXQvd2luZG1pbGwvZnJvbnRlbmQvdml0ZS5jb25maWcuanNcIjtpbXBvcnQgeyBzdmVsdGVraXQgfSBmcm9tICdAc3ZlbHRlanMva2l0L3ZpdGUnXG5pbXBvcnQgeyByZWFkRmlsZVN5bmMgfSBmcm9tICdmcydcbmltcG9ydCB7IGZpbGVVUkxUb1BhdGggfSBmcm9tICd1cmwnXG5pbXBvcnQgbW9uYWNvRWRpdG9yUGx1Z2luIGZyb20gJ3ZpdGUtcGx1Z2luLW1vbmFjby1lZGl0b3InXG5pbXBvcnQgY2lyY2xlRGVwZW5kZW5jeSBmcm9tICd2aXRlLXBsdWdpbi1jaXJjdWxhci1kZXBlbmRlbmN5J1xuXG5jb25zdCBmaWxlID0gZmlsZVVSTFRvUGF0aChuZXcgVVJMKCdwYWNrYWdlLmpzb24nLCBpbXBvcnQubWV0YS51cmwpKVxuY29uc3QganNvbiA9IHJlYWRGaWxlU3luYyhmaWxlLCAndXRmOCcpXG5jb25zdCB2ZXJzaW9uID0gSlNPTi5wYXJzZShqc29uKVxuXG4vKiogQHR5cGUge2ltcG9ydCgndml0ZScpLlVzZXJDb25maWd9ICovXG5jb25zdCBjb25maWcgPSB7XG5cdHNlcnZlcjoge1xuXHRcdHBvcnQ6IDMwMDAsXG5cdFx0cHJveHk6IHtcblx0XHRcdCdeL2FwaS8uKic6IHtcblx0XHRcdFx0dGFyZ2V0OiBwcm9jZXNzLmVudi5SRU1PVEUgPz8gJ2h0dHBzOi8vYXBwLndpbmRtaWxsLmRldi8nLFxuXHRcdFx0XHRjaGFuZ2VPcmlnaW46IHRydWUsXG5cdFx0XHRcdGNvb2tpZURvbWFpblJld3JpdGU6ICdsb2NhbGhvc3QnXG5cdFx0XHR9LFxuXHRcdFx0J14vd3MvLionOiB7XG5cdFx0XHRcdHRhcmdldDogcHJvY2Vzcy5lbnYuUkVNT1RFX0xTUCA/PyAnaHR0cHM6Ly9hcHAud2luZG1pbGwuZGV2Jyxcblx0XHRcdFx0Y2hhbmdlT3JpZ2luOiB0cnVlLFxuXHRcdFx0XHR3czogdHJ1ZVxuXHRcdFx0fSxcblx0XHRcdCdeL3dzX21wLy4qJzoge1xuXHRcdFx0XHR0YXJnZXQ6IHByb2Nlc3MuZW52LlJFTU9URV9NUCA/PyAnaHR0cHM6Ly9hcHAud2luZG1pbGwuZGV2Jyxcblx0XHRcdFx0Y2hhbmdlT3JpZ2luOiB0cnVlLFxuXHRcdFx0XHR3czogdHJ1ZVxuXHRcdFx0fVxuXHRcdH1cblx0fSxcblx0cHJldmlldzoge1xuXHRcdHBvcnQ6IDMwMDBcblx0fSxcblx0cGx1Z2luczogW1xuXHRcdHN2ZWx0ZWtpdCgpLFxuXHRcdG1vbmFjb0VkaXRvclBsdWdpbi5kZWZhdWx0KHtcblx0XHRcdHB1YmxpY1BhdGg6ICd3b3JrZXJzJyxcblx0XHRcdGxhbmd1YWdlV29ya2VyczogW10sXG5cdFx0XHRjdXN0b21Xb3JrZXJzOiBbXG5cdFx0XHRcdHtcblx0XHRcdFx0XHRsYWJlbDogJ2dyYXBocWwnLFxuXHRcdFx0XHRcdGVudHJ5OiAnbW9uYWNvLWdyYXBocWwvZXNtL2dyYXBocWwud29ya2VyJ1xuXHRcdFx0XHR9XG5cdFx0XHRdXG5cdFx0fSksXG5cdFx0Y2lyY2xlRGVwZW5kZW5jeSh7IGNpcmNsZUltcG9ydFRocm93RXJyOiBmYWxzZSB9KVxuXHRdLFxuXHRkZWZpbmU6IHtcblx0XHRfX3BrZ19fOiB2ZXJzaW9uXG5cdH0sXG5cdG9wdGltaXplRGVwczoge1xuXHRcdGluY2x1ZGU6IFsnaGlnaGxpZ2h0LmpzJywgJ2hpZ2hsaWdodC5qcy9saWIvY29yZSddXG5cdH0sXG5cdHJlc29sdmU6IHtcblx0XHRhbGlhczoge1xuXHRcdFx0cGF0aDogJ3BhdGgtYnJvd3NlcmlmeSdcblx0XHR9LFxuXHRcdGRlZHVwZTogWydtb25hY28tZWRpdG9yJywgJ3ZzY29kZSddXG5cdH0sXG5cdGFzc2V0c0luY2x1ZGU6IFsnKiovKi53YXNtJ11cbn1cblxuZXhwb3J0IGRlZmF1bHQgY29uZmlnXG4iXSwKICAibWFwcGluZ3MiOiAiO0FBQW9QLFNBQVMsaUJBQWlCO0FBQzlRLFNBQVMsb0JBQW9CO0FBQzdCLFNBQVMscUJBQXFCO0FBQzlCLE9BQU8sd0JBQXdCO0FBQy9CLE9BQU8sc0JBQXNCO0FBSnVILElBQU0sMkNBQTJDO0FBTXJNLElBQU0sT0FBTyxjQUFjLElBQUksSUFBSSxnQkFBZ0Isd0NBQWUsQ0FBQztBQUNuRSxJQUFNLE9BQU8sYUFBYSxNQUFNLE1BQU07QUFDdEMsSUFBTSxVQUFVLEtBQUssTUFBTSxJQUFJO0FBRy9CLElBQU0sU0FBUztBQUFBLEVBQ2QsUUFBUTtBQUFBLElBQ1AsTUFBTTtBQUFBLElBQ04sT0FBTztBQUFBLE1BQ04sWUFBWTtBQUFBLFFBQ1gsUUFBUSxRQUFRLElBQUksVUFBVTtBQUFBLFFBQzlCLGNBQWM7QUFBQSxRQUNkLHFCQUFxQjtBQUFBLE1BQ3RCO0FBQUEsTUFDQSxXQUFXO0FBQUEsUUFDVixRQUFRLFFBQVEsSUFBSSxjQUFjO0FBQUEsUUFDbEMsY0FBYztBQUFBLFFBQ2QsSUFBSTtBQUFBLE1BQ0w7QUFBQSxNQUNBLGNBQWM7QUFBQSxRQUNiLFFBQVEsUUFBUSxJQUFJLGFBQWE7QUFBQSxRQUNqQyxjQUFjO0FBQUEsUUFDZCxJQUFJO0FBQUEsTUFDTDtBQUFBLElBQ0Q7QUFBQSxFQUNEO0FBQUEsRUFDQSxTQUFTO0FBQUEsSUFDUixNQUFNO0FBQUEsRUFDUDtBQUFBLEVBQ0EsU0FBUztBQUFBLElBQ1IsVUFBVTtBQUFBLElBQ1YsbUJBQW1CLFFBQVE7QUFBQSxNQUMxQixZQUFZO0FBQUEsTUFDWixpQkFBaUIsQ0FBQztBQUFBLE1BQ2xCLGVBQWU7QUFBQSxRQUNkO0FBQUEsVUFDQyxPQUFPO0FBQUEsVUFDUCxPQUFPO0FBQUEsUUFDUjtBQUFBLE1BQ0Q7QUFBQSxJQUNELENBQUM7QUFBQSxJQUNELGlCQUFpQixFQUFFLHNCQUFzQixNQUFNLENBQUM7QUFBQSxFQUNqRDtBQUFBLEVBQ0EsUUFBUTtBQUFBLElBQ1AsU0FBUztBQUFBLEVBQ1Y7QUFBQSxFQUNBLGNBQWM7QUFBQSxJQUNiLFNBQVMsQ0FBQyxnQkFBZ0IsdUJBQXVCO0FBQUEsRUFDbEQ7QUFBQSxFQUNBLFNBQVM7QUFBQSxJQUNSLE9BQU87QUFBQSxNQUNOLE1BQU07QUFBQSxJQUNQO0FBQUEsSUFDQSxRQUFRLENBQUMsaUJBQWlCLFFBQVE7QUFBQSxFQUNuQztBQUFBLEVBQ0EsZUFBZSxDQUFDLFdBQVc7QUFDNUI7QUFFQSxJQUFPLHNCQUFROyIsCiAgIm5hbWVzIjogW10KfQo=
