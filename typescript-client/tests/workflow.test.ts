/**
 * Standalone tests for the Workflow-as-Code TypeScript SDK.
 *
 * Run with: bun test typescript-client/tests/workflow.test.ts
 */
import { expect, test, describe } from "bun:test";

// --- Inline SDK (mirrors client.ts implementation) ---

class StepSuspend extends Error {
  constructor(public dispatchInfo: Record<string, any>) {
    super("__step_suspend__");
    this.name = "StepSuspend";
  }
}

let _workflowCtx: WorkflowCtx | null = null;

class WorkflowCtx {
  private completed: Record<string, any>;
  private stepIndex = 0;
  private pending: Array<{
    name: string;
    script: string;
    args: Record<string, any>;
    key: string;
  }> = [];
  private _suspended = false;
  _executingKey: string | null;

  constructor(checkpoint: Record<string, any> = {}) {
    this.completed = checkpoint?.completed_steps ?? {};
    this._executingKey = checkpoint?._executing_key ?? null;
  }

  _allocKey(): string {
    return `step_${this.stepIndex++}`;
  }

  _nextStep(
    name: string,
    script: string,
    args: Record<string, any> = {}
  ): PromiseLike<any> {
    const key = this._allocKey();

    if (key in this.completed) {
      const value = this.completed[key];
      return { then: (resolve: any) => resolve(value) };
    }

    // Child job mode: execute matching step directly
    if (this._executingKey === key) {
      return {
        then: (resolve: any) => resolve(null),
        _execute_directly: true,
      } as any;
    }

    // Child job mode: non-matching steps never resolve
    if (this._executingKey !== null) {
      return { then: () => new Promise(() => {}) };
    }

    this.pending.push({ name, script, args, key });
    return {
      then: (): never => {
        if (this._suspended) return new Promise(() => {}) as never;
        this._suspended = true;
        const steps = [...this.pending];
        this.pending = [];
        throw new StepSuspend({
          mode: steps.length > 1 ? "parallel" : "sequential",
          steps,
        });
      },
    };
  }

  _flushPending(): Array<{
    name: string;
    script: string;
    args: Record<string, any>;
    key: string;
  }> {
    const steps = [...this.pending];
    this.pending = [];
    return steps;
  }

  async _runInlineStep<T>(
    name: string,
    fn: () => T | Promise<T>
  ): Promise<T> {
    const key = this._allocKey();

    if (key in this.completed) {
      return this.completed[key] as T;
    }

    if (this._executingKey !== null) {
      return new Promise(() => {});
    }

    const result = await fn();
    throw new StepSuspend({
      mode: "inline_checkpoint",
      steps: [],
      key,
      result,
    });
  }
}

function getParamNames(fn: Function): string[] {
  const src = fn.toString();
  const match = src.match(/^(?:async\s+)?(?:function\s*\w*)?\s*\(([^)]*)\)/);
  if (!match) return [];
  return match[1]
    .split(",")
    .map((p) => p.trim().replace(/[:=].*/s, "").trim())
    .filter(Boolean);
}

function task<T extends (...args: any[]) => Promise<any>>(
  fnOrPath: T | string,
  maybeFn?: T
): T {
  let fn: T;
  let taskPath: string | undefined;

  if (typeof fnOrPath === "string") {
    taskPath = fnOrPath;
    fn = maybeFn!;
  } else {
    fn = fnOrPath;
  }

  const taskName = fn.name || "anonymous";

  // Non-async wrapper — returns thenable directly in workflow context so
  // unawaited calls leave steps in pending for _flushPending.
  const wrapper = function (...args: any[]) {
    const ctx = _workflowCtx;
    if (ctx) {
      const script = taskPath ?? taskName;
      const paramNames = getParamNames(fn);
      const kwargs: Record<string, any> = {};
      for (let i = 0; i < args.length; i++) {
        if (paramNames[i]) {
          kwargs[paramNames[i]] = args[i];
        } else {
          kwargs[`arg${i}`] = args[i];
        }
      }
      const stepResult = ctx._nextStep(taskName, script, kwargs);
      if ((stepResult as any)?._execute_directly) {
        return (async () => {
          const result = await fn(...args);
          throw new StepSuspend({
            mode: "step_complete",
            steps: [],
            result,
          });
        })();
      }
      return stepResult;
    } else {
      return fn(...args);
    }
  } as unknown as T;

  Object.defineProperty(wrapper, "name", { value: taskName });
  (wrapper as any)._is_task = true;
  (wrapper as any)._task_path = taskPath;
  return wrapper;
}

