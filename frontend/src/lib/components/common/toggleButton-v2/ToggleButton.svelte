<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { type ToggleGroupElements, type ToggleGroupItemProps, melt } from '@melt-ui/svelte'
	import { Info } from 'lucide-svelte'

	export let label: string | undefined = undefined
	export let iconOnly: boolean = false
	export let tooltip: string | undefined = undefined
	export let icon: any | undefined = undefined
	export let disabled: boolean = false
	export let selectedColor: string = '#3b82f6'
	export let small = false
	export let light = false
	export let iconProps: Record<string, any> = {}
	export let showTooltipIcon: boolean = false
	export let documentationLink: string | undefined = undefined
	export let id: string | undefined = undefined
	export let item: ToggleGroupElements['item']
	export let value: ToggleGroupItemProps
	export let onClick: () => void = () => {}
</script>

<Tooltip
	class={twMerge('flex', disabled ? 'cursor-not-allowed' : 'cursor-pointer')}
	disablePopup={tooltip === undefined}
	disappearTimeout={0}
	{documentationLink}
>
	<button
		{id}
		{disabled}
		class={twMerge(
			'group rounded-md transition-all text-xs flex gap-1 flex-row items-center',
			small ? 'px-1.5 py-0.5 text-2xs' : 'px-2 py-1',
			light ? 'font-medium' : '',
			'data-[state=on]:bg-surface data-[state=on]:shadow-md',
			'bg-surface-secondary hover:bg-surface-hover',
			$$props.class
		)}
		use:melt={$item(value)}
		style={`--selected-color: ${selectedColor}`}
		on:click={onClick}
	>
		{#if icon}
			<svelte:component
				this={icon}
				size={small ? 12 : 14}
				{...iconProps}
				class={twMerge(
					'text-gray-400',
					'group-data-[state=on]:text-[var(--selected-color)]',
					iconProps.class
				)}
			/>
		{/if}
		{#if label && !iconOnly}
			{label}
		{/if}
		{#if showTooltipIcon}
			<Info size={14} class="text-gray-400" />
		{/if}
	</button>

	<svelte:fragment slot="text">
		{tooltip}
	</svelte:fragment>
</Tooltip>
