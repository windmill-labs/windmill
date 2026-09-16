/**
 * The fingerprint a dispatched task carries, against the real client.
 *
 * Run with: bun test typescript-client/tests/workflow_task_identity.test.ts
 *
 * The worker keys a task child's cached result on this `fn_id`, so two tasks
 * that share a step key (anonymous tasks in exclusive branches) or whose source
 * reads the same (bound functions) must not share one. Imports client.ts itself,
 * with the two generated modules stubbed so the import works without ./build.sh.
 */
import { expect, test, describe, mock, beforeAll, afterAll } from "bun:test";

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

const { WorkflowCtx, task, setWorkflowCtx, StepSuspend } = await import("../client.ts");
import { isSuspendSignal } from "../wacError";

/** The steps a first round of `body` dispatches. */
async function dispatched(body: () => Promise<any>): Promise<any[]> {
  const ctx = new WorkflowCtx({ completed_steps: {} } as any);
  setWorkflowCtx(ctx);
  try {
    await body();
  } catch (e: any) {
    if (isSuspendSignal(e, StepSuspend)) return e.dispatchInfo.steps;
    throw e;
  } finally {
    setWorkflowCtx(null);
  }
  throw new Error("the body completed without dispatching");
}

// These assert the suspend a worker acts on, the legacy inline path; the v2 fast
// path is on by default, so pin it off rather than depend on `WM_JOB_ID` being
// absent.
const priorFastPath = process.env.WM_WAC_INLINE_FAST_PATH;
beforeAll(() => {
  process.env.WM_WAC_INLINE_FAST_PATH = "0";
});
afterAll(() => {
  if (priorFastPath === undefined) delete process.env.WM_WAC_INLINE_FAST_PATH;
  else process.env.WM_WAC_INLINE_FAST_PATH = priorFastPath;
});

describe("task fingerprint", () => {
  test("two anonymous tasks dispatched at the same position carry different fingerprints", async () => {
    const tasks = {
      a: task(async () => "A", { cache_ttl: 60 }),
      b: task(async () => "B", { cache_ttl: 60 }),
    };
    const [a] = await dispatched(() => tasks.a() as Promise<any>);
    const [b] = await dispatched(() => tasks.b() as Promise<any>);
    expect(a.key).toBe(b.key);
    expect(typeof a.fn_id).toBe("string");
    expect(a.fn_id).not.toBe(b.fn_id);
  });

  test("two bound methods of one name share a fingerprint, and their step keys separate them", async () => {
    const a = {
      async read() {
        return "A";
      },
    };
    const b = {
      async read() {
        return "B";
      },
    };
    const steps = await dispatched(async () => {
      await Promise.all([task(a.read.bind(a))(), task(b.read.bind(b))()]);
    });
    expect(steps[0].fn_id).toBe(steps[1].fn_id);
    expect(steps.map((s) => s.key)).toEqual(["bound read", "bound read_2"]);
  });

  test("one task carries one fingerprint at every position", async () => {
    const double = task(async (n: number) => n * 2, { cache_ttl: 60 });
    const steps = await dispatched(async () => {
      await Promise.all([double(1), double(2)]);
    });
    expect(steps.map((s) => s.key)).toEqual(["step", "step_2"]);
    expect(steps[0].fn_id).toBe(steps[1].fn_id);
  });

  test("two bound functions carry different fingerprints", async () => {
    const api = {
      async orders() {
        return "orders";
      },
      async users() {
        return "users";
      },
    };
    const [orders] = await dispatched(() => task(api.orders.bind(api))() as Promise<any>);
    const [users] = await dispatched(() => task(api.users.bind(api))() as Promise<any>);
    expect(orders.fn_id).not.toBe(users.fn_id);
  });
});
