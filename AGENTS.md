# Windmill

Open-source platform for internal tools, workflows, API integrations, background jobs, and UIs. Rust backend + Svelte 5 frontend.

## Workflow

1. **Understand**: Before coding, explore the codebase (see Code Navigation below). Use `outline` to understand file structure, `body` to read specific symbols, `def`/`callers`/`callees` to trace code, `Grep` to find usages. Read `docs/` for domain context.
2. **Plan**: For non-trivial changes, use plan mode. For large features, break into reviewable stages
3. **Execute**: Follow coding patterns from skills (`rust-backend`, `svelte-frontend`)
4. **Validate**: After every change, run the appropriate checks per `docs/validation.md`

## Documentation

- **Validation**: `docs/validation.md` — what checks to run based on what you changed
- **Enterprise**: `docs/enterprise.md` — EE file conventions and PR workflow
- **Backend patterns**: use the `rust-backend` skill when writing Rust code
- **Frontend patterns**: use the `svelte-frontend` skill when writing Svelte code. Do NOT edit svelte files unless you have read that skill.
- **Frontend UUIDs**: do not call `crypto.randomUUID()` in frontend code. Import `randomUUID` from `$lib/utils/uuid` instead.
- **Code review**: review the current PR or branch against the shared review policy in `REVIEW.md` (severity triage, public-surface checklist, AGENTS.md compliance, test-coverage assessment). The skill at `.agents/skills/local-review/SKILL.md` orchestrates it. All three CLIs auto-discover the same SKILL — Claude reads `.claude/skills/` (symlinked to the canonical `.agents/skills/` file), Codex and Pi read `.agents/skills/` directly. Invoke with `/local-review` in Claude Code, `$local-review` (or `/skills` selector) in Codex, or `pi --skill local-review` / `/skill:local-review` in Pi.
- **Domain guides**: `.claude/skills/native-trigger/` and `frontend/tutorial-system-guide.mdc`
- **Brand/UI guidelines**: `frontend/brand-guidelines.md`
- **CLI commands**: when adding/modifying/removing a command, subcommand, option, or description in `cli/src/commands/`, run `python system_prompts/generate.py` to refresh `system_prompts/auto-generated/` and `cli/src/guidance/skills.gen.ts`. The CLI docs the agents use to operate `wmill` are derived from the source — stale generated files give agents the wrong flags.

## Dev Environment

- **Backend**: `cargo run` from `backend/` (API at http://localhost:8000)
- **DuckDB local jobs**: before running DuckDB scripts locally, build the FFI shared library with `cd backend/windmill-duckdb-ffi-internal && ./build_dev.sh`. Re-run it after clean builds or when `backend/target/debug/libwindmill_duckdb_ffi_internal.*` is missing.
- **Frontend**: `REMOTE=http://localhost:8000 npm run dev` from `frontend/` (port 3000+)
- **DB**: `psql postgres://postgres:changeme@localhost:5432/windmill`
- **Login**: `admin@windmill.dev` / `changeme`
- **Instance settings**: navigate to `/#superadmin-settings`
- **Migrations**: use `cargo sqlx migrate add -r <name>` from `backend/` to create new migrations (never generate timestamps manually)

## Verifying Frontend Changes

After modifying frontend code, drive the running dev server with the **Playwright MCP** to verify the change in a real browser — don't claim a UI change works without exercising it.

Two MCP servers are registered in `.mcp.json`:
- `playwright` — headless Chromium, default for devboxes (no display required)
- `playwright-headed` — windowed Chromium, when a display is available

**One-time setup:** run `npx playwright install chromium` to download the browser binary (Playwright won't fetch it automatically on first use).

Typical flow:
1. Ensure backend (`cargo run`) and frontend (`REMOTE=http://localhost:8000 npm run dev`) are running
2. `mcp__playwright__browser_navigate` to the relevant page (login at `admin@windmill.dev` / `changeme`)
3. `mcp__playwright__browser_snapshot` to inspect the accessibility tree (preferred over screenshots for reading the DOM)
4. `mcp__playwright__browser_click` / `browser_fill_form` / `browser_type` to interact
5. `mcp__playwright__browser_take_screenshot` for visual confirmation
6. `mcp__playwright__browser_console_messages` / `browser_network_requests` to surface errors

**Attach the screenshots to the PR.** For any change under `frontend/`, embed screenshots of the affected UI in the PR body — the `pr` skill requires this and carries the upload recipe.

If you cannot exercise a UI change (no dev server, etc.), say so explicitly rather than claiming success.

## Banned Patterns

### `$bindable(default_value)` on optional props

Using `$bindable(default_value)` on props that can be `undefined` is **banned**. This pattern causes subtle bugs because the default value masks the `undefined` state.

**Bad:**

```svelte
let { my_prop = $bindable(default_value) }: { my_prop?: string } = $props()
```

**Correct alternatives:**

1. **Use `$derived` with nullish coalescing** — handle the potential `undefined` at the usage site:

   ```svelte
   let { my_prop = $bindable() }: { my_prop?: string } = $props()
   let effective_value = $derived(my_prop ?? default_value)
   ```

2. **Create a `useMyPropState()` helper** — encapsulate the undefined-handling logic in a reusable function and call it higher in the component tree, so the child component always receives a defined value.

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
- Search for existing code to reuse before writing new code
- Follow established patterns in the codebase
- Keep changes focused — don't refactor beyond what's asked
- **Comments record constraints, not narration.** Write a comment only for what the code can't show: why a non-obvious approach is required, what breaks if it's "simplified" away. State each invariant once, at the place where someone would break it, in ≤4 lines. Don't describe what the next line does, don't repeat the same rationale at multiple sites, and don't address the PR reviewer (justifying a change belongs in the PR description, not the code). Describe the code as it is, never its drafting history: "we no longer do X", "unchanged behavior", "instead of the previous approach" are meaningless to a reader who never saw the earlier iteration — before finishing, reread your comments as if the current state is the only state that ever existed.
- **Never attribute work to a specific customer, account, or "requested by a customer" in repo-tracked content** (PR descriptions, commit messages, code comments, docs). Describe changes by their technical motivation instead.
