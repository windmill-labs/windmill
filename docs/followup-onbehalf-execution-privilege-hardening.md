# Follow-up: execution-time on-behalf privilege hardening (post GHSA-hfh4-cx4h-3fcr)

**Pick this up immediately after PR #10124 (GHSA-hfh4-cx4h-3fcr) merges.**

## What #10124 fixed

A job's `WM_TOKEN` (whose identity is an app/flow/schedule/trigger `on_behalf_of`
that a `wm_deployers` member controls) could satisfy superadmin authorization.
#10124 makes a `WM_TOKEN` never count as a global superadmin:

- `ApiAuthed.job_id` is stamped from the resolved token; `require_super_admin(db, &ApiAuthed)`,
  `is_super_admin_authed(db, &ApiAuthed)` and `require_devops_role(db, &ApiAuthed)` reject
  `job_id.is_some()`.
- All `require_super_admin` / `require_devops_role` sites and the direct
  `is_super_admin_email(&authed.email)` boolean gates on **request handlers** (workspace
  deletion, fork drops, dev-workspace attach/archive, object-storage SSRF exemption, custom
  dbname, EE GHES + connected repositories, CUSTOM_INSTANCE_DB) were migrated.
- A `job_id` claim that does not parse as a uuid now rejects the token instead of
  resolving to `None` (which would have cleared the provenance and uncapped it).
- Defense in depth at store time: `validate_on_behalf_of` refuses the reserved internal
  sentinels (`superadmin_secret@`, `superadmin_notification@`, `superadmin_sync@`) as an
  `on_behalf_of` on apps/flows/scripts/schedules/triggers, and app execution refuses a
  policy carrying one — which also covers already-persisted and forked-app rows.

## What is deliberately left for this follow-up

The remaining gaps are **execution-time** decisions that derive superadmin from the
job's *preserved on-behalf email* rather than from a request `ApiAuthed`, so the
`job_id` guard can't reach them. They were flagged by the CI Codex/Pi reviews on #10124.

### 1. Worker-tag enforcement on scheduled fires and flow sub-steps (primary ask)

- `backend/windmill-queue/src/schedule.rs` (~L536) and
  `backend/windmill-worker/src/worker_flow.rs` (~L4361) call
  `check_tag_available_for_workspace_internal` with `is_super_admin` derived from the
  runnable's on-behalf email. A `wm_deployers` member can deploy a runnable with a
  **restricted tag** (deploy and schedule-create do **not** validate the tag today) and
  `preserve_on_behalf_of` → a superadmin email; the scheduled/flow execution then passes
  the restricted-tag gate as "superadmin".
- Direct/interactive runs, preview (`run_preview_script`), and triggers already gate the
  tag against the *acting* `ApiAuthed` via `check_tag_available_for_workspace` (now
  job-aware). The scheduled/flow execution path is the only one that trusts the on-behalf
  email.
- **Proper fix (do this):** validate the worker tag at **create/update time against the
  real actor** — in schedule create/update, trigger create/update, and script/flow deploy —
  using `is_super_admin_authed` (job-aware). A non-superadmin (or `WM_TOKEN`) then cannot
  persist a restricted tag in the first place, and execution can trust the stored tag.
  This does not regress a legitimate superadmin's own restricted-tag schedule (they pass
  the create-time check as themselves). Decide how to treat already-persisted runnables
  carrying restricted tags (re-validate on next edit vs. grandfather).
- **Verify the trigger firing path**: `windmill-trigger/src/trigger_helpers.rs` (~L883)
  calls the wrapper with an `authed`. Confirm whether, when a trigger fires, that `authed`
  is the real actor or the reconstructed on-behalf identity (with `job_id: None`). If the
  latter, triggers share this exact weakness and need the same create/update-time gate.

### 2. Cloud enqueue quota exemption in `push()`

- `backend/windmill-queue/src/jobs.rs` (~L5184) exempts jobs from free-user / queue /
  concurrency / past-due limits when `is_superadmin_cached(db, email)` is true (plus the
  explicit `SUPERADMIN_*` sentinel exemptions just below). A superadmin-email `WM_TOKEN`
  (run endpoint) or a runnable preserved on-behalf of a superadmin reaches `push()` and
  bypasses quotas.
- `push()` takes `windmill_common::Authed` (no `job_id`) and has ~44 callers, so this is
  higher blast radius. Options: add `job_id`/provenance to `Authed`, or thread an
  `is_super_admin`/`is_job_token` bool from callers. Only independently authenticated
  superadmins should get the exemption.

### 3. Two credential mints still reachable by a job token

