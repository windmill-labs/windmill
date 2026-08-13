# Operator capabilities (API surface)

What a user with the **operator** role can already do today through their token, and where
the boundary is actually enforced. This is the baseline for any feature whose scope rule is
*"anything the operator's token can do today, nothing more"*.

Two things are worth knowing before reading the rest:

- **Postgres RLS does not know what an operator is.** `set_session_context` receives
  `is_admin`, username, groups, and the read/write folder lists — not `is_operator`
  (`windmill-common/src/db.rs:242`). So at the data layer an operator is an ordinary
  member, and every table's policies grant them exactly what their ACLs grant.
- **The operator boundary is a hand-maintained deny list in 30 request handlers.**
  Anything not on that list is reachable. The workspace's `operator_settings` is *not*
  part of it: it only filters the sidebar and the AI chat's page list in the browser.

## 1. Where the role comes from

| | |
|---|---|
| Storage | `usr.operator` (per workspace, per user) |
| Resolved into | `Authed.is_operator` (`windmill-common/src/auth.rs:245`) |
| Instance groups | best matching role wins, `"operator"` → `is_operator` (`windmill-common/src/users.rs:277-311`) |
| Superadmins | `is_admin = true`, `is_operator = false` — on the workspace-scoped paths (`windmill-common/src/auth.rs:391`, `windmill-api-auth/src/auth.rs:463`, `:525`), not on the global ones below |
| **Non-workspace-scoped routes** | `is_operator = true` for *every* caller, workspace admins included, with empty groups and folders — there is no `usr` row to read when the path carries no workspace (`windmill-api-auth/src/auth.rs:403-497`). So `is_operator` implies `usr.operator = true` only on `/api/w/{workspace}/…` routes; on global routes it is not a discriminator at all |
| Job tokens | the flag is persisted in `job_perms` and rebuilt from it, so a `$WM_TOKEN` minted for an operator's job is itself operator-flagged (`windmill-queue/src/jobs.rs:6769`, `auth.rs:514`) |
| Token scopes | orthogonal — scopes only ever *narrow*. An operator may mint themselves API tokens (`POST /users/tokens/create`, verified `201`) but never a token more privileged than they are (`ensure_scopes_within_caller`) |

## 2. The enforcement layers

```
frontend  operator_settings ......... sidebar + AI-chat page list only. NOT enforced server-side.
handlers  `if authed.is_operator` ... 30 hard denials (§3) + 16 list-shaping filters (§4).
RLS       (operator-blind) .......... ordinary member ACLs on every table.
```

## 3. Hard denials — the complete list

Every place an operator is rejected outright. Grep-stable via `authed.is_operator` — but run
that grep against a checkout **with `windmill-ee-private` present**, or the `*_ee.rs` symlinks
dangle and the EE rows below are invisible.

| Area | Handler | Location |
|---|---|---|
| Scripts | create (all deploy paths) | `windmill-api-scripts/src/scripts.rs:1004` |
| | archive by path / by hash | `scripts.rs:3317`, `scripts.rs:3408` |
| Flows | create / update / archive / delete | `windmill-api-flows/src/flows.rs:524, 1079, 1734, 1879` |
| Apps | create / update / delete | `windmill-api/src/apps.rs:2044, 2591, 2397` |
| | raw app create / update (bundle and source) | `apps.rs:1976, 2919, 2780, 2644` |
| | raw-app preview SDK token | `apps.rs:1429` |
| | `execute_component` **in preview mode only** | `apps.rs:3493` |
| | app S3 upload/download with a caller-supplied policy | `apps.rs:3989, 4629` |
| Drafts | save any draft (read is deliberately still allowed) | `windmill-api/src/drafts.rs:689` |
| Jobs | `run/preview`, `run_inline/preview`, `run/preview_bundle` | `windmill-api/src/jobs.rs:7912, 8013, 8263` |
| | `run/preview_flow` | `jobs.rs:8947` |
| | `run/dependencies`, `run/flow_dependencies` | `jobs.rs:8444, 8564` |
| | `run/dynamic_select` **when the runnable is inline** | `jobs.rs:9075` |
| | resolve jobs | `jobs.rs:11144` |
| Triggers | use of an admin-configured native integration (calendar/drive/repo pickers) | `windmill-native-triggers/src/lib.rs:1639` |
| OAuth (EE) | connect an account with the shared instance credentials, and the client-credentials token exchange — `is_operator \|\| read_only` → *"Connecting with the shared instance credentials requires a read-write workspace member"* | `windmill-api/src/oauth2_ee.rs:591, 697` (lines as of `backend/ee-repo-ref.txt` `88568d1`) |

