---
name: svelte-frontend
description: Svelte coding guidelines for the Windmill frontend. MUST use when writing or modifying code in the frontend directory.
---

# Windmill Svelte Patterns

Apply these Windmill-specific patterns when writing Svelte code in `frontend/`. For general Svelte 5 syntax (runes, snippets, event handling), use the Svelte MCP server.

## Before writing any UI (MUST)

Do both of these before the first line of markup — not after, and not only when something
looks unfamiliar.

**1. Find the component that already exists.** `frontend/src/lib/components/common/index.ts`
is the design-system barrel — 28 lines, read it in full. It exports far more than the three
documented below: `Alert`, `Badge`, `Breadcrumb`, `Drawer`/`DrawerContent`, `Menu`/`MenuItem`,
`Tabs`/`Tab`/`TabContent`, `Skeleton`, `FileInput`, `RadioCard`, `Section`, `Kbd`, `ActionRow`,
`ClearableInput`, `CopyButton`, `SecondsInput`, `UndoRedo`, `Url`.

The barrel is not the full picture either: `common/` has 34 subdirectories and only 23 exports,
so `modal/`, `popup/`, `stepper/`, `tooltip/`, `checkbox/`, `table/`, `contextmenu/`,
`confirmationModal/`, `calendarPicker/`, `fileUpload/`, `toggleButton-v2/` and more exist but
must be imported by path. Selects, text inputs and melt-based primitives sit next to `common/`
in `components/select/`, `components/text_input/`, `components/meltComponents/`.

The tree holds 1,600+ components — grep `frontend/src/lib/components` for the thing you're about
to build; it almost certainly exists. Building a new one is the last resort, not the first move.

**2. Read the guideline for what you're building.** `frontend/brand-guidelines.md` is the
authority on how it should look and read. Don't load all 34k chars — jump to the section:

| Building | Section to read |
|---|---|
| Any new screen or component | `# Components` (Core Rules, Quick Reference) |
| Buttons, CTAs | `## Buttons` — hierarchy matters, only one Accent per view |
| Colors, surfaces, borders | `# Color system` (Quick Reference, Do's and Don'ts) |
| Text, labels, headings | `# Typography` — note `## Text Casing`, sentence case throughout |
| Spacing, grids, page structure | `# Spacing & Layout`; `# Layout` → `## Form` for forms |
| Shadows, overlays, depth | `# Elevation` |
| Icons | `# Iconography` |
| Wording of any UI copy | `# Voice & Communication`, `# Tone of Voice` |

Get the line range with `grep -n '^#' frontend/brand-guidelines.md`, then read just that span.

## Windmill UI Components (MUST use)

Always use Windmill's design-system components. Never use raw HTML elements. The three below
are the ones you'll reach for most often — they are examples, not the catalog. For anything
else, go back to the barrel and grep.

### Buttons — `<Button>`

```svelte
<script>
  import { Button } from '$lib/components/common'
  import { ChevronLeft } from 'lucide-svelte'
</script>

<Button variant="default" onclick={handleClick}>Label</Button>
<Button startIcon={{ icon: ChevronLeft }} iconOnly onclick={prev} />
```

Props: `variant?: 'accent' | 'accent-secondary' | 'default' | 'subtle'`, `unifiedSize?: '2xs' | 'xs' | 'sm' | 'md' | 'lg'`, `startIcon?: { icon: SvelteComponent }`, `iconOnly?: boolean`, `disabled?: boolean`

**`size` on `<Button>` is banned** — it, `spacingSize` and `extendedSize` are the legacy sizing
system (`xs3`/`xs2`/`xs`/…, marked `@deprecated` in `Button.svelte`). Size every button with
`unifiedSize`, the small ones included: `2xs` and `xs` are `h-5`, `sm` is `h-7`, `md` is `h-8`,
`lg` is `h-10`. Existing `size="xs2"` call sites are legacy, not a precedent to copy. Same for
`variant`: `contained`/`border`/`divider` are deprecated — use the four listed above.

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

## Feature Telemetry

New user-facing UX is the main source of `feature_usage` counters — propose them in the plan, not
as a separate question, and read `docs/feature-telemetry.md` first. `logFeatureUsage()` from
`$lib/utils/featureUsage` is only half the change: the `(feature, kind)` pair must also be
registered in the backend allowlist or every event is silently discarded, and the disclosure copy
in `InstanceSettings.svelte` must name what you added.

## Svelte MCP Server

Use the Svelte MCP tools when working on Svelte code:

1. **list-sections**: Call first to discover available docs
2. **get-documentation**: Fetch relevant sections based on use_cases
3. **svelte-autofixer**: MUST use on all Svelte code before finalizing — keep calling until no issues
4. **playground-link**: Only after user confirms and code was NOT written to project files

## Verifying in the Browser

After changing Svelte code, use the **Playwright MCP** (`mcp__playwright__*`) to drive the running frontend and confirm the change works. See frontend/AGENTS.md → "Verifying Frontend Changes" for the full flow. Use `playwright` (headless) on devboxes; `playwright-headed` when a display is available.
