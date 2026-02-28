---
name: svelte-frontend
description: Svelte coding guidelines for the Windmill frontend. MUST use when writing or modifying code in the frontend directory.
---

# Windmill Svelte Patterns

Apply these Windmill-specific patterns when writing Svelte code in `frontend/`. For general Svelte 5 syntax (runes, snippets, event handling), use the Svelte MCP server.

## Windmill UI Components (MUST use)

Always use Windmill's design-system components. Never use raw HTML elements.

### Buttons — `<Button>`

```svelte
<script>
  import { Button } from '$lib/components/common'
  import { ChevronLeft } from 'lucide-svelte'
</script>

<Button variant="default" onclick={handleClick}>Label</Button>
<Button startIcon={{ icon: ChevronLeft }} iconOnly onclick={prev} />
```

Props: `variant?: 'accent' | 'accent-secondary' | 'default' | 'subtle'`, `unifiedSize?: 'sm' | 'md' | 'lg'`, `startIcon?: { icon: SvelteComponent }`, `iconOnly?: boolean`, `disabled?: boolean`

### Text inputs — `<TextInput>`

```svelte
<script>
  import { TextInput } from '$lib/components/common'
</script>

<TextInput bind:value={val} placeholder="Enter value" />
```

Props: `value?: string | number` (bindable), `placeholder?: string`, `disabled?: boolean`, `error?: string | boolean`, `size?: 'sm' | 'md' | 'lg'`

### Selects — `<Select>`

```svelte
<script>
  import Select from '$lib/components/select/Select.svelte'
</script>

<Select items={[{ label: 'Jan', value: 1 }]} bind:value={selected} />
```

Props: `items?: Array<{ label?: string; value: any }>`, `value` (bindable), `placeholder?: string`, `clearable?: boolean`, `size?: 'sm' | 'md' | 'lg'`

### Icons — `lucide-svelte`

Never write inline SVGs. Import from `lucide-svelte`:

```svelte
<script>
  import { ChevronLeft, X } from 'lucide-svelte'
</script>
<ChevronLeft size={16} />
```

## Form Components

Form components (TextInput, Toggle, Select, etc.) should use the unified size system when placed together.

## Styling

- Use Tailwind CSS for all styling — no custom CSS
- Use Windmill's theming classes for colors/surfaces (see `frontend/brand-guidelines.md`)
- Read component props JSDoc before using them

## Svelte MCP Server

Use the Svelte MCP tools when working on Svelte code:

1. **list-sections**: Call first to discover available docs
2. **get-documentation**: Fetch relevant sections based on use_cases
3. **svelte-autofixer**: MUST use on all Svelte code before finalizing — keep calling until no issues
4. **playground-link**: Only after user confirms and code was NOT written to project files
