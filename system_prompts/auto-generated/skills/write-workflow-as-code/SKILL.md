---
name: write-workflow-as-code
description: MUST use when writing or modifying Windmill Workflow-as-Code scripts using workflow, task, step, sleep, approvals, taskScript, taskFlow, task_script, or task_flow.
---

## CLI Commands

Place scripts in a folder.

After writing, tell the user which command fits what they want to do:

- `wmill script preview <script_path>` — **default when iterating on a local script.** Runs the local file without deploying.
- `wmill script run <path>` — runs the script **already deployed** in the workspace. Use only when the user explicitly wants to test the deployed version, not local edits.
- `wmill generate-metadata` — regenerate the local `.script.yaml` (input schema) and `.lock` (resolved dependencies) for scripts you changed, and refresh their content hashes in `wmill-lock.yaml`. Local files only — **not** a deploy. See "Keep metadata in sync" below.
- Deploy local changes to the workspace — via `git push` or `wmill sync push` depending on how the repo is wired (see the **Deploying** section in `AGENTS.wmill.md`). Only suggest/run a deploy when the user explicitly asks to deploy/publish/push — not when they say "run", "try", or "test".

### Preview vs run — choose by intent, not habit

If the user says "run the script", "try it", "test it", "does it work" while there are **local edits to the script file**, use `script preview`. Do NOT push the script to then `script run` it — pushing is a deploy, and deploying just to test overwrites the workspace version with untested changes.