Both hand back a database-backed token, which authenticates with `job_id: None` and so
sheds the cap. Neither is an *elevation* on its own, which is why they are here rather
than in #10124, but both let a job token trade itself for a credential it should not have.

- `impersonate_service_account` (`windmill-api-users/src/users_ee.rs`) gates only on the
  raw `require_admin(authed.is_admin, ..)` boolean, which a `WM_TOKEN` satisfies — it is
  capped *at* workspace admin. It mints a 24h, **unscoped**, `workspace_id`-NULL token for
  the service account and returns it in `Set-Cookie`. Unscoped means the route-scope
  middleware does not confine it, so it holds whatever the service account's `usr` row
  grants, including a workspace-admin claim with no job provenance.
- `new_webhook_token` (`windmill-native-triggers/src/handler.rs`) reaches
  `create_token_internal` from trigger create/update and from the rename path with no job
  guard. `create_token_internal` copies `super_admin` from the caller's `password` row, and
  `webhook_token_expiration()` is `None` for GitHub and Nextcloud, so the result is a
  permanent superadmin-flagged token — embedded in the webhook URL handed to the external
  service, and readable by whoever controls that repo. Its sibling `rotate_webhook_token`
  (`windmill-native-triggers/src/lib.rs`) copies `email`/`super_admin`/`owner` verbatim from
  the old row, so guarding only the fresh-mint path leaves the update route open.
  The `jobs:run:<kind>:<path>` scopes do bound this one, and scopes are enforced globally at
  auth extraction — unlike the impersonation token above.

Fix both with `forbid_elevated_job_token`, as the other mints do.

### 4. Destructive self-service against the borrowed identity

A different threat model from the GHSA: not escalation, but a `wm_deployers` member
damaging an arbitrary victim through a runnable that preserves their identity. #10124
closed `leave_instance`, `tokens/delete`, `users/leave`, and `workspaces/leave`; these
remain, ranked by damage:

- `update_draft` (`windmill-api/src/drafts.rs`) deliberately **skips**
  `require_can_write_path` when the request is a self-discard, and honours `force`, so a
  job token can destroy the victim's unsaved script/flow/app work at any path. Draft paths
  are enumerable first through `drafts/list` with the same token.
- `decline_invite` (`windmill-api-users/src/users.rs`) takes `workspace_id` from the request
  **body**, not the path, escaping the token's workspace binding: it can delete the victim's
  pending invite to any workspace on the instance, including admin invites.
- `accept_invite` consumes an invite (irreversible, same body-supplied workspace) and, when
  `AUTOMATE_USERNAME_CREATION` is off, lets the caller choose the victim's username in that
  workspace.
- `delete_input` / `update_input` (`windmill-api-inputs/src/lib.rs`) delete or rewrite the
  victim's saved inputs; flipping `is_public` to true leaks their saved arguments.
- `unstar` (`windmill-api/src/favorite.rs`) — cosmetic.

`forbid_job_token_account_destruction` is the right guard for the first three. Decide
deliberately where the line sits for the tail: a job legitimately acts *as* the identity,
so not every self-service write should be refused.

Note `leave_workspace` (both routes) did a *bare* `DELETE FROM usr`, unlike
`delete_workspace_user_internal`, which also strips `extra_perms`, folder owners, drafts,
favorites, inputs and captures. Even now that a job token cannot reach it, a user leaving a
workspace still leaves dangling `u/<username>` ACLs behind — worth fixing separately.

### 5. Missing workspace binding on `get_github_app_token`

`git_sync_ee.rs` gates on `require_admin(authed.is_admin, ..)` but never checks that
`authed`'s workspace matches the `workspace_id` claim inside the supplied job JWT — the JWT
alone drives the installation lookup. Exploiting it requires already holding a git-sync job
JWT from the other workspace, so it is not an escalation path on its own, but the binding
should be explicit.

## Guiding principle

`on_behalf_of` is attacker-influenced (a `wm_deployers` member sets it). It must never
grant *global* superadmin privileges. Where a privilege decision happens on a request,
gate it on `ApiAuthed.job_id` (`require_super_admin` / `is_super_admin_authed` /
`require_devops_role`). Where it happens at execution on a stored/preserved identity,
validate the privilege at **create/update time against the real actor** instead, or thread
job-token provenance into the execution path.

## Tests to add

- Scheduled job + nested flow step on a restricted tag, created by a non-superadmin with
  `preserve_on_behalf_of` → a superadmin email: execution must be rejected.
- Same, created by a real superadmin: must still run.
- Trigger (http/ws) with a restricted tag + preserved superadmin on-behalf: must be rejected.
- Cloud: enqueue from a superadmin-email `WM_TOKEN` in a capped free/past-due workspace →
  normal quota errors still apply.
