/**
 * Dev Server Smoke Tests
 *
 * Tests for `wmill dev` and `wmill app dev` commands.
 * Verifies server startup, WebSocket connectivity, and file-change broadcasting.
 *
 * Run with:
 *   bun test test/dev_server.test.ts
 */

import { expect, test } from "bun:test";
import { writeFile, mkdir } from "node:fs/promises";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "node:net";
import { Subprocess } from "bun";
import WebSocket from "ws";
import { withTestBackend } from "./test_backend.ts";

/** Find a free port by binding to port 0 */
async function findFreePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.listen(0, () => {
      const port = (server.address() as any).port;
      server.close(() => resolve(port));
    });
    server.on("error", reject);
  });
}

/** Wait for a condition with timeout */
async function waitFor<T>(
  fn: () => T | Promise<T>,
  timeoutMs: number,
  label: string,
): Promise<T> {
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown;
  while (Date.now() < deadline) {
    try {
      const result = await fn();
      if (result) return result;
    } catch (e) {
      lastError = e;
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`Timed out waiting for: ${label} (after ${timeoutMs}ms). Last error: ${lastError}`);
}

/** Get CLI main.ts path */
function getCLIMainPath(): string {
  return join(dirname(fileURLToPath(import.meta.url)), "..", "src", "main.ts");
}

// =============================================================================
// TEST 1: `wmill dev` smoke test
// =============================================================================

test(
  "wmill dev: starts server, broadcasts file changes over WebSocket",
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml config
      await writeFile(
        join(tempDir, "wmill.yaml"),
        "defaultTs: bun\n",
        "utf-8",
      );

      // Create a script file
      const scriptDir = join(tempDir, "f", "test");
      await mkdir(scriptDir, { recursive: true });
      await writeFile(
        join(scriptDir, "hello.ts"),
        'export function main() { return "hello"; }\n',
        "utf-8",
      );
      await writeFile(
        join(scriptDir, "hello.script.yaml"),
        `summary: "test"\ndescription: ""\nlock: ""\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
        "utf-8",
      );

      // Push the script so the workspace has content
      const pushResult = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir,
      );
      if (pushResult.code !== 0) {
        console.error("Push stderr:", pushResult.stderr);
        console.error("Push stdout:", pushResult.stdout);
      }
      expect(pushResult.code).toEqual(0);

      // Build the CLI command for `wmill dev`
      const cliMainPath = getCLIMainPath();
      const args = [
        "run",
        cliMainPath,
        "--base-url",
        backend.baseUrl,
        "--workspace",
        backend.workspace,
        "--token",
        backend.token!,
        "--config-dir",
        backend.testConfigDir,
        "dev",
      ];

      let proc: Subprocess | null = null;
      let ws: WebSocket | null = null;

      try {
        // Spawn wmill dev as background process
        proc = Bun.spawn(["bun", ...args], {
          cwd: tempDir,
          stdout: "pipe",
          stderr: "pipe",
          env: { ...process.env },
        });

        // Read stdout to find the port
        const stdoutReader = proc.stdout.getReader();
        let stdoutBuffer = "";
        let port: number | null = null;

        // Wait for "Server listening on port XXXX" message
        const portMatch = await waitFor(
          async () => {
            try {
              const { done, value } = await Promise.race([
                stdoutReader.read(),
                new Promise<{ done: true; value: undefined }>((r) =>
                  setTimeout(() => r({ done: true, value: undefined }), 500),
                ),
              ]);
              if (!done && value) {
                stdoutBuffer += new TextDecoder().decode(value);
              }
            } catch {
              // Reader may be exhausted
            }
            const match = stdoutBuffer.match(
              /Server listening on port (\d+)/,
            );
            return match;
          },
          30000,
          "dev server to start",
        );

        port = parseInt(portMatch[1], 10);
        expect(port).toBeGreaterThan(0);
        stdoutReader.releaseLock();

        // Connect WebSocket
        ws = new WebSocket(`ws://localhost:${port}`);

        // Wait for connection to open
        await new Promise<void>((resolve, reject) => {
          const timeout = setTimeout(
            () => reject(new Error("WebSocket connection timeout")),
            5000,
          );
          ws!.on("open", () => {
            clearTimeout(timeout);
            resolve();
          });
          ws!.on("error", (err) => {
            clearTimeout(timeout);
            reject(err);
          });
        });

        expect(ws.readyState).toEqual(WebSocket.OPEN);

        // Set up a promise to receive the next WebSocket message
        const isWindows = process.platform === "win32";
        const messagePromise = new Promise<any>((resolve, reject) => {
          const timeout = setTimeout(
            () => reject(new Error("WebSocket message timeout")),
            isWindows ? 30000 : 10000,
          );
          ws!.on("message", (data) => {
            clearTimeout(timeout);
            try {
              resolve(JSON.parse(data.toString()));
            } catch (e) {
              reject(e);
            }
          });
        });

        // Modify the script file on disk
        // Windows fs.watch() needs more time to initialize with recursive: true
        await new Promise((r) => setTimeout(r, isWindows ? 2000 : 300));
        await writeFile(
          join(scriptDir, "hello.ts"),
          'export function main() { return "modified"; }\n',
          "utf-8",
        );

        // Wait for WebSocket message
        const message = await messagePromise;

        // Verify the message
        expect(message.type).toEqual("script");
        expect(message.content).toContain("modified");
        expect(message.path).toContain("f/test/hello");
        expect(message.language).toBeTruthy();
      } finally {
        if (ws) {
          ws.close();
        }
        if (proc) {
          proc.kill();
          await proc.exited;
        }
      }
    });
  },
  { timeout: 60000 },
);

