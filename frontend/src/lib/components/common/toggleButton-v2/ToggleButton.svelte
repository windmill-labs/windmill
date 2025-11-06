<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { type ToggleGroupElements, type ToggleGroupItemProps, melt } from '@melt-ui/svelte'
	import { Info } from 'lucide-svelte'
	import { ButtonType } from '$lib/components/common/button/model'

	interface Props {
		label?: string | undefined
		iconOnly?: boolean
		tooltip?: string | undefined
		icon?: any | undefined
		disabled?: boolean
		selectedColor?: string | undefined
		small?: boolean
		size?: 'sm' | 'md' | 'lg'
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
		size = undefined,
		iconProps = {},
		showTooltipIcon = false,
		documentationLink = undefined,
		id = undefined,
		item,
		value,
		class: className = ''
	}: Props = $props()

	// Handle backward compatibility: small prop maps to size="sm"
	const actualSize = $derived(size ?? (small ? 'sm' : 'md'))

	// Direct access to unified sizing
	const horizontalPadding = $derived(
		iconOnly
			? ButtonType.UnifiedIconOnlySizingClasses[actualSize]
			: ButtonType.UnifiedSizingClasses[actualSize]
	)
	const height = $derived(ButtonType.UnifiedHeightClasses[actualSize])
	const iconSize = $derived(ButtonType.UnifiedIconSizes[actualSize])
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
			'group rounded-md transition-all font-normal flex gap-1 flex-row items-center justify-center border text-xs',
			horizontalPadding,
			height,
			'text-primary data-[state=on]:text-primary',
			'data-[state=on]:bg-surface-tertiary data-[state=off]:border-transparent data-[state=on]:border-border-normal/30',
			'bg-surface-transparent hover:bg-surface-hover',
			disabled ? '!shadow-none' : '',
			className
		)}
		use:melt={$item(value)}
		style={selectedColor ? `--selected-color: ${selectedColor}` : ''}
	>
		{#if icon}
			{@const SvelteComponent = icon}
			<SvelteComponent
				size={iconSize}
				{...iconProps}
				class={twMerge(
					'text-primary',
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
			<Info size={iconSize} class="text-gray-400" />
		{/if}
	</button>

	{#snippet text()}
		{tooltip}
	{/snippet}
</Tooltip>