Two threads run through this: **arbitrary request-supplied code** and **deploying runnables**
account for all but the last row. The OAuth pair is a third, narrower one — the `account`
table has no RLS, so spending the admin-configured shared credentials is restricted to
read-write members by an explicit check rather than by ACLs.

## 4. List-shaping (not denials)

These change what a listing returns; the underlying object stays reachable by path.

- Draft-only rows are never appended for an operator, and `/drafts/list` returns `[]`
  (`drafts.rs:106`, `scripts.rs:357`, `flows.rs:235`, `apps.rs:536`, `runnables.rs:510/522/941`,
  `windmill-api-schedule/src/lib.rs:837`, `windmill-store/src/resources.rs:406`,
  `windmill-store/src/variables.rs:232`, `windmill-trigger/src/handler.rs:623`).
- Library scripts (`auto_kind = 'lib'`) are always filtered out (`scripts.rs:254`, `runnables.rs:476/760`).
- `/scripts/list` is forced to `kind = 'script'` (`scripts.rs:309`), hiding failure/trigger/
  approval/preprocessor scripts **from the list only**.
- `/debug` omits `registry_config` (`windmill-api-debug/src/lib.rs:495`).

> Verified: a `kind: failure` script is absent from an operator's `/scripts/list`, yet
> `GET /scripts/get/p/f/shared/fail` returns it with full source (`200`) and
> `POST /jobs/run/p/f/shared/fail` runs it (`201`).

## 5. What an operator's token can do

Runtime-verified against a CE dev backend (`v1.787.0`) with an operator who is a member of
group `all`, which owns folder `f/shared` — i.e. an operator with ordinary write ACLs. Full
transcript in §8.

### Runnables and jobs

| Capability | Result |
|---|---|
| List/read scripts, flows, apps, raw apps they have ACL for | ✅ — including **full script source** |
| Run a deployed script by path (`/jobs/run/p/…`, `/jobs/run_wait_result/p/…`) | ✅ |
| Run a deployed flow by path (`/jobs/run/f/…`) | ✅ |
| Run a script of any `kind` (failure/trigger/approval/preprocessor) by path | ✅ |
| Execute app components through a deployed app policy | ✅ (viewer path; preview path denied) |
| Run request-supplied code (preview / inline / bundle / preview flow / dependencies) | ❌ denied |

### Job history

Visibility is pure RLS on `v2_job` — the same rule as for any member:

```
own jobs (permissioned_as = u/<me>)
  OR jobs permissioned as one of my groups
  OR visible_to_owner AND runnable_path is under a folder/group/user namespace I can read
```

An operator therefore sees **other people's runs**, including an admin's, whenever the
runnable lives in a folder they can read. Verified for that case:

| | |
|---|---|
| `/jobs/list`, `/jobs/completed/list` include the admin's runs | ✅ |
| `/jobs_u/get/{id}`, `/get_logs/{id}`, `/get_args/{id}`, `/completed/get_result/{id}` on an admin's job | ✅ `200` |
| Same, for a job whose runnable is in a folder they cannot read (`f/private`) | ❌ `403` |
| **Cancel another user's running job** (`/jobs_u/queue/cancel/{id}`) | ✅ `200` — an admin's 60 s job was cancelled by the operator |
| `/jobs/delete`, `/jobs/list_filtered_uuids` | ❌ `403` admin-only |
| Saved-input history for a runnable (`/inputs/history`) | ✅ — other users' past run args |

### Everything else is a plain member

These are **not** gated on `is_operator` anywhere; only ACLs apply.