// =============================================================================
// TEST 2: `wmill app dev` smoke test
// =============================================================================

test(
  "wmill app dev: starts HTTP server, serves HTML, provides SSE endpoint",
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      // Create wmill.yaml config
      await writeFile(
        join(tempDir, "wmill.yaml"),
        "defaultTs: bun\n",
        "utf-8",
      );

      // Create a raw app directory with the right suffix
      const appDir = join(tempDir, "f", "test", "myapp.raw_app");
      await mkdir(appDir, { recursive: true });

      // Create raw_app.yaml
      await writeFile(
        join(appDir, "raw_app.yaml"),
        `custom_path: f/test/myapp\n`,
        "utf-8",
      );

      // Create package.json (minimal, with react dependency)
      await writeFile(
        join(appDir, "package.json"),
        JSON.stringify(
          {
            name: "test-app",
            private: true,
            dependencies: {
              react: "^18.0.0",
              "react-dom": "^18.0.0",
            },
          },
          null,
          2,
        ),
        "utf-8",
      );

      // Create index.tsx entry point
      await writeFile(
        join(appDir, "index.tsx"),
        `import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";

const root = createRoot(document.getElementById("root")!);
root.render(<App />);
`,
        "utf-8",
      );

      // Create App.tsx
      await writeFile(
        join(appDir, "App.tsx"),
        `import React from "react";

export default function App() {
  return <div>Hello from test app</div>;
}
`,
        "utf-8",
      );

      // Run npm install in the app directory
      const npmInstall = Bun.spawn(["npm", "install"], {
        cwd: appDir,
        stdout: "pipe",
        stderr: "pipe",
      });
      await Promise.all([
        new Response(npmInstall.stdout).text(),
        new Response(npmInstall.stderr).text(),
      ]);
      const npmExitCode = await npmInstall.exited;
      expect(npmExitCode).toEqual(0);

      // Find a free port
      const port = await findFreePort();

      // Build the CLI command for `wmill app dev`
      const cliMainPath = getCLIMainPath();
      const args = [
        "run",
        cliMainPath,
        "--base-url",
        backend.baseUrl,
        "--workspace",
        backend.workspace,
        "--token",
        backend.token!,
        "--config-dir",
        backend.testConfigDir,
        "app",
        "dev",
        appDir,
        "--no-open",
        "--port",
        String(port),
      ];

      let proc: Subprocess | null = null;

      try {
        // Spawn wmill app dev as background process
        proc = Bun.spawn(["bun", ...args], {
          cwd: tempDir,
          stdout: "pipe",
          stderr: "pipe",
          env: { ...process.env },
        });

        // Collect stderr in background for debugging
        const stderrReader = proc.stderr.getReader();
        let stderrBuffer = "";
        (async () => {
          try {
            while (true) {
              const { done, value } = await stderrReader.read();
              if (done) break;
              stderrBuffer += new TextDecoder().decode(value);
            }
          } catch {
            // Process may have exited
          }
        })();

        // Wait for server to be ready by polling the HTTP endpoint
        await waitFor(
          async () => {
            try {
              const res = await fetch(`http://localhost:${port}/`, {
                signal: AbortSignal.timeout(1000),
              });
              if (res.ok) {
                await res.text();
                return true;
              }
              await res.text();
            } catch {
              // Not ready yet
            }
            return false;
          },
          60000,
          "app dev server to be ready",
        );

        // Verify GET / returns HTML
        const htmlRes = await fetch(`http://localhost:${port}/`);
        const contentType = htmlRes.headers.get("content-type");
        const htmlBody = await htmlRes.text();
        expect(contentType).toContain("text/html");
        expect(htmlBody).toContain("<!DOCTYPE html>");
        expect(htmlBody).toContain("<div id=\"root\">");

        // Verify GET /__events returns SSE stream
        const controller = new AbortController();
        const sseTimeout = setTimeout(() => controller.abort(), 5000);
        try {
          const sseRes = await fetch(`http://localhost:${port}/__events`, {
            signal: controller.signal,
          });
          const sseContentType = sseRes.headers.get("content-type");
          expect(sseContentType).toContain("text/event-stream");
          // Read a small chunk to verify SSE sends data
          const reader = sseRes.body!.getReader();
          const { value } = await reader.read();
          const chunk = new TextDecoder().decode(value);
          expect(chunk).toContain("data: connected");
          reader.cancel();
        } finally {
          clearTimeout(sseTimeout);
        }
      } finally {
        if (proc) {
          proc.kill();
          await proc.exited;
        }
      }
    });
  },
  { timeout: 120000 },
);

