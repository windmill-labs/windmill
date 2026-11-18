// A workspace dependencies push whose request gets no response (the server can take longer
// than the client's or a proxy's timeout to rebuild the dependency map) waits for the pushed
// content to land instead of failing the sync.

import { afterEach, expect, test } from "bun:test";
import { OpenAPI } from "../gen/index.ts";
import { pushWorkspaceDependencies } from "../src/commands/dependencies/dependencies.ts";
import {
  isNoResponseError,
  networkErrorMessage,
} from "../src/utils/utils.ts";

const PUSHED = "requests==2.32.0\n";
const FAST = { intervalMs: 10, timeoutMs: 2_000 };

let server: ReturnType<typeof Bun.serve> | undefined;
afterEach(() => server?.stop(true));

function stubServer(appliedAfterPolls: number | null) {
  let polls = 0;
  server = Bun.serve({
    port: 0,
    fetch(req) {
      const url = new URL(req.url);
      if (url.pathname.endsWith("/workspace_dependencies/create")) {
        return new Response("Gateway Timeout", { status: 504 });
      }
      if (url.pathname.includes("/workspace_dependencies/get_latest/")) {
        // The first read is the up-to-date check that precedes the push.
        const applied = appliedAfterPolls !== null && polls++ > appliedAfterPolls;
        return Response.json({ content: applied ? PUSHED : "requests==2.31.0\n" });
      }
      return new Response("not found", { status: 404 });
    },
  });
  OpenAPI.BASE = `http://localhost:${server.port}/api`;
  return () => polls;
}

test("a push that times out at a proxy succeeds once the server applies it", async () => {
  const polls = stubServer(2);
  await pushWorkspaceDependencies("ws", "dependencies/requirements.in", null, PUSHED, FAST);
  expect(polls()).toBeGreaterThan(2);
});

test("a push the server never applies fails with the push error as its cause", async () => {
  stubServer(null);
  const err = await pushWorkspaceDependencies(
    "ws",
    "dependencies/requirements.in",
    null,
    PUSHED,
    { intervalMs: 10, timeoutMs: 100 },
  ).catch((e) => e);
  expect(err).toBeInstanceOf(Error);
  expect(err.message).toContain("never applied");
  expect(err.cause?.status).toBe(504);
});

test("Node's bare fetch failure is reported with its cause", () => {
  const headersTimeout = new TypeError("fetch failed", {
    cause: Object.assign(new Error("Headers Timeout Error"), {
      code: "UND_ERR_HEADERS_TIMEOUT",
    }),
  });
  expect(isNoResponseError(headersTimeout)).toBe(true);
  expect(networkErrorMessage(headersTimeout)).toContain(
    "did not respond in time: Headers Timeout Error (UND_ERR_HEADERS_TIMEOUT)",
  );

  const refused = new TypeError("fetch failed", {
    cause: Object.assign(new Error("connect ECONNREFUSED 127.0.0.1:8000"), {
      code: "ECONNREFUSED",
    }),
  });
  // Nothing reached the server, so there is nothing to wait for.
  expect(isNoResponseError(refused)).toBe(false);

  const invalidUrl = Object.assign(new TypeError("Invalid URL"), {
    code: "ERR_INVALID_URL",
  });
  expect(networkErrorMessage(invalidUrl)).toBeUndefined();
  expect(networkErrorMessage(refused)).toBe(
    "Could not reach the Windmill server: connect ECONNREFUSED 127.0.0.1:8000 (ECONNREFUSED)",
  );
});