async function step<T>(
  name: string,
  fn: () => T | Promise<T>
): Promise<T> {
  const ctx = _workflowCtx;
  if (ctx) {
    return ctx._runInlineStep(name, fn);
  }
  return fn();
}

function workflow<T>(fn: (...args: any[]) => Promise<T>) {
  (fn as any)._is_workflow = true;
  return fn;
}

// --- Helper to run a workflow with a checkpoint ---

async function runWorkflow(
  fn: Function,
  checkpoint: Record<string, any>,
  args: any[]
): Promise<any> {
  const ctx = new WorkflowCtx(checkpoint);
  _workflowCtx = ctx;
  try {
    const result = await fn(...args);
    // Flush unawaited tasks
    const pending = ctx._flushPending();
    if (pending.length > 0) {
      return {
        type: "dispatch",
        mode: pending.length > 1 ? "parallel" : "sequential",
        steps: pending,
      };
    }
    return { type: "complete", result };
  } catch (e: any) {
    if (e instanceof StepSuspend) {
      const info = e.dispatchInfo;
      if (info.mode === "step_complete") {
        return { type: "complete", result: info.result };
      }
      if (info.mode === "inline_checkpoint") {
        return {
          type: "inline_checkpoint",
          key: info.key,
          result: info.result,
        };
      }
      return { type: "dispatch", ...info };
    }
    throw e;
  } finally {
    _workflowCtx = null;
  }
}

// --- Define tasks ---

const extract_data = task(async function extract_data(url: string) {});
const load_data = task(async function load_data(data?: any) {});
const clean_data = task(async function clean_data(data?: any) {});
const compute_stats = task(async function compute_stats(data?: any) {});
const send_alert = task(async function send_alert(msg: string) {});
const double = task(async function double(x: number) {
  return x * 2;
});
const add_one = task(async function add_one(x: number) {
  return x + 1;
});
const noop_task = task(async function noop_task() {});

// --- Define workflows ---

const simple_workflow = workflow(async (url: string) => {
  const raw = await extract_data(url);
  const result = await load_data(raw);
  return { status: "done", result };
});

const parallel_workflow = workflow(async (url: string) => {
  const raw = await extract_data(url);
  const [cleaned, stats] = await Promise.all([
    clean_data(raw),
    compute_stats(raw),
  ]);
  return { cleaned, stats };
});

const conditional_workflow = workflow(async (count: number) => {
  if (count > 100) {
    await send_alert("large");
  }
  await load_data();
  return { done: true };
});

// =====================================================================
// TESTS
// =====================================================================

describe("task decorator", () => {
  test("marks function as task", () => {
    expect((extract_data as any)._is_task).toBe(true);
  });

  test("standalone execution runs body directly", async () => {
    const result = await extract_data("https://example.com");
    expect(result).toBeUndefined();
  });

  test("preserves function name", () => {
    expect(extract_data.name).toBe("extract_data");
    expect(double.name).toBe("double");
  });
});

describe("workflow decorator", () => {
  test("marks function as workflow", () => {
    expect((simple_workflow as any)._is_workflow).toBe(true);
  });
});

describe("first invocation", () => {
  test("dispatches first step", async () => {
    const result = await runWorkflow(simple_workflow, {}, [
      "https://example.com",
    ]);
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("sequential");
    expect(result.steps).toHaveLength(1);
    expect(result.steps[0].name).toBe("extract_data");
    expect(result.steps[0].script).toBe("extract_data");
    expect(result.steps[0].key).toBe("step_0");
    expect(result.steps[0].args).toEqual({ url: "https://example.com" });
  });
});

describe("replay with checkpoint", () => {
  test("second invocation dispatches second step", async () => {
    const checkpoint = {
      completed_steps: { step_0: [1, 2, 3] },
    };
    const result = await runWorkflow(simple_workflow, checkpoint, [
      "https://example.com",
    ]);
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("sequential");
    expect(result.steps[0].name).toBe("load_data");
    expect(result.steps[0].key).toBe("step_1");
  });

  test("all steps complete returns result", async () => {
    const checkpoint = {
      completed_steps: {
        step_0: [1, 2, 3],
        step_1: { loaded: true },
      },
    };
    const result = await runWorkflow(simple_workflow, checkpoint, [
      "https://example.com",
    ]);
    expect(result.type).toBe("complete");
    expect(result.result.status).toBe("done");
    expect(result.result.result).toEqual({ loaded: true });
  });
});

