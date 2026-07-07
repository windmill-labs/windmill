# Data table permissions (EE)

Advanced, opt-in permissions for data tables, enforced natively by Postgres
**roles** and **row-level security (RLS)**. Enterprise-only; off by default (a
data table with no permission config keeps the legacy behavior where every
workspace member shares the owner role and has full access).

## Why roles, not session variables

Data table queries run **arbitrary user SQL** (a `postgresql` script job with
`database = "datatable://<name>"`). That rules out the usual RLS shortcut of
injecting identity via a session GUC (`SET app.user = ...`) — the user's own SQL
can overwrite it. It also rules out `SET ROLE` down to a low-privilege role — the
user can `RESET ROLE` back to the login role.

The only tamper-proof signal is the **connected role itself**. So a non-admin
query connects *directly* as a per-user login role; `current_user` is then
authoritative and cannot be escalated, because the session never held more
privilege than that role.

## Model

Windmill principals are mirrored into Postgres roles inside the data table's
physical database:

- `wm_u_<hash>` — one **LOGIN** role per user (`INHERIT`, deterministic
  HMAC-derived password). The name hashes `(workspace_id, email)`; the workspace
  id is folded in because Postgres roles are cluster-global and all instance data
  tables share one server.
- `wm_g_<hash>` — one **NOLOGIN** role per group. User roles are granted
  membership in their group roles, so group grants apply automatically. Roles are
  only ever members of their own, minimally-granted groups — there is no path
  upward.
- `_wm_principals(role_name, email, username)` — a mapping table (readable only
  via the SECURITY DEFINER `wm_email()` helper, which returns the current role's
  email) so policies can reference the acting user's email.

Two layers of control, both configured from the **Permissions** modal in the data
table settings:

1. **Access** (the `Access` tab) → Postgres `GRANT`s. Per user/group: no
   access / read (`SELECT`) / read+write (`SELECT,INSERT,UPDATE,DELETE`), scoped
   to one table or all tables (with `ALTER DEFAULT PRIVILEGES` so future tables
   inherit the grant).
2. **Row policies** (the `Row policies` tab) → `CREATE POLICY`. RLS is enabled
   **only** on tables that have a policy. Templates and raw `USING` /
   `WITH CHECK` expressions (e.g. `owner = wm_email()`).

`CREATE` on `public` is **revoked** from principal roles, so a principal can
never create a table that escapes policy — all DDL flows through the migrations
path (which connects as the table owner).

### Admin bypass

We deliberately do **not** `FORCE` RLS. The table owner
(`custom_instance_user` for instance data tables; the resource user for resource
data tables) therefore bypasses RLS. Workspace admins connect as that owner, so
admins always keep full, unfiltered access. Non-admin principals are non-owners
and stay subject to RLS. This is safe because principals can never own a table.

## Enforcement path

`pg_executor` resolves `datatable://<name>` through
`get_datatable_resource_from_db_checked(db, w_id, name, acting_email)`
(`acting_email` = the job's `permissioned_as_email`):

- data table has no permissions / disabled, or the user is a **workspace
  admin** → default owner connection (unchanged);
- other member → connect as their `wm_u_*` role; grants + RLS enforce;
- member with no provisioned role (instance data tables) → **denied** with a
  clear error.

The EE logic lives in `windmill-common/src/datatable_permissions_ee.rs` behind the
`datatable_permissions` module switch (`_ee` under `private`, `_oss` stub
otherwise), mirroring the `pipeline_advanced` / `partition` pattern. On OSS the
resolver is a no-op, so the checked path equals the unchecked one.

## Provisioning

`set_datatable_permissions` writes the config into the catalog JSONB
(`workspace_settings.datatable`) and then **reconciles** the target database:
creates/updates roles, memberships, `_wm_principals` rows, grants, and policies.
Instance data tables provision as the Windmill superuser; resource data tables
provision with the resource's own credentials (best-effort — the reconciliation
log surfaces any privilege it lacked). `sync_datatable_permissions` re-runs
reconciliation from the stored config.

## Known limitations (v1)

- **Instance and resource data tables** are both attempted; resource support is
  best-effort (needs `CREATEROLE` + table ownership on the resource credentials).
- **Group-membership drift**: roles, memberships, grants and policies are
  reconciled to match the config only at save/**Sync** time. A user added to or
  removed from a group (or a datatable getting new tables) takes effect on the
  next Sync — reconcile is declarative and revokes removed access, so Sync both
  grants and revokes. There is no per-query provisioning, so a brand-new member
  is denied until an admin re-syncs.
- **New tables**: run **Sync** after schema migrations so freshly-created tables
  pick up grants/policies (grants also propagate via default privileges; RLS
  policies must be re-applied).
- **Enforced paths**: `postgresql` and DuckDB (`ATTACH 'datatable://...'`) script
  jobs on a normal worker both resolve through the checked path. **Not** enforced
  (resolve as owner): schema-introspection and the DB-manager "explore"
  endpoints, Postgres-trigger capture, and **agent-worker** job execution. If you
  rely on agent workers, treat data table permissions as advisory for those jobs.
- **Row policy expressions** are admin-authored raw SQL (like migrations) — they
  are trusted input.
