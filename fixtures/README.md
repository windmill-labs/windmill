# Fixtures

Helpers for sharing a reproducible test workspace alongside a PR.

The workflow is:

1. While iterating on a PR, snapshot your local test workspace into
   `fixtures/cli-sync/` so a teammate (or CI) can replay it.
2. Commit the snapshot on your branch so reviewers can load it locally.
3. **Before merging**, clear `fixtures/cli-sync/` again. CI fails on `main` /
   PRs that try to merge a non-empty fixture (see `check-empty.sh`).

## Scripts

All scripts assume:

- A local Windmill backend running at `http://localhost:8000` (see top-level
  `AGENTS.md` for `cargo run` / `npm run dev`).
- Default super-admin credentials `admin@windmill.dev` / `changeme`.
- `bun` and `python3` are installed and on `PATH`. No `wmill` install
  required — the scripts invoke `cli/src/main.ts` via `bun run` directly.
  `python3` is used only to build correctly-escaped JSON request bodies.

The login token is passed to the CLI via `--token`, which means it appears in
`/proc/<pid>/cmdline` for the duration of the `bun run` call. This is fine
against a local dev instance; if you point the scripts at a real instance,
the credentials are no more exposed than running `wmill` directly with
explicit flags, but bear it in mind.

### `load.sh` — load the fixture into a fresh workspace

```bash
./fixtures/load.sh
```

Logs in as `admin@windmill.dev`, creates a new workspace with a random id
(`fixture-<8 hex chars>`), and pushes `fixtures/cli-sync/` into it via
`wmill sync push`. Prints the workspace id at the end so you can open it in
the UI.

Flags:
- `--base-url <url>` (default `http://localhost:8000`)
- `--email <email>` (default `admin@windmill.dev`)
- `--password <pwd>` (default `changeme`) — prefer `WMILL_PASSWORD=<pwd>` env
  var when using a real password, since `--password` ends up in `ps` /
  shell history.
- `--workspace <id>` (default `fixture-<random>`)
- `--dir <path>` (default `fixtures/cli-sync`)

### `snapshot.sh` — snapshot a workspace into the fixture folder

```bash
./fixtures/snapshot.sh <workspace-id>
```

Pulls the given workspace into `fixtures/cli-sync/` via `wmill sync pull`.
The target directory is cleared first (everything except `wmill.yaml` and
`.gitkeep`) so the snapshot reflects exactly what's in the workspace.

Flags: same as `load.sh` minus `--workspace` (passed as positional arg).

### `check-empty.sh` — fail if the fixture is non-empty

Used by CI to guard `main`. Run it locally before opening a PR for merge:

```bash
./fixtures/check-empty.sh
```
