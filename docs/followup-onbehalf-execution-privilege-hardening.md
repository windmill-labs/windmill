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
