<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import { ButtonType } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { getModifierKey } from '$lib/utils'
	import {
		PanelBottomClose,
		PanelBottomOpen,
		PanelLeftClose,
		PanelLeftOpen,
		PanelRightClose,
		PanelRightOpen
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		btnClasses?: string | undefined
		size?: ButtonType.Size
		variant?: ButtonType.Variant
		color?: ButtonType.Color
		direction?: 'left' | 'right' | 'bottom'
		hidden?: boolean
		shortcut?: string | undefined
		panelName?: string | undefined
		customHiddenIcon?: ButtonType.Icon | undefined
		usePopoverOverride?: boolean
		popoverOverride?: import('svelte').Snippet
	}

	let {
		btnClasses = undefined,
		size = 'xs',
		variant = 'contained',
		color = 'light',
		direction = 'right',
		hidden = false,
		shortcut = undefined,
		panelName = undefined,
		customHiddenIcon = undefined,
		usePopoverOverride = false,
		popoverOverride
	}: Props = $props()

	const OpenIconMap = {
		left: PanelLeftOpen,
		right: PanelRightOpen,
		bottom: PanelBottomOpen
	}
	const CloseIconMap = {
		left: PanelLeftClose,
		right: PanelRightClose,
		bottom: PanelBottomClose
	}

	const shortcuts = {
		left: 'B',
		right: 'U',
		bottom: 'L',
		top: 'T'
	}
</script>

<Popover>
	{#snippet text()}
		{#if usePopoverOverride && popoverOverride}
			{@render popoverOverride?.()}
		{:else}
			<div class="flex flex-row gap-1">
				{hidden ? 'Show' : 'Hide '} the {panelName ?? direction} panel.

				<div class="flex flex-row items-center !text-md opacity-60 gap-0 font-normal">
					{getModifierKey()}{shortcut ?? shortcuts[direction]}
				</div>
			</div>
		{/if}
	{/snippet}
	<Button
		iconOnly
		startIcon={hidden
			? (customHiddenIcon ?? {
					icon: OpenIconMap[direction]
				})
			: {
					icon: CloseIconMap[direction]
				}}
		{size}
		btnClasses={twMerge(
			'p-1 text-gray-300 hover:!text-gray-600 dark:text-gray-500 dark:hover:!text-gray-200 bg-transparent',
			hidden ? 'bg-surface-selected !text-primary' : '',
			btnClasses
		)}
		on:click
		{variant}
		{color}
	/>
</Popover>
