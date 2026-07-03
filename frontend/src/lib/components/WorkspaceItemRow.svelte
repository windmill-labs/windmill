<!--
@component
Visual row for a workspace item (script / flow / app / resource /
schedule / trigger / …). Matches the leaf-row layout used by
WorkspaceItemDrillPicker: RowIcon + summary line on top with mono path
beneath, or just the mono path when there's no summary. With `singleLine`,
both collapse to one row showing `summary ?? secondary` (summary in normal
text, the mono path as the fallback) — denser, for the diff tree.

Pure presentation — the caller controls highlighting / current state via
props, supplies the onclick/onmouseenter handlers, and can pass an
`extras` snippet for right-side adornments (status dots, badges, …).
The button uses `onmousedown={(e) => e.preventDefault()}` so the click
doesn't steal focus from a sibling search input (matches the picker).
-->
<script module lang="ts">
	import type { ComponentProps } from 'svelte'
	import RowIconType from '$lib/components/common/table/RowIcon.svelte'
	export type WorkspaceItemRowKind = ComponentProps<typeof RowIconType>['kind']
</script>

<script lang="ts">
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import type { Snippet } from 'svelte'

	interface Props {
		kind: WorkspaceItemRowKind
		/** For `kind: 'trigger'`, specifies the concrete trigger subtype.
		 * Forwarded to RowIcon. */
		triggerKind?: string
		/** For `kind: 'raw_app_file'`, the file name/path — forwarded to RowIcon
		 * to pick an extension-specific icon. */
		iconPath?: string
		/** Optional summary text shown above the path. */
		summary?: string
		/** Mono path (or any secondary identifier). When summary is empty
		 * this is the only visible text. */
		secondary: string
		/** Highlighted via keyboard nav. Used for `aria-selected` +
		 * surface-hover background. */
		highlighted?: boolean
		/** "Currently editing this" — the picker uses this to grey out the
		 * active row and disable its click. */
		current?: boolean
		/** DOM id, used for `aria-activedescendant`. */
		id?: string
		/** Stamped on the element as `data-nav-key` so the parent can
		 * `pickerRoot.querySelector(...)` to scroll into view. */
		navKey?: string
		/** Per-row vertical padding class (e.g. `py-1` / `py-1.5`). */
		baseClass?: string
		/** Reserve two lines of height and vertically center the content so
		 * summary and summary-less rows are the same height (diff viewer). */
		uniformHeight?: boolean
		/** Collapse to a single line showing `summary ?? secondary` (summary in
		 * normal text, secondary in mono) instead of stacking both. */
		singleLine?: boolean
		/** Extra left padding (px) for tree-view indentation. Adds to the
		 * default `px-3` horizontal padding. */
		indent?: number
		/** Full path tooltip for the secondary line; defaults to the secondary
		 * text. The summary line gets its own tooltip (the full summary) so each
		 * truncated line reveals its own content on hover. */
		title?: string
		/** When set, the row renders as an `<a href target="_blank">` link
		 * instead of a `<button>`. Used by callers that want native
		 * new-tab / cmd-click behaviour. `onclick` still forwards. */
		href?: string
		onclick?: () => void
		onmouseenter?: () => void
		/** Right-side adornments (status dot, badges, …). The `group` class
		 * is always applied to the root so the snippet can use
		 * `group-hover:*` utilities to reveal hover-only affordances. */
		extras?: Snippet
	}

	let {
		kind,
		triggerKind,
		iconPath,
		summary,
		secondary,
		highlighted = false,
		current = false,
		id,
		navKey,
		baseClass = 'py-1.5',
		indent = 0,
		title,
		href,
		onclick,
		onmouseenter,
		extras,
		uniformHeight = false,
		singleLine = false
	}: Props = $props()

	const rootClass = $derived(
		`group w-full text-left flex items-center gap-2 px-3 transition-colors ${baseClass} ${highlighted ? 'bg-surface-hover' : ''} ${current ? 'cursor-default text-emphasis font-medium' : ''}`
	)

	// Same min-height + centering for both branches so a row with a summary
	// (two lines) and one without (one line) end up identical in height.
	const contentClass = $derived(
		`min-w-0 flex-1${uniformHeight ? ' flex flex-col justify-center min-h-[2.25rem]' : ''}`
	)
</script>

{#snippet body()}
	<RowIcon {kind} {triggerKind} path={iconPath} size={12} />
	<div class={contentClass}>
		{#if singleLine}
			<!-- `||` not `??`: an empty-string summary (some draft rows carry `''`)
			     must fall back to the path, otherwise the row renders blank. -->
			<div
				class="text-xs font-normal text-primary truncate {summary ? '' : 'font-mono'}"
				title={summary || title || secondary}
			>
				{summary || secondary}
			</div>
		{:else if summary}
			<div class="text-xs text-primary truncate" title={summary}>{summary}</div>
			<div
				class="text-2xs text-secondary font-normal font-mono truncate"
				title={title ?? secondary}
			>
				{secondary}
			</div>
		{:else}
			<div class="text-xs text-primary font-mono truncate" title={title ?? secondary}>
				{secondary}
			</div>
		{/if}
	</div>
	{#if extras}
		<div class="shrink-0 flex items-center gap-2">
			{@render extras()}
		</div>
	{/if}
{/snippet}

{#if href}
	<a
		{href}
		target="_blank"
		rel="noopener noreferrer"
		{id}
		role="option"
		aria-selected={highlighted}
		aria-current={current ? 'true' : undefined}
		data-nav-key={navKey}
		style={indent ? `padding-left: calc(0.75rem + ${indent}px)` : undefined}
		class={rootClass}
		{onclick}
		{onmouseenter}
	>
		{@render body()}
	</a>
{:else}
	<button
		type="button"
		{id}
		role="option"
		aria-selected={highlighted}
		aria-current={current ? 'true' : undefined}
		data-nav-key={navKey}
		style={indent ? `padding-left: calc(0.75rem + ${indent}px)` : undefined}
		class={rootClass}
		onmousedown={(e) => e.preventDefault()}
		{onclick}
		{onmouseenter}
	>
		{@render body()}
	</button>
{/if}
