# Windmill

Open-source platform for internal tools, workflows, API integrations, background jobs, and UIs. Rust backend + Svelte 5 frontend.

## Workflow

1. **Understand**: Before coding, explore the codebase (see Code Navigation below). Use `outline` to understand file structure, `body` to read specific symbols, `def`/`callers`/`callees` to trace code, `Grep` to find usages. Read `docs/` for domain context.
2. **Plan**: For non-trivial changes, use plan mode. For large features, break into reviewable stages.
   For a new user-facing feature, put the `feature_usage` telemetry in the plan as a proposed item
   (see `docs/feature-telemetry.md`) so the user can keep or drop it — don't ask separately, and
   don't instrument bugfixes or refactors.
3. **Execute**: Follow coding patterns from skills (`rust-backend`, `svelte-frontend`)
4. **Validate**: After every change, run the appropriate checks per `docs/validation.md`, then
   **exercise the change on the running instance**. Type-checks are not verification. Whatever the
   change touches, get that path actually running, and stand up whatever that takes — this is
   expected, not a last resort. A few examples, not a closed list: drive the UI with the Playwright
   MCP, run a real job of the kind you touched, restart the backend with the cargo features the
   path needs (`backend/AGENTS.md`), put a stub in front of an upstream, start MinIO for an S3
   path, plant state with SQL, exercise it through the `wmill` CLI. If the path you need has no
   obvious way in, invent one rather than skipping it; `docs/` carries recipes for several areas.
   If it needs a credential or a third-party account, ask for one rather than skipping the test or
   inventing a value. If you genuinely cannot exercise it, say which path went unexercised instead
   of implying it was verified.

## Documentation

- **Validation**: `docs/validation.md` — what checks to run based on what you changed
- **Unreleased SDK changes**: `docs/wac-sdk-e2e.md` — exercising a client change on a real worker
- **Agent workers**: `docs/agent-worker-e2e.md` — building and running one locally. An agent
  reaches the DB only through the API, so `Connection::Http` paths are never taken by a plain
  `cargo run`; a normal build cannot start one at all.
- **Enterprise**: `docs/enterprise.md` — EE file conventions and PR workflow
- **Product telemetry**: `docs/feature-telemetry.md` — when to instrument a new feature with
  `feature_usage`, and the four-step recipe. An unregistered `(feature, kind)` pair is dropped
  silently, so frontend-only instrumentation records nothing.
- **Backend patterns**: use the `rust-backend` skill when writing Rust code
- **Frontend patterns**: use the `svelte-frontend` skill when writing Svelte code. Do NOT edit svelte files unless you have read that skill.
- **Frontend UUIDs**: do not call `crypto.randomUUID()` in frontend code. Import `randomUUID` from `$lib/utils/uuid` instead.
- **Code review**: review the current PR or branch against the shared review policy in `REVIEW.md` (severity triage, public-surface checklist, AGENTS.md compliance, test-coverage assessment). The skill at `.agents/skills/local-review/SKILL.md` orchestrates it. All three CLIs auto-discover the same SKILL — Claude reads `.claude/skills/` (symlinked to the canonical `.agents/skills/` file), Codex and Pi read `.agents/skills/` directly. Invoke with `/local-review` in Claude Code, `$local-review` (or `/skills` selector) in Codex, or `pi --skill local-review` / `/skill:local-review` in Pi. For a Codex-driven pass that mirrors the `codex-pr-review` GitHub action against your unpushed work (committed + uncommitted) before you push, use `/local-review-codex` (`.agents/skills/local-review-codex/`) — same `REVIEW.md` policy, `gpt-5.6-sol`, `xhigh` reasoning; requires the `codex` CLI >= 0.144.1.
- **Domain guides**: `.claude/skills/native-trigger/` and `frontend/tutorial-system-guide.mdc`
- **Brand/UI guidelines**: `frontend/brand-guidelines.md`
- **Domain vocabulary**: `CONTEXT.md` — the words this codebase uses for its own concepts (step, step setting, trigger step, …). Name things the way it does.
- **CLI commands**: when adding/modifying/removing a command, subcommand, option, or description in `cli/src/commands/`, run `python system_prompts/generate.py` to refresh `system_prompts/auto-generated/` and `cli/src/guidance/skills.gen.ts`. The CLI docs the agents use to operate `wmill` are derived from the source — stale generated files give agents the wrong flags.
- **Session recorder**: `frontend/src/lib/components/recording/` is also the recorder `wmill app dev --recording` serves, vendored into the CLI as `cli/src/commands/app/devRecorderBundle.gen.ts`. After changing `rawAppSnapshot.ts` or `rawAppRecording.svelte.ts`, run `bun run gen:dev-recorder` from `cli/` (`cli/test/dev_recorder_bundle_unit.test.ts` fails otherwise).
- **Raw-app policy**: `frontend/src/lib/components/raw_apps/rawAppPolicy.ts` also derives the policy the server's raw-app deploy stores, vendored into the bundle job as `backend/windmill-api/src/apps_raw_policy.gen.js`. After changing it or anything it imports, run `bun run gen:app-policy` from `cli/` (`cli/test/app_policy_bundle_unit.test.ts` fails otherwise). It rides in the job rather than being read from the CLI the job runs because the images install `windmill-cli` unpinned, so an image can carry one older than its server.

