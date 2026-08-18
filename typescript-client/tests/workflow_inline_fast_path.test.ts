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

const { WorkflowCtx, step, task, setWorkflowCtx } = await import("../client.ts");
import type { Jsonified } from "../client.ts";

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
    // A body that returns nothing — `step("notify", () => { sendEmail() })`.
    ["nothing", () => undefined, null],
    // `JSON.stringify` drops the key for these, and a checkpoint with no
    // `result` is one neither the endpoint nor the worker can parse.
    ["function", () => () => 1, null],
    // Nested, though, a dropped key is what every other bun path produces —
    // and what `Jsonified` describes.
    ["nested-function", () => ({ a: 1, cb: () => 1 }), { a: 1 }],
    // JSON has no non-finite numbers; `Jsonified` keeps calling these `number`.
    ["non-finite", () => ({ nan: NaN, inf: Infinity }), { nan: null, inf: null }],
    // A property holding an unrepresentable value comes back missing, not null
    // — which is why `Jsonified` makes such a key optional.
    ["union-symbol", () => ({ tag: Symbol("x"), keep: 1 }), { keep: 1 }],
    ["class-value", () => ({ Klass: class W {}, keep: 1 }), { keep: 1 }],
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

  test("outside a workflow, step() and task() return the same shape", async () => {
    // No checkpoint, no replay — but a local run must not hand back a shape a
    // deployed one never produces, or testing a workflow locally proves nothing.
    const jobId = process.env.WM_JOB_ID;
    delete process.env.WM_JOB_ID; // otherwise task() takes the v1 dispatch path
    try {
      expect(await step("pair", () => [1, new Date("2026-01-01T00:00:00Z")])).toEqual([
        1,
        "2026-01-01T00:00:00.000Z",
      ]);
      const makePair = task(async function makePair() {
        return { at: new Date("2026-01-01T00:00:00Z") };
      });
      expect(await makePair()).toEqual({ at: "2026-01-01T00:00:00.000Z" });
      // No `BigInt.prototype.toJSON` outside the worker wrapper, so the
      // encoder has to handle bigint itself or this throws.
      expect(await step("big", () => ({ n: 2n ** 70n }))).toEqual({
        n: "1180591620717411303424",
      });
    } finally {
      process.env.WM_JOB_ID = jobId;
    }
  });

  test("a child job's task result is normalized before it is reported", async () => {
    // The child runs the task body and reports it as `step_complete`, which the
    // worker parses into `WacOutput::Complete { result: Value }` — no serde
    // default, so a result whose key JSON.stringify drops fails the job.
    const ctx = new WorkflowCtx({ _executing_key: "returnsFn" } as any);
    setWorkflowCtx(ctx);
    try {
      const returnsFn = task(async function returnsFn() {
        return () => 1;
      });
      const suspend: any = await returnsFn().then(
        () => null,
        (e: any) => e,
      );
      expect(suspend?.dispatchInfo).toMatchObject({ mode: "step_complete", result: null });
    } finally {
      setWorkflowCtx(null);
    }
  });
});

// `Jsonified` must describe the values asserted above, or `step()` advertises a
// type it never returns. `bun test` strips types, and tsconfig only covers
// `src/`, so these are checked with `npx tsc --ignoreConfig --noEmit --strict
// --allowImportingTsExtensions <this file>` (its bun:test / process errors are noise).
type Exact<A, B> = (<G>() => G extends A ? 1 : 2) extends <G>() => G extends B ? 1 : 2
  ? true
  : false;
const _assertType = <E extends true>(_: E) => {};

_assertType<Exact<Jsonified<Date>, string>>(true);
_assertType<Exact<Jsonified<Map<string, number>>, Record<string, never>>>(true);
_assertType<Exact<Jsonified<Set<number>>, Record<string, never>>>(true);
_assertType<Exact<Jsonified<{ a: number; b: undefined }>, { a: number; b: null }>>(true);
_assertType<Exact<Jsonified<void>, null>>(true);
// Shapes JSON does preserve must survive untouched, methods aside.
_assertType<Exact<Jsonified<[number, Date]>, [number, string]>>(true);
_assertType<
  Exact<Jsonified<{ id: number; at: Date; run(): void }>, { id: number; at: string }>
>(true);

// `unknown` is the idiomatic JSON-blob annotation and must survive as itself:
// collapsing to `never` would make any downstream assignment typecheck.
_assertType<Exact<Jsonified<unknown>, unknown>>(true);
_assertType<Exact<Jsonified<Record<string, unknown>>, Record<string, unknown>>>(true);
_assertType<Exact<Jsonified<unknown[]>, unknown[]>>(true);
_assertType<Exact<Jsonified<bigint>, string>>(true);
// Symbol-valued properties are dropped by the encoder, like methods — but a
// property that only *might* hold one keeps its key, optional.
_assertType<Exact<Jsonified<{ a: number; s: symbol }>, { a: number }>>(true);
_assertType<Exact<Jsonified<{ tag: string | symbol }>, { tag?: string }>>(true);
// The key can be missing at runtime, so the type has to accept it missing.
const _omittable: Jsonified<{ tag: string | symbol; keep: number }> = { keep: 1 };
void _omittable;
_assertType<Exact<Jsonified<(string | symbol)[]>, (string | null)[]>>(true);
// A class is a function at runtime, but its type has only a construct signature.
class _Widget {
  x = 1;
}
_assertType<Exact<Jsonified<{ Klass: typeof _Widget; keep: number }>, { keep: number }>>(true);

// A task's result always crosses JSON too — the checkpoint on the workflow
// path, the API on the v1 path — while its arguments stay checked.
const _typedTask = task(async function _typedTask(id: number) {
  return { id, at: new Date() };
});
_assertType<
  Exact<Awaited<ReturnType<typeof _typedTask>>, { id: number; at: string }>
>(true);
