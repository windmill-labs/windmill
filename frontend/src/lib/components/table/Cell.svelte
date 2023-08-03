<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	export let first: boolean = false
	export let last: boolean = false
	export let numeric: boolean = false
	export let head: boolean = false

	let Tag = head ? 'th' : 'td'
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
		$$restProps.class
	)}
>
	<slot />
</svelte:element>
