/**
 * Round-parity tests for the WAC v2 inline fast path, against the real client.
 *
 * Run with: bun test typescript-client/tests/workflow_inline_fast_path.test.ts
 *
 * Unlike workflow.test.ts (which mirrors the SDK inline), these import
 * client.ts itself — the fast path is what diverges between the round that
 * runs a step body and its replays, so a mirror would not pin it. The two
 * generated modules are stubbed so the import works without ./build.sh.
 */
import { expect, test, describe, mock, beforeEach } from "bun:test";

mock.module("../services.gen", () => ({
  ResourceService: {},
  VariableService: {},
  JobService: {},
  HelpersService: {},
  AppService: {},
  MetricsService: {},
  OidcService: {},
  UserService: {},
  KafkaTriggerService: {},
}));
mock.module("../core/OpenAPI", () => ({
  OpenAPI: { BASE: "http://localhost:8000/api", TOKEN: "tok" },
}));

const { WorkflowCtx } = await import("../client.ts");

process.env.WM_JOB_ID = "job-1";
process.env.WM_WORKSPACE = "admins";

/** Last checkpoint POSTed by the fast path. */
let posted: Record<string, any>;

beforeEach(() => {
  posted = {};
  // @ts-ignore — stand in for the inline_checkpoint endpoint.
  globalThis.fetch = async (_url: any, init: any) => {
    const payload = JSON.parse(init.body);
    posted[payload.key] = payload.result;
    return new Response("{}", { status: 200 });
  };
});

describe("inline step round parity", () => {
  // Values JSON does not round-trip: the fast path used to hand the body the
  // live object, so `result instanceof Date` was true on the round that ran
  // the body and false on every replay of it.
  const cases: Array<[string, () => any, any]> = [
    ["date", () => new Date("2026-01-01T00:00:00Z"), "2026-01-01T00:00:00.000Z"],
    ["map", () => new Map([["a", 1]]), {}],
    ["set", () => new Set([1, 2]), {}],
    ["undefined-prop", () => ({ a: 1, b: undefined }), { a: 1, b: null }],
  ];

  for (const [name, fn, expected] of cases) {
    test(`${name}: the running round sees what the replay sees`, async () => {
      const live = await new WorkflowCtx({} as any)._runInlineStep(name, fn);
      expect(live).toEqual(expected);
      // ...and it is exactly what got checkpointed, so the replay agrees.
      expect(posted[name]).toEqual(expected);
      const replayed = await new WorkflowCtx({
        completed_steps: { [name]: posted[name] },
      } as any)._runInlineStep(name, fn);
      expect(replayed).toEqual(live);
    });
  }
});
