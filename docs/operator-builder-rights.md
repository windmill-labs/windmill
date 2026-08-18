# Operator builder rights

A workspace setting (`operator_settings.builder`) that lets every operator of that workspace
compose flows and full-code apps out of runnables that already exist. It does not make them
authors: the boundary the operator role draws is **authoring code and running arbitrary code**,
and builder rights do not move it.

Read the flag with `windmill_common::workspaces::operator_builder_enabled` (60s cache). Gate a
write with `check_operator_can_build`. The cache is per process, so revoking the setting has to
reach every replica: an `AFTER UPDATE OF operator_settings` trigger writes a
`notify_operator_settings_change` row and `process_notify_event` drops the entry. Keep both ends
if you touch either, or a revoked workspace keeps authorizing writes on every other replica until
its own entry expires.

## What the check has to cover

`check_flow_is_composition_only` (`windmill-common/src/flows.rs`) walks a `FlowValue` and refuses
anything that carries code. Three of its rules exist because the obvious walk misses them:

- **`FlowScript` and any populated `modules_node` / `default_node`.** These name code hoisted into
  a `flow_node` row. Only the dependency job produces them, so an authored value carrying one
  names code stored under some other flow. The walk covers `modules`, so a node reference is a way
  past it.
- **An AI agent step's `tools`.** `ToolValue::FlowModule` wraps a whole `FlowModuleValue`, so a
  tool can be a raw script.
- **An AI agent step's `agent` link.** A linked agent resolves its tools from an `ai_agent`
  resource at run time, and operators may write resources, so the tool list is outside this check
  and can be swapped for a raw script after the flow is approved.

It also returns what a value-only walk cannot authorize, for the caller to check against its own
permissions:

- **the worker tags the steps pin**, or a builder routes a job onto a privileged worker group;
- **every runnable a step references**. `script_to_payload` resolves a step's path with the root DB
  handle (`db_authed = None`) and returns the referenced runnable's `on_behalf_of`, which
  `worker_flow` then applies to the step job. So composing a path is enough to run it, and to run
  it as whoever it runs as: `validate_operator_composed_flow` re-checks each path under the
  caller's RLS. This is the general case; the one below is on top of it, not instead of it.
- **the `(path, hash)` of every version-pinned step**. A step carrying a `hash` is dispatched by
  that hash alone, with the path beside it never consulted, so a readable path paired with another
  script's hash still runs that other script.

The same reasoning applies to a builder-authored app, with one extra step. `execute_component`
resolves the runnable it runs on the root handle, so `validate_operator_composed_app` checks every
referenced path under the caller's RLS and refuses hub ones. But it has to check **two** surfaces,
because they are not the same list: the policy's `script/<path>` and `flow/<path>` triggerables,
and the `runnableByPath` entries in the app value, which is what the deployed bundle resolves a
`runnable_id` against and sends.

Reading a triggerable key is not a `split_once(':')`: `execute_component` looks up
`format!("{component}:{path}")` with an unrestricted component string, so `a:b:script/x` resolves
at run time for `component = "a:b"`. Every colon is a possible split, so the check validates every
suffix that parses as a runnable rather than guessing which one a request will use.

What makes those checks bind is that **`ExecutionMode::Viewer` is refused for a builder app**. In
Viewer mode `execute_component` falls back to a default triggerable for any `script/`/`flow/`
path, so the policy stops being the list of what the app may invoke, and the job runs as the
*viewer*: an admin who merely opened the app would run anything in the workspace as themselves.
`Publisher` and `Anonymous` have no such fallback. If you ever relax the Viewer refusal, the
deploy-time path checks above stop being an authorization boundary.

`execute_component`'s preview branch refuses a hub path for operators too:
`require_path_read_access_for_preview` admits `hub/` for everyone.

Call it on every write **and** every preview: `run_preview_flow_job` and
`push_flow_dependencies_job` both take a request-supplied flow value, so leaving either out makes
it the way to run what the write path refuses.

## The two raw-app deploy paths are not equivalent

`create_app_raw` / `update_app_raw` are multipart: the browser already built the bundle, and
nothing server-side compiles anything. Builders use these.

`create_app_raw_source` / `update_app_raw_source` push a bundler CLI over caller-supplied `files`
as a job **on a worker**, which is arbitrary code execution and is why they already require `jobs:run`
on top of `apps:write`. They stay closed to operators, builder rights or not. Do not "tidy" the
exception away: it costs builders nothing, because the browser and the CLI both bundle locally and
deploy through the multipart endpoints.

A builder-authored app is forced to `policy.sandbox = true` (`check_operator_composed_app`). That
is what makes it safe to let an operator publish a bundle nobody reviewed: without it the bundle
runs same-origin with each viewer's Windmill session.

The same check refuses a `rawscript/<sha>` key in either triggerables map. That key is the
deployed app's authorization to run caller-supplied `raw_code` hashing to it (`execute_component`'s
run mode authorizes inline code by the policy alone; its operator guard only covers preview mode),
so pinning one would be arbitrary code execution behind a value that passed the inline-script
check. It also uses `app_value_has_inline_script` rather than `traverse_app_inline_scripts`: the
latter only reports a script whose `language` parses, and the author picks the fields.

## Accepted risks

- A builder raw app may declare `frontend_sdk_scopes`, and `mint_raw_app_sdk_token` mints as the
  *viewer*. A consenting admin therefore hands the bundle a 12h admin-identity token within the
  curated scope list. The viewer consent prompt is the gate. The lever, if this is ever revisited,
  is dropping `variables:read` / `resources:read` from `FRONTEND_SDK_ALLOWED_SCOPES` for builder
  apps.
- All-or-nothing per workspace: there is no per-user builder role.
- `operator_settings` is git-synced, so a pull can flip every operator's class in a workspace and
  the billed seat count with it.

## Billing

An operator of a builder workspace consumes a full author seat. `consumes_operator_seat` is the
seat-role helper; the EE counting queries share `OPERATOR_SEAT_SQL` so the displayed, enforced and
reported numbers agree. Enabling the setting runs `check_seat_cap_for_operator_builder`, which
prices the change by counting seats twice rather than by counting the workspace's operators: an
operator who already authors elsewhere must not be charged again.
