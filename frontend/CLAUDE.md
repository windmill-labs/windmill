# Frontend (Svelte 5)

- **Coding patterns**: MUST use the `svelte-frontend` skill when writing Svelte code
- **Validation**: `docs/validation.md` — `npm run check:fast` (2s) for iteration, `npm run check` (50s) for final PR
- **UI components**: use Windmill's design-system components (Button, TextInput, Select) — never raw HTML elements
- **Brand/design**: `frontend/brand-guidelines.md`
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
