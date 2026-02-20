# Frontend Development (Svelte 5)

## Core Principles

- Follow the `svelte-frontend` skill for best practices: .claude/skills/svelte-frontend/SKILL.md
- Use Runes ($state, $derived, $effect) for reactivity
- Keep components small and focused
- Always use keys in {#each} blocks

## Data Flow and State Management

### Prefer Unidirectional Data Flow with Composable State

When you can use unidirectional data flow with composable state, prefer that over two-way binding between components. Two-way binding between components creates confusing data flow - as the codebase grows, nothing guarantees that bound state won't be updated from multiple locations, leading to bugs and maintenance issues.

**❌ AVOID: Two-way binding when composable state would work**

```svelte
<Loader bind:loading bind:items {args} />
```

**✅ PREFER: Unidirectional data flow with composables**

```typescript
// loader.svelte.ts
function useLoader(argsGetter: () => Args) {
  let args = $derived(argsGetter())
  let items = $state([])
  let loading = $state(false)

  $effect(() => {
    // Logic reactive to args changes
  })

  return {
    get loading() { return loading },
    get items() { return items }
  }
}

// Component.svelte
<script>
  let loader = useLoader(() => args)
  let loading = $derived(loader.loading)
  let items = $derived(loader.items)
</script>
```

This pattern ensures:
- State responsibility is clearly owned by `useLoader`
- Data flows in one direction (parent → child)
- No ambiguity about where state can be modified
- Better maintainability as the codebase scales

### Async Data Fetching with Runed

For async requests, **always use `resource()` from the Runed library** instead of manual state management:

```typescript
import { resource } from 'runed'

let items = resource(() => args, (args) => YourService.route(args))

// Access loading state
items.loading

// Access data
items.current
```

The `resource()` utility:
- Automatically handles loading states
- Manages async lifecycle
- Provides reactive updates when dependencies change
- Eliminates boilerplate for common async patterns

**Key Takeaway**: Prefer unidirectional data flow with composables over two-way binding between components. Two-way binding is acceptable for simple form inputs, but avoid it when composable state patterns can provide clearer state ownership.

## UI Guidelines

### Styling Guidelines

- **Use Tailwind CSS** for all styling instead of custom CSS
- **Use Windmill's theming classes** for consistent colors and surfaces
- **Avoid custom styles** - prefer Tailwind utility classes
- **Follow existing patterns** - look at other components for reference
- **Respect design guidelines** - rules are defined in 'brand-guidelines.md'

### UI Components

- Use frontend/src/lib/components/common/button/Button.svelte for all buttons
- Use the component TextInput for all text inputs
- Form components (TextInputs, ToggleButtons, Select ...) should all use the same size when put together, using the unified size system.
- Read carefully components props JSDoc before using them

## Code Validation (MUST DO)

After making frontend changes, you MUST run the following and fix all errors and warnings before considering the work done:

```bash
npm run check:fast
```

At the end of a PR to do final validation, you can do the longer one (2s for fast vs 50s for the slow one):
```bash
npm run check
```

## Backend API

- If you need to call the backend API, you can find the available routes in ../backend/windmill-api/openapi.yaml
- You can also use the associated types and services that are auto generated from the openapi file. They are in src/lib/gen/\*gen.ts files

### OpenAPI Autogeneration

Windmill automatically generates TypeScript types and services from the OpenAPI specification.

#### Service Generation Pattern

The autogeneration follows this pattern:

- **Tag** → **Service Name**: The OpenAPI tag becomes the service name with "Service" suffix
- **operationId** → **Method Name**: The operationId becomes the method name in the service

#### Example

Given this OpenAPI specification:

```yaml
/w/{workspace}/audit/list:
  get:
    summary: list audit logs (requires admin privilege)
    operationId: listAuditLogs
    tags:
      - audit
    parameters:
      - $ref: '#/components/parameters/WorkspaceId'
      - $ref: '#/components/parameters/Page'
      - $ref: '#/components/parameters/PerPage'
      - $ref: '#/components/parameters/Before'
      - $ref: '#/components/parameters/After'
      - $ref: '#/components/parameters/Username'
      - $ref: '#/components/parameters/Operation'
      - name: operations
        in: query
        description: comma separated list of exact operations to include
        schema:
          type: string
```

This generates:

- **Service**: `AuditService` (from tag "audit")
- **Method**: `listAuditLogs` (from operationId)

#### Method Arguments

The generated method arguments correspond to the OpenAPI parameters:

```typescript
AuditService.listAuditLogs({
  workspace: string,           // from WorkspaceId parameter
  page?: number,              // from Page parameter
  perPage?: number,           // from PerPage parameter
  before?: string,            // from Before parameter
  after?: string,             // from After parameter
  username?: string,          // from Username parameter
  operation?: string,         // from Operation parameter
  operations?: string         // from operations parameter
})
```

## Svelte 5 documentation

You are able to use the Svelte MCP server, where you have access to comprehensive Svelte 5 and SvelteKit documentation. Here's how to use the available tools effectively:

### 1. list-sections

Use this FIRST to discover all available documentation sections. Returns a structured list with titles, use_cases, and paths.
When asked about Svelte or SvelteKit topics, ALWAYS use this tool at the start of the chat to find relevant sections.

### 2. get-documentation

Retrieves full documentation content for specific sections. Accepts single or multiple sections.
After calling the list-sections tool, you MUST analyze the returned documentation sections (especially the use_cases field) and then use the get-documentation tool to fetch ALL documentation sections that are relevant for the user's task.

### 3. svelte-autofixer

Analyzes Svelte code and returns issues and suggestions.
You MUST use this tool whenever writing Svelte code before sending it to the user. Keep calling it until no issues or suggestions are returned.

### 4. playground-link

Generates a Svelte Playground link with the provided code.
After completing the code, ask the user if they want a playground link. Only call this tool after user confirmation and NEVER if code was written to files in their project.