Only use `script run` when:
- The user explicitly says "run the deployed version" / "run what's on the server".
- There is no local script being edited (you're just invoking an existing script).

Only use `sync push` when:
- The user explicitly asks to deploy, publish, push, or ship.
- The preview has already validated the change and the user wants it in the workspace.

### Keep metadata in sync after editing

`wmill-lock.yaml` tracks a content hash for each item. Editing a script's content — most importantly **adding or removing an import** or **changing `main`'s arguments** — invalidates that hash and leaves the `.lock`, the `.script.yaml` input schema, and the hash row out of date. Run `wmill generate-metadata` (scoped to what you touched) after such edits so the resolved lock, the auto-generated args UI (driven by `.script.yaml`), and `wmill-lock.yaml` all match the code. Leaving them stale produces spurious diffs in git-sync and CI.

This only writes local files (it is **not** a deploy), but it re-resolves dependencies, so it can bump unpinned versions (the same as deploying from the UI; expected, not a bug). So by default offer it and run it once the user agrees, rather than running it silently after every edit — unless the project's `AGENTS.md` opts into running metadata automatically (see the "Keeping metadata in sync" preference there). Either way YOU run the command, not the user. After running it, diff the regenerated `.lock` / `.script.lock` files and tell the user which dependency versions changed (e.g. `requests 2.31.0 → 2.32.0`), so they can catch an unwanted bump before deploying — even under `Metadata: auto`, since it's information, not a confirmation gate. Pin versions in code to keep them fixed.

With no path argument, `generate-metadata` regenerates only the items whose content hash drifted — not everything. Imports propagate: editing a script that others import marks every importer stale too, so a one-line change to a shared module can regenerate many locks (by design — their locks must reflect the imported code). If it touches more than you expect, run `wmill generate-metadata --dry-run` — it lists each stale item with a reason (`content changed` or `depends on <path>`) without changing anything — then narrow with a path argument (`wmill generate-metadata f/foo`) or `--strict-folder-boundaries`.

If the on-disk `.lock` and `.script.yaml` are already correct and only `wmill-lock.yaml` needs its hashes refreshed (hash drift, or bootstrapping missing entries), use `wmill generate-metadata rehash` — it re-records hashes from disk with no backend round-trip and no dependency changes.

### After writing — offer to test, don't wait passively

If the user hasn't already told you to run/test/preview the script, offer it as a one-sentence next step (e.g. "Want me to run `wmill script preview` with sample args?"). Do not present a multi-option menu.

If the user already asked to test/run/try the script in their original request, skip the offer and just execute `wmill script preview <path> -d '<args>'` directly — pick plausible args from the script's declared parameters. The shape varies by language: `main(...)` for code languages, the SQL dialect's own placeholder syntax (`$1` for PostgreSQL, `?` for MySQL/Snowflake, `@P1` for MSSQL, `@name` for BigQuery, etc.), positional `$1`, `$2`, … for Bash, `param(...)` for PowerShell.

`wmill script preview` does not deploy, but it still executes script code and may cause side effects; run it yourself when the user asked to test/preview (or after confirming that execution is intended). `wmill generate-metadata` does not deploy either — it only writes local files (locks, schemas, hashes) — but offer it before running (or run automatically if the project's `AGENTS.md` opts in), per "Keep metadata in sync" above. Deploying to the workspace (`git push` or `wmill sync push` depending on how the repo is wired — see the **Deploying** section) is the only step that mutates remote state — do it only when the user explicitly asks to deploy/publish/push.

For a **visual** open-the-script-in-the-dev-page preview (rather than `script preview`'s run-and-print-result), use the `preview` skill.

Use `wmill resource-type list --schema` to discover available resource types.

Workflow-as-Code files use the normal script CLI workflow. There are no separate WAC deploy commands.

# Windmill Workflow-as-Code Writing Guide

## Scope

Use this guide when writing or modifying Windmill Workflow-as-Code (WAC) scripts.
WAC is authored as a Windmill script and deployed with the normal script workflow. It is not an OpenFlow YAML flow.

Supported WAC authoring targets:
- Bun TypeScript scripts that import from `windmill-client`
- Python 3 scripts that import from `wmill`

## File Shape

Bun TypeScript:

```typescript
import {
  task,
  taskScript,
  taskFlow,
  step,
  sleep,
  waitForApproval,
  getApprovalUrls,
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
from wmill import task, task_script, task_flow, step, sleep, wait_for_approval, get_approval_urls, parallel, workflow

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
- Bun TypeScript should export the workflow entrypoint, preferably `export const main = workflow(async (...) => { ... })`.
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
const startedAt = await step("started_at", () => new Date().toISOString());
```

```python
started_at = await step("started_at", lambda: datetime.now().isoformat())
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

Name the approval step and generate its URLs inside `step()` before sending them.
`getApprovalUrls` / `get_approval_urls` returns the URLs bound to that step, the same
ones its built-in approve/reject buttons use:

```typescript
const urls = await step("urls", () => getApprovalUrls("manager"));
await step("notify", () => sendApprovalEmail(urls.resume, urls.cancel));
const approval = await waitForApproval({ key: "manager", timeout: 3600 });
```

```python
urls = await step("urls", lambda: get_approval_urls("manager"))
await step("notify", lambda: send_approval_email(urls["resume"], urls["cancel"]))
approval = await wait_for_approval(key="manager", timeout=3600)
```

With several approvals in one workflow, give each its own key so each notification
resumes its own step. Keys must be unique — reusing one raises an error rather than
silently renaming the step. `getResumeUrls()` / `get_resume_urls()` still works but signs a
random nonce, so its URLs are not tied to any particular approval step.

`selfApproval: false` and `self_approval=False` are Enterprise-only approval behavior. Do not use them unless the user asks for that behavior.

## Error Handling

Let task errors fail the workflow unless the user asks for recovery logic.

Python: `except Exception` is safe around WAC calls because internal suspension inherits from `BaseException`. Avoid bare `except:` in workflow code. If the user asks for recovery logic around failed child work, catch `TaskError` from `wmill` for task failures.

TypeScript: avoid broad `try/catch` around WAC SDK calls. The SDK uses an internal suspension error during initial dispatch; catching it can break workflow suspension. If a broad catch is unavoidable, rethrow internal suspension errors before handling business errors.


## TypeScript Workflow-as-Code API (windmill-client)

Import: `import { workflow, task, taskScript, taskFlow, step, sleep, waitForApproval, getApprovalUrls, getResumeUrls, parallel } from "windmill-client"`

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
 * Pass `key` to name the step, then `getApprovalUrls(key)` yields the URLs that
 * resume exactly this approval — route them through your own channel. Without a
 * key the steps are named `approval`, `approval_2`, ...
 *
 * @example
 * const urls = await step("urls", () => getApprovalUrls("manager"));
 * await step("notify", () => sendEmail(urls.resume, urls.cancel));
 * const { value, approver } = await waitForApproval({ key: "manager", timeout: 3600 });
 */
export function waitForApproval(options?: { timeout?: number; form?: object; selfApproval?: boolean; key?: string; }): PromiseLike<{ value: any; approver: string; approved: boolean }>

/**
 * Resume/cancel/approval-page URLs bound to one `waitForApproval` step.
 *
 * Unlike `getResumeUrls()`, which signs a random nonce, these address the very
 * `resume_job` record the step's built-in approval buttons use, so they are
 * stable across replays and safe to embed in a custom notification.
 *
 * `stepKey` must match the `key` given to `waitForApproval`. Keys must be unique
 * within a workflow; reusing one throws rather than silently renaming it.
 *
 * @example
 * const urls = await step("urls", () => getApprovalUrls("manager"));
 * await step("notify", () => sendEmail(urls.resume, urls.cancel));
 * await waitForApproval({ key: "manager" });
 */
export async function getApprovalUrls(stepKey: string = "approval", approver?: string): Promise<{ approvalPage: string; resume: string; cancel: string; }>

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

Import: `from wmill import workflow, task, task_script, task_flow, step, sleep, wait_for_approval, get_approval_urls, get_resume_urls, parallel, TaskError`

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
# Pass ``key`` to name the step, then ``get_approval_urls(key)`` yields the URLs
# that resume exactly this approval — route them through your own channel.
# Without a key the steps are named ``approval``, ``approval_2``, ...
#
# Returns a dict with ``value`` (form data), ``approver``, and ``approved``.
#
# Args:
#     timeout: Approval timeout in seconds (default 1800).
#     form: Optional form schema for the approval page.
#     self_approval: Whether the user who triggered the flow can approve it (default True).
#     key: Optional checkpoint key naming this approval step.
#
# Example::
#
#     urls = await step("urls", lambda: get_approval_urls("manager"))
#     await step("notify", lambda: send_email(urls["resume"], urls["cancel"]))
#     result = await wait_for_approval(key="manager", timeout=3600)
async def wait_for_approval(timeout: int = 1800, form: dict | None = None, self_approval: bool = True, key: str | None = None) -> dict

# Get the resume/cancel/approval-page URLs bound to one ``wait_for_approval`` step.
#
# Unlike :func:`get_resume_urls`, which signs a random nonce, these address the
# very ``resume_job`` record the step's built-in approval buttons use, so they
# are stable across replays and safe to embed in a custom notification.
#
# Args:
#     step_key: Checkpoint key of the approval step, as passed to
#         ``wait_for_approval(key=...)``. Keys must be unique within a workflow;
#         reusing one raises rather than silently renaming it.
#     approver: Optional approver name
#
# Returns:
#     Dictionary with approvalPage, resume, and cancel URLs
def get_approval_urls(step_key: str = 'approval', approver: str = None) -> dict

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