| Capability | Result |
|---|---|
| Read a **secret variable's** value (`/variables/get_value/{path}`) | ✅ `"s3cr3t"` |
| Create / update / delete variables, including secrets | ✅ |
| Read resource values, incl. `get_value_interpolated` (resolves secrets) | ✅ |
| Create / update / delete resources | ✅ |
| **Create a workspace-global resource type** | ✅ `201` |
| Create / update / delete / enable-disable **schedules** on any runnable they can read | ✅ |
| Create / update / delete **triggers** (HTTP routes, websocket, kafka, …) | ⚠️ code-verified only — `windmill-trigger/src/handler.rs:478` gates on scopes + RLS with no operator check. Not runtime-verified: no trigger cargo feature is compiled in the dev backend. Schedules, the always-compiled member of the same family, *were* runtime-verified. |
| Create folders; add owners to folders they own | ✅ |
| Create groups | ✅ (adding users to a group they don't own is refused) |
| Change **granular ACLs** on objects they own (`/acls/add`) | ✅ |
| List all workspace users with their emails; `whois` | ✅ |
| List workers and **worker-group configs** (incl. `init_bash`) | ✅ |
| Use the workspace **AI proxy** (`/ai/proxy/…`) | ✅ (reaches provider dispatch) |
| Mint API tokens for themselves with arbitrary scopes | ✅ |
| Star favorites, list assets, list AI skills, flow conversations | ✅ |
| Workspace settings, dependency map, pending invites, operator settings, concurrency groups, user administration | ❌ `403` admin-only |
| Instance settings, service logs, impersonation | ❌ superadmin/devops only |

## 6. `operator_settings` is not an authorization boundary

`workspace_settings.operator_settings` has ten flags (runs, schedules, resources, variables,
assets, triggers, audit_logs, groups, folders, workers). Nothing in the backend reads them
for authorization — the only backend touch points store and return the JSON
(`windmill-api-workspaces/src/workspaces.rs:4991, 8973`). They are consumed by
`OperatorMenu.svelte` and by the AI chat's `allowedOpenPages()`
(`frontend/src/lib/components/copilot/chat/global/core.ts:2289`).

Verified: with **all ten flags set to `false`**, the same operator token still gets `200`
and full payloads from `/jobs/list`, `/schedules/list`, `/variables/list`, `/resources/list`,
`/folders/list`, `/groups/list`, `/audit/list`, `/assets/list` and `/workers/list`.

Anything that treats `operator_settings` as a permission — including chat tool filtering —
is enforcing a UI preference, not a boundary. That is fine for shaping what a tool
*advertises*; it must not be the only thing standing between the operator and the data.

## 7. Consequences for a "nothing more than the token" scope rule

1. **Run script / run flow by path is already fully allowed**, synchronously or async, for
   anything the operator can read. No new capability is needed and none should be added.
2. **Preview / inline / raw code is the one hard line.** Every arbitrary-code entry point is
   already blocked; a tool that accepts code and runs it would breach the baseline even if
   it merely wrapped an existing endpoint.
3. **Job history is folder-scoped, not user-scoped.** A tool may surface other users' runs,
   logs, args and results — but only after the same RLS check, i.e. by going through the
   existing endpoints rather than a root-DB query.
4. **Reading secrets is within the baseline.** `variables/get_value` and interpolated
   resources already return plaintext secrets to an operator with ACL. Redaction is a
   product decision, not a permission one.
5. **Deploy-shaped mutations are not.** Scripts, flows, apps and drafts are denied; but
   variables, resources, resource types, schedules, triggers, folders, groups and ACLs are
   *not*, so "operators are read-only" is false and should not be assumed anywhere.
6. **`operator_settings` cannot be used to justify exposing less or more.** Filter tools with
   it for UX consistency, but derive the security boundary from §3–§5.
7. **Never gate on `is_operator` outside a workspace-scoped route.** On `/api/…` routes with
   no workspace in the path the flag is unconditionally `true` (§1), so such a check would
   reject workspace admins and superadmins too.

## 8. How this was verified

Dev backend `CE v1.787.0`, workspace `aud2`, three principals: `admin` (workspace admin),
`operator` (`usr.operator = true`), `member` (plain). Folder `f/shared` is owned by group
`all` with write, `f/private` by `u/admin` only; both hold a script, and `f/shared` also
holds a flow, an app, a secret variable, a resource and a schedule.

Fixtures are created **before** the operator logs in — `AUTH_CACHE`
(`windmill-api-auth/src/auth.rs:40`) memoizes the resolved `Authed`, so a token minted
before a folder grant reports the *old* folder list and every read spuriously 404s.

Roughly 110 endpoints were exercised with the operator token and the same set with the
member token, recording status and body. The scripts live in the session scratchpad
(`setup.sh`, `probe.sh`); they are throwaway harnesses, not committed.

**The runtime probe only covers the compiled feature set**, which for a CE dev backend is
`quickjs` alone. Anything behind a cargo feature was read rather than called: no trigger
feature is compiled, so the trigger-CRUD row in §5 rests on
`windmill-trigger/src/handler.rs`; `oauth2_ee` needs `oauth2` + `private`
(`windmill-api/src/lib.rs:132`), so §3's OAuth row rests on the EE sources. Re-run the audit
against an EE build before treating §5's ✅/❌ as exhaustive for a feature you care about.

The cheap re-check that does *not* need a running backend: diff
`grep -rn 'authed\.is_operator' backend/ --include=*.rs` (from a checkout with
`windmill-ee-private` linked) against §3's table.
