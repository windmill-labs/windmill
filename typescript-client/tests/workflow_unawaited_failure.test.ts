/**
 * A task the body never awaits still runs and can still fail, and the workflow
 * result cannot express that. Against the real client, not the inline mirror.
 *
 * Run with: bun test typescript-client/tests/workflow_unawaited_failure.test.ts
 */
import { expect, test, describe, mock, beforeEach, afterEach } from "bun:test";

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

const { WorkflowCtx, task, setWorkflowCtx } = await import("../client.ts");

const notify = task(async function notify() {
  throw new Error("boom");
});

const retried = task(async function retried() {
  throw new Error("boom");
}, { retry: { attempts: 1 } });

const marker = { __wmill_error: true, message: "boom", error: { name: "Error", message: "boom" } };

/** The checkpoint a replay reads after the dispatched task failed. */
const failed = { completed_steps: { notify: marker } };

let reported: string[];
const realLog = console.log;
beforeEach(() => {
  reported = [];
  console.log = (m: any) => reported.push(String(m));
});
afterEach(() => {
  console.log = realLog;
  setWorkflowCtx(null);
});

describe("unawaited task failure", () => {
  test("is reported when the body never looked at it", async () => {
    const ctx = new WorkflowCtx(failed);
    setWorkflowCtx(ctx);

    notify();

    ctx._warnUnobservedTaskFailures();
    expect(reported).toHaveLength(1);
    expect(reported[0]).toContain("task 'notify' failed but was never awaited");
    expect(reported[0]).toContain("boom");
  });

  // The body is free to hold the handle and await it further down, so the
  // failure has to be judged at the end of the round rather than at the call.
  test("is not reported when the body awaits it later", async () => {
    const ctx = new WorkflowCtx(failed);
    setWorkflowCtx(ctx);

    const handle = notify();
    await expect(Promise.resolve(handle)).rejects.toThrow("boom");

    ctx._warnUnobservedTaskFailures();
    expect(reported).toEqual([]);
  });

  // Only the attempt handed back to the body counts: the ones a retry moved
  // past are not failures the workflow was ever in a position to see.
  test("is not reported when a retry recovered from it", async () => {
    const ctx = new WorkflowCtx({
      completed_steps: { retried: marker, "retried#retry2": null, "retried#2": 1 },
    });
    setWorkflowCtx(ctx);

    retried();

    ctx._warnUnobservedTaskFailures();
    expect(reported).toEqual([]);
  });

  // A child round replays the whole body to reach one step, so it re-registers
  // every checkpointed failure; reporting them here duplicates them per child.
  test("is not reported by a child round", async () => {
    const ctx = new WorkflowCtx({ ...failed, _executing_key: "other" });
    setWorkflowCtx(ctx);

    notify();

    ctx._warnUnobservedTaskFailures();
    expect(reported).toEqual([]);
  });
});
