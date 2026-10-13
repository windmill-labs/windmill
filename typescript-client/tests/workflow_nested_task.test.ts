/**
 * A task body runs as one child job, so the SDK calls it makes cannot be
 * dispatched as steps of their own. Against the real client.
 *
 * Run with: bun test typescript-client/tests/workflow_nested_task.test.ts
 */
import { expect, test, describe, mock, afterEach } from "bun:test";

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

const { WorkflowCtx, StepSuspend, task, taskScript, step, setWorkflowCtx } = await import(
  "../client.ts"
);

let ran: string[] = [];

const inner = task(async function inner(x: string) {
  ran.push("inner");
  return `inner:${x}`;
});

const outer = task(async function outer(x: string) {
  ran.push("outer");
  await Promise.resolve();
  const viaTask = await inner(x);
  const viaStep = await step("s", () => "stepped");
  return `${viaTask}+${viaStep}`;
});

const dispatching = task(async function dispatching() {
  return await taskScript("f/other/script")({});
});

/** Run `body` as the child round executing `key`, and hand back what it raised. */
async function childRound(key: string, body: () => Promise<unknown>): Promise<any> {
  setWorkflowCtx(new WorkflowCtx({ completed_steps: {}, _executing_key: key }));
  const hung = new Promise((r) => setTimeout(() => r("hung"), 500));
  return await Promise.race([body().then(() => "returned", (e) => e), hung]);
}

afterEach(() => {
  ran = [];
  setWorkflowCtx(null);
});

describe("SDK calls inside a task body", () => {
  test("a nested task and step run in the task's own job", async () => {
    const raised = await childRound("outer", async () => {
      await outer("a");
    });

    expect(raised).toBeInstanceOf(StepSuspend);
    expect(raised.dispatchInfo).toMatchObject({
      mode: "step_complete",
      result: "inner:a+stepped",
    });
  });

  // The workflow body keeps replaying beside the task body, and its own calls
  // must stay parked: running them would execute a sibling task in this job.
  test("a sibling task of the workflow body is not run", async () => {
    const raised = await childRound("outer", async () => {
      await Promise.all([outer("a"), inner("sibling")]);
    });

    expect(raised.dispatchInfo).toMatchObject({ mode: "step_complete" });
    expect(ran).toEqual(["outer", "inner"]);
  });

  test("a step that would need a dispatch fails instead of hanging", async () => {
    const raised = await childRound("dispatching", async () => {
      await dispatching();
    });

    expect(raised).toBeInstanceOf(Error);
    expect(raised.message).toContain("cannot be called from inside a task");
  });
});
