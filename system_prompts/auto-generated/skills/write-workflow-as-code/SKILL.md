---
name: write-workflow-as-code
description: MUST use when writing or modifying Windmill Workflow-as-Code scripts using workflow, task, step, sleep, approvals, taskScript, taskFlow, task_script, or task_flow.
---

## CLI Commands

Place scripts in a folder.

After writing, tell the user which command fits what they want to do:

- `wmill script preview <script_path>` — **default when iterating on a local script.** Runs the local file without deploying.
- `wmill script run <path>` — runs the script **already deployed** in the workspace. Use only when the user explicitly wants to test the deployed version, not local edits.
- `wmill generate-metadata` — generate `.script.yaml` and `.lock` files for the script you modified.
- `wmill sync push` — deploy local changes to the workspace. Only suggest/run this when the user explicitly asks to deploy/publish/push — not when they say "run", "try", or "test".

### Preview vs run — choose by intent, not habit

If the user says "run the script", "try it", "test it", "does it work" while there are **local edits to the script file**, use `script preview`. Do NOT push the script to then `script run` it — pushing is a deploy, and deploying just to test overwrites the workspace version with untested changes.

Only use `script run` when:
- The user explicitly says "run the deployed version" / "run what's on the server".
- There is no local script being edited (you're just invoking an existing script).

Only use `sync push` when:
- The user explicitly asks to deploy, publish, push, or ship.
- The preview has already validated the change and the user wants it in the workspace.

### After writing — offer to test, don't wait passively

If the user hasn't already told you to run/test/preview the script, offer it as a one-sentence next step (e.g. "Want me to run `wmill script preview` with sample args?"). Do not present a multi-option menu.

If the user already asked to test/run/try the script in their original request, skip the offer and just execute `wmill script preview <path> -d '<args>'` directly — pick plausible args from the script's declared parameters. The shape varies by language: `main(...)` for code languages, the SQL dialect's own placeholder syntax (`$1` for PostgreSQL, `?` for MySQL/Snowflake, `@P1` for MSSQL, `@name` for BigQuery, etc.), positional `$1`, `$2`, … for Bash, `param(...)` for PowerShell.

`wmill script preview` does not deploy, but it still executes script code and may cause side effects; run it yourself when the user asked to test/preview (or after confirming that execution is intended). `wmill sync push` and `wmill generate-metadata` modify workspace state or local files — only run these when the user explicitly asks; otherwise tell them which to run.

For a **visual** open-the-script-in-the-dev-page preview (rather than `script preview`'s run-and-print-result), use the `preview` skill.

Use `wmill resource-type list --schema` to discover available resource types.

Workflow-as-Code files use the normal script CLI workflow. There are no separate WAC deploy commands.

# Windmill Workflow-as-Code Writing Guide

## Scope

Use this guide when writing or modifying Windmill Workflow-as-Code (WAC) scripts.
WAC is authored as a Windmill script and deployed with the normal script workflow. It is not an OpenFlow YAML flow.

Supported WAC authoring targets:
- TypeScript scripts that import from `windmill-client`
- Python 3 scripts that import from `wmill`

## File Shape

TypeScript:

```typescript
import {
  task,
  taskScript,
  taskFlow,
  step,
  sleep,
  waitForApproval,
  getResumeUrls,
  parallel,
  workflow,
} from "windmill-client";

const process = task(async (x: string): Promise<string> => {
  return `processed: ${x}`;
});

export const main = workflow(async (x: string) => {
  const result = await process(x);
  return { result };
});
```

Python:

```python
from wmill import task, task_script, task_flow, step, sleep, wait_for_approval, get_resume_urls, parallel, workflow

@task()
async def process(x: str) -> str:
    return f"processed: {x}"

@workflow
async def main(x: str):
    result = await process(x)
    return {"result": result}
```

Rules:
- Do not call `main`.
- TypeScript should export the workflow entrypoint, preferably `export const main = workflow(async (...) => { ... })`.
- Python must use `@workflow` on an async top-level function, usually `main`.
- Define task functions and `taskScript`/`task_script` or `taskFlow`/`task_flow` assignments at module top level with stable names.
- Use the exact SDK names. Do not alias `workflow`, `task`, `taskScript`, `taskFlow`, `step`, `sleep`, `waitForApproval`, `task_script`, `task_flow`, or `wait_for_approval`; the WAC parser recognizes these names directly.

## Checkpoint And Replay Model

The parent workflow may rerun from the top after any suspension, retry, approval, or child task completion. Completed durable steps are replayed from the checkpoint.

Put every side effect or non-deterministic value behind a durable WAC boundary:
- Use `task()` / `@task()` for substantial work that should run as its own child job.
- Use `taskScript()` / `task_script()` for an existing script or a relative module file.
- Use `taskFlow()` / `task_flow()` for an existing Windmill flow.
- Use `step(name, fn)` for lightweight inline work whose result must be checkpointed.
- Use `sleep(seconds)` for server-side sleeps that do not hold a worker.
- Use `waitForApproval()` / `wait_for_approval()` for external approval suspension.

Never put API calls, database writes, notifications, random values, timestamps, or irreversible changes directly in the top-level workflow body. The workflow body can be rerun. Put those operations in a task or in `step()`.

Branching on task or step results is safe because those results are checkpointed. Branching on current time, random data, environment reads, or external state is unsafe unless the value is first captured with `step()`.

## Tasks

Use `task()` / `@task()` for inline functions that become workflow steps:

```typescript
const enrich = task(async (customerId: string) => {
  return await fetchCustomer(customerId);
});
```

```python
@task(timeout=600, tag="etl")
async def enrich(customer_id: str):
    return await fetch_customer(customer_id)
```

In TypeScript, prefer assigning each task to a named top-level const. In Python, prefer top-level async functions decorated with `@task()` or `@task`.

For existing scripts:

```typescript
const helper = taskScript("./helper.ts");
const existing = taskScript("f/data/extract", { timeout: 600 });
const value = await helper({ input: x });
```

```python
helper = task_script("./helper.py")
existing = task_script("f/data/extract", timeout=600)
value = await helper(input=x)
```

For existing flows:

```typescript
const pipeline = taskFlow("f/etl/pipeline");
const output = await pipeline({ input: data });
```

```python
pipeline = task_flow("f/etl/pipeline")
output = await pipeline(input=data)
```

## Inline Steps

Use `step()` for lightweight inline values that must not change during replay:

```typescript
const urls = await step("get_urls", () => getResumeUrls());
const startedAt = await step("started_at", () => new Date().toISOString());
```

```python
urls = await step("get_urls", lambda: get_resume_urls())
```

Use stable, descriptive step names. Do not generate step names dynamically.

## Parallelism

To run independent work in parallel, start task promises/coroutines before awaiting them together:

```typescript
const [a, b] = await Promise.all([process("a"), process("b")]);
const many = await parallel(items, process, { concurrency: 5 });
```

```python
import asyncio

a, b = await asyncio.gather(process("a"), process("b"))
many = await parallel(items, process, concurrency=5)
```

Only parallelize independent steps. Do not read the result of a task before it is awaited.

## Approvals

Generate resume URLs inside `step()` before sending them:

```typescript
const urls = await step("get_urls", () => getResumeUrls());
await step("notify", () => sendApprovalEmail(urls.approvalPage));
const approval = await waitForApproval({ timeout: 3600 });
```

```python
urls = await step("get_urls", lambda: get_resume_urls())
await step("notify", lambda: send_approval_email(urls["approvalPage"]))
approval = await wait_for_approval(timeout=3600)
```

`selfApproval: false` and `self_approval=False` are Enterprise-only approval behavior. Do not use them unless the user asks for that behavior.

## Error Handling

Let task errors fail the workflow unless the user asks for recovery logic.

Python: `except Exception` is safe around WAC calls because internal suspension inherits from `BaseException`. Avoid bare `except:` in workflow code. If the user asks for recovery logic around failed child work, catch `TaskError` from `wmill` for task failures.

TypeScript: avoid broad `try/catch` around WAC SDK calls. The SDK uses an internal suspension error during initial dispatch; catching it can break workflow suspension. If a broad catch is unavoidable, rethrow internal suspension errors before handling business errors.


## TypeScript Workflow-as-Code API (windmill-client)

Import: `import { workflow, task, taskScript, taskFlow, step, sleep, waitForApproval, getResumeUrls, parallel } from "windmill-client"`

```typescript
export interface TaskOptions {
  timeout?: number;
  tag?: string;
  cache_ttl?: number;
  priority?: number;
  concurrency_limit?: number;
  concurrency_key?: string;
  concurrency_time_window_s?: number;
}

/**
 * Get URLs needed for resuming a flow after this step
 * @param approver approver name
 * @param flowLevel if true, generate resume URLs for the parent flow instead of the specific step.
 *                  This allows pre-approvals that can be consumed by any later suspend step in the same flow.
 * @returns approval page UI URL, resume and cancel API URLs for resuming the flow
 */
export async function getResumeUrls(approver?: string, flowLevel?: boolean): Promise<{ approvalPage: string; resume: string; cancel: string; }>

/**
 * Wrap an async function as a workflow task.
 *
 * @example
 * const extract_data = task(async (url: string) => { ... });
 * const run_external = task("f/external_script", async (x: number) => { ... });
 *
 * Inside a `workflow()`, calling a task dispatches it as a step.
 * Outside a workflow, the function body executes directly.
 */
export function task<T extends (...args: any[]) => Promise<any>>(fnOrPath: T | string, maybeFnOrOptions?: T | TaskOptions, maybeOptions?: TaskOptions,): T

/**
 * Create a task that dispatches to a separate Windmill script.
 *
 * @example
 * const extract = taskScript("f/data/extract");
 * // inside workflow: await extract({ url: "https://..." })
 */
export function taskScript(path: string, options?: TaskOptions): (...args: any[]) => PromiseLike<any>

/**
 * Create a task that dispatches to a separate Windmill flow.
 *
 * @example
 * const pipeline = taskFlow("f/etl/pipeline");
 * // inside workflow: await pipeline({ input: data })
 */
export function taskFlow(path: string, options?: TaskOptions): (...args: any[]) => PromiseLike<any>

/**
 * Mark an async function as a workflow-as-code entry point.
 *
 * The function must be **deterministic**: given the same inputs it must call
 * tasks in the same order on every replay. Branching on task results is fine
 * (results are replayed from checkpoint), but branching on external state
 * (current time, random values, external API calls) must use `step()` to
 * checkpoint the value so replays see the same result.
 */
export function workflow<T>(fn: (...args: any[]) => Promise<T>)

export async function step<T>(name: string, fn: () => T | Promise<T>): Promise<T>

export async function sleep(seconds: number): Promise<void>

/**
 * Suspend the workflow and wait for an external approval.
 *
 * Use `getResumeUrls()` (wrapped in `step()`) to obtain resume/cancel/approvalPage
 * URLs before calling this function.
 *
 * @example
 * const urls = await step("urls", () => getResumeUrls());
 * await step("notify", () => sendEmail(urls.approvalPage));
 * const { value, approver } = await waitForApproval({ timeout: 3600 });
 */
export function waitForApproval(options?: { timeout?: number; form?: object; selfApproval?: boolean; }): PromiseLike<{ value: any; approver: string; approved: boolean }>

/**
 * Process items in parallel with optional concurrency control.
 *
 * Each item is processed by calling `fn(item)`, which should be a task().
 * Items are dispatched in batches of `concurrency` (default: all at once).
 *
 * @example
 * const process = task(async (item: string) => { ... });
 * const results = await parallel(items, process, { concurrency: 5 });
 */
export async function parallel<T, R>(items: T[], fn: (item: T) => PromiseLike<R> | R, options?: { concurrency?: number },): Promise<R[]>
```


## Python Workflow-as-Code API (wmill)

Import: `from wmill import workflow, task, task_script, task_flow, step, sleep, wait_for_approval, get_resume_urls, parallel, TaskError`

```python
# Raised when a WAC task step failed.
#
# Attributes:
#     step_key: The checkpoint key of the failed step.
#     child_job_id: The UUID of the failed child job.
#     result: The error result from the child job.
class TaskError(Exception):
    def __init__(self, message: str, *, step_key: str = '', child_job_id: str = '', result = None)

# Get URLs needed for resuming a flow after suspension.
#
# Args:
#     approver: Optional approver name
#     flow_level: If True, generate resume URLs for the parent flow instead of the
#         specific step. This allows pre-approvals that can be consumed by any later
#         suspend step in the same flow.
#
# Returns:
#     Dictionary with approvalPage, resume, and cancel URLs
def get_resume_urls(approver: str = None, flow_level: bool = None) -> dict

# Decorator that marks a function as a workflow task.
#
# Works in both WAC v1 (sync, HTTP-based dispatch) and WAC v2
# (async, checkpoint/replay) modes:
#
# - **v2 (inside @workflow)**: dispatches as a checkpoint step.
# - **v1 (WM_JOB_ID set, no @workflow)**: dispatches via HTTP API.
# - **Standalone**: executes the function body directly.
#
# Usage::
#
#     @task
#     async def extract_data(url: str): ...
#
#     @task(path="f/external_script", timeout=600, tag="gpu")
#     async def run_external(x: int): ...
def task(_func = None, *, path: Optional[str] = None, tag: Optional[str] = None, timeout: Optional[int] = None, cache_ttl: Optional[int] = None, priority: Optional[int] = None, concurrency_limit: Optional[int] = None, concurrency_key: Optional[str] = None, concurrency_time_window_s: Optional[int] = None)

# Create a task that dispatches to a separate Windmill script.
#
# Usage::
#
#     extract = task_script("f/data/extract", timeout=600)
#
#     @workflow
#     async def main():
#         data = await extract(url="https://...")
def task_script(path: str, *, timeout: Optional[int] = None, tag: Optional[str] = None, cache_ttl: Optional[int] = None, priority: Optional[int] = None, concurrency_limit: Optional[int] = None, concurrency_key: Optional[str] = None, concurrency_time_window_s: Optional[int] = None)

# Create a task that dispatches to a separate Windmill flow.
#
# Usage::
#
#     pipeline = task_flow("f/etl/pipeline", priority=10)
#
#     @workflow
#     async def main():
#         result = await pipeline(input=data)
def task_flow(path: str, *, timeout: Optional[int] = None, tag: Optional[str] = None, cache_ttl: Optional[int] = None, priority: Optional[int] = None, concurrency_limit: Optional[int] = None, concurrency_key: Optional[str] = None, concurrency_time_window_s: Optional[int] = None)

# Decorator marking an async function as a workflow-as-code entry point.
#
# The function must be **deterministic**: given the same inputs it must call
# tasks in the same order on every replay. Branching on task results is fine
# (results are replayed from checkpoint), but branching on external state
# (current time, random values, external API calls) must use ``step()`` to
# checkpoint the value so replays see the same result.
def workflow(func)

# Execute ``fn`` inline and checkpoint the result.
#
# On replay the cached value is returned without re-executing ``fn``.
# Use for lightweight deterministic operations (timestamps, random IDs,
# config reads) that should not incur the overhead of a child job.
async def step(name: str, fn)

# Server-side sleep — suspend the workflow for the given duration without holding a worker.
#
# Inside a @workflow, the parent job suspends and auto-resumes after ``seconds``.
# Outside a workflow, falls back to ``asyncio.sleep``.
async def sleep(seconds: int)

# Suspend the workflow and wait for an external approval.
#
# Use ``get_resume_urls()`` (wrapped in ``step()``) to obtain
# resume/cancel/approval URLs before calling this function.
#
# Returns a dict with ``value`` (form data), ``approver``, and ``approved``.
#
# Args:
#     timeout: Approval timeout in seconds (default 1800).
#     form: Optional form schema for the approval page.
#     self_approval: Whether the user who triggered the flow can approve it (default True).
#
# Example::
#
#     urls = await step("urls", lambda: get_resume_urls())
#     await step("notify", lambda: send_email(urls["approvalPage"]))
#     result = await wait_for_approval(timeout=3600)
async def wait_for_approval(timeout: int = 1800, form: dict | None = None, self_approval: bool = True) -> dict

# Process items in parallel with optional concurrency control.
#
# Each item is processed by calling ``fn(item)``, which should be a @task.
# Items are dispatched in batches of ``concurrency`` (default: all at once).
#
# Example::
#
#     @task
#     async def process(item: str):
#         ...
#
#     results = await parallel(items, process, concurrency=5)
async def parallel(items, fn, *, concurrency: Optional[int] = None)
```
