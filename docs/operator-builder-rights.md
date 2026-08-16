# Operator builder rights

A workspace setting (`operator_settings.builder`) that lets every operator of that workspace
compose flows and full-code apps out of runnables that already exist. It does not make them
authors: the boundary the operator role draws is **authoring code and running arbitrary code**,
and builder rights do not move it.

Read the flag with `windmill_common::workspaces::operator_builder_enabled` (60s cache, invalidated
on write by `update_operator_settings`). Gate a write with `check_operator_can_build`.

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

It also returns the worker tags the steps pin. Authorize them (`check_tag_available_for_workspace_internal`)
or a builder routes a job onto a privileged worker group.

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
