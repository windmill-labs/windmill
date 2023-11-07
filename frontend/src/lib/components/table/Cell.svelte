<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { DatatableContext } from './DataTable.svelte'

	export let first: boolean = false
	export let last: boolean = false
	export let numeric: boolean = false
	export let head: boolean = false
	export let shouldStopPropagation: boolean = false

	export let sticky: boolean = false

	let Tag = head ? 'th' : 'td'

	const { size } = getContext<DatatableContext>('datatable')
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<svelte:element
	this={Tag}
	{...$$restProps}
	on:click={(e) => {
		if (shouldStopPropagation) e.stopPropagation()
	}}
	class={twMerge(
		'text-left text-xs text-primary font-normal whitespace-nowrap',
		first ? 'sm:pl-6' : '',
		last ? 'sm:pr-6' : '',

		first && size === 'xs' ? 'sm:pl-3' : '',
		last && size === 'xs' ? 'sm:pr-3' : '',

		numeric ? 'text-right' : '',
		head ? 'font-semibold ' : '',
		$$restProps.class,
		sticky ? `!p-0 sticky ${first ? 'left-0' : 'right-0'}` : 'px-2 py-3.5',
		size === 'sm' ? 'px-1.5 py-2.5' : '',
		size === 'lg' ? 'px-3 py-4' : '',
		size === 'xs' ? 'px-1 py-1.5' : ''
	)}
>
	{#if sticky}
		<div class={twMerge(first ? 'border-r' : ' border-l ')}>
			<slot />
		</div>
	{:else}
		<slot />
	{/if}
</svelte:element>
