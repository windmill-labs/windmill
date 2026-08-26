# Frontend (Svelte 5)

- **Coding patterns**: MUST use the `svelte-frontend` skill when writing Svelte code
- **Validation**: `docs/validation.md` — `npm run check:fast` (2s) for iteration, `npm run check` (50s) for final PR
- **UI components**: use Windmill's design-system components — never raw HTML elements. Start from the barrel `src/lib/components/common/index.ts` and grep `src/lib/components/`; the component you need almost certainly exists
- **Never pass a `@deprecated` prop.** On `<Button>` that means `size`, `spacingSize`, `extendedSize` and the `contained`/`border`/`divider` variants — size buttons with `unifiedSize` (`2xs` | `xs` | `sm` | `md` | `lg`). Deprecated props survive at old call sites; copying one forward is still a bug. Check the prop's JSDoc in the component before using it
- **Brand/design**: `frontend/brand-guidelines.md` — read the relevant section before building UI, not after; the `svelte-frontend` skill maps which section covers what
- **Backend API**: routes in `../backend/windmill-api/openapi.yaml`, generated types in `src/lib/gen/`
- **Regenerate client**: `npm run generate-backend-client` after backend API changes

## Key Frontend Patterns

### Prefer Composable State Over Two-Way Binding

```typescript
// Use resource() from runed for async data
import { resource } from 'runed'
let items = resource(() => args, (args) => SomeService.list(args))
// items.loading, items.current

// Use composables for shared reactive state
function useLoader(argsGetter: () => Args) {
  let items = $state([])
  let loading = $state(false)
  $effect(() => { /* react to argsGetter() */ })
  return { get loading() { return loading }, get items() { return items } }
}
```

Two-way binding is fine for simple form inputs. Avoid it for component-to-component state.

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

Write screenshots to an absolute path under `/tmp` (the MCP servers already do; standalone
Playwright scripts must be told): moving a PNG out of the checkout afterwards needs a `mv` the
permission hooks always prompt on. Same reason to run `rm`/`mv`/`cp` as one plain command per Bash
call: those hooks defer on `&&`, `;`, redirects, quotes and `$VAR`.

**Attach the screenshots to the PR.** For any change under `frontend/`, embed screenshots of the affected UI in the PR body — the `pr` skill requires this and carries the upload recipe.

If you cannot exercise a UI change (no dev server, etc.), say so explicitly rather than claiming success.

### Traps while driving the UI

- `critical_alerts` 404s are expected on CE builds (EE-only endpoint) — ignore them.
- VSCode worker 404s are dev-mode artifacts — ignore them.
- `<Toggle>` hides its checkbox (`sr-only`). Click the `<label>` wrapper, not the checkbox.

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
