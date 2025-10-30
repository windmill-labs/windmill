# Frontend Development (Svelte 5)

## Core Principles

- Follow @svelte5-best-practices.mdc for detailed guidelines
- Use Runes ($state, $derived, $effect) for reactivity
- Keep components small and focused
- Always use keys in {#each} blocks

## UI Guidelines

### Styling Guidelines

- **Use Tailwind CSS** for all styling instead of custom CSS
- **Use Windmill's theming classes** for consistent colors and surfaces
- **Avoid custom styles** - prefer Tailwind utility classes
- **Follow existing patterns** - look at other components for reference
- **Respect design guidelines** - rules are defined in 'brand-guidelines.md'

#### Status Colors

Use standard Tailwind color classes with dark mode variants:

- Success: `text-green-500`, `bg-green-100 dark:bg-green-900/30`
- Error: `text-red-500`, `bg-red-50 dark:bg-red-900/20`
- Warning: `text-yellow-500`, `bg-yellow-100 dark:bg-yellow-900/30`
- Info: `text-blue-500`, `bg-blue-100 dark:bg-blue-900/30`

#### Typography

- `font-mono` - For code/technical content
- `text-xs`, `text-sm`, `text-2xs` - Standard text sizes
- Use `font-medium`, `font-semibold` for emphasis

### Layout Guidelines

- Use Tailwind spacing utilities (`p-3`, `m-2`, `gap-2`, etc.)
- Use flexbox/grid utilities for layouts
- Use `transition-colors` for smooth hover effects
- Use `overflow-hidden`, `rounded-md` for consistent card styles

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
