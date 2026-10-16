# Workspace roles

What a non-admin member may author, manage and see is its **role**. `developer` and `operator` are
the default roles; an EE instance defines its own list in the `workspace_roles` global setting (at
least one role), and any workspace may override a role's capabilities in
`workspace_settings.role_overrides`, or, on EE, add roles of its own there. Workspace admin is not a
role: it is the `is_admin` flag on top of whichever role a member holds, and holds everything. The
code is `windmill_common::roles`.

## Capabilities

| Capability | Values | Enforced |
|---|---|---|
| `code` | bool | yes: scripts, previews, dependency jobs, dynamic selects, resolving jobs, data table migrations, eval datasets |
| `flows` | `none` / `compose` / `full` | yes |
| `apps` | `none` / `compose` / `full` | yes; low-code apps and worker-built apps need `full` |
| `manage_schedules`, `manage_triggers` | bool | yes, see `docs/operator-write-rights.md` |
| `visibility.*` | bool per page | no, UI only |

`compose` is the former operator builder right: the composition-only checks of
`docs/operator-builder-rights.md` run for it and are skipped only at `full`. `full` flows or apps
without `code` is refused at write, since both carry inline code.

Stored values are **patches**: an unset key falls through to the layer below, so a key added later
reads as "never configured". The layers, top first: the workspace override, the instance
definition, then `RoleCapabilities::base_for` (`FULL` for `developer`, the legacy operator for any
other name). A role name no longer defined resolves to `NONE`, so deleting a role fails closed;
deleting one that a member, invite, auto-invite or instance group mapping still names is refused.

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
