// vite.config.js
import { sveltekit } from "file:///home/dieri/windmill/frontend/node_modules/@sveltejs/kit/src/exports/vite/index.js";
import { readFileSync } from "fs";
import { fileURLToPath } from "url";
import circleDependency from "file:///home/dieri/windmill/frontend/node_modules/vite-plugin-circular-dependency/dist/index.mjs";
import importMetaUrlPlugin from "file:///home/dieri/windmill/frontend/node_modules/@windmill-labs/esbuild-import-meta-url-plugin/dist/esbuildImportMetaUrlPlugin.js";
var __vite_injected_original_import_meta_url = "file:///home/dieri/windmill/frontend/vite.config.js";
var file = fileURLToPath(new URL("package.json", __vite_injected_original_import_meta_url));
var json = readFileSync(file, "utf8");
var version = JSON.parse(json);
var config = {
  server: {
    https: false,
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
    include: ["highlight.js", "highlight.js/lib/core", "monaco-vim"],
    exclude: [
      "@codingame/monaco-vscode-standalone-typescript-language-features",
      "@codingame/monaco-vscode-standalone-languages"
    ],
    esbuildOptions: {
      plugins: [importMetaUrlPlugin]
    }
  },
  resolve: {
    alias: {
      path: "path-browserify",
      "monaco-editor/esm/vs/editor/contrib/hover/browser/hover": "vscode/vscode/vs/editor/contrib/hover/browser/hoverContribution"
    },
    dedupe: ["vscode", "monaco-editor"]
  },
  worker: {
    format: "es"
  },
  assetsInclude: ["**/*.wasm"]
};
var vite_config_default = config;
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcuanMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCIvaG9tZS9kaWVyaS93aW5kbWlsbC9mcm9udGVuZFwiO2NvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9maWxlbmFtZSA9IFwiL2hvbWUvZGllcmkvd2luZG1pbGwvZnJvbnRlbmQvdml0ZS5jb25maWcuanNcIjtjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfaW1wb3J0X21ldGFfdXJsID0gXCJmaWxlOi8vL2hvbWUvZGllcmkvd2luZG1pbGwvZnJvbnRlbmQvdml0ZS5jb25maWcuanNcIjtpbXBvcnQgeyBzdmVsdGVraXQgfSBmcm9tICdAc3ZlbHRlanMva2l0L3ZpdGUnXG5pbXBvcnQgeyByZWFkRmlsZVN5bmMgfSBmcm9tICdmcydcbmltcG9ydCB7IGZpbGVVUkxUb1BhdGggfSBmcm9tICd1cmwnXG5pbXBvcnQgY2lyY2xlRGVwZW5kZW5jeSBmcm9tICd2aXRlLXBsdWdpbi1jaXJjdWxhci1kZXBlbmRlbmN5J1xuLy8gaW1wb3J0IG1rY2VydCBmcm9tICd2aXRlLXBsdWdpbi1ta2NlcnQnXG5pbXBvcnQgaW1wb3J0TWV0YVVybFBsdWdpbiBmcm9tICdAd2luZG1pbGwtbGFicy9lc2J1aWxkLWltcG9ydC1tZXRhLXVybC1wbHVnaW4nXG5cbmNvbnN0IGZpbGUgPSBmaWxlVVJMVG9QYXRoKG5ldyBVUkwoJ3BhY2thZ2UuanNvbicsIGltcG9ydC5tZXRhLnVybCkpXG5jb25zdCBqc29uID0gcmVhZEZpbGVTeW5jKGZpbGUsICd1dGY4JylcbmNvbnN0IHZlcnNpb24gPSBKU09OLnBhcnNlKGpzb24pXG5cbi8qKiBAdHlwZSB7aW1wb3J0KCd2aXRlJykuVXNlckNvbmZpZ30gKi9cbmNvbnN0IGNvbmZpZyA9IHtcblx0c2VydmVyOiB7XG5cdFx0aHR0cHM6IGZhbHNlLFxuXHRcdHBvcnQ6IDMwMDAsXG5cdFx0cHJveHk6IHtcblx0XHRcdCdeL2FwaS8uKic6IHtcblx0XHRcdFx0dGFyZ2V0OiBwcm9jZXNzLmVudi5SRU1PVEUgPz8gJ2h0dHBzOi8vYXBwLndpbmRtaWxsLmRldi8nLFxuXHRcdFx0XHRjaGFuZ2VPcmlnaW46IHRydWUsXG5cdFx0XHRcdGNvb2tpZURvbWFpblJld3JpdGU6ICdsb2NhbGhvc3QnXG5cdFx0XHR9LFxuXHRcdFx0J14vd3MvLionOiB7XG5cdFx0XHRcdHRhcmdldDogcHJvY2Vzcy5lbnYuUkVNT1RFX0xTUCA/PyAnaHR0cHM6Ly9hcHAud2luZG1pbGwuZGV2Jyxcblx0XHRcdFx0Y2hhbmdlT3JpZ2luOiB0cnVlLFxuXHRcdFx0XHR3czogdHJ1ZVxuXHRcdFx0fSxcblx0XHRcdCdeL3dzX21wLy4qJzoge1xuXHRcdFx0XHR0YXJnZXQ6IHByb2Nlc3MuZW52LlJFTU9URV9NUCA/PyAnaHR0cHM6Ly9hcHAud2luZG1pbGwuZGV2Jyxcblx0XHRcdFx0Y2hhbmdlT3JpZ2luOiB0cnVlLFxuXHRcdFx0XHR3czogdHJ1ZVxuXHRcdFx0fVxuXHRcdH1cblx0fSxcblx0cHJldmlldzoge1xuXHRcdHBvcnQ6IDMwMDBcblx0fSxcblx0cGx1Z2luczogW3N2ZWx0ZWtpdCgpLCBjaXJjbGVEZXBlbmRlbmN5KHsgY2lyY2xlSW1wb3J0VGhyb3dFcnI6IGZhbHNlIH0pXSxcblx0ZGVmaW5lOiB7XG5cdFx0X19wa2dfXzogdmVyc2lvblxuXHR9LFxuXHRvcHRpbWl6ZURlcHM6IHtcblx0XHRpbmNsdWRlOiBbJ2hpZ2hsaWdodC5qcycsICdoaWdobGlnaHQuanMvbGliL2NvcmUnLCAnbW9uYWNvLXZpbSddLFxuXHRcdGV4Y2x1ZGU6IFtcblx0XHRcdCdAY29kaW5nYW1lL21vbmFjby12c2NvZGUtc3RhbmRhbG9uZS10eXBlc2NyaXB0LWxhbmd1YWdlLWZlYXR1cmVzJyxcblx0XHRcdCdAY29kaW5nYW1lL21vbmFjby12c2NvZGUtc3RhbmRhbG9uZS1sYW5ndWFnZXMnXG5cdFx0XSxcblx0XHRlc2J1aWxkT3B0aW9uczoge1xuXHRcdFx0cGx1Z2luczogW2ltcG9ydE1ldGFVcmxQbHVnaW5dXG5cdFx0fVxuXHR9LFxuXHRyZXNvbHZlOiB7XG5cdFx0YWxpYXM6IHtcblx0XHRcdHBhdGg6ICdwYXRoLWJyb3dzZXJpZnknLFxuXHRcdFx0J21vbmFjby1lZGl0b3IvZXNtL3ZzL2VkaXRvci9jb250cmliL2hvdmVyL2Jyb3dzZXIvaG92ZXInOlxuXHRcdFx0XHQndnNjb2RlL3ZzY29kZS92cy9lZGl0b3IvY29udHJpYi9ob3Zlci9icm93c2VyL2hvdmVyQ29udHJpYnV0aW9uJ1xuXHRcdH0sXG5cdFx0ZGVkdXBlOiBbJ3ZzY29kZScsICdtb25hY28tZWRpdG9yJ11cblx0fSxcblx0d29ya2VyOiB7XG5cdFx0Zm9ybWF0OiAnZXMnXG5cdH0sXG5cdGFzc2V0c0luY2x1ZGU6IFsnKiovKi53YXNtJ11cbn1cblxuZXhwb3J0IGRlZmF1bHQgY29uZmlnXG4iXSwKICAibWFwcGluZ3MiOiAiO0FBQXlRLFNBQVMsaUJBQWlCO0FBQ25TLFNBQVMsb0JBQW9CO0FBQzdCLFNBQVMscUJBQXFCO0FBQzlCLE9BQU8sc0JBQXNCO0FBRTdCLE9BQU8seUJBQXlCO0FBTGtJLElBQU0sMkNBQTJDO0FBT25OLElBQU0sT0FBTyxjQUFjLElBQUksSUFBSSxnQkFBZ0Isd0NBQWUsQ0FBQztBQUNuRSxJQUFNLE9BQU8sYUFBYSxNQUFNLE1BQU07QUFDdEMsSUFBTSxVQUFVLEtBQUssTUFBTSxJQUFJO0FBRy9CLElBQU0sU0FBUztBQUFBLEVBQ2QsUUFBUTtBQUFBLElBQ1AsT0FBTztBQUFBLElBQ1AsTUFBTTtBQUFBLElBQ04sT0FBTztBQUFBLE1BQ04sWUFBWTtBQUFBLFFBQ1gsUUFBUSxRQUFRLElBQUksVUFBVTtBQUFBLFFBQzlCLGNBQWM7QUFBQSxRQUNkLHFCQUFxQjtBQUFBLE1BQ3RCO0FBQUEsTUFDQSxXQUFXO0FBQUEsUUFDVixRQUFRLFFBQVEsSUFBSSxjQUFjO0FBQUEsUUFDbEMsY0FBYztBQUFBLFFBQ2QsSUFBSTtBQUFBLE1BQ0w7QUFBQSxNQUNBLGNBQWM7QUFBQSxRQUNiLFFBQVEsUUFBUSxJQUFJLGFBQWE7QUFBQSxRQUNqQyxjQUFjO0FBQUEsUUFDZCxJQUFJO0FBQUEsTUFDTDtBQUFBLElBQ0Q7QUFBQSxFQUNEO0FBQUEsRUFDQSxTQUFTO0FBQUEsSUFDUixNQUFNO0FBQUEsRUFDUDtBQUFBLEVBQ0EsU0FBUyxDQUFDLFVBQVUsR0FBRyxpQkFBaUIsRUFBRSxzQkFBc0IsTUFBTSxDQUFDLENBQUM7QUFBQSxFQUN4RSxRQUFRO0FBQUEsSUFDUCxTQUFTO0FBQUEsRUFDVjtBQUFBLEVBQ0EsY0FBYztBQUFBLElBQ2IsU0FBUyxDQUFDLGdCQUFnQix5QkFBeUIsWUFBWTtBQUFBLElBQy9ELFNBQVM7QUFBQSxNQUNSO0FBQUEsTUFDQTtBQUFBLElBQ0Q7QUFBQSxJQUNBLGdCQUFnQjtBQUFBLE1BQ2YsU0FBUyxDQUFDLG1CQUFtQjtBQUFBLElBQzlCO0FBQUEsRUFDRDtBQUFBLEVBQ0EsU0FBUztBQUFBLElBQ1IsT0FBTztBQUFBLE1BQ04sTUFBTTtBQUFBLE1BQ04sMkRBQ0M7QUFBQSxJQUNGO0FBQUEsSUFDQSxRQUFRLENBQUMsVUFBVSxlQUFlO0FBQUEsRUFDbkM7QUFBQSxFQUNBLFFBQVE7QUFBQSxJQUNQLFFBQVE7QUFBQSxFQUNUO0FBQUEsRUFDQSxlQUFlLENBQUMsV0FBVztBQUM1QjtBQUVBLElBQU8sc0JBQVE7IiwKICAibmFtZXMiOiBbXQp9Cg==
