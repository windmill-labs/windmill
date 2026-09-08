/**
 * `TaskOptions.retry`, against the real client.
 *
 * Run with: bun test typescript-client/tests/workflow_retry.test.ts
 *
 * Nothing carries a retry across rounds: every round re-derives which attempt
 * comes next from the checkpoint alone, so these drive the rounds a worker
 * would and assert what it would act on. Imports client.ts itself (a mirror
 * would pin the mirror), with the two generated modules stubbed so the import
 * works without ./build.sh.
 */
import { expect, test, describe, mock } from "bun:test";

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

const { WorkflowCtx, task, step, setWorkflowCtx, StepSuspend } = await import("../client.ts");
import { isSuspendSignal } from "../wacError";

/** What the failed child job leaves in `completed_steps`. */
const failed = {
  __wmill_error: true,
  message: "boom",
  step_key: "callApi",
  result: { error: { name: "Error", message: "boom" } },
};

/** One round against `completed`, reduced to what the worker acts on. */
async function round(completed: Record<string, any>, body: () => Promise<any>): Promise<any> {
  const ctx = new WorkflowCtx({ completed_steps: completed } as any);
  setWorkflowCtx(ctx);
  try {
    return { type: "complete", result: await body() };
  } catch (e: any) {
    if (isSuspendSignal(e, StepSuspend)) return { type: "suspend", ...e.dispatchInfo };
    return { type: "error", error: e };
  } finally {
    setWorkflowCtx(null);
  }
}

describe("task retry", () => {
  test("each failure buys a backoff sleep and one more attempt, until attempts run out", async () => {
    const callApi = task(async function callApi(x: number) {
      return x;
    }, { retry: { attempts: 2, delay: 30, multiplier: 2 } });
    const body = () => callApi(1) as Promise<any>;

    let r = await round({}, body);
    expect(r.steps.map((s: any) => s.key)).toEqual(["callApi"]);

    const completed: Record<string, any> = { callApi: failed };
    r = await round(completed, body);
    expect(r).toMatchObject({ mode: "sleep", key: "callApi#retry2", seconds: 30 });

    completed["callApi#retry2"] = null;
    r = await round(completed, body);
    expect(r.steps.map((s: any) => s.key)).toEqual(["callApi#2"]);

    // the delay grows by the multiplier for the second retry
    completed["callApi#2"] = failed;
    r = await round(completed, body);
    expect(r).toMatchObject({ mode: "sleep", key: "callApi#retry3", seconds: 60 });

    completed["callApi#retry3"] = null;
    r = await round(completed, body);
    expect(r.steps.map((s: any) => s.key)).toEqual(["callApi#3"]);

    // attempts spent: the failure reaches the body
    completed["callApi#3"] = failed;
    r = await round(completed, body);
    expect(r.type).toBe("error");
    expect(r.error.name).toBe("TaskError");
  });

  test("retrying one call of a task does not move the keys of the calls beside it", async () => {
    const t = task(async function t(x: number) {
      return x;
    }, { retry: { attempts: 1 } });
    const body = () => Promise.all([t(1), t(2)]);

    let r = await round({}, body);
    expect(r.steps.map((s: any) => s.key)).toEqual(["t", "t_2"]);

    // the first call retries as `t#2`; the second keeps the `t_2` it was
    // dispatched under, rather than being read as the first call's retry
    r = await round({ t: failed, t_2: 20 }, body);
    expect(r.steps.map((s: any) => s.key)).toEqual(["t#2"]);
  });

  test("a task the body never awaits still sleeps and retries", async () => {
    // The runner dispatches unawaited task calls by flushing `pending`, so a
    // backoff that only fired when awaited would drop the retry silently and
    // report the workflow complete.
    const fire = task(async function fire() {
      return 1;
    }, { retry: { attempts: 1, delay: 30 } });
    const body = async () => {
      fire();
      return "done";
    };

    const r = await round({ fire: failed }, body);
    expect(r).toMatchObject({ mode: "sleep", key: "fire#retry2", seconds: 30 });
  });

  test("an inline step named like an attempt key does not stand in for the attempt", async () => {
    // `step()` names are arbitrary strings, so a derived key has to be claimed
    // through the allocator rather than assumed free.
    const t = task(async function t(x: number) {
      return x;
    }, { retry: { attempts: 1 } });
    const body = async () => {
      const decoy = await step("t#2", () => "not an attempt");
      return [decoy, await t(1)];
    };

    const r = await round({ "t#2": "not an attempt", t: failed }, body);
    expect(r.steps.map((s: any) => s.key)).toEqual(["t#2_2"]);
  });

  test("the child dispatched for an attempt walks the loop past the failure and backoff", async () => {
    // The non-matching branch returns a thenable that never resolves, so a
    // child that walks the loop wrong parks the run until its timeout.
    const t = task(async function t(x: number) {
      return x * 10;
    }, { retry: { attempts: 1, delay: 30 } });
    const ctx = new WorkflowCtx({
      completed_steps: { t: failed, "t#retry2": null },
      _executing_key: "t#2",
    } as any);
    setWorkflowCtx(ctx);
    try {
      const suspend: any = await t(4).then(
        () => null,
        (e: any) => e,
      );
      expect(suspend.dispatchInfo).toMatchObject({ mode: "step_complete", result: 40 });
    } finally {
      setWorkflowCtx(null);
    }
  });

  test("max_delay caps the backoff a multiplier grows", async () => {
    const t = task(async function t(x: number) {
      return x;
    }, { retry: { attempts: 2, delay: 60, multiplier: 100, max_delay: 300 } });
    const body = () => t(1) as Promise<any>;

    const r = await round({ t: failed, "t#retry2": null, "t#2": failed }, body);
    expect(r).toMatchObject({ mode: "sleep", key: "t#retry3", seconds: 300 });
  });

  test("a retry that succeeds resolves to its value, and later steps keep their own keys", async () => {
    // No delay, so the retry is dispatched without a sleep round in between.
    const flaky = task(async function flaky(x: number) {
      return x;
    }, { retry: { attempts: 1 } });
    const double = task(async function double(v: number) {
      return v * 2;
    });
    const body = async () => double((await flaky(1)) as number);

    const r = await round({ flaky: failed, "flaky#2": 7 }, body);
    expect(r.steps).toMatchObject([{ key: "double", args: { v: 7 } }]);
  });
});
