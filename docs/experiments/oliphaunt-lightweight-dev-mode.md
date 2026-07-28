# Experiment: oliphaunt / pglite-oxide as a lightweight dev-mode database (WIN-2130)

**Goal:** find out whether Windmill can run against
[`oliphaunt`](https://github.com/f0rr0/oliphaunt) — published on crates.io as
[`pglite-oxide`](https://crates.io/crates/pglite-oxide) — as a "lightweight dev
mode": an embedded Postgres with no Docker and no separate Postgres install, so
that `cargo run` alone gives you a working instance. The bar was "ideally get
scripts and flows running".

**Verdict (TL;DR): it works.** With a small migration-bootstrap fix (merged to
`main` in #9970), a full Windmill server+worker boots on the embedded
`pglite-oxide` database (`PostgreSQL 17.5 on wasm32-unknown-wasix`) and **runs
scripts and flows** — over a single database connection.

```
SCRIPT: {"started":true,"success":true,"completed":true,"result":"6x7=42"}
FLOW:   {"started":true,"success":true,"completed":true,"result":"step-b-ok-flow-complete"}
PostgreSQL version: PostgreSQL 17.5 on wasm32-unknown-wasix … 32-bit
```

The reproducible harness lives in [`dev/embedded-db/`](../../dev/embedded-db).

---

## What oliphaunt / pglite-oxide is

- **PostgreSQL 17.5 compiled to WASIX** (PGlite lineage), run in-process through a
  bundled `wasmer` runtime. No Docker, no local Postgres, no runtime build.
- `PgliteServer::builder().path(dir).start()` boots a local server and
  `server.database_url()` returns a normal `postgresql://…` URL for SQLx /
  `tokio-postgres` / `psql`.
- Bundles `pgvector`, `pg_trgm`, `hstore`, `citext`, `ltree`, `pg_dump`.
- **Serves one client connection at a time** — the crate is explicit:
  *"the server owns one embedded backend, so downstream pools should use a single
  connection."* This single-connection property is the whole story below.

---

## The key insight: single-connection ≡ `max_connections=1`

pglite's "one backend at a time" behaves **exactly** like pointing Windmill at any
normal Postgres with a `max_connections=1` SQLx pool: the first physical
connection is served; a *concurrently-needed* second one blocks. That equivalence
let the whole investigation run against the fast local dev Postgres (with
`DATABASE_CONNECTIONS=1`) and only use pglite for final confirmation.

### The runtime does NOT need two connections

Running Windmill standalone against a normal Postgres with the runtime pool capped
at one connection (`DATABASE_CONNECTIONS=1`), the server and worker share that
single connection and **scripts and flows execute fine** — the instance holds
exactly one live Postgres connection the whole time (verified via
`pg_stat_activity`). Everything else people assume "needs" concurrency — HTTP
handlers, the worker job-poll loop, the health checker — just *serializes* on the
pool. That's slower under load, but perfectly correct for dev.

So most of what looks like a concurrency requirement is only pool *contention*
that queues. The real blockers are the few places that **hold one connection open
and then await a second acquisition** — a genuine self-deadlock at
`max_connections=1`. There are exactly three, and all three are in the **one-time
migration bootstrap**:

| # | Site | Problem |
|---|---|---|
| 1 | `migrate()` housekeeping (`windmill-api/src/db.rs`) | Holds `db.acquire()` (conn #1) while running `DELETE … _sqlx_migrations` via `.execute(db)` (needs conn #2) |
| 2 | `migrate()` stale-migration cleanup (same file) | Same pattern inside the per-migration loop |
| 3 | `fix_flow_versioning_migration` (`windmill-api/src/live_migrations.rs`) | Holds the migrator connection (with the migration advisory lock) while doing `fetch_one(db)` / `db.begin()` on the pool |

`initial_connection()` also hardcodes `max_connections(2)`, but that's only a
*ceiling* — once the three sites above stop asking for a second connection, the
migration phase only ever uses one, so the ceiling is never hit.

---

## The fix (merged in #9970)

Route those three migration-phase queries onto the connection the migrator has
**already checked out** (and, for #3, already holds the advisory lock on) instead
of re-acquiring from the pool. A `CustomMigrator::connection()` accessor
(`windmill-api/src/db.rs`) exposes that held connection; `migrate()` and
`fix_flow_versioning_migration` (`windmill-api/src/live_migrations.rs`) use it.

This is not a pglite-specific hack — it is a strict improvement for **any**
connection-constrained Postgres (managed providers with tight `max_connections`,
PgBouncer transaction pooling, etc.): fewer connections during migration, and for
#3 the existence-check and the write now run on the same advisory-locked
connection (tighter, not looser). Default behavior is unchanged for normal
multi-connection setups.

With it in place, Windmill boots on pglite and runs scripts and flows (see the
TL;DR output). No `SKIP_MIGRATION`, no schema edits.

---

## What else was needed (already handled by Windmill)

- **`uuid-ossp`** is the only extension pglite lacks, and Windmill doesn't need it:
  `uuid_generate_v*` is never used (the schema uses built-in `gen_random_uuid()`),
  and Windmill already strips that `CREATE EXTENSION` line in
  `OVERRIDDEN_MIGRATIONS` (a compat path for managed providers that forbid
  `CREATE EXTENSION`). All 1182 migrations otherwise apply cleanly — verified
  independently by the `schema-compat` probe (~950 ms on a single connection).

---

## Caveats / known limitations

- **Serialized on one connection** — correct but not fast under concurrent load.
  Fine for a single-developer dev instance; not for anything multi-user.
- **32-bit wasm backend** (`wasm32-unknown-wasix`) — 4 GiB address-space ceiling
  and lower memory limits than native Postgres.
- **Alpha dependency pin.** `pglite-oxide 0.5.1` pulls `wasmer`/`wasmer-wasix`
  `0.702.0-alpha.3`; cargo resolves the transitive `virtual-net` to stable
  `0.702.0` (which added `NetworkError::MessageSize`), and the alpha
  `wasmer-wasix` doesn't handle it → won't compile. Pin with
  `cargo update -p virtual-net --precise 0.702.0-alpha.3` (the harness commits a
  `Cargo.lock` with this pin).
- **Language runtimes are orthogonal.** Scripts/flows run to the extent their
  language is compiled into the binary (e.g. a `--features quickjs`-only build
  runs bash but not Python) — unrelated to the database.

---

## Recommendation

- The migration-bootstrap fix is worth taking on its own merits — it removes an
  unnecessary second-connection requirement during migration that also affects
  tightly connection-limited managed Postgres, not just pglite.
- With it, `pglite-oxide` is a viable **zero-dependency dev database**: one binary,
  no Docker, no Postgres install, boots in ~200 ms, runs scripts and flows. A
  natural next step is a first-class `--embedded-db` dev flag that boots pglite and
  wires `DATABASE_URL` automatically (the harness in `dev/embedded-db` prototypes
  exactly this).
- Keep the single-connection caveat in mind: it's a dev-mode convenience, not a
  path to running production or load tests embedded.

---

## Reproducing

See [`dev/embedded-db/README.md`](../../dev/embedded-db/README.md). In short:

```bash
# Prove the schema migrates on a single connection:
cd dev/embedded-db && cargo run --bin schema-compat

# Boot a real Windmill (built with the migration fix) on embedded pglite:
cargo run -- ../../backend/target/debug/windmill
# then hit http://localhost:$PORT and run a script / flow
```

Isolate the blockers yourself without pglite, using the equivalence above:

```bash
# fresh empty DB + runtime pool capped at 1 connection
DATABASE_URL=postgres://postgres:changeme@127.0.0.1:5432/<empty_db> \
DATABASE_CONNECTIONS=1 PORT=8273 ./backend/target/debug/windmill
# scripts and flows run; pg_stat_activity shows a single connection
```
