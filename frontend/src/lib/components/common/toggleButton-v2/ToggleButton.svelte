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
	export let selectedColor: string | undefined = undefined
	export let small = false
	export let light = false
	export let iconProps: Record<string, any> = {}
	export let showTooltipIcon: boolean = false
	export let documentationLink: string | undefined = undefined
	export let id: string | undefined = undefined
	export let item: ToggleGroupElements['item']
	export let value: ToggleGroupItemProps
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
			'group rounded-md transition-all text-xs font-normal flex gap-1 flex-row items-center border',
			small ? 'px-1.5 py-0.5 text-2xs' : 'px-2 py-1',
			light ? 'font-medium' : '',
			'data-[state=on]:bg-surface data-[state=off]:border-transparent data-[state=on]:border-gray-300 dark:data-[state=on]:border-gray-500',
			'text-hint hover:text-secondary data-[state=on]:text-tertiary',
			'bg-surface-secondary hover:bg-surface-hover',
			disabled ? '!shadow-none' : '',
			$$props.class
		)}
		use:melt={$item(value)}
		style={selectedColor ? `--selected-color: ${selectedColor}` : ''}
	>
		{#if icon}
			<svelte:component
				this={icon}
				size={small ? 12 : 14}
				{...iconProps}
				class={twMerge(
					'text-gray-400',
					selectedColor
						? 'group-data-[state=on]:text-[var(--selected-color)]'
						: 'group-data-[state=on]:text-nord-950 dark:group-data-[state=on]:text-nord-900',
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
