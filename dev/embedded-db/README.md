# wm-embedded-db — oliphaunt/pglite-oxide dev-mode harness (WIN-2130)

Experiment harness for running Windmill against an **embedded Postgres**
([`pglite-oxide`](https://crates.io/crates/pglite-oxide), the crates.io release of
[`oliphaunt`](https://github.com/f0rr0/oliphaunt)) — Postgres 17.5 compiled to
WASIX, no Docker, no Postgres install.

See the write-up and conclusions in
[`docs/experiments/oliphaunt-lightweight-dev-mode.md`](../../docs/experiments/oliphaunt-lightweight-dev-mode.md).
**Short version: it works** — with the migration-bootstrap fix (merged in #9970),
a full Windmill server+worker boots on the embedded single-connection database and
runs scripts and flows.

This crate is intentionally **not** a member of the backend workspace: it pulls
the heavy `wasmer`/`wasix` tree and pins alpha versions, which must not leak into
the main backend `Cargo.lock`. The committed `Cargo.lock` here carries the
required `virtual-net = 0.702.0-alpha.3` pin (without it, `wasmer-wasix` fails to
compile — see the write-up).

## Binaries

### `wm-embedded-db` (launcher)

Boots an embedded Postgres, exports `DATABASE_URL`, and execs the command after
`--` with that env set. The DB lives in this parent process and shuts down when
the child exits.

```bash
cd dev/embedded-db

# Just boot the DB and print its URL, then idle (Ctrl-C to stop):
cargo run

# Boot the DB, then run the windmill binary against it (requires the migration
# fix from #9970, now on main):
cargo run -- ../../backend/target/debug/windmill
# -> windmill migrates, boots server+worker, and runs scripts/flows on pglite.
#    Run the windmill binary with DATABASE_CONNECTIONS=1 to match pglite's model.
```

Env:
- `WM_PGDATA` — data dir (default `./.wm-embedded-pgdata`; set empty for a temp DB)

### `schema-compat` (migration-compatibility probe)

Boots embedded Postgres and applies **Windmill's full migration set** on a single
connection, then samples core tables. This is the test that proved the schema is
Postgres-17 compatible (all migrations apply in ~1 s). It neutralizes the single
`uuid-ossp` `CREATE EXTENSION` line the same way Windmill's own migrator does.

```bash
cargo run --bin schema-compat
# -> [schema-compat] ALL 1182 migrations applied OK in ~950ms
```
