<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { DatatableContext } from './DataTable.svelte'

	export let first: boolean = false
	export let last: boolean = false
	export let numeric: boolean = false
	export let head: boolean = false

	let Tag = head ? 'th' : 'td'

	const { size } = getContext<DatatableContext>('datatable')
</script>

<svelte:element
	this={Tag}
	{...$$restProps}
	class={twMerge(
		'px-2 py-3.5 text-left text-xs text-primary font-normal whitespace-nowrap',
		first ? 'sm:pl-6' : '',
		last ? 'sm:pr-6' : '',
		numeric ? 'text-right' : '',
		head ? 'font-semibold ' : '',
		$$restProps.class,
		size === 'sm' ? 'px-1.5 py-2.5' : '',
		size === 'lg' ? 'px-3 py-4' : ''
	)}
>
	<slot />
</svelte:element>