describe("parallel dispatch", () => {
  test("first invocation dispatches extract", async () => {
    const result = await runWorkflow(parallel_workflow, {}, [
      "https://example.com",
    ]);
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].name).toBe("extract_data");
  });

  test("dispatches parallel steps after extract completes", async () => {
    const checkpoint = {
      completed_steps: { step_0: { raw: "data" } },
    };
    const result = await runWorkflow(parallel_workflow, checkpoint, [
      "https://example.com",
    ]);
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("parallel");
    expect(result.steps).toHaveLength(2);
    expect(result.steps[0].name).toBe("clean_data");
    expect(result.steps[1].name).toBe("compute_stats");
  });

  test("completes when all parallel steps done", async () => {
    const checkpoint = {
      completed_steps: {
        step_0: { raw: "data" },
        step_1: { cleaned: true },
        step_2: { count: 42 },
      },
    };
    const result = await runWorkflow(parallel_workflow, checkpoint, [
      "https://example.com",
    ]);
    expect(result.type).toBe("complete");
    expect(result.result.cleaned).toEqual({ cleaned: true });
    expect(result.result.stats).toEqual({ count: 42 });
  });
});

describe("conditional workflow", () => {
  test("condition true dispatches send_alert", async () => {
    const result = await runWorkflow(conditional_workflow, {}, [200]);
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].name).toBe("send_alert");
  });

  test("condition false skips to load_data", async () => {
    const result = await runWorkflow(conditional_workflow, {}, [50]);
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].name).toBe("load_data");
  });
});

describe("task with external path", () => {
  const run_external = task(
    "f/external_script",
    async function run_external(x: number) {}
  );

  test("uses external path as script", async () => {
    const wf = workflow(async (x: number) => {
      const result = await run_external(x);
      return result;
    });
    const result = await runWorkflow(wf, {}, [42]);
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].name).toBe("run_external");
    expect(result.steps[0].script).toBe("f/external_script");
    expect(result.steps[0].args).toEqual({ x: 42 });
  });
});

// =====================================================================
// EDGE CASE TESTS
// =====================================================================

describe("full sequential lifecycle (3 steps)", () => {
  const three_step_wf = workflow(async (n: number) => {
    const doubled = await double(n);
    const incremented = await add_one(doubled);
    const final_val = await double(incremented);
    return { doubled, incremented, final: final_val };
  });

  test("replay 0: dispatches step_0", async () => {
    const result = await runWorkflow(three_step_wf, {}, [5]);
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].key).toBe("step_0");
    expect(result.steps[0].name).toBe("double");
    expect(result.steps[0].args).toEqual({ x: 5 });
  });

  test("replay 1: dispatches step_1 with step_0 result as arg", async () => {
    const result = await runWorkflow(
      three_step_wf,
      { completed_steps: { step_0: 10 } },
      [5]
    );
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].key).toBe("step_1");
    expect(result.steps[0].name).toBe("add_one");
    expect(result.steps[0].args).toEqual({ x: 10 });
  });

  test("replay 2: dispatches step_2 with step_1 result as arg", async () => {
    const result = await runWorkflow(
      three_step_wf,
      { completed_steps: { step_0: 10, step_1: 11 } },
      [5]
    );
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].key).toBe("step_2");
    expect(result.steps[0].name).toBe("double");
    expect(result.steps[0].args).toEqual({ x: 11 });
  });

  test("replay 3: all complete, returns final result", async () => {
    const result = await runWorkflow(
      three_step_wf,
      { completed_steps: { step_0: 10, step_1: 11, step_2: 22 } },
      [5]
    );
    expect(result.type).toBe("complete");
    expect(result.result).toEqual({ doubled: 10, incremented: 11, final: 22 });
  });
});

