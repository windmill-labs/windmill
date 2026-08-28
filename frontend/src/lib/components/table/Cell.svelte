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
		actions ? 'w-0 text-right [&>*]:ml-auto sticky right-0 wm-cell-pinned' : '',
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
	   make the cell translucent and stop it occluding what scrolls under: the tint goes on a
	   layer over the opaque colour instead, which composites to the same result.
	   That layer is a pseudo-element rather than a `background-image`, because the row fades
	   its tint in and `background-image` is not animatable — a gradient toggled on and off
	   would snap while the rest of the row faded. `opacity` animates, on the row's own
	   150ms curve, so the two stay in step. */
	.wm-cell-pinned {
		isolation: isolate;
	}

	.wm-cell-pinned::after {
		content: '';
		position: absolute;
		inset: 0;
		/* Above the cell's own background, below its buttons. */
		z-index: -1;
		pointer-events: none;
		background-color: rgb(var(--color-surface-hover));
		opacity: 0;
		transition: opacity 150ms cubic-bezier(0.4, 0, 0.2, 1);
	}

	:global(tr.wm-row-hoverable:hover) > .wm-cell-pinned::after {
		opacity: 1;
	}

	/* The seam marks that content is passing under the pinned column, so it is drawn only
	   while the table overflows — DataTable measures that. It has to be a shadow rather than
	   a border: under `border-collapse: collapse` a cell's borders belong to the table and
	   scroll away with it, while a shadow is painted by the cell and stays on its edge. */
	:global(.wm-table-x-overflow) .wm-cell-pinned {
		box-shadow: -1px 0 0 0 rgb(var(--color-border-light));
	}
</style>
