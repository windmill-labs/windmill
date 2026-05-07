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