describe("step after parallel group", () => {
  const seq_par_seq_wf = workflow(async (url: string) => {
    const raw = await extract_data(url);
    const [cleaned, stats] = await Promise.all([
      clean_data(raw),
      compute_stats(raw),
    ]);
    const loaded = await load_data({ cleaned, stats });
    return loaded;
  });

  test("dispatches first sequential step", async () => {
    const result = await runWorkflow(seq_par_seq_wf, {}, ["http://x"]);
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].name).toBe("extract_data");
  });

  test("dispatches parallel group", async () => {
    const result = await runWorkflow(
      seq_par_seq_wf,
      { completed_steps: { step_0: "raw" } },
      ["http://x"]
    );
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("parallel");
    expect(result.steps).toHaveLength(2);
  });

  test("dispatches final step after parallel completes", async () => {
    const result = await runWorkflow(
      seq_par_seq_wf,
      {
        completed_steps: {
          step_0: "raw",
          step_1: "cleaned",
          step_2: { count: 5 },
        },
      },
      ["http://x"]
    );
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("sequential");
    expect(result.steps[0].name).toBe("load_data");
    expect(result.steps[0].key).toBe("step_3");
  });

  test("completes when final step done", async () => {
    const result = await runWorkflow(
      seq_par_seq_wf,
      {
        completed_steps: {
          step_0: "raw",
          step_1: "cleaned",
          step_2: { count: 5 },
          step_3: "final",
        },
      },
      ["http://x"]
    );
    expect(result.type).toBe("complete");
    expect(result.result).toBe("final");
  });
});

describe("parallel after parallel (back to back)", () => {
  const double_parallel_wf = workflow(async () => {
    const [a, b] = await Promise.all([double(1), double(2)]);
    const [c, d] = await Promise.all([add_one(a), add_one(b)]);
    return { a, b, c, d };
  });

  test("dispatches first parallel group", async () => {
    const result = await runWorkflow(double_parallel_wf, {}, []);
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("parallel");
    expect(result.steps).toHaveLength(2);
    expect(result.steps[0].name).toBe("double");
    expect(result.steps[1].name).toBe("double");
    expect(result.steps[0].key).toBe("step_0");
    expect(result.steps[1].key).toBe("step_1");
  });

  test("dispatches second parallel group after first completes", async () => {
    const result = await runWorkflow(
      double_parallel_wf,
      { completed_steps: { step_0: 2, step_1: 4 } },
      []
    );
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("parallel");
    expect(result.steps).toHaveLength(2);
    expect(result.steps[0].name).toBe("add_one");
    expect(result.steps[1].name).toBe("add_one");
    expect(result.steps[0].args).toEqual({ x: 2 });
    expect(result.steps[1].args).toEqual({ x: 4 });
  });

  test("completes when all done", async () => {
    const result = await runWorkflow(
      double_parallel_wf,
      { completed_steps: { step_0: 2, step_1: 4, step_2: 3, step_3: 5 } },
      []
    );
    expect(result.type).toBe("complete");
    expect(result.result).toEqual({ a: 2, b: 4, c: 3, d: 5 });
  });
});

describe("conditional based on step result", () => {
  const cond_on_result = workflow(async () => {
    const val = await double(5);
    if (val > 8) {
      await send_alert("big");
    }
    await load_data(val);
    return { val };
  });

  test("condition true path (val=10 > 8)", async () => {
    const result = await runWorkflow(
      cond_on_result,
      { completed_steps: { step_0: 10 } },
      []
    );
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].name).toBe("send_alert");
    expect(result.steps[0].key).toBe("step_1");
  });

  test("condition false path (val=4 <= 8)", async () => {
    const result = await runWorkflow(
      cond_on_result,
      { completed_steps: { step_0: 4 } },
      []
    );
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].name).toBe("load_data");
    // When condition is false, send_alert is skipped so step index for
    // load_data is step_1 (not step_2)
    expect(result.steps[0].key).toBe("step_1");
  });

  test("condition true: step after alert has key step_2", async () => {
    const result = await runWorkflow(
      cond_on_result,
      { completed_steps: { step_0: 10, step_1: "alerted" } },
      []
    );
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].name).toBe("load_data");
    expect(result.steps[0].key).toBe("step_2");
  });
});

describe("empty workflow (no tasks)", () => {
  const empty_wf = workflow(async () => {
    return { status: "empty" };
  });

  test("completes immediately with no dispatch", async () => {
    const result = await runWorkflow(empty_wf, {}, []);
    expect(result.type).toBe("complete");
    expect(result.result).toEqual({ status: "empty" });
  });
});

