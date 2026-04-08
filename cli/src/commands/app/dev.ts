import { Command } from "@cliffy/command";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
import * as windmillUtils from "@windmill-labs/shared-utils";
import { yamlParseFile } from "../../utils/yaml.ts";
import * as getPort from "get-port";
import * as open from "open";
import { GlobalOptions } from "../../types.ts";
import * as http from "node:http";
import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";
import { Buffer } from "node:buffer";
import { writeFileSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { WebSocket, WebSocketServer } from "ws";
import {
  createFrameworkPlugins,
  detectFrameworks,
  ensureNodeModules,
  getDevBuildOptions,
} from "./bundle.ts";
import { wmillTsDev as wmillTs } from "./wmillTsDev.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { requireLogin } from "../../core/auth.ts";
import { GLOBAL_CONFIG_OPT } from "../../core/conf.ts";
import { replaceInlineScripts, repopulateFields } from "./app.ts";
import { Runnable } from "./metadata.ts";
import {
  APP_BACKEND_FOLDER,
  inferRunnableSchemaFromFile,
} from "./app_metadata.ts";
import { loadRunnablesFromBackend } from "./raw_apps.ts";
import { regenerateAgentDocs } from "./generate_agents.ts";
import {
  getFolderSuffix,
  hasFolderSuffix,
  loadNonDottedPathsSetting,
} from "../../utils/resource_folders.ts";

const DEFAULT_PORT = 4000;
const DEFAULT_HOST = "localhost";

// HTML template with live reload and SQL migration modal
const createHTML = (jsPath: string, cssPath: string) => `
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Windmill App Dev Preview</title>
  <link rel="stylesheet" href="${cssPath}">
  <style>
    * {
      margin: 0;
      padding: 0;
      box-sizing: border-box;
    }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen',
        'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue', sans-serif;
      -webkit-font-smoothing: antialiased;
      -moz-osx-font-smoothing: grayscale;
    }
    #root {
      width: 100%;
      height: 100vh;
    }
    /* SQL Migration Modal Styles */
    .sql-modal-overlay {
      display: none;
      position: fixed;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      background: rgba(0, 0, 0, 0.5);
      z-index: 10000;
      justify-content: center;
      align-items: center;
    }
    .sql-modal-overlay.visible {
      display: flex;
    }
    .sql-modal {
      background: white;
      border-radius: 8px;
      max-width: 700px;
      width: 90%;
      max-height: 80vh;
      display: flex;
      flex-direction: column;
      box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
    }
    .sql-modal-header {
      padding: 16px 20px;
      border-bottom: 1px solid #e0e0e0;
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
    .sql-modal-header h2 {
      margin: 0;
      font-size: 18px;
      color: #333;
    }
    .sql-modal-header .close-btn {
      background: none;
      border: none;
      font-size: 24px;
      cursor: pointer;
      color: #666;
    }
    .sql-modal-body {
      padding: 20px;
      overflow-y: auto;
      flex: 1;
    }
    .sql-modal-body p {
      margin-bottom: 12px;
      color: #555;
    }
    .sql-modal-body pre {
      background: #f5f5f5;
      padding: 12px;
      border-radius: 4px;
      overflow-x: auto;
      font-size: 13px;
      max-height: 300px;
      overflow-y: auto;
    }
    .sql-modal-footer {
      padding: 16px 20px;
      border-top: 1px solid #e0e0e0;
      display: flex;
      justify-content: flex-end;
      gap: 12px;
    }
    .sql-modal-footer button {
      padding: 10px 20px;
      border-radius: 4px;
      cursor: pointer;
      font-size: 14px;
    }
    .sql-modal-footer .cancel-btn {
      background: #f0f0f0;
      border: 1px solid #ccc;
      color: #333;
    }
    .sql-modal-footer .apply-btn {
      background: #3b82f6;
      border: none;
      color: white;
    }
    .sql-modal-footer .apply-btn:hover {
      background: #2563eb;
    }
    .sql-modal-footer .apply-btn:disabled {
      background: #9ca3af;
      cursor: not-allowed;
    }
    .sql-result {
      margin-top: 12px;
      padding: 12px;
      border-radius: 4px;
    }
    .sql-result.success {
      background: #dcfce7;
      color: #166534;
    }
    .sql-result.error {
      background: #fee2e2;
      color: #991b1b;
    }
  </style>
</head>
<body>
  <div id="root"></div>
  <!-- SQL Migration Modal -->
  <div id="sql-modal-overlay" class="sql-modal-overlay">
    <div class="sql-modal">
      <div class="sql-modal-header">
        <h2>Apply SQL Migration</h2>
        <button class="close-btn" onclick="closeSqlModal()">&times;</button>
      </div>
      <div class="sql-modal-body">
        <p><strong>File:</strong> <span id="sql-file-name"></span></p>
        <p><strong>Datatable:</strong> <span id="sql-datatable"></span></p>
        <p><strong>SQL to execute:</strong></p>
        <pre id="sql-content"></pre>
        <div id="sql-result"></div>
      </div>
      <div class="sql-modal-footer">
        <button class="cancel-btn" onclick="closeSqlModal()">Cancel</button>
        <button class="apply-btn" id="apply-sql-btn" onclick="applySql()">Apply SQL</button>
      </div>
    </div>
  </div>
  <script src="${jsPath}"></script>
  <script>
    // Live reload via EventSource
    const evtSource = new EventSource('/__events');
    evtSource.addEventListener('change', () => {
      console.log('🔄 Files changed, reloading...');
      location.reload();
    });
    evtSource.addEventListener('error', () => {
      console.log('⚠️ Lost connection to dev server, retrying...');
    });

    // SQL Migration Modal handling
    let pendingSqlMigration = null;
    let sqlWebSocket = null;
    let closeModalTimeout = null;

    function initSqlWebSocket() {
      const wsProtocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
      sqlWebSocket = new WebSocket(wsProtocol + '//' + location.host);

      sqlWebSocket.onmessage = (event) => {
        const data = JSON.parse(event.data);
        if (data.type === 'sqlMigration') {
          showSqlModal(data);
        } else if (data.type === 'sqlMigrationResult') {
          showSqlResult(data);
        }
      };

      sqlWebSocket.onclose = () => {
        // Reconnect after a delay
        setTimeout(initSqlWebSocket, 1000);
      };
    }

    function showSqlModal(data) {
      // Cancel any pending close from a previous migration's success
      if (closeModalTimeout) {
        clearTimeout(closeModalTimeout);
        closeModalTimeout = null;
      }
      pendingSqlMigration = data;
      document.getElementById('sql-file-name').textContent = data.fileName;
      document.getElementById('sql-datatable').textContent = data.datatable || 'Not configured';
      document.getElementById('sql-content').textContent = data.sql;
      document.getElementById('sql-result').innerHTML = '';
      document.getElementById('sql-result').className = 'sql-result';
      document.getElementById('apply-sql-btn').disabled = !data.datatable;
      document.getElementById('apply-sql-btn').textContent = 'Apply SQL';
      document.getElementById('sql-modal-overlay').classList.add('visible');
    }

    function closeSqlModal(skip = true) {
      if (skip && pendingSqlMigration && sqlWebSocket) {
        // Notify server that user skipped this file
        sqlWebSocket.send(JSON.stringify({
          type: 'skipSqlMigration',
          fileName: pendingSqlMigration.fileName
        }));
      }
      document.getElementById('sql-modal-overlay').classList.remove('visible');
      pendingSqlMigration = null;
    }

    function applySql() {
      if (!pendingSqlMigration || !sqlWebSocket) return;

      document.getElementById('apply-sql-btn').disabled = true;
      document.getElementById('apply-sql-btn').textContent = 'Applying...';

      sqlWebSocket.send(JSON.stringify({
        type: 'applySqlMigration',
        fileName: pendingSqlMigration.fileName,
        sql: pendingSqlMigration.sql,
        datatable: pendingSqlMigration.datatable
      }));
    }

    function showSqlResult(data) {
      const resultDiv = document.getElementById('sql-result');
      if (data.error) {
        resultDiv.className = 'sql-result error';
        resultDiv.innerHTML = '<strong>Error:</strong> ' + escapeHtml(data.message);
        document.getElementById('apply-sql-btn').disabled = false;
        document.getElementById('apply-sql-btn').textContent = 'Apply SQL';
      } else {
        resultDiv.className = 'sql-result success';
        resultDiv.innerHTML = '<strong>Success!</strong> SQL applied and file deleted.';
        // Auto-close after success (don't send skip since server already handled it)
        // Save timeout ID so it can be cancelled if a new migration arrives
        closeModalTimeout = setTimeout(() => {
          closeModalTimeout = null;
          closeSqlModal(false);
        }, 1500);
      }
    }

    function escapeHtml(text) {
      const div = document.createElement('div');
      div.textContent = text;
      return div.innerHTML;
    }

    // Initialize WebSocket for SQL migrations
    initSqlWebSocket();
  </script>
</body>
</html>
`;

interface DevOptions extends GlobalOptions {
  port?: number;
  host?: string;
  entry?: string;
  open?: boolean;
}

async function dev(opts: DevOptions, appFolder?: string) {
  GLOBAL_CONFIG_OPT.noCdToRoot = true;

  // Search for wmill.yaml by traversing upward (without git root constraint)
  // to initialize nonDottedPaths setting before using folder suffix functions
  await loadNonDottedPathsSetting();

  // Resolve target directory from argument or use current directory
  const originalCwd = process.cwd();
  let targetDir = originalCwd;

  if (appFolder) {
    targetDir = path.isAbsolute(appFolder)
      ? appFolder
      : path.join(originalCwd, appFolder);

    if (!fs.existsSync(targetDir)) {
      log.error(colors.red(`Error: Directory not found: ${targetDir}`));
      process.exit(1);
    }
  }

  // Validate that target is a .raw_app folder
  const targetDirName = path.basename(targetDir);

  if (!hasFolderSuffix(targetDirName, "raw_app")) {
    log.error(
      colors.red(
        `Error: The dev command must be run inside a ${
          getFolderSuffix("raw_app")
        } folder.\n` +
          `Target directory: ${targetDirName}\n` +
          `Please navigate to a folder ending with '${
            getFolderSuffix("raw_app")
          }' or specify one as argument.`,
      ),
    );
    process.exit(1);
  }

  // Check for raw_app.yaml in target directory
  const rawAppPath = path.join(targetDir, "raw_app.yaml");
  if (!fs.existsSync(rawAppPath)) {
    log.error(
      colors.red(
        `Error: raw_app.yaml not found in ${targetDir}.\n` +
          `The dev command requires a ${
            getFolderSuffix("raw_app")
          } folder containing a raw_app.yaml file.`,
      ),
    );
    process.exit(1);
  }

  // Resolve workspace and authenticate (from original cwd to find wmill.yaml)
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const workspaceId = workspace.workspaceId;

  // Change to target directory for the rest of the command
  if (appFolder) {
    process.chdir(targetDir);
  }

  // Load app path from raw_app.yaml
  const rawApp = (await yamlParseFile(rawAppPath)) as any;
  const appPath = rawApp?.custom_path ?? "u/unknown/newapp";

  // Dynamically import esbuild only when the dev command is called
  const esbuild = await import("esbuild");

  const port = opts.port ??
    (await getPort.default({
      port: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map((p) => p + DEFAULT_PORT),
    }));
  const host = opts.host ?? DEFAULT_HOST;
  const shouldOpen = opts.open ?? true;

  // Detect frameworks to determine default entry point
  const frameworks = detectFrameworks(process.cwd());
  const defaultEntry = frameworks.svelte || frameworks.vue
    ? "index.ts"
    : "index.tsx";
  const entryPoint = opts.entry ?? defaultEntry;

  // Verify entry point exists
  if (!fs.existsSync(entryPoint)) {
    log.error(
      colors.red(
        `Entry point "${entryPoint}" not found. Please specify a valid entry point with --entry.`,
      ),
    );
    process.exit(1);
  }

  // Ensure node_modules exists
  const appDir = path.dirname(entryPoint) || process.cwd();
  await ensureNodeModules(appDir);

  // In-memory cache of inferred schemas (runnableId -> schema)
  // Used to generate wmill.d.ts without modifying raw_app.yaml
  const inferredSchemas: Record<string, any> = {};

  genRunnablesTs(inferredSchemas);

  // Ensure dist directory exists
  const distDir = path.join(process.cwd(), "dist");
  if (!fs.existsSync(distDir)) {
    fs.mkdirSync(distDir);
  }

  // SSE clients for live reload
  const clients: http.ServerResponse[] = [];

  function notifyClients() {
    clients.forEach((client) => {
      client.write(`event: change\ndata: reload\n\n`);
    });
  }

  const buildOptions = getDevBuildOptions(entryPoint);

  // Load framework-specific plugins (svelte, vue) based on package.json
  const frameworkPlugins = await createFrameworkPlugins(appDir);

  const wmillPlugin = {
    name: "wmill-virtual",
    setup(build: any) {
      // Intercept imports of wmill with various path formats:
      // - wmill, wmill.ts (bare import)
      // - /wmill, /wmill.ts (absolute)
      // - ./wmill, ./wmill.ts (same directory)
      // - ../wmill, ../../wmill, etc. (parent directories)
      build.onResolve(
        { filter: /^(\.\.\/)+wmill(\.ts)?$|^(\.\/|\/)?wmill(\.ts)?$/ },
        (args: any) => {
          log.info(colors.yellow(`[wmill-virtual] Intercepted: ${args.path}`));
          return {
            path: args.path,
            namespace: "wmill-virtual",
          };
        },
      );

      // Provide the virtual module content
      build.onLoad(
        { filter: /.*/, namespace: "wmill-virtual" },
        (args: any) => {
          const contents = wmillTs(port);
          log.info(
            colors.yellow(
              `[wmill-virtual] Loading virtual module: ${args.path}`,
            ),
          );
          log.info(
            colors.gray(
              `[wmill-virtual] Exports: ${
                contents.match(/export (const|function) \w+/g)?.join(", ") ??
                  "none"
              }`,
            ),
          );
          return {
            contents,
            loader: "ts",
          };
        },
      );
    },
  };

  // Create esbuild context
  const ctx = await esbuild.context({
    ...buildOptions,
    plugins: [
      ...frameworkPlugins,
      {
        name: "notify-on-rebuild",
        setup(build: any) {
          build.onEnd((result: any) => {
            if (result.errors.length === 0) {
              log.info(
                colors.green("✅ Build succeeded, notifying clients..."),
              );
              notifyClients();
            } else {
              log.error(colors.red("❌ Build failed:"));
              result.errors.forEach((error: any) => {
                log.error(colors.red(error.text));
              });
            }
          });
        },
      },
      wmillPlugin,
    ],
  });

  // Start watching
  await ctx.watch();
  log.info(colors.blue("👀 Watching for file changes...\n"));

  // Initial build
  await ctx.rebuild();

  // Watch runnables folder for changes
  const runnablesPath = path.join(process.cwd(), APP_BACKEND_FOLDER);
  let runnablesWatcher: fs.FSWatcher | undefined;

  if (fs.existsSync(runnablesPath)) {
    log.info(
      colors.blue(`👁️  Watching runnables folder at: ${runnablesPath}\n`),
    );
    runnablesWatcher = fs.watch(runnablesPath, { recursive: true });

    // Per-file debounce timeouts for schema inference (longer debounce for typing)
    const schemaInferenceTimeouts: Record<string, ReturnType<typeof setTimeout>> = {};
    const SCHEMA_DEBOUNCE_MS = 500; // Wait 500ms after last change before inferring schema

    // Handle runnables file changes via callback
    runnablesWatcher.on("change", (_eventType, filename) => {
      if (!filename) return;
      const fileStr = typeof filename === "string" ? filename : filename.toString();
      const changedPath = path.join(runnablesPath, fileStr);
      const relativePath = path.relative(process.cwd(), changedPath);
      const relativeToRunnables = fileStr;

      // Skip lock files
      if (changedPath.endsWith(".lock")) {
        return;
      }

      // Log the change event
      log.info(
        colors.cyan(
          `📝 Runnable changed [${_eventType}]: ${relativePath}`,
        ),
      );

      // Debounce schema inference per file (wait for typing to finish)
      if (schemaInferenceTimeouts[changedPath]) {
        clearTimeout(schemaInferenceTimeouts[changedPath]);
      }

      schemaInferenceTimeouts[changedPath] = setTimeout(async () => {
        delete schemaInferenceTimeouts[changedPath];

        try {
          log.info(
            colors.cyan(
              `📝 Inferring schema for: ${relativeToRunnables}`,
            ),
          );
          // Infer schema for this runnable (returns schema in memory, doesn't write to file)
          const result = await inferRunnableSchemaFromFile(
            process.cwd(),
            relativeToRunnables,
          );
          if (result) {
            // Store inferred schema in memory
            inferredSchemas[result.runnableId] = result.schema;
            log.info(
              colors.green(
                `  Inferred Schemas: ${
                  JSON.stringify(
                    inferredSchemas,
                    null,
                    2,
                  )
                }`,
              ),
            );
            // Regenerate wmill.d.ts with updated schema from memory
            await genRunnablesTs(inferredSchemas);
          }
        } catch (error: any) {
          log.error(
            colors.red(`Error inferring schema: ${error.message}`),
          );
        }
      }, SCHEMA_DEBOUNCE_MS);
    });

    runnablesWatcher.on("error", (error: Error) => {
      log.error(colors.red(`Error watching runnables: ${error.message}`));
    });
  } else {
    log.info(
      colors.gray(
        "ℹ️  No runnables folder found (will not watch for runnable changes)\n",
      ),
    );
  }

  // Create HTTP server
  const server = http.createServer((req, res) => {
    const url = req.url || "/";

    // SSE endpoint for live reload
    if (url === "/__events") {
      res.writeHead(200, {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        Connection: "keep-alive",
      });
      res.write("data: connected\n\n");
      clients.push(res);

      req.on("close", () => {
        const index = clients.indexOf(res);
        if (index !== -1) clients.splice(index, 1);
      });
      return;
    }

    // Serve the bundled JS
    if (url === "/dist/bundle.js" || url === "/bundle.js") {
      const jsPath = path.join(process.cwd(), "dist/bundle.js");
      if (fs.existsSync(jsPath)) {
        res.writeHead(200, { "Content-Type": "application/javascript" });
        res.end(fs.readFileSync(jsPath));
      } else {
        res.writeHead(404);
        res.end("Bundle not found");
      }
      return;
    }

    // Serve the bundled CSS
    if (url === "/dist/bundle.css" || url === "/bundle.css") {
      const cssPath = path.join(process.cwd(), "dist/bundle.css");
      if (fs.existsSync(cssPath)) {
        res.writeHead(200, { "Content-Type": "text/css" });
        res.end(fs.readFileSync(cssPath));
      } else {
        res.writeHead(404);
        res.end("CSS not found");
      }
      return;
    }

    // Serve source maps
    if (url === "/dist/bundle.js.map" || url === "/bundle.js.map") {
      const mapPath = path.join(process.cwd(), "dist/bundle.js.map");
      if (fs.existsSync(mapPath)) {
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(fs.readFileSync(mapPath));
      } else {
        res.writeHead(404);
        res.end("Source map not found");
      }
      return;
    }

    if (url === "/dist/bundle.css.map" || url === "/bundle.css.map") {
      const mapPath = path.join(process.cwd(), "dist/bundle.css.map");
      if (fs.existsSync(mapPath)) {
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(fs.readFileSync(mapPath));
      } else {
        res.writeHead(404);
        res.end("Source map not found");
      }
      return;
    }

    // Serve injected HTML for root and any other path
    res.writeHead(200, { "Content-Type": "text/html" });
    res.end(createHTML("/dist/bundle.js", "/dist/bundle.css"));
  });

  // Create WebSocket server on the same HTTP server
  const wss = new WebSocketServer({ server });

  // SQL migration state (defined here so it's accessible in WebSocket handler)
  const sqlToApplyPath = path.join(process.cwd(), "sql_to_apply");
  const sqlFileQueue: string[] = [];
  let currentSqlFile: string | null = null;

  // Helper to read datatable from raw_app.yaml
  async function getDatatableConfig(): Promise<string | undefined> {
    try {
      const rawApp = (await yamlParseFile(
        path.join(process.cwd(), "raw_app.yaml"),
      )) as any;
      return rawApp?.data?.datatable;
    } catch {
      return undefined;
    }
  }

  // Helper to broadcast SQL migration notification to all WebSocket clients
  function broadcastSqlMigration(
    fileName: string,
    sql: string,
    datatable: string | undefined,
  ) {
    const message = JSON.stringify({
      type: "sqlMigration",
      fileName,
      sql,
      datatable,
    });

    wss.clients.forEach((client: WebSocket) => {
      if (client.readyState === WebSocket.OPEN) {
        client.send(message);
      }
    });
  }

  // Helper to add a SQL file to the queue (if not already queued)
  function queueSqlFile(filePath: string): void {
    if (currentSqlFile === filePath) {
      return; // Already being shown
    }
    if (sqlFileQueue.includes(filePath)) {
      return; // Already in queue
    }
    sqlFileQueue.push(filePath);
  }

  // Helper to process the next SQL file in the queue
  async function processNextSqlFile(): Promise<void> {
    if (currentSqlFile !== null) {
      // Already showing a file, wait for it to be processed
      return;
    }

    if (sqlFileQueue.length === 0) {
      return;
    }

    const filePath = sqlFileQueue.shift()!;

    // Check if file still exists (might have been deleted)
    if (!fs.existsSync(filePath)) {
      log.info(
        colors.gray(`File no longer exists: ${path.basename(filePath)}`),
      );
      // Try next file
      await processNextSqlFile();
      return;
    }

    currentSqlFile = filePath;
    const fileName = path.basename(filePath);

    try {
      const sqlContent = await readFile(filePath, "utf-8");

      if (!sqlContent.trim()) {
        log.info(colors.gray(`Skipping empty file: ${fileName}`));
        currentSqlFile = null;
        await processNextSqlFile();
        return;
      }

      const datatable = await getDatatableConfig();

      log.info(
        colors.magenta(
          `📋 SQL file ready: ${fileName} (datatable: ${
            datatable || "not configured"
          }) [${sqlFileQueue.length} more in queue]`,
        ),
      );

      broadcastSqlMigration(fileName, sqlContent, datatable);
    } catch (error: any) {
      log.error(colors.red(`Error reading SQL file: ${error.message}`));
      currentSqlFile = null;
      await processNextSqlFile();
    }
  }

  // Helper to handle SQL file completion (success or skip)
  async function onSqlFileCompleted(
    filePath: string,
    deleteFile: boolean,
  ): Promise<void> {
    if (deleteFile && fs.existsSync(filePath)) {
      try {
        fs.unlinkSync(filePath);
        log.info(colors.green(`✓ Deleted: ${path.basename(filePath)}`));
      } catch (error: any) {
        log.error(
          colors.red(
            `Failed to delete ${path.basename(filePath)}: ${error.message}`,
          ),
        );
      }
    }

    currentSqlFile = null;
    // Process next file in queue
    await processNextSqlFile();
  }

  wss.on("connection", async (ws: WebSocket) => {
    log.info(colors.cyan("[WebSocket] Client connected"));

    // If there's a current SQL file being shown, send it to the new client
    if (currentSqlFile && fs.existsSync(currentSqlFile)) {
      try {
        const sqlContent = await readFile(currentSqlFile, "utf-8");
        const datatable = await getDatatableConfig();
        const fileName = path.basename(currentSqlFile);

        ws.send(JSON.stringify({
          type: "sqlMigration",
          fileName,
          sql: sqlContent,
          datatable,
        }));
        log.info(
          colors.magenta(`📋 Sent pending SQL file to new client: ${fileName}`),
        );
      } catch {
        // File might have been deleted, ignore
      }
    } else if (fs.existsSync(sqlToApplyPath)) {
      // No current file, but check if there are files to process
      const entries = fs.readdirSync(sqlToApplyPath);
      const sqlFiles = entries.filter((entry: string) =>
        entry.endsWith(".sql")
      );

      if (sqlFiles.length > 0 && sqlFileQueue.length === 0) {
        // Queue and process files
        for (const sqlFile of sqlFiles.sort()) {
          queueSqlFile(path.join(sqlToApplyPath, sqlFile));
        }
        await processNextSqlFile();
      }
    }

    ws.on("message", async (data: Buffer) => {
      try {
        const message = JSON.parse(data.toString());
        log.info(
          colors.cyan(`[WebSocket] Received: ${JSON.stringify(message)}`),
        );

        const { type, reqId, runnable_id, v, jobId } = message;

        // Helper to send response
        const respond = (responseType: string, result: any, error: boolean) => {
          ws.send(JSON.stringify({ type: responseType, reqId, result, error }));
        };

        // Helper to execute and wait for result
        const runAndWaitForResult = async (
          runnableId: string,
          args: any,
        ): Promise<{ uuid: string; result: any }> => {
          const runnables = await loadRunnables();
          const runnable = runnables[runnableId];

          if (!runnable) {
            throw new Error(`Runnable not found: ${runnableId}`);
          }

          const uuid = await executeRunnable(
            runnable,
            workspaceId,
            appPath,
            runnableId,
            args,
          );
          log.info(colors.gray(`[backend] Job started: ${uuid}`));

          const result = await waitForJob(workspaceId, uuid);
          return { uuid, result };
        };

        switch (type) {
          case "backend": {
            // Run a runnable synchronously and wait for result
            log.info(colors.blue(`[backend] Running runnable: ${runnable_id}`));
            try {
              const { result } = await runAndWaitForResult(runnable_id, v);
              respond("backendRes", result, false);
            } catch (error: any) {
              log.error(colors.red(`[backend] Error: ${error.message}`));
              respond(
                "backendRes",
                { message: error.message, stack: error.stack },
                true,
              );
            }
            break;
          }

          case "backendAsync": {
            // Run a runnable asynchronously and return job ID immediately
            log.info(
              colors.blue(
                `[backendAsync] Running runnable async: ${runnable_id}`,
              ),
            );
            try {
              const runnables = await loadRunnables();
              const runnable = runnables[runnable_id];

              if (!runnable) {
                throw new Error(`Runnable not found: ${runnable_id}`);
              }

              const uuid = await executeRunnable(
                runnable,
                workspaceId,
                appPath,
                runnable_id,
                v,
              );
              log.info(colors.gray(`[backendAsync] Job started: ${uuid}`));

              // Return job ID immediately
              respond("backendAsyncRes", uuid, false);

              // Wait for result in the background and send it when done
              waitForJob(workspaceId, uuid)
                .then((result) => {
                  respond("backendRes", result, false);
                })
                .catch((error: any) => {
                  respond(
                    "backendRes",
                    { message: error.message, stack: error.stack },
                    true,
                  );
                });
            } catch (error: any) {
              log.error(colors.red(`[backendAsync] Error: ${error.message}`));
              respond(
                "backendAsyncRes",
                { message: error.message, stack: error.stack },
                true,
              );
            }
            break;
          }

          case "waitJob": {
            // Wait for a job to complete and return its result
            log.info(colors.blue(`[waitJob] Waiting for job: ${jobId}`));
            try {
              const result = await waitForJob(workspaceId, jobId);
              respond("backendRes", result, false);
            } catch (error: any) {
              log.error(colors.red(`[waitJob] Error: ${error.message}`));
              respond(
                "backendRes",
                { message: error.message, stack: error.stack },
                true,
              );
            }
            break;
          }

          case "getJob": {
            // Get the current status/result of a job
            log.info(colors.blue(`[getJob] Getting job status: ${jobId}`));
            try {
              const result = await getJobStatus(workspaceId, jobId);
              respond("backendRes", result, false);
            } catch (error: any) {
              log.error(colors.red(`[getJob] Error: ${error.message}`));
              respond(
                "backendRes",
                { message: error.message, stack: error.stack },
                true,
              );
            }
            break;
          }

          case "streamJob": {
            // Stream job results using SSE
            log.info(colors.blue(`[streamJob] Streaming job: ${jobId}`));
            try {
              await streamJobWithSSE(
                workspaceId,
                jobId,
                reqId,
                ws,
                workspace.remote,
                workspace.token,
              );
            } catch (error: any) {
              log.error(colors.red(`[streamJob] Error: ${error.message}`));
              ws.send(
                JSON.stringify({
                  type: "streamJobRes",
                  reqId,
                  error: true,
                  result: { message: error.message, stack: error.stack },
                }),
              );
            }
            break;
          }

          case "applySqlMigration": {
            // Execute SQL migration against a datatable
            const { sql, datatable, fileName } = message;
            log.info(
              colors.blue(
                `[SQL Migration] Applying SQL from ${fileName} to datatable: ${datatable}`,
              ),
            );

            if (!datatable) {
              ws.send(
                JSON.stringify({
                  type: "sqlMigrationResult",
                  error: true,
                  message:
                    "No datatable configured. Set data.datatable in raw_app.yaml.",
                }),
              );
              break;
            }

            try {
              // Run SQL as a preview script against the datatable
              const uuid = await wmill.runScriptPreview({
                workspace: workspaceId,
                requestBody: {
                  language: "postgresql",
                  content: sql,
                  args: { database: `datatable://${datatable}` },
                },
              });

              log.info(
                colors.gray(`[SQL Migration] Job started: ${uuid}`),
              );

              // Wait for the result
              const result = await waitForJob(workspaceId, uuid);

              log.info(
                colors.green(`[SQL Migration] SQL applied successfully`),
              );
              ws.send(
                JSON.stringify({
                  type: "sqlMigrationResult",
                  error: false,
                  result,
                }),
              );

              // Delete the SQL file and process next in queue
              if (currentSqlFile) {
                await onSqlFileCompleted(currentSqlFile, true);
              }

              // Regenerate AGENTS.md and DATATABLES.md to reflect schema changes
              try {
                await regenerateAgentDocs(workspaceId, process.cwd(), true);
                log.info(
                  colors.gray(
                    `[SQL Migration] Refreshed AGENTS.md and DATATABLES.md`,
                  ),
                );
              } catch (regenError: any) {
                log.warn(
                  colors.yellow(
                    `[SQL Migration] Could not refresh docs: ${regenError.message}`,
                  ),
                );
              }
            } catch (error: any) {
              log.error(
                colors.red(`[SQL Migration] Error: ${error.message}`),
              );
              ws.send(
                JSON.stringify({
                  type: "sqlMigrationResult",
                  error: true,
                  message: error.message || String(error),
                }),
              );
              // Don't delete file on error, but clear current so user can retry
              currentSqlFile = null;
            }
            break;
          }

          case "skipSqlMigration": {
            // User chose to skip this SQL file
            const { fileName } = message;
            log.info(
              colors.yellow(`[SQL Migration] Skipped: ${fileName}`),
            );

            // Don't delete file, just move to next
            if (currentSqlFile) {
              await onSqlFileCompleted(currentSqlFile, false);
            }
            break;
          }

          default:
            log.warn(
              colors.yellow(`[WebSocket] Unknown message type: ${type}`),
            );
            respond(
              "error",
              { message: `Unknown message type: ${type}` },
              true,
            );
        }
      } catch (error: any) {
        log.error(
          colors.red(`[WebSocket] Failed to parse message: ${error.message}`),
        );
      }
    });

    ws.on("close", () => {
      log.info(colors.cyan("[WebSocket] Client disconnected"));
    });

    ws.on("error", (error: Error) => {
      log.error(colors.red(`[WebSocket] Error: ${error.message}`));
    });
  });

  // Watch sql_to_apply folder for SQL migration files
  let sqlWatcher: fs.FSWatcher | undefined;

  // Helper to scan for existing SQL files and add them to the queue
  async function scanExistingSqlFiles(): Promise<void> {
    if (!fs.existsSync(sqlToApplyPath)) {
      return;
    }

    try {
      const entries = fs.readdirSync(sqlToApplyPath);
      const sqlFiles = entries
        .filter((entry: string) => entry.endsWith(".sql"))
        .sort(); // Sort alphabetically to process in order

      if (sqlFiles.length === 0) {
        return;
      }

      log.info(
        colors.blue(
          `🔍 Found ${sqlFiles.length} SQL file(s) in sql_to_apply/`,
        ),
      );

      // Add each SQL file to the queue
      for (const sqlFile of sqlFiles) {
        const filePath = path.join(sqlToApplyPath, sqlFile);
        queueSqlFile(filePath);
      }

      // Start processing the queue
      await processNextSqlFile();
    } catch (error: any) {
      log.error(
        colors.red(`Error scanning sql_to_apply folder: ${error.message}`),
      );
    }
  }

  if (fs.existsSync(sqlToApplyPath)) {
    log.info(
      colors.blue(`🗃️  Watching sql_to_apply folder at: ${sqlToApplyPath}\n`),
    );
    sqlWatcher = fs.watch(sqlToApplyPath, { recursive: true });

    // Debounce timeout for SQL file changes
    const sqlDebounceTimeouts: Record<string, ReturnType<typeof setTimeout>> = {};
    const SQL_DEBOUNCE_MS = 300;

    // Handle SQL file changes via callback
    sqlWatcher.on("change", (_eventType, filename) => {
      if (!filename) return;
      const fileStr = typeof filename === "string" ? filename : filename.toString();
      const changedPath = path.join(sqlToApplyPath, fileStr);

      // Only handle .sql files
      if (!changedPath.endsWith(".sql")) {
        return;
      }

      const fileName = path.basename(changedPath);

      // Debounce per file
      if (sqlDebounceTimeouts[changedPath]) {
        clearTimeout(sqlDebounceTimeouts[changedPath]);
      }

      sqlDebounceTimeouts[changedPath] = setTimeout(async () => {
        delete sqlDebounceTimeouts[changedPath];

        log.info(colors.cyan(`📋 SQL file detected: ${fileName}`));

        // Add to queue and process
        queueSqlFile(changedPath);
        await processNextSqlFile();
      }, SQL_DEBOUNCE_MS);
    });

    sqlWatcher.on("error", (error: Error) => {
      log.error(
        colors.red(`Error watching sql_to_apply: ${error.message}`),
      );
    });

    // Scan for existing SQL files after a delay (to let WebSocket clients connect)
    setTimeout(() => {
      scanExistingSqlFiles();
    }, 2000);
  } else {
    log.info(
      colors.gray(
        "ℹ️  No sql_to_apply folder found (will not watch for SQL migrations)\n",
      ),
    );
  }

  server.listen(port, host, () => {
    const url = `http://${host}:${port}`;
    log.info(colors.bold.green(`🚀 Dev server running at ${url}`));
    log.info(
      colors.cyan(`🔌 WebSocket server running at ws://${host}:${port}`),
    );
    log.info(colors.gray(`📦 Serving files from: ${process.cwd()}`));
    log.info(colors.gray(`🔄 Live reload enabled\n`));

    // Open browser if requested
    if (shouldOpen) {
      try {
        open
          .openApp(open.apps.browser, { arguments: [url] })
          .catch((error: any) => {
            log.error(
              colors.yellow(
                `Failed to open browser automatically: ${error.message}`,
              ),
            );
          });
        log.info(colors.gray("Opened browser for you"));
      } catch (error: any) {
        log.error(colors.yellow(`Failed to open browser: ${error.message}`));
      }
    }
  });

  // Graceful shutdown
  process.on("SIGINT", async () => {
    log.info(colors.yellow("\n\n🛑 Shutting down..."));
    clients.forEach((client) => client.end());
    server.close();

    // Close runnables watcher if it exists
    if (runnablesWatcher) {
      runnablesWatcher.close();
    }

    // Close SQL watcher if it exists
    if (sqlWatcher) {
      sqlWatcher.close();
    }

    await ctx.dispose();
    process.exit(0);
  });
}

const command = new Command()
  .description(
    "Start a development server for building apps with live reload and hot module replacement",
  )
  .arguments("[app_folder:string]")
  .option(
    "--port <port:number>",
    "Port to run the dev server on (will find next available port if occupied)",
  )
  .option("--host <host:string>", "Host to bind the dev server to", {
    default: DEFAULT_HOST,
  })
  .option(
    "--entry <entry:string>",
    "Entry point file (default: index.ts for Svelte/Vue, index.tsx otherwise)",
  )
  .option("--no-open", "Don't automatically open the browser")
  .action(dev as any);

export default command;

/**
 * Generates wmill.d.ts with type definitions for runnables.
 * Loads runnables from separate YAML files in the backend folder (new format)
 * or falls back to raw_app.yaml (old format).
 * Merges in-memory inferred schemas with runnables.
 *
 * @param schemaOverrides - In-memory schema overrides (runnableId -> schema)
 */
async function genRunnablesTs(schemaOverrides: Record<string, any> = {}) {
  log.info(colors.blue("🔄 Generating wmill.d.ts..."));

  const localPath = process.cwd();
  const backendPath = path.join(localPath, APP_BACKEND_FOLDER);

  // Load runnables from separate files (new format) or fall back to raw_app.yaml (old format)
  let runnables = await loadRunnablesFromBackend(backendPath);

  if (Object.keys(runnables).length === 0) {
    // Fall back to old format
    try {
      const rawApp = (await yamlParseFile(
        path.join(localPath, "raw_app.yaml"),
      )) as any;
      runnables = rawApp?.["runnables"] ?? {};
    } catch {
      runnables = {};
    }
  }

  // Apply schema overrides from in-memory cache
  if (Object.keys(schemaOverrides).length > 0) {
    for (const [runnableId, schema] of Object.entries(schemaOverrides)) {
      if (runnables[runnableId]?.inlineScript) {
        runnables[runnableId].inlineScript.schema = schema;
        runnables[runnableId].type = "inline";
      }
    }
  }

  try {
    const newWmillTs = windmillUtils.genWmillTs(runnables);
    writeFileSync(path.join(process.cwd(), "wmill.d.ts"), newWmillTs);
  } catch (error: any) {
    log.error(colors.red(`Failed to generate wmill.d.ts: ${error.message}`));
  }
}

/**
 * Convert runnables from file format to API format.
 * File format uses type: "script"|"hubscript"|"flow" for path-based runnables.
 * API format uses type: "path" with runType: "script"|"hubscript"|"flow".
 */
function convertRunnablesToApiFormat(runnables: Record<string, any>): void {
  for (const [runnableId, runnable] of Object.entries(runnables)) {
    if (
      runnable?.type === "script" ||
      runnable?.type === "hubscript" ||
      runnable?.type === "flow"
    ) {
      // Convert from file format to API format
      // { type: "script" } -> { type: "path", runType: "script" }
      const originalType = runnable.type;
      runnable.runType = originalType;
      runnable.type = "path";
      log.debug(
        `Converted runnable '${runnableId}' from type='${originalType}' to type='path', runType='${originalType}'`,
      );
    }
  }
}

async function loadRunnables(): Promise<Record<string, Runnable>> {
  try {
    const localPath = process.cwd();
    const backendPath = path.join(localPath, APP_BACKEND_FOLDER);

    // Load runnables from separate files (new format) or fall back to raw_app.yaml (old format)
    let runnables = await loadRunnablesFromBackend(backendPath);

    if (Object.keys(runnables).length === 0) {
      // Fall back to old format
      const rawApp = (await yamlParseFile(
        path.join(localPath, "raw_app.yaml"),
      )) as any;
      runnables = rawApp?.runnables ?? {};
    }

    // Always convert path-based runnables from file format to API format
    // This handles both backend folder runnables and raw_app.yaml runnables
    convertRunnablesToApiFormat(runnables);

    replaceInlineScripts(runnables, backendPath + SEP, true);
    repopulateFields(runnables);

    return runnables;
  } catch (error: any) {
    log.error(colors.red(`Failed to load runnables: ${error.message}`));
    return {};
  }
}

async function executeRunnable(
  runnable: Runnable,
  workspace: string,
  appPath: string,
  runnableId: string,
  args: any,
): Promise<string> {
  const requestBody: any = {
    component: runnableId,
    args: args ?? {},
    force_viewer_static_fields: {},
    force_viewer_one_of_fields: {},
    force_viewer_allow_user_resources: [],
  };

  // Handle fields (static, ctx, user)
  if (runnable.fields) {
    for (const [key, field] of Object.entries(runnable.fields)) {
      if (field?.type === "static") {
        requestBody.force_viewer_static_fields[key] = field.value;
      } else if (field?.type === "ctx" && field?.ctx) {
        // Convert ctx fields to $ctx:property format for backend resolution
        requestBody.args[key] = `$ctx:${field.ctx}`;
      }
      if (field?.type === "user" && field?.allowUserResources) {
        requestBody.force_viewer_allow_user_resources.push(key);
      }
    }
  }

  if (
    (runnable.type === "inline" || runnable.type === "runnableByName") &&
    runnable.inlineScript
  ) {
    const inlineScript = runnable.inlineScript;
    if (inlineScript.id !== undefined) {
      requestBody.id = inlineScript.id;
    }
    requestBody.raw_code = {
      content: inlineScript.id === undefined ? inlineScript.content : "",
      language: inlineScript.language ?? "",
      path: `${appPath}/${runnableId}`,
      lock: inlineScript.id === undefined ? inlineScript.lock : undefined,
      cache_ttl: inlineScript.cache_ttl,
    };
  } else if (
    (runnable.type === "path" || runnable.type === "runnableByPath") &&
    runnable.runType &&
    runnable.path
  ) {
    // Path-based runnables have type: "path" (or legacy "runnableByPath") and runType: "script"|"hubscript"|"flow"
    const prefix = runnable.runType;
    requestBody.path = prefix !== "hubscript"
      ? `${prefix}/${runnable.path}`
      : `script/${runnable.path}`;
  } else {
    // Neither inline script nor valid path-based runnable
    const debugInfo =
      `type=${(runnable as any).type}, runType=${(runnable as any).runType}, ` +
      `path=${(runnable as any).path}, hasInlineScript=${!!(runnable as any)
        .inlineScript}`;
    log.error(
      colors.red(
        `[executeRunnable] Invalid runnable configuration for '${runnableId}': ${debugInfo}`,
      ),
    );
    throw new Error(
      `Invalid runnable '${runnableId}': ${debugInfo}. ` +
        `Must have either inlineScript (for inline type) or type="path" with runType and path fields`,
    );
  }

  const uuid = await wmill.executeComponent({
    workspace,
    path: appPath,
    requestBody,
  });

  return uuid;
}

const ITERATIONS_BEFORE_SLOW_REFRESH = 10;
const ITERATIONS_BEFORE_SUPER_SLOW_REFRESH = 100;

async function waitForJob(workspace: string, jobId: string): Promise<any> {
  if (!jobId) {
    throw new Error("Job ID is required");
  }

  let syncIteration = 0;

  return new Promise((resolve, reject) => {
    async function checkJob() {
      try {
        const maybeJob = await wmill.getCompletedJobResultMaybe({
          workspace,
          id: jobId,
          getStarted: false,
        });

        if (maybeJob.completed) {
          if (
            !maybeJob.success &&
            typeof maybeJob.result === "object" &&
            maybeJob.result !== null &&
            "error" in maybeJob.result
          ) {
            reject((maybeJob.result as any).error);
          } else {
            resolve(maybeJob.result);
          }
          return;
        }
      } catch (err: any) {
        log.error(colors.red(`Error checking job ${jobId}: ${err.message}`));
      }

      syncIteration++;

      let nextIteration = 50;
      if (syncIteration > ITERATIONS_BEFORE_SUPER_SLOW_REFRESH) {
        nextIteration = 2000;
      } else if (syncIteration > ITERATIONS_BEFORE_SLOW_REFRESH) {
        nextIteration = 500;
      }

      setTimeout(checkJob, nextIteration);
    }

    checkJob();
  });
}

async function getJobStatus(workspace: string, jobId: string): Promise<any> {
  return await wmill.getJob({
    workspace,
    id: jobId,
  });
}

async function streamJobWithSSE(
  workspace: string,
  jobId: string,
  reqId: string,
  ws: WebSocket,
  baseUrl: string,
  token: string,
): Promise<void> {
  const sseUrl =
    `${baseUrl}api/w/${workspace}/jobs_u/getupdate_sse/${jobId}?fast=true`;

  const response = await fetch(sseUrl, {
    headers: {
      Accept: "text/event-stream",
      Authorization: `Bearer ${token}`,
    },
  });

  if (!response.ok) {
    throw new Error(
      `SSE request failed: ${response.status} ${response.statusText}`,
    );
  }

  const reader = response.body?.getReader();
  if (!reader) {
    throw new Error("No response body for SSE stream");
  }

  const decoder = new TextDecoder();
  let buffer = "";

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";

      for (const line of lines) {
        if (line.startsWith("data: ")) {
          const data = line.slice(6);
          try {
            const update = JSON.parse(data);
            const type = update.type;

            if (type === "ping" || type === "timeout") {
              if (type === "timeout") {
                reader.cancel();
                return;
              }
              continue;
            }

            if (type === "error") {
              ws.send(
                JSON.stringify({
                  type: "streamJobRes",
                  reqId,
                  error: true,
                  result: { message: update.error || "SSE error" },
                }),
              );
              reader.cancel();
              return;
            }

            if (type === "not_found") {
              ws.send(
                JSON.stringify({
                  type: "streamJobRes",
                  reqId,
                  error: true,
                  result: { message: "Job not found" },
                }),
              );
              reader.cancel();
              return;
            }

            // Send stream update if there's new stream data
            if (update.new_result_stream !== undefined) {
              ws.send(
                JSON.stringify({
                  type: "streamJobUpdate",
                  reqId,
                  new_result_stream: update.new_result_stream,
                  stream_offset: update.stream_offset,
                }),
              );
            }

            // Check if job is completed
            if (update.completed) {
              ws.send(
                JSON.stringify({
                  type: "streamJobRes",
                  reqId,
                  error: false,
                  result: update.only_result,
                }),
              );
              reader.cancel();
              return;
            }
          } catch (parseErr) {
            log.warn(`Failed to parse SSE data: ${parseErr}`);
          }
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
}
