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

## CLI: `nonDottedPaths` Convention

The CLI supports two folder/file naming modes controlled by `nonDottedPaths` in `wmill.yaml`:

| | Dotted (default) | Non-dotted (`nonDottedPaths: true`) |
|---|---|---|
| Flow folders | `name.flow/` | `name__flow/` |
| App folders | `name.app/` | `name__app/` |
| Raw app folders | `name.raw_app/` | `name__raw_app/` |
| Inline scripts | `a.inline_script.ts` | `a.ts` |
| Lock files | `a.inline_script.lock` | `a.lock` |

**Required pattern**: Every call to `newPathAssigner()` or `extractInlineScripts*()` in CLI code **must** pass the current setting:

```typescript
newPathAssigner(defaultTs, { skipInlineScriptSuffix: getNonDottedPaths() })
```

Use `getFolderSuffix(type)` for folder suffixes — never hardcode `.flow`/`__flow`.

Key files: `cli/src/utils/resource_folders.ts` (config), `cli/windmill-utils-internal/src/path-utils/path-assigner.ts` (path generation).

## Code Navigation

`wm-ts-nav` is an AST-aware code navigator. Use **wm-ts-nav** for structural queries — it skips comments/strings and understands symbol boundaries.

**MUST use `outline` before `Read`** on unfamiliar files — a 500-line file costs ~500 lines of context, while `outline` costs ~20. Then **MUST use `body "X"`** instead of reading a full file to see one function/struct. Use `Read` with offset/limit only when you need surrounding context that `body` doesn't capture.
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

**Limitations** — syntax-level analysis, no type inference. Use **Grep** instead when completeness matters (finding all usages, exhaustiveness checks):
- `refs`/`callers`/`callees` can't follow re-exports, glob imports, or different import paths to the same symbol
- Trait impls, macro-generated symbols (`sqlx::FromRow`), and namespace member access (`ns.X`) are invisible
- `callees` shows all identifiers in a function body, not just actual calls

## Core Principles

- **MUST `outline` before `Read`** on unfamiliar files — then `body` or `Read` with offset/limit for specifics
- Search for existing code to reuse before writing new code
- Follow established patterns in the codebase
- Keep changes focused — don't refactor beyond what's asked