describe("single task workflow", () => {
  const single_wf = workflow(async (x: number) => {
    const result = await double(x);
    return result;
  });

  test("dispatches single step", async () => {
    const result = await runWorkflow(single_wf, {}, [7]);
    expect(result.type).toBe("dispatch");
    expect(result.steps).toHaveLength(1);
    expect(result.steps[0].name).toBe("double");
  });

  test("completes with single result", async () => {
    const result = await runWorkflow(
      single_wf,
      { completed_steps: { step_0: 14 } },
      [7]
    );
    expect(result.type).toBe("complete");
    expect(result.result).toBe(14);
  });
});

describe("task with no arguments", () => {
  const no_arg_wf = workflow(async () => {
    const result = await noop_task();
    return result;
  });

  test("dispatches with empty args", async () => {
    const result = await runWorkflow(no_arg_wf, {}, []);
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].args).toEqual({});
  });
});

describe("many steps (10+)", () => {
  const many_steps_wf = workflow(async (n: number) => {
    let val = n;
    for (let i = 0; i < 10; i++) {
      val = await add_one(val);
    }
    return val;
  });

  test("first invocation dispatches step_0", async () => {
    const result = await runWorkflow(many_steps_wf, {}, [0]);
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].key).toBe("step_0");
  });

  test("with 5 steps complete, dispatches step_5", async () => {
    const completed: Record<string, any> = {};
    for (let i = 0; i < 5; i++) completed[`step_${i}`] = i + 1;
    const result = await runWorkflow(
      many_steps_wf,
      { completed_steps: completed },
      [0]
    );
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].key).toBe("step_5");
    expect(result.steps[0].args).toEqual({ x: 5 });
  });

  test("all 10 steps complete returns final value", async () => {
    const completed: Record<string, any> = {};
    for (let i = 0; i < 10; i++) completed[`step_${i}`] = i + 1;
    const result = await runWorkflow(
      many_steps_wf,
      { completed_steps: completed },
      [0]
    );
    expect(result.type).toBe("complete");
    expect(result.result).toBe(10);
  });
});

describe("falsy values preserved in checkpoint", () => {
  const falsy_wf = workflow(async () => {
    const a = await double(0); // result will be 0
    const b = await load_data(a); // result will be null
    const c = await extract_data(""); // result will be ""
    return { a, b, c };
  });

  test("zero is preserved", async () => {
    const result = await runWorkflow(
      falsy_wf,
      { completed_steps: { step_0: 0 } },
      []
    );
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].name).toBe("load_data");
    expect(result.steps[0].args).toEqual({ data: 0 });
  });

  test("null is preserved", async () => {
    const result = await runWorkflow(
      falsy_wf,
      { completed_steps: { step_0: 0, step_1: null } },
      []
    );
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].name).toBe("extract_data");
  });

  test("all falsy values complete correctly", async () => {
    const result = await runWorkflow(
      falsy_wf,
      { completed_steps: { step_0: 0, step_1: null, step_2: "" } },
      []
    );
    expect(result.type).toBe("complete");
    expect(result.result).toEqual({ a: 0, b: null, c: "" });
  });

  test("false is preserved", async () => {
    const flag_wf = workflow(async () => {
      const val = await load_data("check");
      if (val) {
        await send_alert("truthy");
      }
      return { val };
    });
    // false should be treated as completed (key exists), not as missing
    const result = await runWorkflow(
      flag_wf,
      { completed_steps: { step_0: false } },
      []
    );
    expect(result.type).toBe("complete");
    expect(result.result).toEqual({ val: false });
  });
});

describe("inline step (step function)", () => {
  const step_wf = workflow(async (x: number) => {
    const ts = await step("timestamp", () => 1234567890);
    const doubled = await double(x);
    const rid = await step("random_id", () => "abc-123");
    return { ts, doubled, id: rid };
  });

  test("first invocation returns inline_checkpoint", async () => {
    const result = await runWorkflow(step_wf, {}, [7]);
    expect(result.type).toBe("inline_checkpoint");
    expect(result.key).toBe("step_0");
    expect(result.result).toBe(1234567890);
  });

  test("step cached, dispatches task", async () => {
    const result = await runWorkflow(
      step_wf,
      { completed_steps: { step_0: 1234567890 } },
      [7]
    );
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].name).toBe("double");
    expect(result.steps[0].key).toBe("step_1");
  });

  test("step + task cached, returns second inline step", async () => {
    const result = await runWorkflow(
      step_wf,
      { completed_steps: { step_0: 1234567890, step_1: 14 } },
      [7]
    );
    expect(result.type).toBe("inline_checkpoint");
    expect(result.key).toBe("step_2");
    expect(result.result).toBe("abc-123");
  });

  test("all complete returns final result", async () => {
    const result = await runWorkflow(
      step_wf,
      {
        completed_steps: {
          step_0: 1234567890,
          step_1: 14,
          step_2: "abc-123",
        },
      },
      [7]
    );
    expect(result.type).toBe("complete");
    expect(result.result).toEqual({
      ts: 1234567890,
      doubled: 14,
      id: "abc-123",
    });
  });
});

