import { afterEach, describe, expect, it } from "bun:test";
import type { WindmillBackendSettings } from "../../core/windmillBackendSettings";
import {
  WindmillBackendClient,
  assertWindmillBackendReachable,
} from "./windmillBackend";

const ORIGINAL_FETCH = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = ORIGINAL_FETCH;
});

describe("assertWindmillBackendReachable", () => {
  it("logs in to verify backend reachability", async () => {
    const requests: Array<{ url: string; init?: RequestInit }> = [];
    globalThis.fetch = mockFetch(requests, textResponse(200, "token"));

    await expect(
      assertWindmillBackendReachable(
        buildSettings({ baseUrl: "http://backend.test/reachable" }),
      ),
    ).resolves.toBeUndefined();

    expect(requests.map((entry) => entry.url)).toEqual([
      "http://backend.test/reachable/api/auth/login",
    ]);
  });

  it("adds setup guidance when the backend cannot be initialized", async () => {
    globalThis.fetch = mockFetch(
      [],
      textResponse(401, "invalid password"),
    );

    await expect(
      assertWindmillBackendReachable(
        buildSettings({ baseUrl: "http://backend.test/auth-failure" }),
      ),
    ).rejects.toThrow(
      "Start a Windmill backend at that URL, or set WMILL_AI_EVAL_BACKEND_URL=<url>.",
    );
  });
});

describe("WindmillBackendClient", () => {
  it("creates or reuses the specified backend workspace without deleting it", async () => {
    const requests: Array<{ url: string; init?: RequestInit }> = [];
    globalThis.fetch = mockFetch(
      requests,
      textResponse(200, "token"),
      textResponse(200, "false"),
      textResponse(200, ""),
    );

    const client = new WindmillBackendClient(
      buildSettings({
        baseUrl: "http://backend.test/shared-workspace",
        workspaceOverride: "shared-evals",
      }),
    );

    await expect(
      client.withWorkspace("case-a", 1, async (workspaceId) => workspaceId),
    ).resolves.toBe("shared-evals");

    expect(requests.map((entry) => entry.url)).toEqual([
      "http://backend.test/shared-workspace/api/auth/login",
      "http://backend.test/shared-workspace/api/workspaces/exists",
      "http://backend.test/shared-workspace/api/workspaces/create",
    ]);
  });
});

function buildSettings(
  overrides: Partial<WindmillBackendSettings> = {},
): WindmillBackendSettings {
  return {
    baseUrl: "http://backend.test/default",
    email: "admin@windmill.dev",
    password: "changeme",
    ...overrides,
  };
}

function mockFetch(
  requests: Array<{ url: string; init?: RequestInit }>,
  ...responses: Response[]
): typeof fetch {
  const queue = [...responses];
  return async (input, init) => {
    const url = String(input);
    requests.push({ url, init });
    const next = queue.shift();
    if (!next) {
      throw new Error(`Unexpected fetch: ${url}`);
    }
    return next;
  };
}

function textResponse(status: number, body: string): Response {
  return new Response(body, { status });
}
