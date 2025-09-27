<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { type ToggleGroupElements, type ToggleGroupItemProps, melt } from '@melt-ui/svelte'
	import { Info } from 'lucide-svelte'

	interface Props {
		label?: string | undefined
		iconOnly?: boolean
		tooltip?: string | undefined
		icon?: any | undefined
		disabled?: boolean
		selectedColor?: string | undefined
		small?: boolean
		light?: boolean
		iconProps?: Record<string, any>
		showTooltipIcon?: boolean
		documentationLink?: string | undefined
		id?: string | undefined
		item: ToggleGroupElements['item']
		value: ToggleGroupItemProps
		class?: string
	}

	let {
		label = undefined,
		iconOnly = false,
		tooltip = undefined,
		icon = undefined,
		disabled = false,
		selectedColor = undefined,
		small = false,
		light = false,
		iconProps = {},
		showTooltipIcon = false,
		documentationLink = undefined,
		id = undefined,
		item,
		value,
		class: className = ''
	}: Props = $props()
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
			'group rounded-md transition-all font-normal data-[state=on]:font-medium hover:font-medium  flex gap-1 flex-row items-center border',
			small ? 'px-1.5 py-0.5 text-2xs' : 'px-2 py-1 text-sm',
			light
				? 'hover:text-secondary data-[state=on]:text-secondary text-tertiary'
				: 'hover:text-primary data-[state=on]:text-primary text-secondary',
			'data-[state=on]:bg-surface data-[state=off]:border-transparent data-[state=on]:border-gray-300 dark:data-[state=on]:border-gray-500',
			'bg-surface-secondary hover:bg-surface-hover',
			disabled ? '!shadow-none' : '',
			className
		)}
		use:melt={$item(value)}
		style={selectedColor ? `--selected-color: ${selectedColor}` : ''}
	>
		{#if icon}
			{@const SvelteComponent = icon}
			<SvelteComponent
				size={small ? 12 : 14}
				{...iconProps}
				class={twMerge(
					selectedColor
						? 'group-data-[state=on]:text-[var(--selected-color)]'
						: 'group-data-[state=on]:text-blue-500 dark:group-data-[state=on]:text-nord-800',
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

	{#snippet text()}
		{tooltip}
	{/snippet}
</Tooltip>
