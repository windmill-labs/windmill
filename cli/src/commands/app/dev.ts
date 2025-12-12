// deno-lint-ignore-file no-explicit-any
import {
  Command,
  colors,
  log,
  getPort,
  open,
  windmillUtils,
  yamlParseFile,
  SEP,
} from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import * as http from "node:http";
import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";
import { Buffer } from "node:buffer";
import { writeFileSync } from "node:fs";
import { WebSocketServer, WebSocket } from "npm:ws";
import {
  getDevBuildOptions,
  ensureNodeModules,
  createFrameworkPlugins,
  detectFrameworks,
} from "./bundle.ts";
import { wmillTsDev as wmillTs } from "./wmillTsDev.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { requireLogin } from "../../core/auth.ts";
import { GLOBAL_CONFIG_OPT } from "../../core/conf.ts";
import { replaceInlineScripts } from "./app.ts";
import { Runnable } from "./metadata.ts";
import {
  APP_BACKEND_FOLDER,
  inferRunnableSchemaFromFile,
} from "./app_metadata.ts";
import { loadRunnablesFromBackend } from "./raw_apps.ts";

const DEFAULT_PORT = 4000;
const DEFAULT_HOST = "localhost";

// HTML template with live reload
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
  </style>
</head>
<body>
  <div id="root"></div>
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

