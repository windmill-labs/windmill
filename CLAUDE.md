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

Use `wm-ts-nav` for fast symbol navigation. It uses tree-sitter with a SQLite index that auto-updates on each call (~13ms warm, ~2s cold). Supports Rust (.rs), TypeScript (.ts/.tsx/.js), and Svelte (.svelte — extracts `<script>` blocks).

If the binary is not available, build it first: `cd wm-ts-nav && cargo build --release`

```bash
# Find where a symbol is defined
wm-ts-nav/target/release/wm-ts-nav --root backend def "ServiceName"

# Search symbols by pattern (SQL LIKE wildcards supported)
wm-ts-nav/target/release/wm-ts-nav --root backend search "Trigger" --kind struct

# Show all symbols in a file (works with .rs, .ts, .svelte)
wm-ts-nav/target/release/wm-ts-nav --root backend outline backend/path/to/file.rs

# Search frontend symbols
wm-ts-nav/target/release/wm-ts-nav --root frontend/src search "favoriteManager"
wm-ts-nav/target/release/wm-ts-nav --root frontend/src outline frontend/src/lib/components/path/to/Component.svelte
```

Use `--root backend` for Rust, `--root frontend/src` for frontend. Kinds: `function`, `struct`, `enum`, `trait`, `impl`, `type_alias`, `const`, `static`, `mod`, `macro`, `interface`, `class`

## Core Principles

- Search for existing code to reuse before writing new code
- Follow established patterns in the codebase
- Keep changes focused — don't refactor beyond what's asked
