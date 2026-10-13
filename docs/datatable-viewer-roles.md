# Data table connections and whose roles they hold

A data table under roles (Enterprise Edition) resolves a connection that names a role to that
role alone. A connection that names none is resolved from the data table's default role and the
roles of one **identity**:

| Default role | A connection naming no role holds |
|---|---|
| `admin` (until changed) | `admin`, as before identity roles existed |
| another role | that role when the identity may use it, plus **every role the identity is a tenant of** |
| none (`NO_DEFAULT_DATATABLE_ROLE`) | the identity's roles only |

The identity is the job's own, `permissioned_as`, like everything else a job reaches: the
publisher for a publisher-mode app, the owner of a schedule or trigger, whoever ran a script. It
is the job's **viewer** only for an inline runnable of a full-code app whose policy entry sets
`datatable_as_viewer` (`DatatableConnectAs::Viewer`). Postgres row-level security then answers per
viewer from a runnable that runs as the publisher, with nothing written in the code.

A connection left with no role at all is refused. Policies test membership, which is true for a
role's own login too, so one policy serves a named connection and an unnamed one:

```sql
CREATE POLICY emea ON orders FOR SELECT
  USING (region = 'emea' AND pg_has_role(current_user, 'sales_emea', 'member'));
```

A policy names its role as text, so renaming the role breaks it.

## Code

| Where | What |
|---|---|
| `windmill-common/src/workspaces.rs` | `DatatableAccess`, `DatatableConnectAs`, `get_datatable_resource_from_db` |
| `windmill-common/src/datatable_roles_ee.rs` | `resolve_datatable_role_connection`, `resolve_unnamed_login`, `governing_identity`, `ensure_role_set_login`, `retire_role_set_logins_of`, `sweep_unused_role_set_logins` |
| `windmill-api/src/apps.rs` | `execute_component`, which sets `job_perms.datatable_as_viewer` for a flagged inline runnable |
| `windmill-api-jobs/src/execution.rs` | `run_end_user_email`, where a run's viewer is recorded, and `run_inline_preview_end_user` |
| `frontend/.../raw_apps/DatatableIdentityPopup.svelte` | the runnable setting, shown when the code uses a data table |
| `frontend/.../workspaceSettings/DataTablePermissionsButton.svelte` | the roles drawer, which warns while the default is `admin` |

## What holds it together

- **A default of `admin` is not a floor.** `admin` owns the tables and bypasses row-level
  security, so nothing could be added to it, and it is the default until someone changes it.
  Such a data table resolves exactly as it did before identity roles existed. Nothing ever sets a
  default back to `admin`: removing or deleting the default role leaves none.
- **One identity decides both the default and the added roles.** The default is held when that
  identity may use it, and the roles added are those whose tenants name that same identity. A
  job never mixes its own identity's default with its viewer's roles.
- **Connecting as the viewer is the publisher's choice, made per runnable and stored in the
  policy.** It is honored on inline runnables only, whose code is deployed with the app: a path
  runnable is someone else's script, which would read with the viewer's roles without its author
  choosing to. The server stamps `job_perms.datatable_as_viewer` in the transaction that pushes
  the job. Nested jobs do not inherit it; an inline preview run with the job's token
  (`run_inline/preview`, what `wmill.datatable()` uses) does, since that code is the job's own.
- **A job that connects as its viewer and has none is refused**, never resolved as the job: that
  would undo what the runnable asked for. An anonymous app viewer is the usual case.
- **No role at all is a refusal, never an empty answer.** There is no login that would read
  nothing: what a role sees is up to each table's grants and policies.
- **A set login owns what it creates.** Postgres gives a new object to `current_user`, and
  default privileges go by the creating role, so a table created over a connection that holds
  several roles belongs to that set's login: no other role or identity reaches it, and it passes
  to `custom_instance_user` when the login is swept. Unnamed connections are for rows. To create
  an object, name a role or run `SET ROLE <role>` first, which a set login may do for any role it
  holds. Migrations are not affected: one that declares no role runs as `admin`.
- **The viewer is `job_perms.end_user_email`, and nothing else.** The server records it when the
  run starts: the session behind an app request, the caller of a direct run or preview, and for a
  job started with another job's token, that job's viewer. No argument reaches it.
- **A cached result belongs to one identity, one viewer and one mode.** `cached_result_path`
  hashes the job's `permissioned_as`, its recorded viewer and `datatable_as_viewer` into the key,
  since a cache hit returns before any connection is resolved. Turning the setting on changes no
  code, so without the mode a result read with the app's roles would be served to the viewer.
- **Only a tenant list naming the identity counts.** A workspace admin may *use* every role,
  `admin` included. The roles added are those whose tenants name the identity (user, group,
  folder, `*`), and never `admin`.
- **One role is that role's login. Several are a login per distinct set.** `wm_set_<hash>` is a
  member of exactly those roles, with `INHERIT`; the hash is over their catalog ids, so a rename
  moves nothing and a change in someone's groups resolves them to another login instead of
  editing one. Its privileges are its memberships, so `SET ROLE` only narrows and `RESET ROLE`
  comes back to it. The prefix is reserved in `validate_role_name`.
- **A `datatable_role_set` row with a password names a login that can authenticate with it.** The
  row is written after the login exists and removed, in its own commit, before the login is
  dropped or made `NOLOGIN`. A login without a row is taken back with a new password on next use,
  and so is one whose row has an empty password: that is what a sweep leaves when it could
  neither drop a login nor let it back in, so the next sweep still finds it.
- **Set logins are bounded by use, not by the number of role combinations.** One exists only for
  a set some identity actually connected with, so there are at most as many as (identity, data
  table) pairs. `last_used_at` is recorded at most once a day, and `sweep_unused_role_set_logins`,
  run from the monitor loop, drops those unused for 30 days; a dropped one is made again on
  demand. The sweep goes by this instance's rows, never by the cluster's `wm_set_*` roles: a
  cluster can serve several Windmill instances.
- **Disabling or dropping a role retires every set login holding it.** `NOLOGIN` on a role does
  not reach a login that is a member of it.
- **Mixed fleets.** A worker or server older than v1.831 connects as the default role alone, and
  refuses a data table with no default role, whose marker it reads as a role the data table
  lacks. One from v1.831 or v1.832 adds the recorded viewer's roles to every job that has one,
  flagged or not, and the job's own roles to none.

## Trust

A job that names no role reads with its own identity's roles, the same trust as the resources and
variables it reaches: every viewer of a publisher-mode app sees what the publisher's roles see.
An inline runnable that connects as its viewer runs its publisher's code with the viewer's reach
into the data table, the same trust as an app in viewer execution mode. Naming a role in the code
pins the connection to that one role either way.
