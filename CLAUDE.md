# Windmill

Open-source platform for internal tools, workflows, API integrations, background jobs, and UIs. Rust backend + Svelte 5 frontend.

## Workflow

1. **Understand**: Before coding, read relevant docs from `docs/` to understand the area you're changing
2. **Plan**: For non-trivial changes, use plan mode. For large features, break into reviewable stages
3. **Execute**: Follow coding patterns from skills (`rust-backend`, `svelte-frontend`)
4. **Validate**: After every change, run the appropriate checks per `docs/validation.md`

## Documentation

- **Validation**: `docs/validation.md` — what checks to run based on what you changed
- **Enterprise**: `docs/enterprise.md` — EE file conventions and PR workflow
- **Backend patterns**: use the `rust-backend` skill when writing Rust code
- **Frontend patterns**: use the `svelte-frontend` skill when writing Svelte code. Do NOT edit svelte files unless you have read that skill.
- **Code review**: use `/local-review` to review a PR for bugs and CLAUDE.md compliance
- **Domain guides**: `.claude/skills/native-trigger/` and `frontend/tutorial-system-guide.mdc`
- **Brand/UI guidelines**: `frontend/brand-guidelines.md`

## Dev Environment

- **Backend**: `cargo run` from `backend/` (API at http://localhost:8000)
- **Frontend**: `REMOTE=http://localhost:8000 npm run dev` from `frontend/` (port 3000+)
- **DB**: `psql postgres://postgres:changeme@localhost:5432/windmill`
- **Login**: `admin@windmill.dev` / `changeme`
- **Instance settings**: navigate to `/#superadmin-settings`

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

`wm-ts-nav` is an AST-aware code navigator. Use **Grep** for regex/pattern search. Use **wm-ts-nav** for structural queries — it skips comments/strings and understands symbol boundaries.

**Prefer wm-ts-nav over Read** to save context window:
- `outline <file>` instead of reading a full file — understand structure first, then `body` or Read for specifics
- `body "X"` instead of reading a full file to see one function/struct
- `refs "X" --caller` instead of reading files to find which function contains each reference
- `callers "X"` / `callees "X"` for call-graph questions

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

## Core Principles

- Search for existing code to reuse before writing new code
- Follow established patterns in the codebase
- Keep changes focused — don't refactor beyond what's asked
