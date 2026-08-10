<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { Snippet } from 'svelte'

	/**
	 * One selectable row in a version-history list. Owns only the card chrome, so the several
	 * history drawers (script, resource, …) stay visually identical while each supplies its own
	 * row content.
	 *
	 * `action` is a sibling of the select button rather than part of `children` so a row can
	 * carry its own control (a link, a menu) without nesting one button inside another.
	 */
	let {
		selected = false,
		onclick,
		children,
		action
	}: {
		selected?: boolean
		onclick?: () => void
		children: Snippet
		action?: Snippet
	} = $props()
</script>

<div
	class={classNames(
		'border flex gap-1 truncate justify-between flex-row w-full items-center rounded-md',
		selected ? 'bg-surface-selected' : '',
		'hover:bg-surface-hover focus-within:border-border-selected'
	)}
>
	<button type="button" class="flex-1 min-w-0 truncate text-left p-2 cursor-pointer" {onclick}>
		{@render children()}
	</button>
	{@render action?.()}
</div>
