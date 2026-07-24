<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { DatatableContext } from './DataTable.svelte'

	interface Props {
		first?: boolean
		last?: boolean
		numeric?: boolean
		head?: boolean
		shouldStopPropagation?: boolean
		selected?: boolean
		sticky?: boolean
		stickyEnd?: boolean
		wrap?: boolean
		children?: import('svelte').Snippet
		[key: string]: any
	}

	let {
		first = false,
		last = false,
		numeric = false,
		head = false,
		shouldStopPropagation = false,
		selected = false,
		sticky = false,
		stickyEnd = false,
		wrap = false,
		children,
		...rest
	}: Props = $props()

	let Tag = untrack(() => head) ? 'th' : 'td'

	const { size } = getContext<DatatableContext>('datatable')
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<svelte:element
	this={Tag}
	{...rest}
	onclick={(e) => {
		if (shouldStopPropagation) e.stopPropagation()
	}}
	class={twMerge(
		'text-left font-normal',
		// Typography based on brand guidelines
		head ? 'text-2xs text-primary' : 'text-xs text-primary',
		wrap ? 'break-words' : 'whitespace-nowrap',
		first ? 'sm:pl-6' : '',
		last ? 'sm:pr-6' : '',

		first && size === 'xs' ? 'sm:pl-3' : '',
		last && size === 'xs' ? 'sm:pr-3' : '',

		numeric ? 'text-right' : '',
		// Pin an actions column to the right so it stays visible when a wide table
		// scrolls horizontally. The background must be opaque so cells sliding under it
		// are occluded — the row's hover tint is translucent and would bleed through.
		stickyEnd ? 'sticky right-0 border-l' : '',
		stickyEnd ? (head ? 'bg-surface-secondary' : 'bg-surface') : '',
		sticky ? `!p-0 sticky ${first ? 'left-0' : 'right-0'}` : 'px-2 py-2',
		size === 'sm' ? 'px-1.5 py-2.5' : '',
		size === 'lg' ? 'px-3 py-4' : '',
		size === 'xs' ? 'px-1 py-1.5' : '',
		selected ? 'bg-blue-50 dark:bg-blue-900/50' : '',
		'transition-all',
		rest.class
	)}
>
	{#if sticky}
		<div class={twMerge(first ? 'border-r' : ' border-l ')}>
			{@render children?.()}
		</div>
	{:else}
		{@render children?.()}
	{/if}
</svelte:element>
