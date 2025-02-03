<script lang="ts">
	import { Tab } from '$lib/components/common'
	import { twMerge } from 'tailwind-merge'
	export let value: string
	export let otherValues: string[] = []
	export let icon: any
	export let label: string
	export let selected: boolean = false

	let spanWidth: number
</script>

<Tab
	{value}
	{otherValues}
	class={twMerge('grow basis-0 min-w-0', selected && 'min-w-fit shrink-0')}
>
	<span
		class="flex flex-row gap-2 items-center justify-center text-xs w-full min-w-0"
		bind:clientWidth={spanWidth}
	>
		<svelte:component this={icon} size={12} class="shrink-0" />
		{#if spanWidth > 54 || selected}
			<span class={selected ? 'min-w-fit' : 'min-w-0 truncate'} title={label}>
				{label}
			</span>
		{/if}
	</span>
</Tab>
