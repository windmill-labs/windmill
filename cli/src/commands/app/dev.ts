// deno-lint-ignore-file no-explicit-any
import {
  Command,
  colors,
  log,
  getPort,
  open,
  windmillUtils,
  yamlParseFile,
} from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import * as http from "node:http";
import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";
import { writeFileSync } from "node:fs";

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
  // Dynamically import esbuild only when the dev command is called
  const esbuild = await import("npm:esbuild@0.24.2");

  const port =
    opts.port ??
    (await getPort.default({
      port: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map((p) => p + DEFAULT_PORT),
    }));
  const host = opts.host ?? DEFAULT_HOST;
  const entryPoint = opts.entry ?? "index.tsx";
  const shouldOpen = opts.open ?? true;

  // Verify entry point exists
  if (!fs.existsSync(entryPoint)) {
    log.error(
      colors.red(
        `Entry point "${entryPoint}" not found. Please specify a valid entry point with --entry.`
      )
    );
    Deno.exit(1);
  }

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

  const buildOptions = {
    entryPoints: [entryPoint],
    bundle: true,
    outfile: "dist/bundle.js",
    format: "iife" as const,
    platform: "browser" as const,
    target: "es2020",
    jsx: "automatic" as const,
    loader: {
      ".css": "css" as const,
    },
    define: {
      "process.env.NODE_ENV": '"development"',
    },
    sourcemap: true,
    logLevel: "info" as const,
    write: true,
  };

  // Create esbuild context
  const ctx = await esbuild.context({
    ...buildOptions,
    plugins: [
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
    ],
  });

  // Start watching
  await ctx.watch();
  log.info(colors.blue("👀 Watching for file changes...\n"));

  // Initial build
  await ctx.rebuild();

  // Watch runnables folder for changes
  const runnablesPath = path.join(process.cwd(), "runnables");
  let runnablesWatcher: Deno.FsWatcher | undefined;

  if (fs.existsSync(runnablesPath)) {
    log.info(
      colors.blue(`👁️  Watching runnables folder at: ${runnablesPath}\n`)
    );
    runnablesWatcher = Deno.watchFs(runnablesPath);

    const runnablesChangeTimeouts: Record<string, number> = {};

    // Handle runnables file changes in the background
    (async () => {
      try {
        for await (const event of runnablesWatcher!) {
          const key = event.paths.join(",");

          // Debounce file changes (wait 100ms for rapid successive changes)
          if (runnablesChangeTimeouts[key]) {
            clearTimeout(runnablesChangeTimeouts[key]);
          }

          runnablesChangeTimeouts[key] = setTimeout(() => {
            delete runnablesChangeTimeouts[key];

            // Process each changed path
            event.paths.forEach(async (changedPath) => {
              const relativePath = path.relative(process.cwd(), changedPath);

              // Log the change event
              log.info(
                colors.cyan(
                  `📝 Runnable changed [${event.kind}]: ${relativePath}`
                )
              );
              await genRunnablesTs();
              // TODO: Add logic here to handle runnable changes
              // For example:
              // - Parse the runnable file
              // - Update app configuration
              // - Trigger rebuild if needed
              // - Sync with backend
            });
          }, 100);
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

  server.listen(port, host, () => {
    const url = `http://${host}:${port}`;
    log.info(colors.bold.green(`🚀 Dev server running at ${url}`));
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
  .option("--entry <entry:string>", "Entry point file for the application", {
    default: "index.tsx",
  })
  .option("--no-open", "Don't automatically open the browser")
  .action(dev as any);

export default command;

async function genRunnablesTs() {
  log.info(colors.blue("🔄 Generating runnables.ts..."));
  const rawApp = (await yamlParseFile(
    path.join(process.cwd(), "raw_app.yaml")
  )) as any;
  const runnables = rawApp?.["value"]?.["runnables"] as any;
  const newWmillTs = windmillUtils.genWmillTs(runnables);
  writeFileSync(path.join(process.cwd(), "wmill.d.ts"), newWmillTs);
}
