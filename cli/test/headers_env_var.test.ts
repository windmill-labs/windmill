/**
 * Integration test: HEADERS env var is forwarded on every CLI fetch call.
 *
 * Spins up an auth-gateway proxy in front of the test backend that:
 *   - 403s + Cloudflare-style HTML if the request is missing CF-Access-Client-Id /
 *     CF-Access-Client-Secret (mirrors the real-world Cloudflare Access challenge),
 *   - otherwise reverse-proxies to the backend.
 *
 * Then runs `wmill sync push` with --base-url pointed at the proxy. If any fetch
 * in the CLI bypasses HEADERS, the proxy returns the challenge page and the push
 * fails (or the request is logged as un-authenticated). Negative case verifies
 * the gateway actually rejects un-headered requests, so a passing positive case
 * is meaningful.
 *
 * Regression coverage for #6421 and the script.ts pushScript /
 * jobs/run/preview_bundle / app dev SSE fetch calls that used to skip getHeaders().
 */

import { expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Server } from "bun";
import { withTestBackend } from "./test_backend.ts";

const HEADER_NAMES = ["CF-Access-Client-Id", "CF-Access-Client-Secret"] as const;
const HEADER_VALUES = {
  "CF-Access-Client-Id": "test-cf-id",
  "CF-Access-Client-Secret": "test-cf-secret",
} as const;

const HEADERS_ENV = HEADER_NAMES
  .map((h) => `${h}: ${HEADER_VALUES[h]}`)
  .join(", ");

interface ProxyState {
  authenticatedRequests: { method: string; path: string }[];
  rejectedRequests: { method: string; path: string }[];
}

interface RunningProxy {
  server: Server;
  state: ProxyState;
  url: string;
}

function startGatewayProxy(backendUrl: string): RunningProxy {
  const state: ProxyState = {
    authenticatedRequests: [],
    rejectedRequests: [],
  };

  const server = Bun.serve({
    port: 0,
    hostname: "127.0.0.1",
    async fetch(req) {
      const reqUrl = new URL(req.url);
      const pathAndQuery = reqUrl.pathname + reqUrl.search;

      const id = req.headers.get("cf-access-client-id");
      const secret = req.headers.get("cf-access-client-secret");
      const passes =
        id === HEADER_VALUES["CF-Access-Client-Id"] &&
        secret === HEADER_VALUES["CF-Access-Client-Secret"];

      if (!passes) {
        state.rejectedRequests.push({ method: req.method, path: pathAndQuery });
        return new Response(
          '<!DOCTYPE html><title>Sign in ・ Cloudflare Access</title>' +
            '<body>Authenticate to reach this site.</body>',
          {
            status: 403,
            headers: {
              "content-type": "text/html; charset=utf-8",
              "cf-ray": "0000000000000000-TEST",
              "cf-mitigated": "challenge",
            },
          },
        );
      }

      state.authenticatedRequests.push({ method: req.method, path: pathAndQuery });

      const target = new URL(pathAndQuery, backendUrl);
      const forwardHeaders = new Headers(req.headers);
      forwardHeaders.delete("cf-access-client-id");
      forwardHeaders.delete("cf-access-client-secret");
      forwardHeaders.set("host", new URL(backendUrl).host);

      const body =
        req.method === "GET" || req.method === "HEAD"
          ? undefined
          : await req.arrayBuffer();

      return await fetch(target, {
        method: req.method,
        headers: forwardHeaders,
        body,
        redirect: "manual",
      });
    },
  });

  return {
    server,
    state,
    url: `http://127.0.0.1:${server.port}`,
  };
}

async function runCliThroughProxy(
  backend: { workspace: string; testConfigDir: string; token?: string },
  proxyUrl: string,
  cliArgs: string[],
  cwd: string,
  env: Record<string, string>,
): Promise<{ stdout: string; stderr: string; code: number }> {
  // Use fileURLToPath + node:path so the CLI entrypoint is a real OS path on
  // Windows. `new URL(..).pathname` yields `/C:/...` which Bun.spawn fails to
  // resolve, so the negative case "rejected" on launch instead of reaching
  // the proxy and the assertion on rejectedRequests > 0 flaked.
  const cliDir = join(dirname(fileURLToPath(import.meta.url)), "..");
  const useNode = process.env["TEST_CLI_RUNTIME"] === "node";
  const runtime = useNode ? "node" : "bun";
  const entrypoint = useNode
    ? join(cliDir, "npm", "esm", "main.js")
    : join(cliDir, "src", "main.ts");
  const runtimeArgs = useNode ? [entrypoint] : ["run", entrypoint];

  const fullArgs = [
    "--base-url", proxyUrl,
    "--workspace", backend.workspace,
    "--token", backend.token ?? "",
    "--config-dir", backend.testConfigDir,
    ...cliArgs,
  ];

  const proc = Bun.spawn([runtime, ...runtimeArgs, ...fullArgs], {
    cwd,
    env: { ...(process.env as Record<string, string>), ...env },
    stdout: "pipe",
    stderr: "pipe",
  });

  const [stdout, stderr] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  const code = await proc.exited;
  return { stdout, stderr, code };
}

