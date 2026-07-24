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
 * within a workflow; reusing one throws rather than silently renaming it. The URL
 * only resumes while that step is awaiting approval; used at any other moment it is
 * rejected rather than banking a row a different approval would consume. Send it
 * ahead of time — approvers just cannot act before the workflow reaches the step.
 *
 * `resume` and `cancel` are step-bound; `approvalPage` is not — it opens the job's
 * approval page, which acts on whichever approval is pending when it is used.
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
