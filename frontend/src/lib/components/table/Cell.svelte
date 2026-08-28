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
		/** The column holding a row's action buttons. It hugs its content at the table's
		 *  right edge instead of absorbing the width the other columns leave over. */
		actions?: boolean
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
		actions = false,
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
		// `w-0` makes the column shrink to its buttons rather than take the leftover width;
		// `text-right` then places inline content and `ml-auto` a block-level child, which a
		// button wrapper is. Pinned to the right edge so the buttons stay reachable when a
		// wide table scrolls horizontally; the background must be opaque for the cells
		// sliding under it to be occluded, and `wm-cell-pinned` below repaints the row's
		// own tint over it so it still reads as part of its row.
		// The seam is drawn only while the table overflows (DataTable measures it), so a table
		// that fits shows no stray line beside its last column.
		actions
			? 'w-0 text-right [&>*]:ml-auto sticky right-0 wm-cell-pinned [.wm-table-x-overflow_&]:border-l'
			: '',
		actions ? (head ? 'bg-surface-secondary' : 'bg-surface') : '',
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

<style>
	/* A row's hover tint is set on the `tr`, which a `position: sticky` cell paints over
	   rather than inheriting. The token carries its own alpha, so adopting it directly would
	   make the cell translucent and stop it occluding what scrolls under; layering it over
	   the opaque colour composites to the same result while keeping the cell opaque. */
	:global(tr.wm-row-hoverable:hover) > .wm-cell-pinned {
		background-image: linear-gradient(
			rgb(var(--color-surface-hover)),
			rgb(var(--color-surface-hover))
		);
	}
</style>
