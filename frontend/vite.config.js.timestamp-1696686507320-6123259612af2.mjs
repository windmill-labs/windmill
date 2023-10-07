// vite.config.js
import { sveltekit } from "file:///git/windmill/frontend/node_modules/@sveltejs/kit/src/exports/vite/index.js";
import { readFileSync } from "fs";
import { fileURLToPath } from "url";
import monacoEditorPlugin from "file:///git/windmill/frontend/node_modules/vite-plugin-monaco-editor/dist/index.js";
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
    })
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
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcuanMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCIvZ2l0L3dpbmRtaWxsL2Zyb250ZW5kXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ZpbGVuYW1lID0gXCIvZ2l0L3dpbmRtaWxsL2Zyb250ZW5kL3ZpdGUuY29uZmlnLmpzXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ltcG9ydF9tZXRhX3VybCA9IFwiZmlsZTovLy9naXQvd2luZG1pbGwvZnJvbnRlbmQvdml0ZS5jb25maWcuanNcIjtpbXBvcnQgeyBzdmVsdGVraXQgfSBmcm9tICdAc3ZlbHRlanMva2l0L3ZpdGUnXG5pbXBvcnQgeyByZWFkRmlsZVN5bmMgfSBmcm9tICdmcydcbmltcG9ydCB7IGZpbGVVUkxUb1BhdGggfSBmcm9tICd1cmwnXG5pbXBvcnQgbW9uYWNvRWRpdG9yUGx1Z2luIGZyb20gJ3ZpdGUtcGx1Z2luLW1vbmFjby1lZGl0b3InXG5cbmNvbnN0IGZpbGUgPSBmaWxlVVJMVG9QYXRoKG5ldyBVUkwoJ3BhY2thZ2UuanNvbicsIGltcG9ydC5tZXRhLnVybCkpXG5jb25zdCBqc29uID0gcmVhZEZpbGVTeW5jKGZpbGUsICd1dGY4JylcbmNvbnN0IHZlcnNpb24gPSBKU09OLnBhcnNlKGpzb24pXG5cbi8qKiBAdHlwZSB7aW1wb3J0KCd2aXRlJykuVXNlckNvbmZpZ30gKi9cbmNvbnN0IGNvbmZpZyA9IHtcblx0c2VydmVyOiB7XG5cdFx0cG9ydDogMzAwMCxcblx0XHRwcm94eToge1xuXHRcdFx0J14vYXBpLy4qJzoge1xuXHRcdFx0XHR0YXJnZXQ6IHByb2Nlc3MuZW52LlJFTU9URSA/PyAnaHR0cHM6Ly9hcHAud2luZG1pbGwuZGV2LycsXG5cdFx0XHRcdGNoYW5nZU9yaWdpbjogdHJ1ZSxcblx0XHRcdFx0Y29va2llRG9tYWluUmV3cml0ZTogJ2xvY2FsaG9zdCdcblx0XHRcdH0sXG5cdFx0XHQnXi93cy8uKic6IHtcblx0XHRcdFx0dGFyZ2V0OiBwcm9jZXNzLmVudi5SRU1PVEVfTFNQID8/ICdodHRwczovL2FwcC53aW5kbWlsbC5kZXYnLFxuXHRcdFx0XHRjaGFuZ2VPcmlnaW46IHRydWUsXG5cdFx0XHRcdHdzOiB0cnVlXG5cdFx0XHR9LFxuXHRcdFx0J14vd3NfbXAvLionOiB7XG5cdFx0XHRcdHRhcmdldDogcHJvY2Vzcy5lbnYuUkVNT1RFX01QID8/ICdodHRwczovL2FwcC53aW5kbWlsbC5kZXYnLFxuXHRcdFx0XHRjaGFuZ2VPcmlnaW46IHRydWUsXG5cdFx0XHRcdHdzOiB0cnVlXG5cdFx0XHR9XG5cdFx0fVxuXHR9LFxuXHRwcmV2aWV3OiB7XG5cdFx0cG9ydDogMzAwMFxuXHR9LFxuXHRwbHVnaW5zOiBbXG5cdFx0c3ZlbHRla2l0KCksXG5cdFx0bW9uYWNvRWRpdG9yUGx1Z2luLmRlZmF1bHQoe1xuXHRcdFx0cHVibGljUGF0aDogJ3dvcmtlcnMnLFxuXHRcdFx0bGFuZ3VhZ2VXb3JrZXJzOiBbXSxcblx0XHRcdGN1c3RvbVdvcmtlcnM6IFtcblx0XHRcdFx0e1xuXHRcdFx0XHRcdGxhYmVsOiAnZ3JhcGhxbCcsXG5cdFx0XHRcdFx0ZW50cnk6ICdtb25hY28tZ3JhcGhxbC9lc20vZ3JhcGhxbC53b3JrZXInXG5cdFx0XHRcdH1cblx0XHRcdF1cblx0XHR9KVxuXHRdLFxuXHRkZWZpbmU6IHtcblx0XHRfX3BrZ19fOiB2ZXJzaW9uXG5cdH0sXG5cdG9wdGltaXplRGVwczoge1xuXHRcdGluY2x1ZGU6IFsnaGlnaGxpZ2h0LmpzJywgJ2hpZ2hsaWdodC5qcy9saWIvY29yZScsICdhZy1ncmlkLXN2ZWx0ZSddXG5cdH0sXG5cdHJlc29sdmU6IHtcblx0XHRhbGlhczoge1xuXHRcdFx0cGF0aDogJ3BhdGgtYnJvd3NlcmlmeSdcblx0XHR9XG5cdH0sXG5cdGFzc2V0c0luY2x1ZGU6IFsnKiovKi53YXNtJ11cbn1cblxuZXhwb3J0IGRlZmF1bHQgY29uZmlnXG4iXSwKICAibWFwcGluZ3MiOiAiO0FBQW9QLFNBQVMsaUJBQWlCO0FBQzlRLFNBQVMsb0JBQW9CO0FBQzdCLFNBQVMscUJBQXFCO0FBQzlCLE9BQU8sd0JBQXdCO0FBSHFILElBQU0sMkNBQTJDO0FBS3JNLElBQU0sT0FBTyxjQUFjLElBQUksSUFBSSxnQkFBZ0Isd0NBQWUsQ0FBQztBQUNuRSxJQUFNLE9BQU8sYUFBYSxNQUFNLE1BQU07QUFDdEMsSUFBTSxVQUFVLEtBQUssTUFBTSxJQUFJO0FBRy9CLElBQU0sU0FBUztBQUFBLEVBQ2QsUUFBUTtBQUFBLElBQ1AsTUFBTTtBQUFBLElBQ04sT0FBTztBQUFBLE1BQ04sWUFBWTtBQUFBLFFBQ1gsUUFBUSxRQUFRLElBQUksVUFBVTtBQUFBLFFBQzlCLGNBQWM7QUFBQSxRQUNkLHFCQUFxQjtBQUFBLE1BQ3RCO0FBQUEsTUFDQSxXQUFXO0FBQUEsUUFDVixRQUFRLFFBQVEsSUFBSSxjQUFjO0FBQUEsUUFDbEMsY0FBYztBQUFBLFFBQ2QsSUFBSTtBQUFBLE1BQ0w7QUFBQSxNQUNBLGNBQWM7QUFBQSxRQUNiLFFBQVEsUUFBUSxJQUFJLGFBQWE7QUFBQSxRQUNqQyxjQUFjO0FBQUEsUUFDZCxJQUFJO0FBQUEsTUFDTDtBQUFBLElBQ0Q7QUFBQSxFQUNEO0FBQUEsRUFDQSxTQUFTO0FBQUEsSUFDUixNQUFNO0FBQUEsRUFDUDtBQUFBLEVBQ0EsU0FBUztBQUFBLElBQ1IsVUFBVTtBQUFBLElBQ1YsbUJBQW1CLFFBQVE7QUFBQSxNQUMxQixZQUFZO0FBQUEsTUFDWixpQkFBaUIsQ0FBQztBQUFBLE1BQ2xCLGVBQWU7QUFBQSxRQUNkO0FBQUEsVUFDQyxPQUFPO0FBQUEsVUFDUCxPQUFPO0FBQUEsUUFDUjtBQUFBLE1BQ0Q7QUFBQSxJQUNELENBQUM7QUFBQSxFQUNGO0FBQUEsRUFDQSxRQUFRO0FBQUEsSUFDUCxTQUFTO0FBQUEsRUFDVjtBQUFBLEVBQ0EsY0FBYztBQUFBLElBQ2IsU0FBUyxDQUFDLGdCQUFnQix5QkFBeUIsZ0JBQWdCO0FBQUEsRUFDcEU7QUFBQSxFQUNBLFNBQVM7QUFBQSxJQUNSLE9BQU87QUFBQSxNQUNOLE1BQU07QUFBQSxJQUNQO0FBQUEsRUFDRDtBQUFBLEVBQ0EsZUFBZSxDQUFDLFdBQVc7QUFDNUI7QUFFQSxJQUFPLHNCQUFROyIsCiAgIm5hbWVzIjogW10KfQo=
