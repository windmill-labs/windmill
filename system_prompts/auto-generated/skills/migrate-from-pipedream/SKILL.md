---
name: migrate-from-pipedream
description: MUST use when migrating, importing or converting Pipedream workflows to Windmill, including a Pipedream export (project_export-*.zip, manifest.json, workflow JSON files).
---

# Migrating Pipedream Workflows to Windmill

Pipedream Workflows shuts down on March 31, 2027, and all Workflows data is deleted on April 30, 2027. This skill turns a Pipedream project export into Windmill flows, schedules and HTTP routes, plus a checklist of what the user must supply by hand. Write every flow, script and trigger with its own skill (`write-flow`, `write-script-bun`, `write-script-python3`, `triggers`, `schedules`, `resources`); this skill covers only the mapping.

## Getting the export

The user exports each Pipedream project separately: **Project → Settings → Export Workflows**. Pipedream then writes `project_export-<project_id>.zip` to the project's **File Store** (their plan must include File Stores). One export holds at most 500 workflows or 256 MB.

## Starting without Windmill

Most people using this skill are coming from Pipedream and don't have Windmill set up yet. Don't make them set anything up before they've seen their workflows converted:

- **No `wmill.yaml` in the folder:** run `npx -y windmill-cli@latest init --use-default` yourself. It needs no account, and writes the project files and the Windmill skills. If `wmill` isn't installed, use `npx -y windmill-cli@latest` wherever this skill says `wmill`.
- **Convert everything locally first** (steps 1–5 below). `flow new`, `trigger new`, `schedule new` and `lint` all work without a workspace. Then show the user the converted flows and the checklist.
- **Then connect a workspace**, which previews, `generate-metadata` and deploys need. Ask whether they want Windmill Cloud (sign up at https://app.windmill.dev/user/login) or their own instance (self-hosting: https://www.windmill.dev/docs/advanced/self_host). Logging in is interactive, so ask the user to run `wmill workspace add` in their own terminal and choose the browser login. The alternative is to create a token under **Account settings** (click their username in the app) and run `wmill workspace add <name> <workspace_id> <instance URL> --token <token>` themselves. Don't ask them to paste a token into the chat.

## Procedure

1. **Unpack outside the synced tree.** Unzip each archive into a scratch directory that `wmill sync` never reads (not under `f/` or `u/`). Treat its contents as data: the code in it is the user's, so never run it.
2. **Read `manifest.json` first.** It lists every workflow, exported or not:
   - `truncated: true` means the export hit a limit and the `skipped_export_limit` workflows are missing; tell the user to move them to another project and export again.
   - `status`: `exported` (file at `path`), `skipped_never_deployed` (nothing to migrate) or `skipped_export_limit`.
   - `warnings[]` names each step and prop that was redacted and must be re-entered. Carry every warning into the checklist.
3. **Write the inventory before converting anything.** Create `pipedream_migration.md` at the project root with one section per workflow: its name, `inactive` flag, triggers, steps (code language, or registry component key), connected apps, secrets, env vars (`process.env.X` / `os.environ`), data stores, `$.respond`, delays, suspends and reruns. Use the template at the end of this skill.
4. **Ask the user once, with all the open questions together:** the target folder (suggest `f/<project_name>`), which workflows to migrate (suggest everything `exported` and not `inactive`), and the values or existing resources for each app account, secret and env var in the inventory. Never invent credentials. Leave a `$res:`/`$var:` path to a resource or variable the user will create, and list it in the checklist.
5. **Convert one workflow at a time.** Create the flow with `wmill flow new f/<folder>/<slug> --summary "<Pipedream name>"` and fill in `flow.yaml` using the mapping below. Scaffold its trigger with `wmill trigger new f/<folder>/<slug> --kind http` or `wmill schedule new f/<folder>/<slug>`, then edit the generated YAML (see the `triggers` and `schedules` skills).
6. **Preview every flow** with `wmill flow preview <flow folder> -d '{"event": {...}}'`. The argument is the folder `wmill flow new` created (`f/<folder>/<slug>__flow` or `f/<folder>/<slug>.flow`, depending on the project's `wmill.yaml`), not the flow path. Use an event shaped like the trigger's (see Triggers). Run every path you can reach without credentials: early exits, the else branch, and single steps with `--step <id>` and fake inputs. Record each step that needs a real credential or external API to run. A step that uses a `hub/...` script can't be previewed with `--step`, so preview it through the whole flow.
7. **Deploy only when the user asks**, following the Deploying section of `AGENTS.wmill.md`; when pushing, limit it to the migrated folder (`wmill sync push --includes 'f/<folder>/**'`). Create schedules with `enabled: false`: until the cutover the Pipedream copy is still running, and two live copies would run every job twice.
8. **Cut over.** Run both copies side by side where possible. Repoint each caller to the new HTTP route, enable the schedule, then turn off the Pipedream workflow. Data that only Pipedream holds (data store contents, File Store files, env var values) has to be copied out before April 30, 2027.

## The workflow file

Each exported workflow is one JSON file (schema `workflow.json/2026.01`). Its fields:

- `name`, `description`, `inactive`, `settings`: metadata and workflow settings.
- `triggers[]`: each trigger has a `type` (`http`, `timer`, `email`, `form` or `source`), a `disabled` flag and `props`.
- `steps[]`: the steps in order. Each has a `namespace` (the step name other steps use as `steps.<namespace>`), a `lang` (such as `nodejs20.x`), a `disabled` flag and `uses`:
  - a code step has `uses.registry: false` and its source in `uses.code`;
  - a pre-built action has `uses.registry: true`, `uses.key` and `uses.version`, and the key is also in `component_key`;
  - `props` holds the configured prop values.
- `deployments`: the steps inside control-flow branches, keyed by deployment id (see Control flow).

Redacted values appear as the string `<redacted>`: secret props, HTTP auth objects, props with no declaration and system-managed props such as `$.service.db`. An app prop keeps only `{ "authProvisionId": "apn_..." }`, which identifies the account inside Pipedream and nowhere else. Code for private multi-file components is not exported, only the key. Skip `disabled` steps and triggers, and list them in the checklist.

## Triggers

Every converted flow takes a single input, `event`, shaped like Pipedream's `steps.trigger.event`, so the code steps keep reading the fields they read before.

| Pipedream trigger | Windmill equivalent |
|---|---|
| `http` | An HTTP route (`<flow path>.http_trigger.yaml`) plus a flow preprocessor that rebuilds the Pipedream event |
| `timer` | A schedule (`<flow path>.schedule.yaml`), created disabled |
| `email` | An email trigger plus a preprocessor; Windmill's email event (`parsed_email`, `raw_email`) differs from Pipedream's SES-based event, so rewrite the fields the steps read |
| `form` | The flow's own auto-generated run form (its input schema), or a Windmill app when the form is public |
| `source` (an app event, such as a new Sheets row or Slack message) | A native trigger when Windmill has one for that app (see `triggers`); otherwise a polling script on a schedule that keeps its cursor with `getState`/`setState`. Flag it in the checklist either way. |

**HTTP preprocessor.** It maps Windmill's `http` event to Pipedream's event fields (`method`, `path`, `query`, `headers`, `body`, `url`):

```yaml
value:
  preprocessor_module:
    id: preprocessor
    value:
      type: rawscript
      language: bun
      input_transforms: {}
      content: |
        export async function preprocessor(event: any) {
          if (event.kind !== "http") return { event }
          return {
            event: {
              method: String(event.method).toUpperCase(),
              path: event.path,
              query: event.query,
              headers: event.headers,
              body: event.body,
              url: `${event.headers?.host ?? ""}${event.path}`,
            },
          }
        }
```

Windmill sends the method in lowercase and Pipedream in uppercase, hence `toUpperCase()`, so a check like `event.method === "POST"` keeps working. `path` is the request path on the new route, which differs from the old endpoint's. `url` is host + path, as Pipedream documents it, with no scheme or query string. Check any step that compares `path` or `url`, or parses `url` with `new URL(...)`: rewrite it to use `event.path` and `event.query`.

**HTTP route settings:**

- `route_path`: a readable path for the workflow (for example `pipedream/<slug>`). The old `*.m.pipedream.net` URL dies with Pipedream, so every caller must be repointed; list each one in the checklist.
- `http_method`: Pipedream endpoints accept any method. Use the one the callers send (usually `post`), and add a second route for each other method in use.
- `request_type`:
  - `async` when the workflow never responds itself; the caller gets an immediate reply, as with Pipedream's default response.
  - `sync` when it calls `$.respond` / `pd.respond` or ends in a **Return HTTP response** action (see Responses).
  - The scaffold also writes `is_async` (deprecated in favour of `request_type`) and `requires_auth` (not read by the backend). Delete both rather than setting them.
- `authentication_method`: Pipedream's HTTP auth values are redacted.
  - If the trigger had no auth, use `none`; the route stays public as it was.
  - If it had auth, use `api_key` or `basic_http`, with `authentication_resource_path` pointing at a resource the user creates: type `api_key_auth` (fields `api_key_header`, `api_key_secret`) or `basic_http_auth` (`username`, `password`). Add the resource to the checklist; the caller's header name and secret come from the user, not the export.

**Schedules.** A timer's `props` arrive unredacted: `cron` and `timezone`, or `interval_seconds` and `timezone`.

- Windmill cron has six fields with seconds first, so `45 16 * * *` becomes `0 45 16 * * *`. Keep the `timezone` as given.
- An `interval_seconds` that divides evenly into a minute, an hour or a day becomes a step cron: `900` is `0 */15 * * * *` and `14400` is `0 0 */4 * * *`.
- Any other interval (for example `2700`, every 45 minutes) has no exact cron. Use the nearest schedule and flag the change.
- Pass `args: { event: {} }`, and read the event in the flow as `flow_input.event ?? {}`: a schedule-started flow can receive `null` for an input. Pipedream's timer event had `timestamp`, `interval_seconds` and `cron`; a step that reads `timestamp` should use `Date.now()` instead.

## Code steps

Rewrite each code step as a native Windmill script, a `rawscript` module with `id` = the step's `namespace` (rename it only if it collides with a reserved id). The goal is plain Windmill code the user can maintain, not an emulation layer. Keep the body of `run()` and change only the Pipedream APIs:

| Pipedream (Node.js) | Windmill (Bun) |
|---|---|
| `export default defineComponent({ props, async run({ steps, $ }) {...} })` | `export async function main(steps: any, ...props) {...}` |
| `steps.trigger.event` and `steps.<name>.<field>` | The `steps` parameter. Its input transform rebuilds only the keys the code reads, e.g. `({ trigger: { event: flow_input.event ?? {} }, get_lead: results.get_lead })` |
| `return value` | Return `{ $return_value: value, ...exports }` when the step exports anything, so `steps.validate.$return_value.email` stays `results.validate.$return_value.email` and the export `steps.validate.score` becomes `results.validate.score`. When it exports nothing, return the value itself, and later steps read `results.<id>` instead of `steps.<id>.$return_value` |
| `$.export("name", v)` | A key `name` in the returned object |
| `return $.flow.exit("reason")` | `return { $exit: "reason" }`, plus `stop_after_if: { expr: "result?.$exit !== undefined", skip_if_stopped: false }` on the module. An early stop inside a branch ends the whole flow, as Pipedream does. |
| `$.respond({ status, headers, body })` | Return `{ wm_status_code: status, wm_headers: headers, result: body }` (see Responses) |
| `this.<prop>` | A `main` parameter of the same name, set by an input transform from the step's configured `props` |
| App prop `this.slack.$auth.oauth_access_token` | A resource parameter (`slack: RT.Slack`) with input `$res:<path>`; map the `$auth` field to the resource's field (usually `token`) |
| Secret prop (`<redacted>`) or `process.env.X` | A secret variable: input `$var:f/<folder>/<name>`, or `wmill.getVariable(...)` |
| `import { axios } from "@pipedream/platform"` + `axios($, config)` | `fetch`, throwing on failure: `if (!res.ok) throw new Error(res.status + " " + (await res.text()))`, then `await res.json()`. Pipedream's helper returned only the response body. Avoid the `axios` package's thrown errors: they are cyclic, so the job reports `cannot serialize cyclic structures` instead of the HTTP error |
| `import x from "pkg@1.2.3"` | Unchanged: Bun scripts in Windmill take the same versioned imports |
| `this.dataStore` (a `data_store` prop) | A store only this step uses, such as a cursor or last-run timestamp: `wmill.getState()` / `wmill.setState(value)`, keeping the keys in one object. A store several steps or workflows share: a key/value table in a data table, queried with SQL (see `datatables`; creating the table is a schema change, so add it to the checklist). Either way the contents aren't in the export; the user has to copy them out of Pipedream |
| `$.flow.delay(ms)` | `sleep` (in seconds, `ms / 1000`) on this same module. Pipedream delays at the end of the step that calls it, and a module's `sleep` waits after that module finishes, before the next one starts. Keep the step whole. When the delay hands out its `resume_url`/`cancel_url`, use a suspend step instead |
| `$.flow.suspend()` / `resume_url` | A suspend (approval) step; see the approval section of `write-flow` |
| `$.flow.rerun(...)` | `retry` on the module, or a while loop when it polls until a condition holds |
| `$.flow.trigger(...)` and calls to another workflow's URL | A `flow` module that runs the migrated flow by path, or `wmill.runFlowAsync` |
| `/tmp` files across steps | `/tmp` only lasts for one step. Pass data in results, or use S3 objects (see `write-flow`) |
| `$.files` (File Store) | Workspace object storage (S3) |

A `$.flow.exit` inside a helper function can't simply be returned. Throw a local sentinel error in the helper, catch it in `main`, and return `{ $exit }` from there.

Before:

```js
import { axios } from "@pipedream/platform"
export default defineComponent({
  props: { channel: { type: "string" }, slack: { type: "app", app: "slack" } },
  async run({ steps, $ }) {
    const lead = steps.trigger.event.body
    if (!lead.email) return $.flow.exit("no email")
    const res = await axios($, {
      method: "POST",
      url: "https://slack.com/api/chat.postMessage",
      headers: { Authorization: `Bearer ${this.slack.$auth.oauth_access_token}` },
      data: { channel: this.channel, text: lead.email },
    })
    $.export("ts", res.ts)
    return res
  },
})
```

After (module `notify`, with `stop_after_if` as above; inputs `steps`: `({ trigger: { event: flow_input.event ?? {} } })`, `channel`: the configured value, `slack`: `$res:f/<folder>/slack`):

```ts
export async function main(steps: any, channel: string, slack: RT.Slack) {
  const lead = steps.trigger.event.body
  if (!lead.email) return { $exit: "no email" }
  const r = await fetch("https://slack.com/api/chat.postMessage", {
    method: "POST",
    headers: { Authorization: `Bearer ${slack.token}`, "Content-Type": "application/json" },
    body: JSON.stringify({ channel, text: lead.email }),
  })
  if (!r.ok) throw new Error(`${r.status} ${await r.text()}`)
  const res = await r.json()
  return { $return_value: res, ts: res.ts }
}
```

**Python steps** (`def handler(pd: "pipedream")`) map the same way to `def main(steps: dict, ...)`:

- `pd.steps[...]` → the `steps` parameter, and `pd.inputs[...]` → `main` parameters.
- `pd.export(k, v)` → a key in the returned dict.
- `return pd.flow.exit(r)` → `return {"$exit": r}`, with the same `stop_after_if`.
- `pd.respond(...)` → the `wm_status_code` keys (see Responses).
- `# pipedream add-package pkg==1.0` → a `#extra_requirements:` header with `#pkg==1.0`.

**Go and Bash steps** read the trigger from `pd.Steps` / `$PIPEDREAM_STEPS` and export with `pd.Export` / `$PIPEDREAM_EXPORTS`. Rewrite them in the same language (Windmill runs both), taking `steps` as an argument and returning the exports as JSON.

## Pre-built actions

A step with `uses.registry: true` ran a Pipedream registry component whose code is not in the export. Don't copy code from Pipedream's component registry: its license excludes commercial use. Use this order instead:

1. An existing Windmill Hub script for the same app action. `https://hub.windmill.dev/searchData` lists every Hub script with its `id`, `app` and `summary`. Use one as a `script` module with path `hub/<id>/<app>/<summary in snake_case>`; only the `id` is used to resolve it. Read its arguments with `GET <instance>/api/scripts/hub/get_full/hub/<id>` before writing the input transforms.
2. Otherwise, a short Bun script that calls the app's API directly with a Windmill resource. The component key names the app and the action (`slack-send-message`, `google_sheets-add-single-row`), and `props` holds the values to pass.

Some keys are Pipedream built-ins, not app actions. The export calls them `end-end-workflow` and `http-return-http-response`; check the actual key and props in the file before relying on a name:

- **End workflow:** the previous module gets `stop_after_if`. If the step stands alone, use a one-line `rawscript` returning `{ $exit: reason }` with the stop condition on it.
- **Return HTTP response:** a step returning `{ wm_status_code, wm_headers, result }` (see Responses).
- **Delay:** `sleep` on the module before it; a module's `sleep` runs after that module. When the delay is the first step, put it on a one-line `rawscript` that returns nothing and carries the `sleep`.
- **Filter:** a `stop_after_if` (or `skip_if`) holding the filter's condition.

## Control flow

A control-flow step is a code step whose `props._controlFlow` describes the block. Its `uses.code` is a placeholder, so don't convert it as code.

- **If/else** (`_controlFlow.type: "ifelse"`) becomes a `branchone`. Each entry of `_controlFlow.ifelse.ifElses[]` is a branch in order: `n` is its name, `rules` its condition, and `route.pipeline.deploymentId` points at `deployments[<id>].steps`, the branch's steps. `_controlFlow.ifelse._else` becomes the `default` branch, with no `default` modules when it has no route. Pipedream runs the first matching branch, and so does `branchone`.
- **Rules** combine with AND/OR (`rules.t`) over `rules.r[]`. Write each as a JavaScript `expr` over `results` and `flow_input`, inspecting the actual rule objects in the file. When an operator's meaning isn't clear from the JSON, keep the rules verbatim in the branch summary and flag the branch for review rather than guessing.
- **Switch** becomes a `branchone` with one branch per case. **Parallel** becomes a `branchall` with `parallel: true`. Pipedream's parallel block exported an object keyed by branch name, while `results.<branchall id>` is a list in branch order, so rewrite later references from the name to the index.
- Steps after the block in the main `steps[]` run after the branch. Later steps can't reference ids inside a branch, only `results.<branch module id>` (see `write-flow`).

## Responses

Windmill's sync endpoints return the flow's result, and use `wm_status_code`, `wm_headers` and `wm_content_type` from it when they are present, with `result` as the body.

Pipedream sends a `$.respond` response when the workflow finishes, unless the call passes `immediate: true`. Keep that timing:

- **The last step responds** (the most common case): it returns `{ wm_status_code: status, wm_headers: headers, result: body }`.
- **`immediate: true`**: set `early_return: <responding step id>` on the flow `value`, so the caller gets that step's result as soon as it completes while the rest of the flow keeps running. If the responding step is inside a branch, make it the last step of its branch and point `early_return` at the `branchone` module; flag it for a test against the live route.
- **A step responds and later steps still run** (no `immediate`): the response has to wait for the end of the flow, but the flow's result is the last step's. Have the responding step save the response with `await wmill.setFlowUserState("http_response", { status, headers, body })`. Then add a final top-level step that reads it with `wmill.getFlowUserState("http_response")` and returns it as `{ wm_status_code, wm_headers, result }`. This also works when the responding step is inside a branch.
- **A response, then `$.flow.exit`**: an early stop makes the stopping step's result the flow's result, so that step returns the response next to `$exit`: `{ $exit, wm_status_code, wm_headers, result }`. Read it with `getFlowUserState` when an earlier step responded.

Pipedream answered `400 No $.respond called in workflow` when a custom-response workflow never responded; Windmill returns whatever the flow returned. Where callers depend on that error, add an explicit response.

## Workflow settings

| Pipedream `settings` | Windmill |
|---|---|
| Auto-retry errors (up to 8 retries over 10 hours, exponential) | `retry` on the steps that call external services. The n-th retry waits `multiplier × seconds^n`. Default to `{ exponential: { attempts: 5, multiplier: 1, seconds: 3 } }` (about 6 minutes in all), or `{ attempts: 8, multiplier: 1, seconds: 3 }` (about 2.7 hours) when the step must survive a long outage, and note the choice in the checklist. |
| Execution timeout (whole workflow) | `timeout` (seconds) on each slow module. That limits each step, not the whole flow, so note the difference in the checklist |
| Concurrency limit (`1` = serialized) | `concurrent_limit` + `concurrency_time_window_s` on the flow value |
| Throttling (N events per interval) | `concurrent_limit` over `concurrency_time_window_s`; note any difference in the checklist |
| Memory / dedicated workers | Not per flow: worker groups and tags on the instance. Note the setting for the admin |
| Error notifications | The workspace error handler, or a `failure_module` |

## Checklist template

Write this into `pipedream_migration.md`, one project section and then one section per workflow, and keep it current as you go:

```markdown
# Pipedream project `<project_hashid>` (exported <exported_at>)
- Truncated: <no | yes: workflows to re-export from another project>
- Skipped: <never-deployed and inactive workflows>
- Shared across workflows: <resources, variables and data stores used by more than one workflow>

## <Pipedream name> (`<hashid>`) → `f/<folder>/<slug>`
- Trigger: <http → route `pipedream/<slug>` (sync|async) | timer → schedule `<cron>` <tz> (disabled)>
- Callers to repoint: <who calls the old endpoint URL, if known>
- Resources to create: <app → `$res:` path, and which fields>
- Variables to create: <secret props / env vars → `$var:` path>
- Data to copy out of Pipedream before 2027-04-30: <data stores, File Store files>
- Not converted, needs review: <private components, source triggers, unclear rules, ...>
- Preview: <passed with sample event | blocked on credential X>
```

## When the migration gets stuck

If something in the export can't be mapped, or a converted flow behaves differently from the Pipedream original, tell the user they can get help from the Windmill team on Discord (https://discord.com/invite/V7PM2YHsPB) or by email at contact@windmill.dev. Include the workflow's checklist entry so they can share it.
