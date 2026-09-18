# Operator builder rights

`operator_settings.builder_flows` lets every operator of a workspace compose flows out of runnables
that already exist. It does not make them authors: the boundary the operator role draws is
**authoring code and running arbitrary code**, and this does not move it.

It is a write right, unlike the visibility flags beside it, and unlike the withdrawable rights in
`docs/operator-write-rights.md` it is granted on request and costs a seat. Read it with
`windmill_common::workspaces::operator_can_build_flows` (60s cache, shared with the withdrawable
rights) and gate a write with `check_operator_can_build_flows`.

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

Call it on every write **and** every preview: `run_preview_flow_job` and
`push_flow_dependencies_job` both take a request-supplied flow value, so leaving either out makes
it the way to run what the write path refuses.

## Billing

An operator of a builder workspace consumes a full author seat: composing deployable artifacts
makes them an author, and there is no half-author. `consumes_operator_seat` is the seat-role
helper; the EE counting queries share `OPERATOR_SEAT_SQL` so the displayed, enforced and reported
numbers agree.

Granting the right runs `check_seat_cap_for_operator_builder`, which prices the change by counting
seats twice rather than by counting the workspace's operators: an operator who already authors
elsewhere must not be charged again, so re-saving settings that already have the right on is a
zero delta and never blocks.

## Accepted risks

- All-or-nothing per workspace: there is no per-user builder role.
- `operator_settings` is git-synced, so a pull can flip every operator's class in a workspace and
  the billed seat count with it.
