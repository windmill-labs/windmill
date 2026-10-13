# Data table connections and the viewer's roles

A data table under roles (Enterprise Edition) resolves a connection that names a role to that
role alone. A connection that names none is resolved from the data table's default role:

| Default role | A connection naming no role holds |
|---|---|
| `admin` (until changed) | `admin`, as before viewer roles existed |
| another role | that role when the job's own identity may use it, plus **every role the job's viewer is a tenant of** |
| none (`NO_DEFAULT_DATATABLE_ROLE`) | the viewer's roles only |

A connection left with no role at all is refused. Postgres row-level security can then answer
per viewer from a backend runnable that runs as the app's publisher, with nothing written in the
runnable.

Policies test membership, which is true for a role's own login too, so one policy serves a named
connection and an unnamed one:

```sql
CREATE POLICY emea ON orders FOR SELECT
  USING (region = 'emea' AND pg_has_role(current_user, 'sales_emea', 'member'));
```

A policy names its role as text, so renaming the role breaks it.

## Code

| Where | What |
|---|---|
| `windmill-common/src/workspaces.rs` | `DatatableAccess`, `get_datatable_resource_from_db` |
| `windmill-common/src/datatable_roles_ee.rs` | `resolve_datatable_role_connection`, `viewer_role_ids`, `ensure_role_set_login`, `retire_role_set_logins_of`, `sweep_unused_role_set_logins` |
| `windmill-api-jobs/src/execution.rs` | `run_end_user_email`, where a run's viewer is recorded |
| `frontend/.../workspaceSettings/DataTablePermissionsButton.svelte` | the roles drawer, which warns while the default is `admin` |

## What holds it together

- **A default of `admin` is not a floor.** `admin` owns the tables and bypasses row-level
  security, so nothing could be added to it, and it is the default until someone changes it.
  Such a data table resolves exactly as it did before viewer roles existed. Nothing ever sets a
  default back to `admin`: removing or deleting the default role leaves none.
- **The default role is tenant-checked against the job's own identity**, the publisher for a
  publisher-mode app, and held only when that passes. The viewer's roles are added either way.
- **No role at all is a refusal, never an empty answer.** There is no login that would read
  nothing: what a role sees is up to each table's grants and policies. A job without a recorded
  viewer (anonymous app viewer, schedule, trigger, a tool job started by an AI agent step), or
  whose viewer is not a member of the governing workspace, therefore needs a default it may use
  or a named role.
- **A set login owns what it creates.** Postgres gives a new object to `current_user`, and
  default privileges go by the creating role, so a table created over a connection that holds
  several roles belongs to that set's login: no other role or viewer reaches it, and it passes to
  `custom_instance_user` when the login is swept. Unnamed connections are for rows. To create an
  object, name a role or run `SET ROLE <role>` first, which a set login may do for any role it
  holds. Migrations are not affected: one that declares no role runs as `admin`.
- **The viewer is `job_perms.end_user_email`, and nothing else.** The server records it when the
  run starts: the session behind an app request, the caller of a direct run or preview, and for a
  job started with another job's token, that job's viewer. No argument reaches it.
- **A cached result belongs to one viewer.** `cached_result_path` hashes the run's recorded viewer
  into the key, since a cache hit returns before any connection is resolved. Runs with no viewer
  recorded keep sharing one entry.
- **Only a tenant list naming the viewer counts.** A workspace admin may *use* every role,
  `admin` included. The viewer's roles are those whose tenants name them (user, group, folder,
  `*`), and never `admin`.
- **One role is that role's login. Several are a login per distinct set.** `wm_set_<hash>` is a
  member of exactly those roles, with `INHERIT`; the hash is over their catalog ids, so a rename
  moves nothing and a change in someone's groups resolves them to another login instead of
  editing one. Its privileges are its memberships, so `SET ROLE` only narrows and `RESET ROLE`
  comes back to it. The prefix is reserved in `validate_role_name`.
- **Resolving stays cheap where no viewer could add anything.** The viewer is only looked up when
  some role other than the default and `admin` has tenants on that data table.
- **A `datatable_role_set` row with a password names a login that can authenticate with it.** The
  row is written after the login exists and removed, in its own commit, before the login is
  dropped or made `NOLOGIN`. A login without a row is taken back with a new password on next use,
  and so is one whose row has an empty password: that is what a sweep leaves when it could
  neither drop a login nor let it back in, so the next sweep still finds it.
- **Set logins are bounded by use, not by the number of role combinations.** One exists only for
  a set some viewer actually connected with, so there are at most as many as (viewer, data table)
  pairs. `last_used_at` is recorded at most once a day, and `sweep_unused_role_set_logins`, run
  from the monitor loop, drops those unused for 30 days; a dropped one is made again on demand.
  The sweep goes by this instance's rows, never by the cluster's `wm_set_*` roles: a cluster can
  serve several Windmill instances.
- **Disabling or dropping a role retires every set login holding it.** `NOLOGIN` on a role does
  not reach a login that is a member of it.
- **A worker or server that predates this connects as the default role alone**, so a mixed fleet
  shows a viewer fewer rows, never more. It refuses a data table with no default role, whose
  marker it reads as a role the data table lacks.

## Trust

A job that names no role runs its author's code with the viewer's reach into the data table, the
same trust as an app in viewer execution mode or a script someone else wrote and you run. Whoever
can deploy a publisher-mode app, or a script that others run, can therefore read through the
non-`admin` roles of the people who use it. Naming a role in the code is how an author pins the
connection to one role instead.
