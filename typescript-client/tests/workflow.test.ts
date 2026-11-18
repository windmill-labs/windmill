/**
 * Standalone tests for the Workflow-as-Code TypeScript SDK.
 *
 * Run with: bun test typescript-client/tests/workflow.test.ts
 */
import { expect, test, describe } from "bun:test";

// From the shipped module, not the mirror below: these decide what a caught
// failure looks like and what is the SDK's own control flow, so a copy here
// would guard nothing.
import { isSuspendSignal, stepErrorMarker, taskErrorFromMarker } from "../wacError";

// The cases both SDKs must agree on, from the corpus the python suite also
// reads. Its `_readme` states the contract and why it is shared.
import corpus from "../../backend/windmill-common/src/wac_failure_corpus.json";

describe("shared failure-record corpus", () => {
  const construct = (spec: any): any => {
    const e: any = Object.assign(new Error(spec.message), { name: spec.name });
    for (const [k, v] of Object.entries(spec.props ?? {})) e[k] = v;
    if (spec.circular_prop) {
      const cyclic: any = {};
      cyclic.self = cyclic;
      e[spec.circular_prop] = cyclic;
    }
    return e;
  };

  for (const c of (corpus as any).cases) {
    test(c.case, () => {
      const error = stepErrorMarker("k", construct(c.thrown)).result.error;
      expect(error.name).toBe(c.expect.name);
      expect(error.message).toBe(c.expect.message);
      expect("stack" in error).toBe(c.expect.stack === "present");
      if (c.expect.extra) expect(error.extra).toEqual(c.expect.extra);
      for (const absent of c.expect.absent ?? []) expect(absent in error).toBe(false);
      // whatever it kept has to survive the trip to the checkpoint
      expect(() => JSON.stringify(error)).not.toThrow();
    });
  }
});

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
  private _pendingSuspend: StepSuspend | null = null;
  private _pendingStepFailure: { error: unknown } | null = null;
  _executingKey: string | null;

  _raiseSuspend(dispatchInfo: Record<string, any>): never {
    const suspend = new StepSuspend(dispatchInfo);
    this._pendingSuspend = suspend;
    throw suspend;
  }

  _raiseStepFailure(error: unknown): never {
    this._pendingStepFailure = { error };
    throw error;
  }

  private _rethrowSwallowed(): void {
    if (this._pendingStepFailure) throw this._pendingStepFailure.error;
    if (this._pendingSuspend) throw this._pendingSuspend;
  }

  _takePendingSuspend(): StepSuspend | null {
    const s = this._pendingSuspend;
    this._pendingSuspend = null;
    return s;
  }

  _takePendingStepFailure(): { error: unknown } | null {
    const f = this._pendingStepFailure;
    this._pendingStepFailure = null;
    return f;
  }

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
    args: Record<string, any> = {},
    options?: Record<string, any>,
  ): PromiseLike<any> {
    this._rethrowSwallowed();
    const key = this._allocKey();

    if (key in this.completed) {
      const value = this.completed[key];
      if (value && typeof value === "object" && (value as any).__wmill_error) {
        const err = taskErrorFromMarker(value, `Task '${name}' failed`);
        return { then: (_resolve: any, reject?: any) => { if (reject) reject(err); else throw err; } };
      }
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

    const stepInfo: any = { name, script, args, key };
    if (options) Object.assign(stepInfo, options);
    this.pending.push(stepInfo);
    return {
      then: (): never => {
        if (this._suspended) return new Promise(() => {}) as never;
        this._suspended = true;
        const steps = [...this.pending];
        this.pending = [];
        this._raiseSuspend({
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

  _sleep(seconds: number): PromiseLike<void> {
    this._rethrowSwallowed();
    const key = this._allocKey();
    if (key in this.completed) {
      return { then: (resolve: any) => resolve(undefined) };
    }
    if (this._executingKey !== null) {
      return { then: () => new Promise(() => {}) };
    }
    this._raiseSuspend({
      mode: "sleep",
      key,
      seconds: Math.max(1, Math.round(seconds)),
      steps: [],
    });
  }

  async _runInlineStep<T>(
    name: string,
    fn: () => T | Promise<T>
  ): Promise<T> {
    this._rethrowSwallowed();
    const key = this._allocKey();

    if (key in this.completed) {
      const value = this.completed[key];
      if (value && typeof value === "object" && (value as any).__wmill_error) {
        throw taskErrorFromMarker(value, `Step '${name}' failed`);
      }
      return value as T;
    }

    if (this._executingKey !== null) {
      return new Promise(() => {});
    }

    let result: any;
    try {
      result = await fn();
    } catch (e: any) {
      if (isSuspendSignal(e, StepSuspend)) throw e;
      result = stepErrorMarker(key, e);
    }
    this._raiseSuspend({
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
  maybeFnOrOptions?: T | Record<string, any>,
  maybeOptions?: Record<string, any>,
): T {
  let fn: T;
  let taskPath: string | undefined;
  let taskOptions: Record<string, any> | undefined;

  if (typeof fnOrPath === "string") {
    taskPath = fnOrPath;
    fn = maybeFnOrOptions as T;
    taskOptions = maybeOptions;
  } else {
    fn = fnOrPath;
    taskOptions = maybeFnOrOptions as Record<string, any> | undefined;
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
      const stepResult = ctx._nextStep(taskName, script, kwargs, taskOptions);
      if ((stepResult as any)?._execute_directly) {
        return (async () => {
          let result: any;
          try {
            result = await fn(...args);
          } catch (e: any) {
            if (isSuspendSignal(e, StepSuspend)) throw e;
            ctx._raiseStepFailure(e);
          }
          ctx._raiseSuspend({
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

async function sleep(seconds: number): Promise<void> {
  const ctx = _workflowCtx;
  if (ctx) {
    return ctx._sleep(seconds) as Promise<void>;
  }
  await new Promise((r) => setTimeout(r, seconds * 1000));
}

async function parallel<T, R>(
  items: T[],
  fn: (item: T) => PromiseLike<R> | R,
  options?: { concurrency?: number },
): Promise<R[]> {
  const concurrency = options?.concurrency ?? items.length;
  if (concurrency <= 0 || items.length === 0) return [];
  const results: R[] = [];
  for (let i = 0; i < items.length; i += concurrency) {
    const batch = items.slice(i, i + concurrency);
    const batchResults = await Promise.all(batch.map((item) => fn(item)));
    results.push(...batchResults);
  }
  return results;
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
    // Mirrors bun_executor.rs: honour a step failure or suspend the body caught
    // and swallowed.
    const failed = ctx._takePendingStepFailure?.();
    if (failed) throw failed.error;
    const swallowed = ctx._takePendingSuspend?.();
    if (swallowed) throw swallowed;
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
      if (info.mode === "approval") {
        return { type: "approval", key: info.key, timeout: info.timeout, form: info.form };
      }
      if (info.mode === "sleep") {
        return { type: "sleep", key: info.key, seconds: info.seconds };
      }
      return { type: "dispatch", ...info };
    }
    const failed = ctx._takePendingStepFailure?.();
    if (failed) throw failed.error;
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

// =====================================================================
// ERROR PROPAGATION TESTS
// =====================================================================

describe("error propagation via __wmill_error marker", () => {
  test("task error is thrown on replay", async () => {
    const wf = workflow(async (x: number) => {
      const result = await double(x);
      return result;
    });
    // Simulate child failure stored as __wmill_error marker
    const checkpoint = {
      completed_steps: {
        step_0: {
          __wmill_error: true,
          message: "WAC task 'double' failed (child job abc-123)",
          result: { message: "division by zero" },
          step_key: "double",
          child_job_id: "abc-123",
        },
      },
    };
    try {
      await runWorkflow(wf, checkpoint, [5]);
      expect(true).toBe(false); // should not reach here
    } catch (e: any) {
      expect(e.name).toBe("TaskError");
      expect(e.message).toContain("double");
      expect(e.result).toEqual({ message: "division by zero" });
      expect(e.child_job_id).toBe("abc-123");
    }
  });

  test("error is catchable with try/catch in workflow", async () => {
    const wf = workflow(async (x: number) => {
      try {
        const result = await double(x);
        return { success: true, result };
      } catch (e: any) {
        return { success: false, error: e.message };
      }
    });
    const checkpoint = {
      completed_steps: {
        step_0: {
          __wmill_error: true,
          message: "Task 'double' failed",
          result: { message: "boom" },
        },
      },
    };
    const result = await runWorkflow(wf, checkpoint, [5]);
    expect(result.type).toBe("complete");
    expect(result.result.success).toBe(false);
    expect(result.result.error).toContain("double");
  });

  test("error in parallel — one fails, caught by Promise.all reject", async () => {
    const wf = workflow(async () => {
      try {
        const [a, b] = await Promise.all([double(1), add_one(2)]);
        return { a, b };
      } catch (e: any) {
        return { caught: true, error: e.message };
      }
    });
    const checkpoint = {
      completed_steps: {
        step_0: { __wmill_error: true, message: "double failed", result: {} },
        step_1: 3, // add_one succeeded
      },
    };
    const result = await runWorkflow(wf, checkpoint, []);
    expect(result.type).toBe("complete");
    expect(result.result.caught).toBe(true);
  });

  test("retry pattern with try/catch + loop", async () => {
    // Simulates: first attempt fails, second succeeds
    const wf = workflow(async (x: number) => {
      for (let i = 0; i < 3; i++) {
        try {
          const result = await double(x);
          return { result, attempts: i + 1 };
        } catch (e) {
          if (i === 2) throw e;
          // retry on next iteration
        }
      }
    });
    // First double (step_0) fails, second double (step_1) succeeds
    const checkpoint = {
      completed_steps: {
        step_0: { __wmill_error: true, message: "temporary failure", result: {} },
        step_1: 10,
      },
    };
    const result = await runWorkflow(wf, checkpoint, [5]);
    expect(result.type).toBe("complete");
    expect(result.result.result).toBe(10);
    expect(result.result.attempts).toBe(2);
  });

  test("inline step error is thrown", async () => {
    const wf = workflow(async () => {
      try {
        const val = await step("risky", () => 42);
        return { val };
      } catch (e: any) {
        return { caught: e.message };
      }
    });
    const checkpoint = {
      completed_steps: {
        step_0: { __wmill_error: true, message: "inline step failed", result: {} },
      },
    };
    const result = await runWorkflow(wf, checkpoint, []);
    expect(result.type).toBe("complete");
    expect(result.result.caught).toContain("inline step failed");
  });

  test("non-error object with __wmill_error field is NOT treated as error", async () => {
    // An object with __wmill_error: false should be treated as a normal value
    const wf = workflow(async () => {
      const val = await double(5);
      return val;
    });
    const checkpoint = {
      completed_steps: {
        step_0: { __wmill_error: false, data: "not an error" },
      },
    };
    const result = await runWorkflow(wf, checkpoint, [5]);
    expect(result.type).toBe("complete");
    expect(result.result).toEqual({ __wmill_error: false, data: "not an error" });
  });
});

// A step() whose body throws must still land in completed_steps. Otherwise a
// workflow that catches the error and later dispatches a task replays with
// _executingKey set, reaches the unrecorded key, and parks on the
// never-resolving promise forever.
describe("throwing inline step is checkpointed", () => {
  // A failed child job reports `{ error: { name, message, stack } }`, and a
  // failed step() has to be indistinguishable from it. The stack is a
  // JS stack string, asserted separately.
  const marker = {
    __wmill_error: true,
    message: "boom",
    step_key: "step_0",
    result: { error: { name: "TypeError", message: "boom" } },
  };
  const withoutStack = (m: any) => {
    const { stack, ...error } = m.result.error;
    return { ...m, result: { ...m.result, error } };
  };

  // The workflow body catches — the shape a failing step is written for, and
  // the one that makes StepSuspend (an Error) swallowable in TS.
  const catchingWf = () =>
    workflow(async (x: number) => {
      let caught = null;
      try {
        await step("risky", () => {
          throw new TypeError("boom");
        });
      } catch (e: any) {
        caught = `${e.name}: ${e.message}`;
      }
      return { caught, doubled: await double(x) };
    });

  test("a throwing step suspends with an error checkpoint", async () => {
    const ctx = new WorkflowCtx({});
    let caught: any;
    try {
      await ctx._runInlineStep("risky", () => {
        throw new TypeError("boom");
      });
    } catch (e) {
      caught = e;
    }
    expect(caught).toBeInstanceOf(StepSuspend);
    expect(caught.dispatchInfo.mode).toBe("inline_checkpoint");
    expect(caught.dispatchInfo.key).toBe("step_0");
    expect(withoutStack(caught.dispatchInfo.result)).toEqual(marker);
    expect(caught.dispatchInfo.result.result.error.stack).toContain("TypeError: boom");
  });

  test("a swallowed suspend still reaches the runner", async () => {
    // Without _pendingSuspend the catch eats the suspend and the run reports a
    // dispatch (or a complete) with `risky` missing from completed_steps.
    const result = await runWorkflow(catchingWf(), {}, [5]);
    expect(result.type).toBe("inline_checkpoint");
    expect(result.key).toBe("step_0");
    expect(withoutStack(result.result)).toEqual(marker);
  });

  test("a swallowed suspend from a succeeding step still reaches the runner", async () => {
    const wf = workflow(async () => {
      try {
        await step("fine", () => 42);
      } catch {
        // a body that catches broadly must not be able to erase the suspend
      }
      return "never reached on the first run";
    });
    const result = await runWorkflow(wf, {}, []);
    expect(result.type).toBe("inline_checkpoint");
    expect(result.result).toBe(42);
  });

  // The backend records `name: e.name` for a failed child job
  // (bun_executor.rs). A step reporting the constructor name instead would make
  // the same failure read differently depending on whether it ran as a task or
  // as a step, in the one field handlers are told to branch on.
  test("error.name is e.name, as a failed child job reports it", async () => {
    class MyError extends Error {}
    const unnamed = stepErrorMarker("k", new MyError("boom"));
    expect(unnamed.result.error.name).toBe("Error");

    class NamedError extends Error {
      name = "NamedError";
    }
    const named = stepErrorMarker("k", new NamedError("boom"));
    expect(named.result.error.name).toBe("NamedError");

    const domish = Object.assign(new Error("aborted"), { name: "AbortError" });
    expect(stepErrorMarker("k", domish).result.error.name).toBe("AbortError");
  });

  // A failed child job reports custom properties under `error.extra`
  // (bun_executor.rs). A step dropping them would make the same error carry
  // less information depending on how it was run.
  test("custom error properties survive under error.extra, as a task's do", async () => {
    const e = Object.assign(new Error("429"), { code: 429, retryAfter: 5 });
    const marker = stepErrorMarker("k", e);
    expect(marker.result.error.extra).toEqual({ code: 429, retryAfter: 5 });
    // the named fields the executors report separately are not duplicated
    expect(marker.result.error.extra.message).toBeUndefined();
    expect(marker.result.error.extra.stack).toBeUndefined();

    expect(stepErrorMarker("k", new Error("plain")).result.error.extra).toBeUndefined();
  });

  // The marker is stringified while the workflow is still running (checkpoint
  // POST, then wrapper output). A property that can't survive that would end
  // the job instead of reaching the user's catch.
  test("a property that cannot be serialized is dropped, not propagated", () => {
    const circular: any = { name: "req" };
    circular.self = circular;
    const withCircular = Object.assign(new Error("boom"), { code: 429, request: circular });
    const marker = stepErrorMarker("k", withCircular);
    expect(() => JSON.stringify(marker)).not.toThrow();
    expect(marker.result.error.extra).toEqual({ code: 429 });

    const withThrowingAccessor = new Error("boom");
    Object.defineProperty(withThrowingAccessor, "boobytrap", {
      enumerable: true,
      get() {
        throw new Error("read me and die");
      },
    });
    expect(() => stepErrorMarker("k", withThrowingAccessor)).not.toThrow();
  });

  // A task executor reads `name`/`message`/`stack` off whatever was thrown, not
  // off an Error instance, so a step must too or a handler can tell the two
  // apart in the fields the contract tells it to branch on.
  test("a non-Error throw records what a task would record", () => {
    const thrown = stepErrorMarker("k", { name: "Thrown", message: "boom", code: 429 });
    expect(thrown.result.error.name).toBe("Thrown");
    expect(thrown.result.error.message).toBe("boom");
    expect(thrown.result.error.extra).toEqual({ code: 429 });

    // A string carries none of the three, so the record is left for the backend
    // to fill — the same fallback a task throwing a string produces. Its
    // character indices are not custom fields.
    const str = stepErrorMarker("k", "boom");
    expect(str.result.error).toEqual({});

    // `String()` on a value with no `toString` to reach throws in turn, and
    // this runs inside the catch reporting the user's failure.
    expect(() => stepErrorMarker("k", Object.create(null))).not.toThrow();
  });

  // Reporting a failure must not be able to fail: every read of the thrown
  // value happens inside the catch that is reporting it, so an escape replaces
  // the user's error with an unrelated one and leaves the step uncheckpointed.
  test("a hostile thrown value cannot make failure reporting throw", () => {
    const hostile = new Proxy(
      {},
      {
        get() {
          throw new Error("get trap");
        },
        ownKeys() {
          throw new Error("ownKeys trap");
        },
      },
    );
    expect(() => stepErrorMarker("k", hostile)).not.toThrow();
    expect(() => JSON.stringify(stepErrorMarker("k", hostile))).not.toThrow();

    const throwingToString = { toString() { throw new Error("no"); } };
    expect(() => stepErrorMarker("k", throwingToString)).not.toThrow();
  });

  // Probing the original value only proves it serialized once. The marker is
  // serialized again to reach the checkpoint, and by then the failure has
  // nowhere left to go, so what survived the probe is what gets kept.
  test("a property that serializes only once cannot break the checkpoint", () => {
    let calls = 0;
    const onceOnly = {
      toJSON() {
        if (calls++ > 0) throw new Error("second time");
        return { ok: true };
      },
    };
    const marker = stepErrorMarker("k", Object.assign(new Error("boom"), { payload: onceOnly }));
    expect(marker.result.error.extra.payload).toEqual({ ok: true });
    // re-serialized on the way to the checkpoint, and again by the wrapper
    expect(() => JSON.stringify(marker)).not.toThrow();
    expect(() => JSON.stringify(marker)).not.toThrow();
  });

  // The caller reads `.name` off the thrown value to spot a suspend, before it
  // ever reaches the hardened marker. A hostile value escaping there leaves the
  // step uncheckpointed and a later replay parks on it forever.
  test("a hostile throw still reaches the checkpoint through _runInlineStep", async () => {
    const hostile = new Proxy(
      {},
      {
        get() {
          throw new Error("get trap");
        },
        ownKeys() {
          throw new Error("ownKeys trap");
        },
      },
    );
    const ctx = new WorkflowCtx({});
    let caught: any;
    try {
      await ctx._runInlineStep("risky", () => {
        throw hostile;
      });
    } catch (e) {
      caught = e;
    }
    // the suspend carrying the checkpoint, not the hostile value itself
    expect(caught).toBeInstanceOf(StepSuspend);
    expect(caught.dispatchInfo.key).toBe("step_0");
    expect(caught.dispatchInfo.result.__wmill_error).toBe(true);
  });

  // `instanceof` consults a proxy's `getPrototypeOf` trap, so the suspend check
  // itself can throw — before anything is checkpointed.
  test("suspend detection survives a value that refuses to be inspected", () => {
    const hostilePrototype = new Proxy(new StepSuspend({ mode: "sequential" }), {
      getPrototypeOf() {
        throw new Error("getPrototypeOf trap");
      },
    });
    // the name is still readable, so it is still recognised as the signal
    expect(isSuspendSignal(hostilePrototype, StepSuspend)).toBe(true);

    const opaque = new Proxy(
      {},
      {
        getPrototypeOf() {
          throw new Error("getPrototypeOf trap");
        },
        get() {
          throw new Error("get trap");
        },
      },
    );
    expect(() => isSuspendSignal(opaque, StepSuspend)).not.toThrow();
    expect(isSuspendSignal(opaque, StepSuspend)).toBe(false);
  });

  test("a replayed step failure is named TaskError, like the python client", async () => {
    const ctx = new WorkflowCtx({ completed_steps: { step_0: marker } });
    let caught: any;
    try {
      await ctx._runInlineStep("risky", () => 1);
    } catch (e) {
      caught = e;
    }
    expect(`${caught.name}: ${caught.message}`).toBe("TaskError: boom");
    // the failing body's own type stays addressable here, in the shape a
    // failed task hands over too
    expect(caught.result).toEqual({ error: { name: "TypeError", message: "boom" } });
    expect(caught.step_key).toBe("step_0");
    // a step runs in the workflow job, so there is no child job to name
    expect(caught.child_job_id).toBeUndefined();
    // nothing is chained onto `cause`: a replay has no original error to
    // chain, so neither round does
    expect(caught.cause).toBeUndefined();
  });

  test("a child job cannot swallow its own completion signal", async () => {
    // The catch below is reached only if step_complete escapes the parking
    // mechanism; the child would then report the catch branch as the result.
    const wf = workflow(async (x: number) => {
      try {
        await double(x);
      } catch {
        return "swallowed";
      }
      return "unreachable";
    });
    const result = await runWorkflow(wf, { _executing_key: "step_0" }, [5]);
    expect(result.type).toBe("complete");
    expect(result.result).toBe(10);
  });

  test("a child job cannot swallow the failure of the step it executes", async () => {
    // Without parking, the catch below turns the child into a success returning
    // "swallowed" and the parent records that as the step's value.
    const boom = task(async function boom() {
      throw new TypeError("nope");
    });
    const wf = workflow(async () => {
      try {
        await boom();
      } catch {
        return "swallowed";
      }
      return "unreachable";
    });
    await expect(runWorkflow(wf, { _executing_key: "step_0" }, [])).rejects.toThrow("nope");
  });

  test("a parked failure is re-raised at the next SDK call, not left to hang", async () => {
    // A body that catches and carries on reaches an SDK call that, in child mode,
    // never resolves — so without the re-raise the child parks there and hangs
    // until timeout instead of reporting the failure. Raced against a deadline so
    // that regression fails the test rather than wedging the suite.
    const boom = task(async function boom() {
      throw new TypeError("nope");
    });
    for (const carryOn of [() => double(1), () => sleep(1)]) {
      const wf = workflow(async () => {
        try {
          await boom();
        } catch {
          // swallowed on purpose
        }
        await carryOn();
        return "unreachable";
      });
      const run = Promise.race([
        runWorkflow(wf, { _executing_key: "step_0" }, []),
        new Promise((_, reject) => setTimeout(() => reject(new Error("parked")), 500)),
      ]);
      await expect(run).rejects.toThrow("nope");
    }
  });

  test("a swallowed suspend from a task dispatch still reaches the runner", async () => {
    const wf = workflow(async (x: number) => {
      try {
        await double(x);
      } catch {
        // ditto for task steps
      }
      return "never reached on the first run";
    });
    const result = await runWorkflow(wf, {}, [5]);
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].key).toBe("step_0");
  });

  test("replay rethrows the error and does not hang", async () => {
    const result = await runWorkflow(
      catchingWf(),
      { completed_steps: { step_0: marker }, _executing_key: "step_1" },
      [5],
    );
    // The child runs only the dispatched task, so its result is that task's —
    // what matters is that it got there instead of parking on `risky`.
    expect(result.type).toBe("complete");
    expect(result.result).toBe(10);
  });
});

// =====================================================================
// TASK OPTIONS TESTS
// =====================================================================

describe("task options", () => {
  test("options are forwarded in dispatch step info", async () => {
    const heavy = task(
      async function heavy(x: number) { return x; },
      { timeout: 600, tag: "gpu", cache_ttl: 3600, priority: 10 },
    );
    const wf = workflow(async (x: number) => {
      return await heavy(x);
    });
    const result = await runWorkflow(wf, {}, [42]);
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].timeout).toBe(600);
    expect(result.steps[0].tag).toBe("gpu");
    expect(result.steps[0].cache_ttl).toBe(3600);
    expect(result.steps[0].priority).toBe(10);
  });

  test("task without options has no extra fields", async () => {
    const simple = task(async function simple(x: number) { return x; });
    const wf = workflow(async (x: number) => {
      return await simple(x);
    });
    const result = await runWorkflow(wf, {}, [1]);
    expect(result.steps[0].timeout).toBeUndefined();
    expect(result.steps[0].tag).toBeUndefined();
  });

  test("concurrency options forwarded", async () => {
    const limited = task(
      async function limited(x: number) { return x; },
      { concurrent_limit: 5, concurrency_key: "my-key", concurrency_time_window_s: 60 },
    );
    const wf = workflow(async (x: number) => {
      return await limited(x);
    });
    const result = await runWorkflow(wf, {}, [1]);
    expect(result.steps[0].concurrent_limit).toBe(5);
    expect(result.steps[0].concurrency_key).toBe("my-key");
    expect(result.steps[0].concurrency_time_window_s).toBe(60);
  });

  test("task with path and options", async () => {
    const ext = task(
      "f/gpu_script",
      async function ext(x: number) { return x; },
      { timeout: 300, tag: "gpu" },
    );
    const wf = workflow(async (x: number) => {
      return await ext(x);
    });
    const result = await runWorkflow(wf, {}, [1]);
    expect(result.steps[0].script).toBe("f/gpu_script");
    expect(result.steps[0].timeout).toBe(300);
    expect(result.steps[0].tag).toBe("gpu");
  });
});

// =====================================================================
// SLEEP TESTS
// =====================================================================

describe("sleep", () => {
  test("first invocation returns sleep output", async () => {
    const wf = workflow(async () => {
      await double(1);
      await sleep(60);
      await add_one(2);
      return "done";
    });
    // step_0 (double) complete, step_1 is sleep
    const result = await runWorkflow(
      wf,
      { completed_steps: { step_0: 2 } },
      [],
    );
    expect(result.type).toBe("sleep");
    expect(result.key).toBe("step_1");
    expect(result.seconds).toBe(60);
  });

  test("sleep completes on replay when stored in checkpoint", async () => {
    const wf = workflow(async () => {
      await double(1);
      await sleep(60);
      await add_one(2);
      return "done";
    });
    // step_0 (double) and step_1 (sleep) complete
    const result = await runWorkflow(
      wf,
      { completed_steps: { step_0: 2, step_1: true } },
      [],
    );
    expect(result.type).toBe("dispatch");
    expect(result.steps[0].name).toBe("add_one");
    expect(result.steps[0].key).toBe("step_2");
  });

  test("all steps including sleep complete returns result", async () => {
    const wf = workflow(async () => {
      await double(1);
      await sleep(60);
      await add_one(2);
      return "done";
    });
    const result = await runWorkflow(
      wf,
      { completed_steps: { step_0: 2, step_1: true, step_2: 3 } },
      [],
    );
    expect(result.type).toBe("complete");
    expect(result.result).toBe("done");
  });

  test("sleep enforces minimum of 1 second", async () => {
    const wf = workflow(async () => {
      await sleep(0);
      return "done";
    });
    const result = await runWorkflow(wf, {}, []);
    expect(result.type).toBe("sleep");
    expect(result.seconds).toBe(1);
  });

  test("sleep rounds to nearest integer", async () => {
    const wf = workflow(async () => {
      await sleep(3.7);
      return "done";
    });
    const result = await runWorkflow(wf, {}, []);
    expect(result.type).toBe("sleep");
    expect(result.seconds).toBe(4);
  });
});

// =====================================================================
// PARALLEL UTILITY TESTS
// =====================================================================

describe("parallel utility", () => {
  test("processes all items with default concurrency", async () => {
    const wf = workflow(async () => {
      const items = [1, 2, 3];
      const results = await parallel(items, double);
      return results;
    });
    // All 3 items dispatched in parallel: step_0, step_1, step_2
    const result = await runWorkflow(wf, {}, []);
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("parallel");
    expect(result.steps).toHaveLength(3);
    expect(result.steps[0].args).toEqual({ x: 1 });
    expect(result.steps[1].args).toEqual({ x: 2 });
    expect(result.steps[2].args).toEqual({ x: 3 });
  });

  test("completes when all parallel items done", async () => {
    const wf = workflow(async () => {
      const items = [1, 2, 3];
      const results = await parallel(items, double);
      return results;
    });
    const result = await runWorkflow(
      wf,
      { completed_steps: { step_0: 2, step_1: 4, step_2: 6 } },
      [],
    );
    expect(result.type).toBe("complete");
    expect(result.result).toEqual([2, 4, 6]);
  });

  test("batched concurrency dispatches first batch", async () => {
    const wf = workflow(async () => {
      const items = [1, 2, 3, 4, 5];
      const results = await parallel(items, double, { concurrency: 2 });
      return results;
    });
    // First batch: items[0..2] → step_0, step_1
    const result = await runWorkflow(wf, {}, []);
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("parallel");
    expect(result.steps).toHaveLength(2);
    expect(result.steps[0].args).toEqual({ x: 1 });
    expect(result.steps[1].args).toEqual({ x: 2 });
  });

  test("batched concurrency dispatches second batch after first completes", async () => {
    const wf = workflow(async () => {
      const items = [1, 2, 3, 4, 5];
      const results = await parallel(items, double, { concurrency: 2 });
      return results;
    });
    // First batch done, second batch: items[2..4] → step_2, step_3
    const result = await runWorkflow(
      wf,
      { completed_steps: { step_0: 2, step_1: 4 } },
      [],
    );
    expect(result.type).toBe("dispatch");
    expect(result.mode).toBe("parallel");
    expect(result.steps).toHaveLength(2);
    expect(result.steps[0].args).toEqual({ x: 3 });
    expect(result.steps[1].args).toEqual({ x: 4 });
  });

  test("batched concurrency last batch may be smaller", async () => {
    const wf = workflow(async () => {
      const items = [1, 2, 3, 4, 5];
      const results = await parallel(items, double, { concurrency: 2 });
      return results;
    });
    // Two batches done, third batch: items[4..5] → step_4
    const result = await runWorkflow(
      wf,
      { completed_steps: { step_0: 2, step_1: 4, step_2: 6, step_3: 8 } },
      [],
    );
    expect(result.type).toBe("dispatch");
    expect(result.steps).toHaveLength(1);
    expect(result.steps[0].args).toEqual({ x: 5 });
  });

  test("batched concurrency completes with all results in order", async () => {
    const wf = workflow(async () => {
      const items = [1, 2, 3, 4, 5];
      const results = await parallel(items, double, { concurrency: 2 });
      return results;
    });
    const result = await runWorkflow(
      wf,
      { completed_steps: { step_0: 2, step_1: 4, step_2: 6, step_3: 8, step_4: 10 } },
      [],
    );
    expect(result.type).toBe("complete");
    expect(result.result).toEqual([2, 4, 6, 8, 10]);
  });

  test("empty items returns empty array", async () => {
    const wf = workflow(async () => {
      const results = await parallel([], double);
      return results;
    });
    const result = await runWorkflow(wf, {}, []);
    expect(result.type).toBe("complete");
    expect(result.result).toEqual([]);
  });
});