## Dev Environment

> **In a git worktree, the ports and database below are NOT the ones to use.** Each
> worktree gets its own backend port, frontend port and Postgres database, so the
> defaults in this section apply only to a plain single checkout. **Discover the real
> values before running anything** — see "Per-worktree ports and database" below.

**Check whether they are already running before starting anything.** In a webmux worktree
(`$WEBMUX_WORKTREE_PATH` is set) the backend and frontend are already up in sibling tmux panes —
use those, don't spawn your own. `tmux list-panes -t "$(tmux display-message -p -t "$TMUX_PANE"
'#{window_id}')" -F '#{pane_index} #{pane_current_command}'` shows what is running; read its log
with `tmux capture-pane`, and see `backend/AGENTS.md` to restart it with different cargo features.
A second server started in your own shell fights the first one for the port. The commands below
are for a plain checkout with nothing running.

- **Backend**: `cargo run` from `backend/` (API at http://localhost:8000)
- **Frontend**: `REMOTE=http://localhost:8000 npm run dev` from `frontend/` (port 3000+)
- **DB**: `psql postgres://postgres:changeme@localhost:5432/windmill`
- **Login**: `admin@windmill.dev` / `changeme`
- **Instance settings**: navigate to `/#superadmin-settings`
- **Migrations**: use `cargo sqlx migrate add -r <name>` from `backend/` to create new migrations (never generate timestamps manually)

### Per-worktree ports and database

In a webmux worktree the authoritative values live in
`$(git rev-parse --git-dir)/webmux/runtime.env` — `BACKEND_PORT`, `FRONTEND_PORT`,
`DATABASE_URL`, `CARGO_FEATURES`, `WM_DB_NAME`. Every pane sources it at startup. Read that
first: it is not a `.env*` file, so the repo's secret-file read rules don't stand in the way.

In a plain checkout, fall back to `.env` / `.env.local` (repo root) and `backend/.env`.

Each worktree gets a **brand-new database**, created and migrated from scratch by the post-create
hook. It is not a copy of the main dev instance: you get the `admins` workspace, the
`admin@windmill.dev` superadmin, the license key copied from the base database, and whatever the
migrations seed — and none of your own workspaces, scripts, flows or apps. Create whatever a test
needs. Cloning the base `windmill` database instead is
opt-in per project via `WM_CLONE_DB` in `.webmux.yaml`; read the note there before turning it on.

The database is named after the **worktree directory, not the branch** (`scripts/worktree-common.sh`):
`windmill_` + the directory basename with `-` → `_`, which Postgres then truncates at 63
characters. Branch `hugo/win-2340-ai-agent-evals-standalone-agent-runs-and-eval-datasets` sits in
a worktree directory named `win-2340-…`, so its database is
`windmill_win_2340_ai_agent_evals_standalone_agent_runs_and_eval` — no `hugo_`, and the tail
chopped. Take `WM_DB_NAME` from `runtime.env` instead of reconstructing the name. Read those, or
discover from what is already running:

```bash
psql postgres://postgres:changeme@localhost:5432/postgres -tAc \
  "select datname from pg_database where datname like 'windmill%'" | grep "$(git branch --show-current | tr - _)"
# the port the frontend actually proxies to (REMOTE of this worktree's vite):
for p in $(pgrep -f vite); do case "$(readlink /proc/$p/cwd)" in *"$(basename "$(git rev-parse --show-toplevel)")"*)
  tr '\0' '\n' < /proc/$p/environ | grep -E '^REMOTE=|^PORT=';; esac; done
```

Getting these wrong is not a cheap mistake:

- **`DATABASE_URL` pointed at another worktree's database silently destroys the sqlx
  cache.** `cargo run` and `cargo sqlx prepare` both compile `sqlx::query!` against the
  **live** database, so the wrong one fails with `relation "<your_new_table>" does not
  exist` — and `prepare` deletes the whole `.sqlx/` directory *before* it fails, leaving
  it gutted. Always `cp -r backend/.sqlx <tmp>/sqlx_backup` first (see the `update-sqlx`
  skill).
- **The frontend proxies to its own worktree's backend port, not 8000.** Starting a
  backend on the wrong port leaves the UI up but every API call 502s, which reads like an
  application bug rather than a misconfiguration.
- **Kill backends by pid scoped to this worktree's cwd** (`readlink /proc/<pid>/cwd`),
  never `pkill -f target/debug/windmill` — that kills every sibling worktree's backend.
  Beware that a `pgrep -f "<pattern>"` in a shell whose own command line contains
  `<pattern>` matches the shell itself.

## Code Navigation

`wm-ts-nav` is an AST-aware code navigator. Use **wm-ts-nav** for structural queries — it skips comments/strings and understands symbol boundaries.

**MUST use `outline` before `Read`** on unfamiliar files — a 500-line file costs ~500 lines of context, while `outline` costs ~20. Then **MUST use `body "X"`** instead of reading a full file to see one function/struct. Use `Read` with offset/limit only when you need surrounding context that `body` doesn't capture.
- `refs "X" --caller` instead of reading files to find which function contains each reference
- `callers "X"` / `callees "X"` for call-graph questions

EE files (`*_ee.rs`, `*_ee.ts`, `*_ee.svelte`) are indexed — you can `outline`, `def`, `body`, `refs` etc. on them just like regular files.

```bash
NAV="sh wm-ts-nav/nav"
# Use --root backend for Rust, --root frontend/src for TS/Svelte
$NAV --root backend outline backend/path/to/file.rs      # file structure
$NAV --root backend def "ServiceName"                     # find definition
$NAV --root backend body "decrypt_oauth_data"             # extract source code
$NAV --root backend search "%" --parent ServiceName       # methods on a type
$NAV --root backend search "Trigger" --kind struct        # find by kind
$NAV --root backend refs "X" --file handler.rs --caller   # scoped refs with caller
$NAV --root backend callers "X"                           # who calls X?
$NAV --root backend callees "X"                           # what does X call?
```

**Limitations** — syntax-level analysis, no type inference. Use **Grep** instead when completeness matters (finding all usages, exhaustiveness checks):
- `refs`/`callers`/`callees` can't follow re-exports, glob imports, or different import paths to the same symbol
- Trait impls, macro-generated symbols (`sqlx::FromRow`), and namespace member access (`ns.X`) are invisible
- `callees` shows all identifiers in a function body, not just actual calls

## Core Principles

- **MUST `outline` before `Read`** on unfamiliar files — then `body` or `Read` with offset/limit for specifics
- **Scratch stays outside the checkout.** Temp scripts, data dumps, cache backups and
  screenshots go in the session scratch directory or `/tmp`, so nothing temporary can end up
  committed. Write the paths in `rm`/`mv`/`cp` out literally: a PreToolUse hook proves each
  operand, and auto-allows deletes, moves, copies and mode changes under `/tmp`, inside a git
  checkout under `$HOME`, or in the Playwright MCP browser caches (`~/Library/Caches/ms-playwright`
  and `ms-playwright-mcp`, `~/.cache/…` on Linux), as long as one operation stays within a single
  root — a sibling checkout is a root of its own (`tar` and `unzip` stay `/tmp`-only). Chain
  deletes freely, each proved on its own operands, but keep writes to one per line, name the
  destination rather than a directory to drop it in, and put anything else on its own line: a
  command the hook does not prove drops the whole line back to the normal permission flow. A
  leading `~/` or `$HOME/` is expanded and proved; a quoted operand, any other `$VAR`, a redirect,
  a `$(…)`, a relative `cd`, or a wrapper like `xargs rm` cannot be, and that deferral is what
  turns a cleanup into a prompt.
- **Change files with Edit/Write, not the shell.** `sed -i`, `cat > file <<'EOF'` and inline
  `python3 - <<'PY'` scripts put an edit through the PreToolUse guards and the permission
  classifier, which match `Bash` and nothing else, so a routine edit arrives as a prompt. Bash
  stays right for running things — tests, builds, git, one-off queries.
- Search for existing code to reuse before writing new code
- Follow established patterns in the codebase
- Keep changes focused — don't refactor beyond what's asked
- **A simpler design found late is still the design.** Work already spent is not an argument
  for a shape, and neither is a clean review round, a passing suite, or a long PR thread. The
  signal to stop and re-derive rather than patch again is a change that keeps growing to defend
  its own structure: each review finding fixing an assumption the previous fix broke, the same
  class of bug reappearing somewhere new, or most of the diff being consequences of one early
  choice rather than the thing you set out to do. When that happens, say plainly what the
  simpler design is and what switching costs — a migration, a review cycle restarted from zero,
  work discarded — and let the user decide. Do not keep paying down the harder one because it
  is nearly finished, and do not present the accumulated cost as a reason to continue.
- **Ship only the tests the PR needs.** A committed test must pin behavior a future change could plausibly break, and be the smallest setup that exercises the new logic. While developing, write as many exhaustive tests and do as much manual testing as you need to convince yourself the change works — then remove that scaffolding before marking the PR ready, keeping only the essential regression guard(s). A test that merely re-exercises pre-existing behavior, or needs elaborate fixtures to assert something trivial, is scaffolding: delete it. If nothing meaningful is left to guard, ship no test rather than a ceremonial one.
- **Comments record constraints, not narration.** Write a comment only for what the code can't show: why a non-obvious approach is required, what breaks if it's "simplified" away. State each invariant once, at the place where someone would break it, in ≤4 lines. Don't describe what the next line does, don't repeat the same rationale at multiple sites, and don't address the PR reviewer (justifying a change belongs in the PR description, not the code). Reference nothing ephemeral — no numbered steps from your dev flow, no "the poller / the test does X" scaffolding, no transient state that won't exist for the next reader; keep only the essential, durable rationale. Describe the code as it is, never its drafting history: "we no longer do X", "unchanged behavior", "instead of the previous approach" are meaningless to a reader who never saw the earlier iteration — before finishing, reread your comments as if the current state is the only state that ever existed.
- **Never attribute work to a specific customer, account, or "requested by a customer" in repo-tracked content** (PR descriptions, commit messages, code comments, docs). Describe changes by their technical motivation instead.
