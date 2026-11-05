# Workspace roles

What a non-admin member may author, manage and see is its **role**. `developer` and `operator` are
the default roles; a licensed EE instance defines its own list in the `workspace_roles` global
setting (at least one role), and any workspace may override a role's capabilities in
`workspace_settings.role_overrides`, or, when licensed, add roles of its own there. A lapsed
license freezes them rather than dropping them (`custom_roles_enabled`): the stored list and the
workspace-only roles keep applying, writes that change them are refused, and resetting the list to
the defaults or removing a workspace-only role stays open. Falling back to the defaults instead
would hand a restricted `developer` everything. Workspace admin is not a
role: it is the `is_admin` flag on top of whichever role a member holds, and holds everything. The
code is `windmill_common::roles`.

## Capabilities

| Capability | Values | Enforced |
|---|---|---|
| `code` | bool | yes: scripts, previews, dependency jobs, dynamic selects, resolving jobs, data table migrations, eval datasets |
| `flows` | `none` / `compose` / `full` | yes; also AI agents, whose tools are checked as composed steps |
| `apps` | `none` / `compose` / `full` | yes; low-code apps and worker-built apps need `full` |
| `manage_schedules`, `manage_triggers` | bool | yes, see `docs/operator-write-rights.md` |
| `run` | bool | yes, see [Runs](#runs) |
| `fork` | bool | yes, at both fork routes; unset by every layer, it is granted exactly to an unrestricted role |
| `visibility.*` | bool per page | no, UI only |

`compose` is the former operator builder right: the composition-only checks of
`docs/operator-builder-rights.md` run for it and are skipped only at `full`. `full` flows or apps
without `code` is refused at write, since both carry inline code.

Stored values are **patches**: an unset key falls through to the layer below, so a key added later
reads as "never configured". The layers, top first: the workspace override, the instance
definition, then `RoleCapabilities::base_for` (`FULL` for `developer`, the legacy operator for any
other name). A role name no longer defined resolves to `NONE`, so deleting a role fails closed;
deleting one that a member, invite, auto-invite or instance group mapping still names is refused,
and so is deleting an instance role that a workspace still overrides (the override would fall onto
`base_for` instead of the definition it was written against).

## Runs

`run` withdrawn makes a role view-only: its members see what folder and item rights grant,
runs, logs and results included, and start nothing. `gate_role_runs` sits on the jobs router and
refuses every path under `run/`, `run_wait_result/`, `run_and_stream/`, `run_inline/`, `restart/`
and `add_batch_jobs/` (GET included, which starts runs there), so a run route added under those
prefixes is gated for free. It lets through the dependency-lock and dynamic-select routes, which
`code` gates, and `run/workflow_as_code`, a step of a run already started. The three script
preview routes (`run/preview`, `run_inline/preview`, `run_wait_result/preview`) check `run` in
their handlers instead, after the exemption that lets a running job's SDK send a data table query
there, since jobs a view-only member owns still run as them. Callers that push
without the router check `check_role_can_run` themselves: the MCP per-runnable tools, eval
experiment runs and members-only Slack agent shares (which run as the asking member). Resuming or approving a suspended step is not a run. Schedules and triggers keep
their own rights, apps run their runnables on the app's policy and are not gated, and a job
started by a schedule or trigger acts as its owner, so from inside it a view-only owner cannot
start further runs through the API either.

Unlike `fork`, withdrawing `run` makes a role restricted: it is an enforced gate on hot routes,
and the fast path is what spares every unrestricted caller a role lookup on each run. It does not
change seats: a role is a full seat by what it authors, not by whether it runs.

## Forks

A member's role carries into a fork: the forker, and every member `copy_members` or
`add_admins_and_developers_to_forks` brings, joins the fork with the role they hold in the parent,
and the fork gets the parent's `role_overrides` so the name means the same there. A protection
ruleset's `fork_role_mapping` (`{ role here: role in the fork }`) replaces that role; where several
rulesets map one role, the first by name wins. A role named by a mapping cannot be deleted. A
fork's non-admin creator may add a parent member only when the mapping makes them a developer, so
re-adding cannot undo a mapping that restricts.

`fork`, like `visibility`, does not make a role restricted: withdrawing it from `developer` must not
turn every developer into an operator for the raw `is_operator` checks. So the fork routes read it
from the member's role (`caller_can_fork`), never through `caller_capabilities`' fast path.

## Who is restricted

`ApiAuthed::is_operator` means "this member's role is restricted", computed at authentication from
the role, never from `usr.operator` alone. Every gate fast-paths on it (`caller_capabilities`
returns `FULL` for an unrestricted or admin caller without reading the role), so it must stay
derived from the role: a stale flag lets a restricted member through. The authentication cache
holds it for up to 120s.

`usr.operator` is kept as "this role cannot write code", which is what clients that predate roles
read it as. A member's role is `usr.workspace_role`, or, while that is NULL, `operator` /
`developer` from `usr.operator`: rows written by paths that know nothing of roles stay meaningful.
The old `usr.role` column is unrelated and holds legacy display strings.

## Seats

A role that writes or composes anything costs a full seat; anything else half. Billing SQL cannot
merge capability patches, so `role_seat` materializes the author flag per role
(`workspace_id = ''` for the instance, a row per workspace override), rewritten by
`sync_role_seats` after every write to either setting. Every count reads it through
`AUTHOR_SEAT_SQL`, and usage telemetry reports builders (authors anywhere) against non-builders the
same way. A change that adds seats under an offline license is priced inside the transaction that
writes it, by `check_seat_cap_for_role_change`.

## Legacy `operator_settings`

`/operator_settings` still works: it is applied as an override of the `operator` role, dropping any
key equal to what the instance gives that role, and the payload is mirrored into the
`operator_settings` column for git sync and older servers. The migration moved every key a
workspace had changed from its default into `role_overrides.operator` the same way. The workspace
list still returns an `operator_settings` object for a restricted member, synthesized from its
role, alongside `role` and `capabilities`.

## In the settings document

`role_overrides` in the settings document (export `settings.json`, `settings.yaml`, git sync) is
the stored map, except that the entry of a role the instance does not define carries
`workspace_only: true`. The stored map cannot tell an override of instance role X from adding role
X, and the target of a promotion may lack X. So the apply refuses the whole document, dry run
included, before writing anything, if:

- an unmarked entry names a role the target instance does not define, unless the target workspace
  already has it as a workspace-only role (a file pulled before the marker existed), or
- a marked entry names a role the target instance defines.

The marker is never stored. Roles are written before every other setting, so an instance group
mapping in the same file can name a role it adds. A workspace-only role the file removes is kept
until a final write, after whatever named it has been rewritten, so the in-use check passes.

The document's `operator_settings` is derived from the effective `operator` role, not read from
the column: only the legacy endpoint rewrites the column, so after a role edit it holds stale
flags. A document that states `role_overrides` never applies `operator_settings`, and warns when
it was edited to disagree with the operator role. A document without `role_overrides` (pulled
before roles existed) applies it as before, diffed against the column.