describe("unawaited tasks (flush pending)", () => {
  test("single unawaited task at end is flushed", async () => {
    const wf = workflow(async () => {
      await extract_data("x");
      load_data("y"); // forgotten await
    });
    const result = await runWorkflow(
      wf,
      { completed_steps: { step_0: "raw" } },
      []
    );
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("sequential");
    expect(result.steps).toHaveLength(1);
    expect(result.steps[0].name).toBe("load_data");
  });

  test("multiple unawaited tasks flushed as parallel", async () => {
    const wf = workflow(async () => {
      await extract_data("x");
      clean_data("y"); // forgotten await
      compute_stats("y"); // forgotten await
    });
    const result = await runWorkflow(
      wf,
      { completed_steps: { step_0: "raw" } },
      []
    );
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("parallel");
    expect(result.steps).toHaveLength(2);
    expect(result.steps[0].name).toBe("clean_data");
    expect(result.steps[1].name).toBe("compute_stats");
  });

  test("no unawaited tasks means normal complete", async () => {
    const wf = workflow(async () => {
      const val = await double(5);
      return val;
    });
    const result = await runWorkflow(
      wf,
      { completed_steps: { step_0: 10 } },
      []
    );
    expect(result.type).toBe("complete");
    expect(result.result).toBe(10);
  });
});

describe("child mode (_executingKey)", () => {
  test("executes matching task directly", async () => {
    const wf = workflow(async (x: number) => {
      const val = await double(x);
      return val;
    });
    const result = await runWorkflow(
      wf,
      { completed_steps: {}, _executing_key: "step_0" },
      [7]
    );
    expect(result.type).toBe("complete");
    expect(result.result).toBe(14); // double(7) = 14
  });

  test("replays cached steps before executing key", async () => {
    const wf = workflow(async (x: number) => {
      const doubled = await double(x);
      const result = await add_one(doubled);
      return result;
    });
    const result = await runWorkflow(
      wf,
      {
        completed_steps: { step_0: 10 },
        _executing_key: "step_1",
      },
      [5]
    );
    expect(result.type).toBe("complete");
    expect(result.result).toBe(11); // add_one(10) = 11
  });

  test("child mode with external path task", async () => {
    const ext = task(
      "f/external",
      async function ext_task(x: number) {
        return x * 3;
      }
    );
    const wf = workflow(async (x: number) => {
      const result = await ext(x);
      return result;
    });
    const result = await runWorkflow(
      wf,
      { completed_steps: {}, _executing_key: "step_0" },
      [4]
    );
    expect(result.type).toBe("complete");
    expect(result.result).toBe(12); // 4 * 3
  });
});

describe("key determinism across replays", () => {
  const det_wf = workflow(async (n: number) => {
    const a = await double(n);
    const b = await add_one(a);
    const c = await double(b);
    return c;
  });

  test("keys are consistent: step_0 always maps to first double", async () => {
    // Empty checkpoint
    const r1 = await runWorkflow(det_wf, {}, [3]);
    expect(r1.steps[0].key).toBe("step_0");
    expect(r1.steps[0].name).toBe("double");

    // With step_0 completed
    const r2 = await runWorkflow(
      det_wf,
      { completed_steps: { step_0: 6 } },
      [3]
    );
    expect(r2.steps[0].key).toBe("step_1");
    expect(r2.steps[0].name).toBe("add_one");

    // With step_0 and step_1 completed
    const r3 = await runWorkflow(
      det_wf,
      { completed_steps: { step_0: 6, step_1: 7 } },
      [3]
    );
    expect(r3.steps[0].key).toBe("step_2");
    expect(r3.steps[0].name).toBe("double");
  });
});

