---
name: write-script-bun
description: MUST use when writing Bun/TypeScript scripts.
---

## CLI Commands

Place scripts in a folder.

After writing, tell the user which command fits what they want to do:

- `wmill script preview <script_path>` — **default when iterating on a local script.** Runs the local file without deploying.
- `wmill script run <path>` — runs the script **already deployed** in the workspace. Use only when the user explicitly wants to test the deployed version, not local edits.
- `wmill generate-metadata` — regenerate the local `.script.yaml` (input schema) and `.lock` (resolved dependencies) for scripts you changed, and refresh their content hashes in `wmill-lock.yaml`. Local files only — **not** a deploy. See "Keep metadata in sync" below.
- Deploy local changes to the workspace. If the repo deploys on git push (backend auto-pull or CI — check `wmill gitsync-settings status`), deploy with `git push`; otherwise use `wmill sync push` directly. Only suggest/run a deploy when the user explicitly asks to deploy/publish/push — not when they say "run", "try", or "test".

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

`wmill script preview` does not deploy, but it still executes script code and may cause side effects; run it yourself when the user asked to test/preview (or after confirming that execution is intended). `wmill generate-metadata` does not deploy either — it only writes local files (locks, schemas, hashes) — but offer it before running (or run automatically if the project's `AGENTS.md` opts in), per "Keep metadata in sync" above. Deploying to the workspace (`git push` if the repo deploys on push, otherwise `wmill sync push`) is the only step that mutates remote state — do it only when the user explicitly asks to deploy/publish/push.

For a **visual** open-the-script-in-the-dev-page preview (rather than `script preview`'s run-and-print-result), use the `preview` skill.

Use `wmill resource-type list --schema` to discover available resource types.

# TypeScript (Bun)

Bun runtime with full npm ecosystem and fastest execution.

## Structure

Export a single **async** function called `main`:

```typescript
export async function main(param1: string, param2: number) {
  // Your code here
  return { result: param1, count: param2 };
}
```

Do not call the main function. Libraries are installed automatically.

## Resource Types

On Windmill, credentials and configuration are stored in resources and passed as parameters to main.

Use the `RT` namespace for resource types:

```typescript
export async function main(stripe: RT.Stripe) {
  // stripe contains API key and config from the resource
}
```

Only use resource types if you need them to satisfy the instructions. Always use the RT namespace.

Before using a resource type, check the `rt.d.ts` file in the project root to see all available resource types and their fields. This file is generated by `wmill resource-type generate-namespace`.

## Imports

```typescript
import Stripe from "stripe";
import { someFunction } from "some-package";
```

## Prefer `//native` when the runtime allows it

If a script only needs `fetch` and the JavaScript standard library — including when it uses `windmill-client` — prefer making it a **native** script: add `//native` as the first line and write it with the `write-script-bunnative` skill. Native scripts run on a lightweight V8 isolate, start faster, and parallelize heavily. `windmill-client` works on the native worker (its calls go over `fetch`), so needing the Windmill client is **not** a reason to avoid `//native`. Use the regular `bun` language only when the code (or a dependency) needs Node/Bun runtime APIs — `node:*` modules, the filesystem, child processes, or native addons.

## Windmill Client

Import the windmill client for platform interactions:

```typescript
import * as wmill from "windmill-client";
```

**Prefer `windmill-client` over raw `fetch` for anything that talks to Windmill** — reading resources/variables/states, running scripts and flows, S3 object operations, etc. It handles auth, the workspace, and the base URL for you, so you don't hand-roll URLs or tokens. Reserve `fetch` for calling *external* HTTP APIs that aren't Windmill.

The full `windmill-client` API reference (every exported function and its signature) is included in this skill below — consult it for the exact method to use instead of guessing or falling back to `fetch`.

## Preprocessor Scripts

For preprocessor scripts, the function should be named `preprocessor` and receives an `event` parameter:

```typescript
type Event = {
  kind:
    | "webhook"
    | "http"
    | "websocket"
    | "kafka"
    | "email"
    | "nats"
    | "postgres"
    | "sqs"
    | "mqtt"
    | "gcp";
  body: any;
  headers: Record<string, string>;
  query: Record<string, string>;
};

export async function preprocessor(event: Event) {
  return {
    param1: event.body.field1,
    param2: event.query.id,
  };
}
```

## S3 Object Operations

Windmill provides built-in support for S3-compatible storage operations. The `wmill.S3Object` type covers both the `s3://storage/key` URI form (`s3:///key` for the workspace default storage) and the `{ s3, storage? }` record form — always use it instead of redefining your own.

### Receiving an S3Object as a script parameter

```typescript
import * as wmill from "windmill-client";

export async function main(file: wmill.S3Object) {
  const content = await wmill.loadS3File(file);
  // ...
}
```

### S3 operations

```typescript
import * as wmill from "windmill-client";

// Load file content from S3
const content: Uint8Array = await wmill.loadS3File(s3object);

// Load file as stream
const blob: Blob = await wmill.loadS3FileStream(s3object);

// Write file to S3
const result: wmill.S3Object = await wmill.writeS3File(
  s3object, // Target path (or undefined to auto-generate)
  fileContent, // string or Blob
  s3ResourcePath // Optional: specific S3 resource to use
);
```


# TypeScript SDK (windmill-client)

Import: import * as wmill from 'windmill-client'

workerHasInternalServer(): boolean

/**
 * Initialize the Windmill client with authentication token and base URL
 * @param token - Authentication token (defaults to WM_TOKEN env variable)
 * @param baseUrl - API base URL (defaults to BASE_INTERNAL_URL or BASE_URL env variable)
 */
setClient(token?: string, baseUrl?: string): void

/**
 * Create a client configuration from env variables
 * @returns client configuration
 */
getWorkspace(): string

/**
 * Get a resource value by path
 * @param path path of the resource,  default to internal state path
 * @param undefinedIfEmpty if the resource does not exist, return undefined instead of throwing an error
 * @returns resource value
 */
async getResource(path?: string, undefinedIfEmpty?: boolean): Promise<any>

/**
 * Get the true root job id
 * @param jobId job id to get the root job id from (default to current job)
 * @returns root job id
 */
async getRootJobId(jobId?: string): Promise<string>

/**
 * @deprecated Use runScriptByPath or runScriptByHash instead
 */
async runScript(path: string | null = null, hash_: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false, tag: string | null = null): Promise<any>

/**
 * Run a script synchronously by its path and wait for the result
 * @param path - Script path in Windmill
 * @param args - Arguments to pass to the script
 * @param verbose - Enable verbose logging
 * @param tag - Override the worker tag the job runs on
 * @returns Script execution result
 */
async runScriptByPath(path: string, args: Record<string, any> | null = null, verbose: boolean = false, tag: string | null = null): Promise<any>

/**
 * Run a script synchronously by its hash and wait for the result
 * @param hash_ - Script hash in Windmill
 * @param args - Arguments to pass to the script
 * @param verbose - Enable verbose logging
 * @param tag - Override the worker tag the job runs on
 * @returns Script execution result
 */
async runScriptByHash(hash_: string, args: Record<string, any> | null = null, verbose: boolean = false, tag: string | null = null): Promise<any>

/**
 * Append a text to the result stream
 * @param text text to append to the result stream
 */
appendToResultStream(text: string): void

/**
 * Stream to the result stream
 * @param stream stream to stream to the result stream
 */
async streamResult(stream: AsyncIterable<string>): Promise<void>

/**
 * Run a flow synchronously by its path and wait for the result
 * @param path - Flow path in Windmill
 * @param args - Arguments to pass to the flow
 * @param verbose - Enable verbose logging
 * @param tag - Override the worker tag the job runs on
 * @returns Flow execution result
 */
async runFlow(path: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false, tag: string | null = null): Promise<any>

/**
 * Wait for a job to complete and return its result
 * @param jobId - ID of the job to wait for
 * @param verbose - Enable verbose logging
 * @returns Job result when completed
 */
async waitJob(jobId: string, verbose: boolean = false): Promise<any>

/**
 * Get the result of a completed job
 * @param jobId - ID of the completed job
 * @returns Job result
 */
async getResult(jobId: string): Promise<any>

/**
 * Get the result of a job if completed, or its current status
 * @param jobId - ID of the job
 * @returns Object with started, completed, success, and result properties
 */
async getResultMaybe(jobId: string): Promise<any>

/**
 * @deprecated Use runScriptByPathAsync or runScriptByHashAsync instead
 */
async runScriptAsync(path: string | null, hash_: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null, tag: string | null = null): Promise<string>

/**
 * Run a script asynchronously by its path
 * @param path - Script path in Windmill
 * @param args - Arguments to pass to the script
 * @param scheduledInSeconds - Schedule execution for a future time (in seconds)
 * @param tag - Override the worker tag the job runs on
 * @returns Job ID of the created job
 */
async runScriptByPathAsync(path: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null, tag: string | null = null): Promise<string>

/**
 * Run a script asynchronously by its hash
 * @param hash_ - Script hash in Windmill
 * @param args - Arguments to pass to the script
 * @param scheduledInSeconds - Schedule execution for a future time (in seconds)
 * @param tag - Override the worker tag the job runs on
 * @returns Job ID of the created job
 */
async runScriptByHashAsync(hash_: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null, tag: string | null = null): Promise<string>

/**
 * Run a flow asynchronously by its path
 * @param path - Flow path in Windmill
 * @param args - Arguments to pass to the flow
 * @param scheduledInSeconds - Schedule execution for a future time (in seconds)
 * @param doNotTrackInParent - If false, tracks state in parent job (only use when fully awaiting the job)
 * @param tag - Override the worker tag the job runs on
 * @returns Job ID of the created job
 */
async runFlowAsync(path: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null, // can only be set to false if this the job will be fully await and not concurrent with any other job // as otherwise the child flow and its own child will store their state in the parent job which will // lead to incorrectness and failures doNotTrackInParent: boolean = true, tag: string | null = null): Promise<string>

/**
 * Resolve a resource value in case the default value was picked because the input payload was undefined
 * @param obj resource value or path of the resource under the format `$res:path`
 * @returns resource value
 */
async resolveDefaultResource(obj: any): Promise<any>

/**
 * Get the state file path from environment variables
 * @returns State path string
 */
getStatePath(): string

/**
 * Set a resource value by path
 * @param path path of the resource to set, default to state path
 * @param value new value of the resource to set
 * @param initializeToTypeIfNotExist if the resource does not exist, initialize it with this type
 */
async setResource(value: any, path?: string, initializeToTypeIfNotExist?: string): Promise<void>

/**
 * Set the state
 * @param state state to set
 * @deprecated use setState instead
 */
async setInternalState(state: any): Promise<void>

/**
 * Set the state
 * @param state state to set
 * @param path Optional state resource path override. Defaults to `getStatePath()`.
 */
async setState(state: any, path?: string): Promise<void>

/**
 * Set the progress
 * Progress cannot go back and limited to 0% to 99% range
 * @param percent Progress to set in %
 * @param jobId? Job to set progress for
 */
async setProgress(percent: number, jobId?: any): Promise<void>

/**
 * Get the progress
 * @param jobId? Job to get progress from
 * @returns Optional clamped between 0 and 100 progress value
 */
async getProgress(jobId?: any): Promise<number | null>

/**
 * Set a flow user state
 * @param key key of the state
 * @param value value of the state
 */
async setFlowUserState(key: string, value: any, errorIfNotPossible?: boolean): Promise<void>

/**
 * Get a flow user state
 * @param path path of the variable
 */
async getFlowUserState(key: string, errorIfNotPossible?: boolean): Promise<any>

/**
 * Get the internal state
 * @deprecated use getState instead
 */
async getInternalState(): Promise<any>

/**
 * Get the state shared across executions
 * @param path Optional state resource path override. Defaults to `getStatePath()`.
 */
async getState(path?: string): Promise<any>

/**
 * Get a variable by path
 * @param path path of the variable
 * @returns variable value
 */
async getVariable(path: string): Promise<string>

/**
 * Set a variable by path, create if not exist
 * @param path path of the variable
 * @param value value of the variable
 * @param isSecretIfNotExist if the variable does not exist, create it as secret or not (default: false)
 * @param descriptionIfNotExist if the variable does not exist, create it with this description (default: "")
 */
async setVariable(path: string, value: string, isSecretIfNotExist?: boolean, descriptionIfNotExist?: string): Promise<void>

/**
 * Build a PostgreSQL connection URL from a database resource
 * @param path - Path to the database resource
 * @returns PostgreSQL connection URL string
 */
async databaseUrlFromResource(path: string): Promise<string>

async polarsConnectionSettings(s3_resource_path: string | undefined): Promise<any>

async duckdbConnectionSettings(s3_resource_path: string | undefined): Promise<any>

/**
 * Get S3 client settings from a resource or workspace default
 * @param s3_resource_path - Path to S3 resource (uses workspace default if undefined)
 * @param workspace - Workspace to read from (defaults to the `WM_WORKSPACE` env var)
 * @returns S3 client configuration settings
 */
async denoS3LightClientSettings(s3_resource_path: string | undefined, workspace: string | undefined = undefined): Promise<DenoS3LightClientSettings>

/**
 * Load the content of a file stored in S3. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 * 
 * ```typescript
 * let fileContent = await wmill.loadS3FileContent(inputFile)
 * // if the file is a raw text file, it can be decoded and printed directly:
 * const text = new TextDecoder().decode(fileContentStream)
 * console.log(text);
 * ```
 * 
 * @param workspace - Workspace to read from (defaults to the `WM_WORKSPACE` env var)
 */
async loadS3File(s3object: S3Object, s3ResourcePath: string | undefined = undefined, workspace: string | undefined = undefined): Promise<Uint8Array | undefined>

/**
 * Load the content of a file stored in S3 as a stream. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 * 
 * ```typescript
 * let fileContentBlob = await wmill.loadS3FileStream(inputFile)
 * // if the content is plain text, the blob can be read directly:
 * console.log(await fileContentBlob.text());
 * ```
 * 
 * @param workspace - Workspace to read from (defaults to the `WM_WORKSPACE` env var)
 */
async loadS3FileStream(s3object: S3Object, s3ResourcePath: string | undefined = undefined, workspace: string | undefined = undefined): Promise<Blob | undefined>

/**
 * Persist a file to the S3 bucket. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 * 
 * ```typescript
 * const s3object = await writeS3File(s3Object, "Hello Windmill!")
 * const fileContentAsUtf8Str = (await s3object.toArray()).toString('utf-8')
 * console.log(fileContentAsUtf8Str)
 * ```
 * 
 * @param workspace - Workspace to write to (defaults to the `WM_WORKSPACE` env var)
 */
async writeS3File(s3object: S3Object | undefined, fileContent: string | Blob, s3ResourcePath: string | undefined = undefined, contentType: string | undefined = undefined, contentDisposition: string | undefined = undefined, workspace: string | undefined = undefined): Promise<S3Object>

/**
 * Permanently delete a file from S3 by key.
 * 
 * ```typescript
 * await wmill.deleteS3File({ s3: "path/to/file.txt" })
 * ```
 * 
 * @param s3object - S3 object identifying the file to delete (must have `s3` set)
 * @param workspace - Workspace to delete from (defaults to the `WM_WORKSPACE` env var)
 */
async deleteS3File(s3object: S3Object, workspace: string | undefined = undefined): Promise<void>

/**
 * Sign S3 objects to be used by anonymous users in public apps
 * @param s3objects s3 objects to sign
 * @returns signed s3 objects
 */
async signS3Objects(s3objects: S3Object[]): Promise<S3Object[]>

/**
 * Sign S3 object to be used by anonymous users in public apps
 * @param s3object s3 object to sign
 * @returns signed s3 object
 */
async signS3Object(s3object: S3Object): Promise<S3Object>

/**
 * Generate a presigned public URL for an array of S3 objects.
 * If an S3 object is not signed yet, it will be signed first.
 * @param s3Objects s3 objects to sign
 * @returns list of signed public URLs
 */
async getPresignedS3PublicUrls(s3Objects: S3Object[], { baseUrl }: { baseUrl?: string } = {}): Promise<string[]>

/**
 * Generate a presigned public URL for an S3 object. If the S3 object is not signed yet, it will be signed first.
 * @param s3Object s3 object to sign
 * @returns signed public URL
 */
async getPresignedS3PublicUrl(s3Objects: S3Object, { baseUrl }: { baseUrl?: string } = {}): Promise<string>

/**
 * Get URLs needed for resuming a flow after this step
 * @param approver approver name
 * @param flowLevel if true, generate resume URLs for the parent flow instead of the specific step.
 *                  This allows pre-approvals that can be consumed by any later suspend step in the same flow.
 * @returns approval page UI URL, resume and cancel API URLs for resuming the flow
 */
async getResumeUrls(approver?: string, flowLevel?: boolean): Promise<{
  approvalPage: string;
  resume: string;
  cancel: string;
}>

/**
 * @deprecated use getResumeUrls instead
 */
getResumeEndpoints(approver?: string): Promise<{
  approvalPage: string;
  resume: string;
  cancel: string;
}>

/**
 * Get an OIDC jwt token for auth to external services (e.g: Vault, AWS) (ee only)
 * @param audience audience of the token
 * @param expiresIn Optional number of seconds until the token expires
 * @returns jwt token
 */
async getIdToken(audience: string, expiresIn?: number): Promise<string>

/**
 * Convert a base64-encoded string to Uint8Array
 * @param data - Base64-encoded string
 * @returns Decoded Uint8Array
 */
base64ToUint8Array(data: string): Uint8Array

/**
 * Convert a Uint8Array to base64-encoded string
 * @param arrayBuffer - Uint8Array to encode
 * @returns Base64-encoded string
 */
uint8ArrayToBase64(arrayBuffer: Uint8Array): string

/**
 * Get email from workspace username
 * This method is particularly useful for apps that require the email address of the viewer.
 * Indeed, in the viewer context, WM_USERNAME is set to the username of the viewer but WM_EMAIL is set to the email of the creator of the app.
 * @param username
 * @returns email address
 */
async usernameToEmail(username: string): Promise<string>

/**
 * Sends an interactive approval request via Slack, allowing optional customization of the message, approver, and form fields.
 * 
 * **[Enterprise Edition Only]** To include form fields in the Slack approval request, go to **Advanced -> Suspend -> Form**
 * and define a form. Learn more at [Windmill Documentation](https://www.windmill.dev/docs/flows/flow_approval#form).
 * 
 * @param {Object} options - The configuration options for the Slack approval request.
 * @param {string} options.slackResourcePath - The path to the Slack resource in Windmill.
 * @param {string} options.channelId - The Slack channel ID where the approval request will be sent.
 * @param {string} [options.message] - Optional custom message to include in the Slack approval request.
 * @param {string} [options.approver] - Optional user ID or name of the approver for the request.
 * @param {DefaultArgs} [options.defaultArgsJson] - Optional object defining or overriding the default arguments to a form field.
 * @param {Enums} [options.dynamicEnumsJson] - Optional object overriding the enum default values of an enum form field.
 * @param {string} [options.resumeButtonText] - Optional text for the resume button.
 * @param {string} [options.cancelButtonText] - Optional text for the cancel button.
 * 
 * @returns {Promise<void>} Resolves when the Slack approval request is successfully sent.
 * 
 * @throws {Error} If the function is not called within a flow or flow preview.
 * @throws {Error} If the `JobService.getSlackApprovalPayload` call fails.
 * 
 * **Usage Example:**
 * ```typescript
 * await requestInteractiveSlackApproval({
 *   slackResourcePath: "/u/alex/my_slack_resource",
 *   channelId: "admins-slack-channel",
 *   message: "Please approve this request",
 *   approver: "approver123",
 *   defaultArgsJson: { key1: "value1", key2: 42 },
 *   dynamicEnumsJson: { foo: ["choice1", "choice2"], bar: ["optionA", "optionB"] },
 *   resumeButtonText: "Resume",
 *   cancelButtonText: "Cancel",
 * });
 * ```
 * 
 * **Note:** This function requires execution within a Windmill flow or flow preview.
 */
async requestInteractiveSlackApproval({ slackResourcePath, channelId, message, approver, defaultArgsJson, dynamicEnumsJson, resumeButtonText, cancelButtonText, }: SlackApprovalOptions): Promise<void>

/**
 * Sends an interactive approval request via Teams, allowing optional customization of the message, approver, and form fields.
 * 
 * **[Enterprise Edition Only]** To include form fields in the Teams approval request, go to **Advanced -> Suspend -> Form**
 * and define a form. Learn more at [Windmill Documentation](https://www.windmill.dev/docs/flows/flow_approval#form).
 * 
 * @param {Object} options - The configuration options for the Teams approval request.
 * @param {string} options.teamName - The Teams team name where the approval request will be sent.
 * @param {string} options.channelName - The Teams channel name where the approval request will be sent.
 * @param {string} [options.message] - Optional custom message to include in the Teams approval request.
 * @param {string} [options.approver] - Optional user ID or name of the approver for the request.
 * @param {DefaultArgs} [options.defaultArgsJson] - Optional object defining or overriding the default arguments to a form field.
 * @param {Enums} [options.dynamicEnumsJson] - Optional object overriding the enum default values of an enum form field.
 * 
 * @returns {Promise<void>} Resolves when the Teams approval request is successfully sent.
 * 
 * @throws {Error} If the function is not called within a flow or flow preview.
 * @throws {Error} If the `JobService.getTeamsApprovalPayload` call fails.
 * 
 * **Usage Example:**
 * ```typescript
 * await requestInteractiveTeamsApproval({
 *   teamName: "admins-teams",
 *   channelName: "admins-teams-channel",
 *   message: "Please approve this request",
 *   approver: "approver123",
 *   defaultArgsJson: { key1: "value1", key2: 42 },
 *   dynamicEnumsJson: { foo: ["choice1", "choice2"], bar: ["optionA", "optionB"] },
 * });
 * ```
 * 
 * **Note:** This function requires execution within a Windmill flow or flow preview.
 */
async requestInteractiveTeamsApproval({ teamName, channelName, message, approver, defaultArgsJson, dynamicEnumsJson, }: TeamsApprovalOptions): Promise<void>

setWorkflowCtx(ctx: WorkflowCtx | null): void

async sleep(seconds: number): Promise<void>

async step<T>(name: string, fn: () => T | Promise<T>): Promise<T>

/**
 * Create a task that dispatches to a separate Windmill script.
 * 
 * @example
 * const extract = taskScript("f/data/extract");
 * // inside workflow: await extract({ url: "https://..." })
 */
taskScript(path: string, options?: TaskOptions): (...args: any[]) => PromiseLike<any>

/**
 * Create a task that dispatches to a separate Windmill flow.
 * 
 * @example
 * const pipeline = taskFlow("f/etl/pipeline");
 * // inside workflow: await pipeline({ input: data })
 */
taskFlow(path: string, options?: TaskOptions): (...args: any[]) => PromiseLike<any>

/**
 * Mark an async function as a workflow-as-code entry point.
 * 
 * The function must be **deterministic**: given the same inputs it must call
 * tasks in the same order on every replay. Branching on task results is fine
 * (results are replayed from checkpoint), but branching on external state
 * (current time, random values, external API calls) must use `step()` to
 * checkpoint the value so replays see the same result.
 */
workflow<T>(fn: (...args: any[]) => Promise<T>): void

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
waitForApproval(options?: { timeout?: number; form?: object; selfApproval?: boolean; }): PromiseLike<{ value: any; approver: string; approved: boolean }>

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
async parallel<T, R>(items: T[], fn: (item: T) => PromiseLike<R> | R, options?: { concurrency?: number },): Promise<R[]>

/**
 * Commit Kafka offsets for a trigger with auto_commit disabled.
 * @param triggerPath - Path to the Kafka trigger (from event.wm_trigger.trigger_path)
 * @param topic - Kafka topic name (from event.topic)
 * @param partition - Partition number (from event.partition)
 * @param offset - Message offset to commit (from event.offset)
 */
async commitKafkaOffsets(triggerPath: string, topic: string, partition: number, offset: number,): Promise<void>

/**
 * Parse an S3 object from URI string or record format
 * @param s3Object - S3 object as URI string (`s3://storage/key`, `s3:///key`
 *   for the default storage) or record. Any other string throws rather than
 *   falling back to an auto-generated key: an auto key is requested by
 *   omitting the object, and a fallback would silently misplace the upload
 *   on any typo.
 * @returns S3 object record with storage and s3 key
 */
parseS3Object(s3Object: S3Object): S3ObjectRecord

/**
 * Create a SQL template function for PostgreSQL/datatable queries
 * @param name - Database/datatable name (default: "main")
 * @returns SQL template function for building parameterized queries
 * @example
 * let sql = wmill.datatable()
 * let name = 'Robin'
 * let age = 21
 * await sql`
 *   SELECT * FROM friends
 *     WHERE name = ${name} AND age = ${age}::int
 * `.fetch()
 */
datatable(name: string = "main"): DatatableSqlTemplateFunction

/**
 * Create a SQL template function for DuckDB/ducklake queries
 * @param name - DuckDB database name, optionally with a schema as `name:schema` (default: "main")
 * @returns SQL template function for building parameterized queries
 * @example
 * let sql = wmill.ducklake()
 * let name = 'Robin'
 * let age = 21
 * await sql`
 *   SELECT * FROM friends
 *     WHERE name = ${name} AND age = ${age}
 * `.fetch()
 * @example
 * // Target a specific schema within the ducklake
 * let sql = wmill.ducklake("my_lake:analytics")
 */
ducklake(name: string = "main"): SqlTemplateFunction

/**
 * Idempotently materialize `selectSql` into a ducklake table for one
 * partition (or the whole table when `partition` is omitted) — the client-side
 * equivalent of the `// materialize` engine.
 * With `uniqueKey` it upserts the slice (delete-by-key + insert); otherwise it
 * replaces it (whole table → `CREATE OR REPLACE`; partition → delete + insert).
 * Safe to re-run for the same partition (backfill / failure-recovery).
 * 
 * Returns a lazy statement — call `.execute()` to run it:
 * `await wmill.upsertPartition({ table, selectSql, partition }).execute()`.
 */
upsertPartition(opts: DucklakeMaterializeOptions): SqlStatement<any>

/**
 * INSERT-only materialization (no dedup/replace) for append-only tables.
 * Re-running the same partition duplicates rows — use only for immutable
 * event-log sources.
 * 
 * Returns a lazy statement — call `.execute()` to run it:
 * `await wmill.appendPartition({ table, selectSql, partition }).execute()`.
 */
appendPartition(opts: Omit<DucklakeMaterializeOptions, "uniqueKey">,): SqlStatement<any>
