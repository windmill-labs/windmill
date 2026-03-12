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

`wm-ts-nav` is a tree-sitter navigator with SQLite index (~12ms warm). The `wm-ts-nav/nav` wrapper auto-rebuilds when source changes.

**When to use `wm-ts-nav` vs Grep:**
- "What's in this file?" → `outline` (full structure with signatures, saves reading the whole file)
- "Where is X defined?" → `def "X"` (finds declarations only, not usages)
- "What methods does type Y have?" → `search "%" --parent Y` (across all files)
- "What structs/functions match Y?" → `search "Y" --kind struct`
- "Where is X used in code?" → `refs "X"` (skips comments/strings, slower than grep but no noise)
- "Where is X used in this file?" → `refs "X" --file handler.rs` (scoped to one file)
- "Which function uses X?" → `refs "X" --caller` (shows containing function for each ref)
- "Show me function X's code" → `body "X"` (extracts just that symbol, no full file read)
- "What calls function X?" → `callers "X"` (cross-file call graph from refs+symbols)
- "What does function X call?" → `callees "X"` (all refs inside X's body)
- "Find a string/pattern in code" → use **Grep** (faster, but matches comments/strings too)

```bash
NAV="sh wm-ts-nav/nav"
$NAV --root backend outline backend/path/to/file.rs
$NAV --root backend def "ServiceName"
$NAV --root backend search "%" --kind function --parent ServiceName
$NAV --root backend search "Trigger" --kind struct
$NAV --root backend refs "ServiceName"
$NAV --root backend refs "Error" --file handler.rs
$NAV --root backend refs "ServiceName" --caller
$NAV --root backend body "update_index"
$NAV --root backend callers "ServiceName"
$NAV --root backend callees "update_index"
$NAV --root frontend/src outline frontend/src/path/to/Component.svelte
$NAV --root frontend/src search "favoriteManager"
```

Use `--root backend` for Rust, `--root frontend/src` for frontend. Kinds: `function`, `struct`, `enum`, `trait`, `impl`, `type_alias`, `const`, `static`, `mod`, `macro`, `interface`, `class`

## Core Principles

- Search for existing code to reuse before writing new code
- Follow established patterns in the codebase
- Keep changes focused — don't refactor beyond what's asked
