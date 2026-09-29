# External instance data tables

A data table is backed by one of three things: a Postgres resource a workspace brings
(`postgresql`), a database on Windmill's own cluster (`instance`), or a database on a separate
cluster Windmill administers (`external_instance`, Enterprise Edition). The third is what this
document covers; Ducklake catalogs take the same three shapes.

Windmill administers the external cluster the way it administers its own: it creates and drops
databases there, owns `custom_instance_user` and `custom_instance_replication_user`, and creates
the data table roles of that cluster. It logs in as the admin in the `external_instance_pg`
instance setting, and keeps what it generates in the hidden `external_instance_pg_state` setting.

## Code

| Where | What |
|---|---|
| `windmill-common/src/external_instance_pg.rs` | Setting, state, usage accounting, the lifecycle lock, the OSS forwarders |
| `windmill-common/src/external_instance_pg_ee.rs` | Setup, database create and drop, the admin connection |
| `windmill-common/src/datatable_roles.rs` | Per-cluster role catalogs (`DatatableRoleCluster`) |
| `windmill-common/src/workspaces.rs` | Resolution (`resolve_datatable_connection_unchecked`), `managed_database_uses` |
| `windmill-api-settings/src/lib.rs` | `/settings/external_instance_pg/*`, `/settings/datatable_roles` |

## What holds it together

- **One lifecycle lock.** `lock_external_instance_pg_state` serializes everything that changes
  which databases exist on the cluster or which entries name them: setup, create, drop, data table
  and Ducklake saves, external role DDL, and writes to the setting itself. Anything reading the
  configuration to reach the cluster reads it under that lock, so a database is never created on
  one cluster and registered while the setting names another.
- **Windmill only touches what it made.** Databases it creates carry a comment, and a drop
  requires it. The two managed roles and every data table role carry their own comment, and setup
  refuses a `custom_instance_user` without it rather than resetting the password of someone else's
  role.
- **Creation needs a successful setup.** `set_up_for` records the `host:port` the last successful
  setup converged. Creating a database on a cluster that setup has not succeeded on is refused.
- **Nothing is dropped from under a user.** `managed_database_uses` lists every data table naming
  a database, every fork pointing at those, every Ducklake catalog on it, and every fork Ducklake
  metadata schema still to be dropped. Fork cleanup exempts exactly the entry it is cleaning up.
- **Fork copies belong to a workspace.** `wm_fork_*` is a name, not an authorization: every
  database of a cluster answers to the same `custom_instance_user`. The registry records the
  workspace a copy was created for, and a member can only import into or fork onto a copy of their
  own workspace.
- **Roles are per cluster.** `datatable_role.cluster` splits the catalog, so the same role name can
  exist on both clusters. Role names are unique per cluster, as they are in Postgres.

## Running one locally

```bash
docker run -d --name wm-external-pg -e POSTGRES_PASSWORD=external -p 5497:5432 postgres:18 \
  -c wal_level=logical
psql "postgresql://postgres:external@127.0.0.1:5497/postgres" \
  -c "CREATE ROLE wm_admin LOGIN PASSWORD 'adminpw' CREATEDB CREATEROLE REPLICATION"
```

A non-superuser admin with `CREATEDB` and `CREATEROLE` is the realistic case: managed Postgres
gives nothing more. `REPLICATION` is only needed for Postgres triggers on external data tables.

Then, as superadmin (`$T` is a token):

```bash
api=http://localhost:8000/api
curl -s -X POST $api/settings/global/external_instance_pg -H "Authorization: Bearer $T" \
  -H 'Content-Type: application/json' \
  --data '{"value":{"host":"127.0.0.1","port":5497,"user":"wm_admin","password":"adminpw","sslmode":"disable"}}'
curl -s -X POST $api/settings/external_instance_pg/setup -H "Authorization: Bearer $T" \
  -H 'Content-Type: application/json' --data '{}'          # report per step
curl -s -X POST $api/settings/external_instance_pg/databases/dt_demo -H "Authorization: Bearer $T" \
  -H 'Content-Type: application/json' --data '{}'
curl -s -X POST $api/w/admins/workspaces/edit_datatable_config -H "Authorization: Bearer $T" \
  -H 'Content-Type: application/json' \
  --data '{"settings":{"datatables":{"demo":{"database":{"resource_type":"external_instance","resource_path":"dt_demo"}}}}}'
```

`sslmode` defaults to `verify-full`; `disable` is for a local container only. With `verify-full`
against a server with a private CA, put the CA in `root_certificate_pem` — `pg_dump`, `psql` and
DuckDB attaches all verify against the system trust store plus that certificate.

Jobs then reach it as any data table: `ATTACH 'datatable://demo' AS d` from DuckDB, or
`datatable://demo` as the database of a PostgreSQL script, with `-- role <name>` to connect as a
data table role of that cluster.

Worth knowing while testing:

- A worker needs the `postgresql` and `duckdb` tags for those jobs
  (`update config set config = jsonb_set(config, '{worker_tags}', …) where name = 'worker__default'`).
- DuckDB jobs load `libwindmill_duckdb_ffi_internal.so` by name, so a binary built into its own
  `CARGO_TARGET_DIR` needs that library on `LD_LIBRARY_PATH`.
- Setup holds the lifecycle lock for its whole run, so a settings save during it waits.
