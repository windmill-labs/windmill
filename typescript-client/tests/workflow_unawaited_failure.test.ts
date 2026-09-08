/**
 * A task the workflow body never awaits still runs and can still fail, and the
 * workflow result cannot express that. Pins what the ctx reports at the end of
 * the round, against the real client.
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

/** The checkpoint a replay reads after the dispatched task failed. */
const failed = {
  completed_steps: {
    notify: { __wmill_error: true, message: "boom", error: { name: "Error", message: "boom" } },
  },
};

let warnings: string[];
const realWarn = console.warn;
beforeEach(() => {
  warnings = [];
  console.warn = (m: any) => warnings.push(String(m));
});
afterEach(() => {
  console.warn = realWarn;
  setWorkflowCtx(null);
});

describe("unawaited task failure", () => {
  test("is reported when the body never looked at it", async () => {
    const ctx = new WorkflowCtx(failed);
    setWorkflowCtx(ctx);

    notify();

    ctx._warnUnobservedTaskFailures();
    expect(warnings).toHaveLength(1);
    expect(warnings[0]).toContain("task 'notify' failed but was never awaited");
    expect(warnings[0]).toContain("boom");
  });

  // The body is free to hold the handle and await it further down, so the
  // failure has to be judged at the end of the round rather than at the call.
  test("is not reported when the body awaits it later", async () => {
    const ctx = new WorkflowCtx(failed);
    setWorkflowCtx(ctx);

    const handle = notify();
    await expect(Promise.resolve(handle)).rejects.toThrow("boom");

    ctx._warnUnobservedTaskFailures();
    expect(warnings).toEqual([]);
  });
});
