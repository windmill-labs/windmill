<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'

	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'
	import { twMerge } from 'tailwind-merge'

	let {
		items,
		value = $bindable(),
		title,
		tooltip
	}: {
		items: string[]
		value: string[] | undefined
		title: string
		tooltip: string
	} = $props()

	let width = $state(0)
	const inputWidth = 280
</script>

<div
	class={twMerge(
		'flex flex-col justify-between w-full',
		width < inputWidth ? 'flex-col gap-2' : 'flex-row gap-16 '
	)}
	bind:clientWidth={width}
>
	<div class="flex items-center text-xs h-8 whitespace-nowrap">
		{title}
		<Tooltip light small>{tooltip}</Tooltip>
	</div>

	<div class="w-full">
		{#if items.length === 0}
			<div class="text-xs text-secondary w-full flex items-center justify-end h-8">
				No components to recompute.
			</div>
		{:else}
			<MultiSelect
				items={safeSelectItems(items)}
				bind:value={() => value ?? [], (v) => (value = v?.length ? v : undefined)}
			/>
		{/if}
	</div>
</div>
