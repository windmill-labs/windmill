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

## Git sync and export

The settings document (`settings.yaml`, `settings.json` in an export) states `role_overrides` in
two keys, split by the instance that writes it: `role_overrides` for overrides of instance roles,
`workspace_only_roles` for roles the workspace adds. The stored column cannot tell them apart, and
a promotion target may define different instance roles. Each key is applied only when stated.
An override naming a role the target instance does not define refuses the whole apply rather than
skipping it: stored, it would become a workspace-only role; dropped, the role would take the
instance's capabilities once defined there. A pulled file also states the `operator` role as
legacy `operator_settings`: the roles are written after it, and an `operator` entry the file
leaves as it was pulled takes what the legacy write stored rather than undoing it.

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