describe("parallel dispatch includes correct args from cached results", () => {
  test("parallel steps receive cached parent result as args", async () => {
    const wf = workflow(async (x: number) => {
      const base = await double(x);
      const [a, b] = await Promise.all([add_one(base), double(base)]);
      return { a, b };
    });
    const result = await runWorkflow(
      wf,
      { completed_steps: { step_0: 20 } },
      [10]
    );
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("parallel");
    expect(result.steps[0].args).toEqual({ x: 20 });
    expect(result.steps[1].args).toEqual({ x: 20 });
  });
});

describe("inline step with async function", () => {
  test("async step function resolves correctly", async () => {
    const wf = workflow(async () => {
      const val = await step("async_step", async () => {
        return 42;
      });
      return val;
    });
    const result = await runWorkflow(wf, {}, []);
    expect(result.type).toBe("inline_checkpoint");
    expect(result.key).toBe("step_0");
    expect(result.result).toBe(42);
  });
});

describe("workflow returning undefined", () => {
  test("undefined return value is captured", async () => {
    const wf = workflow(async () => {
      await double(1);
    });
    const result = await runWorkflow(
      wf,
      { completed_steps: { step_0: 2 } },
      []
    );
    expect(result.type).toBe("complete");
    expect(result.result).toBeUndefined();
  });
});

describe("large parallel group", () => {
  test("dispatches 5 parallel steps at once", async () => {
    const wf = workflow(async () => {
      const results = await Promise.all([
        double(1),
        double(2),
        double(3),
        double(4),
        double(5),
      ]);
      return results;
    });
    const result = await runWorkflow(wf, {}, []);
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("parallel");
    expect(result.steps).toHaveLength(5);
    for (let i = 0; i < 5; i++) {
      expect(result.steps[i].key).toBe(`step_${i}`);
      expect(result.steps[i].args).toEqual({ x: i + 1 });
    }
  });
});

describe("complex mixed workflow: seq → par → seq → par → seq", () => {
  const complex_wf = workflow(async () => {
    const init = await extract_data("start");
    const [a, b] = await Promise.all([double(1), double(2)]);
    const mid = await load_data({ a, b });
    const [c, d] = await Promise.all([add_one(3), add_one(4)]);
    const fin = await clean_data({ mid, c, d });
    return fin;
  });

  test("replay 0: dispatches extract_data", async () => {
    const r = await runWorkflow(complex_wf, {}, []);
    expect(r.steps[0].name).toBe("extract_data");
  });

  test("replay 1: dispatches parallel [double, double]", async () => {
    const r = await runWorkflow(
      complex_wf,
      { completed_steps: { step_0: "init" } },
      []
    );
    expect(r.mode).toBe("parallel");
    expect(r.steps).toHaveLength(2);
    expect(r.steps[0].name).toBe("double");
  });

  test("replay 2: dispatches load_data", async () => {
    const r = await runWorkflow(
      complex_wf,
      { completed_steps: { step_0: "init", step_1: 2, step_2: 4 } },
      []
    );
    expect(r.mode).toBe("sequential");
    expect(r.steps[0].name).toBe("load_data");
    expect(r.steps[0].key).toBe("step_3");
  });

  test("replay 3: dispatches parallel [add_one, add_one]", async () => {
    const r = await runWorkflow(
      complex_wf,
      {
        completed_steps: {
          step_0: "init",
          step_1: 2,
          step_2: 4,
          step_3: "mid",
        },
      },
      []
    );
    expect(r.mode).toBe("parallel");
    expect(r.steps).toHaveLength(2);
    expect(r.steps[0].name).toBe("add_one");
  });

  test("replay 4: dispatches clean_data", async () => {
    const r = await runWorkflow(
      complex_wf,
      {
        completed_steps: {
          step_0: "init",
          step_1: 2,
          step_2: 4,
          step_3: "mid",
          step_4: 4,
          step_5: 5,
        },
      },
      []
    );
    expect(r.mode).toBe("sequential");
    expect(r.steps[0].name).toBe("clean_data");
    expect(r.steps[0].key).toBe("step_6");
  });

  test("replay 5: all complete", async () => {
    const r = await runWorkflow(
      complex_wf,
      {
        completed_steps: {
          step_0: "init",
          step_1: 2,
          step_2: 4,
          step_3: "mid",
          step_4: 4,
          step_5: 5,
          step_6: "final",
        },
      },
      []
    );
    expect(r.type).toBe("complete");
    expect(r.result).toBe("final");
  });
});
