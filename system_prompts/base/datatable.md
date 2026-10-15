# Data tables: schema changes, migrations and permissions

A data table is a PostgreSQL database managed by Windmill and shared by the whole workspace: scripts, flows and apps reach it by name, `main` being the default. It is configured in the workspace settings (Data Tables), never created by SQL.

## Changing the schema

A data table with migrations enabled keeps its schema as an ordered list of versioned SQL migrations, each an `up` and an optional `down`. That list is what git sync and a workspace merge carry, and what rebuilds the schema in another workspace. So first find out whether migrations are enabled on the data table:

- **Enabled**: make every schema change (CREATE, ALTER, DROP, anything but row data) as a new migration. A change made with plain SQL exists in that one database only, and the history no longer describes it.
- **Not enabled**: schema changes are plain SQL. Enabling migrations is a workspace admin's decision, made in the Data Tables settings, not a step to take on the way to something else.

Writing a migration:

- Wrap the statements in `BEGIN; ... END;` so it applies atomically. Leave the frame out only for a statement that cannot run in a transaction, such as `CREATE INDEX CONCURRENTLY`: such a migration can fail half applied.
- Give it a `down` that undoes the `up` whenever the change is reversible. Without one it cannot be rolled back.
- Declare the schemas it changes on a leading line of its `up`, `-- schemas sales, billing`, and qualify every name with its schema (`sales.orders`). A migration waits only on earlier migrations whose schemas overlap its own. One that declares none applies to the whole database and every later migration waits for it, so leave the line out only for a change that really spans the database, such as an extension.
- Migrations apply oldest first. One that is applied is not edited: correct it with a new migration.
- Row data is not schema. Insert, update and delete rows with plain SQL, migrations or not.
<!-- cli-only -->

`wmill datatable migrate new <name>` scaffolds `migrations/datatable/<datatable>/<timestamp>_<name>.up.sql` and `.down.sql`, `wmill datatable migrate up` pushes the local files and applies what is pending, `wmill datatable migrate down` rolls back the most recent one and `wmill datatable migrate status` lists them. All take `-d <datatable>` and default to `main`. `migrate new --schemas sales,billing` writes the `-- schemas` line, and `migrate up --schema sales` applies only the pending migrations declaring that schema.
<!-- /cli-only -->

## A migration is not a draft

A migration is saved to the workspace and changes the database as soon as it is applied, for everything that reads those tables. To try a schema out without touching tables already in use:

- put the new tables in a schema of their own, created by the same migration (`CREATE SCHEMA IF NOT EXISTS`), and point what you are building at it. A raw app names its schema in its `data` config, which is why a new app gets a new schema by default;
- or work in a workspace fork whose data table was cloned when the fork was created ("Clone schema only" or "Clone schema and data"): migrations applied there change that copy alone, and reach the parent workspace when the fork is merged. A fork that kept the original data table points at the parent's database, so a migration applied in it changes the parent's tables at once.

Altering or dropping a table other scripts or apps use is neither of those: say what depends on it before doing it.

## Roles and permissions

Data table roles are a self-hosted Enterprise Edition feature, for data tables on managed Postgres (the instance database or the external managed instance). Without them, and always on a data table backed by a Postgres resource, everyone who can use the data table reaches all of it: there is no per-user, per-schema or per-row access to promise.

A data table under roles is reached as a Postgres role, and Postgres enforces what that role was granted:

- roles are assigned, per data table, to users, groups or folders. `admin` is the connection the data table had before roles: it owns the existing objects and ignores grants;
- a connection either names a role (`wmill.datatable('main', { role: 'analyst' })`, `role='analyst'` in Python, `datatable://main?role=analyst`) and holds that role alone, or names none and is resolved from the data table's default role;
- `permission denied` means the role in use lacks a grant. Say which role and which object: the grant is a data table admin's to give, in the roles drawer of the Database Manager. Do not switch to `admin` to get past it;
- a migration runs as the role its SQL declares on its first line, `-- role <name>`, the same one in its `down`. One that declares none runs as `admin`, which only those who may use `admin` can do.

Read the Permissions section of https://www.windmill.dev/docs/core_concepts/persistent_storage/data_tables before designing who reaches what: it covers tenants, grants, the default role and the roles a connection that names none ends up holding, rows that differ per viewer through row-level security, and how forks and raw apps behave under roles.
