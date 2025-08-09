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

### Windmill Theme Classes

Use these semantic color classes that automatically handle light/dark modes:

#### Backgrounds
- `bg-surface` - Main surface background
- `bg-surface-secondary` - Secondary/elevated surfaces  
- `bg-surface-hover` - Hover states for interactive elements

#### Text Colors
- `text-primary` - Primary text color
- `text-secondary` - Secondary text (less prominent)
- `text-tertiary` - Tertiary text (subtle/muted)

#### Borders
- `border-gray-200 dark:border-gray-700` - Standard borders that adapt to theme

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
