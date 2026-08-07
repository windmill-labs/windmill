<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { Snippet } from 'svelte'

	/**
	 * One selectable row in a version-history list. Owns only the card chrome, so the several
	 * history drawers (script, resource, …) stay visually identical while each supplies its own
	 * row content.
	 */
	let {
		selected = false,
		onclick,
		children
	}: {
		selected?: boolean
		onclick?: () => void
		children: Snippet
	} = $props()
</script>

<button
	type="button"
	class={classNames(
		'border flex gap-1 justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer text-left',
		selected ? 'bg-surface-selected' : '',
		'hover:bg-surface-hover'
	)}
	{onclick}
>
	{@render children()}
</button>