async function dev(opts: DevOptions) {
  GLOBAL_CONFIG_OPT.noCdToRoot = true;

  // Validate that we're in a .raw_app folder
  const cwd = process.cwd();
  const currentDirName = path.basename(cwd);

  if (!currentDirName.endsWith(".raw_app")) {
    log.error(
      colors.red(
        `Error: The dev command must be run inside a .raw_app folder.\n` +
          `Current directory: ${currentDirName}\n` +
          `Please navigate to a folder ending with '.raw_app' before running this command.`
      )
    );
    Deno.exit(1);
  }

  // Check for raw_app.yaml
  const rawAppPath = path.join(cwd, "raw_app.yaml");
  if (!fs.existsSync(rawAppPath)) {
    log.error(
      colors.red(
        `Error: raw_app.yaml not found in current directory.\n` +
          `The dev command must be run in a .raw_app folder containing a raw_app.yaml file.`
      )
    );
    Deno.exit(1);
  }

  // Resolve workspace and authenticate
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const workspaceId = workspace.workspaceId;

  // Load app path from raw_app.yaml
  const rawApp = (await yamlParseFile(rawAppPath)) as any;
  const appPath = rawApp?.custom_path ?? "u/unknown/newapp";

  // Dynamically import esbuild only when the dev command is called
  const esbuild = await import("npm:esbuild@0.24.2");

  const port =
    opts.port ??
    (await getPort.default({
      port: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map((p) => p + DEFAULT_PORT),
    }));
  const host = opts.host ?? DEFAULT_HOST;
  const shouldOpen = opts.open ?? true;

  // Detect frameworks to determine default entry point
  const frameworks = detectFrameworks(process.cwd());
  const defaultEntry =
    frameworks.svelte || frameworks.vue ? "index.ts" : "index.tsx";
  const entryPoint = opts.entry ?? defaultEntry;

  // Verify entry point exists
  if (!fs.existsSync(entryPoint)) {
    log.error(
      colors.red(
        `Entry point "${entryPoint}" not found. Please specify a valid entry point with --entry.`
      )
    );
    Deno.exit(1);
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
        }
      );

      // Provide the virtual module content
      build.onLoad(
        { filter: /.*/, namespace: "wmill-virtual" },
        (args: any) => {
          log.info(
            colors.yellow(
              `[wmill-virtual] Loading virtual module: ${args.path}`
            )
          );
          return {
            contents: wmillTs(port),
            loader: "ts",
          };
        }
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
                colors.green("✅ Build succeeded, notifying clients...")
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
  let runnablesWatcher: Deno.FsWatcher | undefined;

  if (fs.existsSync(runnablesPath)) {
    log.info(
      colors.blue(`👁️  Watching runnables folder at: ${runnablesPath}\n`)
    );
    runnablesWatcher = Deno.watchFs(runnablesPath);

    // Per-file debounce timeouts for schema inference (longer debounce for typing)
    const schemaInferenceTimeouts: Record<string, NodeJS.Timeout> = {};
    const SCHEMA_DEBOUNCE_MS = 500; // Wait 500ms after last change before inferring schema

    // Handle runnables file changes in the background
    (async () => {
      try {
        for await (const event of runnablesWatcher!) {
          // Process each changed path with individual debouncing
          for (const changedPath of event.paths) {
            const relativePath = path.relative(process.cwd(), changedPath);
            const relativeToRunnables = path.relative(
              runnablesPath,
              changedPath
            );

            // Skip non-modify events for schema inference
            if (event.kind !== "modify" && event.kind !== "create") {
              continue;
            }

            // Skip lock files
            if (changedPath.endsWith(".lock")) {
              continue;
            }

            // Log the change event
            log.info(
              colors.cyan(
                `📝 Runnable changed [${event.kind}]: ${relativePath}`
              )
            );

            // Debounce schema inference per file (wait for typing to finish)
            if (schemaInferenceTimeouts[changedPath]) {
              clearTimeout(schemaInferenceTimeouts[changedPath]);
            }

            schemaInferenceTimeouts[changedPath] = setTimeout(async () => {
              delete schemaInferenceTimeouts[changedPath];

              try {
                log.info(
                  colors.cyan(`📝 Inferring schema for: ${relativeToRunnables}`)
                );
                // Infer schema for this runnable (returns schema in memory, doesn't write to file)
                const result = await inferRunnableSchemaFromFile(
                  process.cwd(),
                  relativeToRunnables
                );
                if (result) {
                  // log.info(colors.green(`  Schema: ${JSON.stringify(result.schema, null, 2)}`));
                  // log.info(colors.green(`  Runnable ID: ${result.runnableId}`));
                  // Store inferred schema in memory
                  inferredSchemas[result.runnableId] = result.schema;
                  log.info(
                    colors.green(
                      `  Inferred Schemas: ${JSON.stringify(
                        inferredSchemas,
                        null,
                        2
                      )}`
                    )
                  );
                  // Regenerate wmill.d.ts with updated schema from memory
                  await genRunnablesTs(inferredSchemas);
                }
              } catch (error: any) {
                log.error(
                  colors.red(`Error inferring schema: ${error.message}`)
                );
              }
            }, SCHEMA_DEBOUNCE_MS);
          }
        }
      } catch (error: any) {
        if (error.name !== "Interrupted") {
          log.error(colors.red(`Error watching runnables: ${error.message}`));
        }
      }
    })();
  } else {
    log.info(
      colors.gray(
        "ℹ️  No runnables folder found (will not watch for runnable changes)\n"
      )
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

  wss.on("connection", (ws: WebSocket) => {
    log.info(colors.cyan("[WebSocket] Client connected"));

    ws.on("message", async (data: Buffer) => {
      try {
        const message = JSON.parse(data.toString());
        log.info(
          colors.cyan(`[WebSocket] Received: ${JSON.stringify(message)}`)
        );

        const { type, reqId, runnable_id, v, jobId } = message;

        // Helper to send response
        const respond = (responseType: string, result: any, error: boolean) => {
          ws.send(JSON.stringify({ type: responseType, reqId, result, error }));
        };

        // Helper to execute and wait for result
        const runAndWaitForResult = async (
          runnableId: string,
          args: any
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
            args
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
                true
              );
            }
            break;
          }

          case "backendAsync": {
            // Run a runnable asynchronously and return job ID immediately
            log.info(
              colors.blue(
                `[backendAsync] Running runnable async: ${runnable_id}`
              )
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
                v
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
                    true
                  );
                });
            } catch (error: any) {
              log.error(colors.red(`[backendAsync] Error: ${error.message}`));
              respond(
                "backendAsyncRes",
                { message: error.message, stack: error.stack },
                true
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
                true
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
                true
              );
            }
            break;
          }

          default:
            log.warn(
              colors.yellow(`[WebSocket] Unknown message type: ${type}`)
            );
            respond(
              "error",
              { message: `Unknown message type: ${type}` },
              true
            );
        }
      } catch (error: any) {
        log.error(
          colors.red(`[WebSocket] Failed to parse message: ${error.message}`)
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

  server.listen(port, host, () => {
    const url = `http://${host}:${port}`;
    log.info(colors.bold.green(`🚀 Dev server running at ${url}`));
    log.info(
      colors.cyan(`🔌 WebSocket server running at ws://${host}:${port}`)
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
                `Failed to open browser automatically: ${error.message}`
              )
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

    await ctx.dispose();
    process.exit(0);
  });
}

const command = new Command()
  .description(
    "Start a development server for building apps with live reload and hot module replacement"
  )
  .option(
    "--port <port:number>",
    "Port to run the dev server on (will find next available port if occupied)"
  )
  .option("--host <host:string>", "Host to bind the dev server to", {
    default: DEFAULT_HOST,
  })
  .option(
    "--entry <entry:string>",
    "Entry point file (default: index.ts for Svelte/Vue, index.tsx otherwise)"
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
        path.join(localPath, "raw_app.yaml")
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

async function loadRunnables(): Promise<Record<string, Runnable>> {
  try {
    const localPath = process.cwd();
    const backendPath = path.join(localPath, APP_BACKEND_FOLDER);

    // Load runnables from separate files (new format) or fall back to raw_app.yaml (old format)
    let runnables = await loadRunnablesFromBackend(backendPath);

    if (Object.keys(runnables).length === 0) {
      // Fall back to old format
      const rawApp = (await yamlParseFile(
        path.join(localPath, "raw_app.yaml")
      )) as any;
      runnables = rawApp?.runnables ?? {};
    }

    replaceInlineScripts(runnables, backendPath + SEP, true);

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
  args: any
): Promise<string> {
  const requestBody: any = {
    component: runnableId,
    args,
    force_viewer_static_fields: {},
    force_viewer_one_of_fields: {},
    force_viewer_allow_user_resources: [],
  };

  // Handle static fields
  if (runnable.fields) {
    for (const [key, field] of Object.entries(runnable.fields)) {
      if (field?.type === "static") {
        requestBody.force_viewer_static_fields[key] = field.value;
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
  } else if (runnable.type === "path" && runnable.runType && runnable.path) {
    // Path-based runnables have type: "path" and runType: "script"|"hubscript"|"flow"
    const prefix = runnable.runType;
    requestBody.path =
      prefix !== "hubscript"
        ? `${prefix}/${runnable.path}`
        : `script/${runnable.path}`;
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