test(
  "HEADERS env var is forwarded on every CLI fetch (sync push of new script)",
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      const proxy = startGatewayProxy(backend.baseUrl);
      try {
        await writeFile(
          `${tempDir}/wmill.yaml`,
          `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`,
          "utf-8",
        );

        const uniqueId = Date.now();
        const scriptName = `headers_${uniqueId}`;
        const scriptDir = `${tempDir}/f/test`;
        await mkdir(scriptDir, { recursive: true });
        await writeFile(
          `${scriptDir}/${scriptName}.ts`,
          `export async function main() {\n  return "headers test ${uniqueId}";\n}\n`,
          "utf-8",
        );
        await writeFile(
          `${scriptDir}/${scriptName}.script.yaml`,
          [
            `summary: "headers regression"`,
            `description: "Push covers /scripts/create + /jobs/run/dependencies_async"`,
            `lock: ""`,
            `schema:`,
            `  $schema: "https://json-schema.org/draft/2020-12/schema"`,
            `  type: object`,
            `  properties: {}`,
            `  required: []`,
            `is_template: false`,
            `kind: script`,
            `language: bun`,
            ``,
          ].join("\n"),
          "utf-8",
        );

        const includesGlob = `f/test/${scriptName}**`;

        // Negative case: no HEADERS env -> the proxy should return the
        // gateway challenge and the CLI should bail out non-zero. Proves the
        // proxy is actually gating, so the positive case isn't a false pass.
        const noHeaders = await runCliThroughProxy(
          backend,
          proxy.url,
          ["sync", "push", "--yes", "--includes", includesGlob],
          tempDir,
          {},
        );
        expect(noHeaders.code).not.toEqual(0);
        expect(proxy.state.rejectedRequests.length).toBeGreaterThan(0);
        expect(proxy.state.authenticatedRequests.length).toEqual(0);

        // Reset proxy state between cases.
        proxy.state.authenticatedRequests.length = 0;
        proxy.state.rejectedRequests.length = 0;

        // Positive case: HEADERS env set -> the proxy must see those headers
        // on every request the CLI makes for sync push to succeed.
        const withHeaders = await runCliThroughProxy(
          backend,
          proxy.url,
          ["sync", "push", "--yes", "--includes", includesGlob],
          tempDir,
          { HEADERS: HEADERS_ENV },
        );
        expect(withHeaders.code).toEqual(0);
        expect(proxy.state.rejectedRequests).toEqual([]);

        const paths = proxy.state.authenticatedRequests.map((r) => r.path);

        // Print the full path list when an assertion below fails so the
        // failure is debuggable without re-running with extra logging.
        const debug = () => paths.join("\n  ");

        // Tarball download (sync diff): src/commands/sync/pull.ts
        expect(
          paths.some((p) =>
            p.startsWith(`/api/w/${backend.workspace}/workspaces/tarball`),
          ),
          `expected tarball request, got:\n  ${debug()}`,
        ).toBe(true);

        // Script create: src/commands/script/script.ts pushScript().
        // This is the regression: PR #8936 switched from wmill.createScript()
        // (SDK, inherits OpenAPI.HEADERS) to a raw fetch that didn't forward
        // HEADERS. Without the fix, the proxy would 403 this request and the
        // CLI exit would be non-zero — so this assertion is the load-bearing
        // one for #8936 / #6421-style regressions.
        expect(
          paths.some((p) =>
            p.includes(`/api/w/${backend.workspace}/scripts/create`),
          ),
          `expected /scripts/create request, got:\n  ${debug()}`,
        ).toBe(true);

        // Lock generation: src/utils/metadata.ts. Was the original #6421 fix
        // (PR #6422) — a soft check; the backend may skip queueing a lock job
        // for trivial bun scripts under some feature configurations, so we
        // only verify it *if* the CLI tried to generate one.
        // (No assertion — covered by the tarball+create checks above plus
        // the negative case proving the proxy actually gates requests.)
      } finally {
        proxy.server.stop(true);
      }
    });
  },
  240_000,
);
