# Data table connections as the viewer

A data table under roles (Enterprise Edition) normally resolves to one role: the one a caller
names, or the data table's default. `?as=viewer` resolves to **every role the job's viewer is a
tenant of**, so Postgres row-level security can answer per viewer from a backend runnable that
runs as the app's publisher.

```ts
wmill.datatable('main', { as: 'viewer' })            // TypeScript
wmill.datatable('main', as_viewer=True)              # Python
ATTACH 'datatable://main?as=viewer' AS dt;           -- DuckDB
-- PostgreSQL script: `datatable://main?as=viewer` as its `database` argument
```

Policies test membership, which is true for a role's own login too, so one policy serves both
kinds of connection:

```sql
CREATE POLICY emea ON orders FOR SELECT
  USING (region = 'emea' AND pg_has_role(current_user, 'sales_emea', 'member'));
```

A policy names its role as text, so renaming the role breaks it.

## Code

| Where | What |
|---|---|
| `windmill-common/src/workspaces.rs` | `DATATABLE_VIEWER_ROLE`, `parse_datatable_ref`, `DatatableAccess` |
| `windmill-common/src/datatable_roles_ee.rs` | `resolve_viewer_login`, `ensure_role_set_login`, `retire_role_set_logins_of` |
| `windmill-api-jobs/src/execution.rs` | `run_end_user_email`, where a run's viewer is recorded |

## What holds it together

- **The viewer is `job_perms.end_user_email`, and nothing else.** The server records it when the
  run starts: the session behind an app request, the caller of a direct run or preview, and for a
  job started with another job's token, that job's viewer. No argument reaches it. A job without
  one (anonymous app viewer, schedule, trigger, a tool job started by an AI agent step) is refused
  rather than resolved as its owner, which in a publisher-mode app would be the publisher.
- **A cached result belongs to one viewer.** `cached_result_path` hashes the run's recorded viewer
  into the key, since a cache hit returns before any connection is resolved. Runs with no viewer
  recorded keep sharing one entry.
- **Only a tenant list naming the viewer counts.** A workspace admin may *use* every role,
  `admin` included, and `admin` is the connection that owns the tables and bypasses row-level
  security. The viewer's roles are those whose tenants name them (user, group, folder, `*`), and
  never `admin`.
- **One role is that role's login. Several are a login per distinct set.** `wm_set_<hash>` is a
  member of exactly those roles, with `INHERIT`; the hash is over their catalog ids, so a rename
  moves nothing and a change in someone's groups resolves them to another login instead of
  editing one. Its privileges are its memberships, so `SET ROLE` only narrows and `RESET ROLE`
  comes back to it. The prefix is reserved in `validate_role_name`.
- **A `datatable_role_set` row always names a login that can authenticate.** The row is written
  after the login exists and removed, in its own commit, before the login is dropped or made
  `NOLOGIN`. A login without a row is taken back with a new password on next use.
- **Disabling or dropping a role retires every set login holding it.** `NOLOGIN` on a role does
  not reach a login that is a member of it.
- **It travels in the reference, not in an annotation.** A worker that predates `?as=` refuses the
  reference as malformed. It would skip an unknown `-- ` annotation and connect as the default
  role, which is `admin` unless changed. For the same reason the request reaches an agent worker's
  server in the `role` parameter, as a name no role can have.

## Trust

A runnable connecting as the viewer runs its author's code with the viewer's reach into the data
table, the same trust as an app in viewer execution mode or a script someone else wrote and you
run. Whoever can deploy a publisher-mode app, or a script that others run, can therefore read
through the non-`admin` roles of the people who use it; nothing on the data table opts in to
that.

A set login is dropped only with one of its roles. One that no viewer resolves to any more stays
on the cluster, holding only roles that still exist.
