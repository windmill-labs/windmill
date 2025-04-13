<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import { ButtonType } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { getModifierKey } from '$lib/utils'
	import {
		type Icon,
		PanelBottomClose,
		PanelBottomOpen,
		PanelLeftClose,
		PanelLeftOpen,
		PanelRightClose,
		PanelRightOpen
	} from 'lucide-svelte'
	import type { ComponentType } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let btnClasses: string | undefined = undefined
	export let size: ButtonType.Size = 'xs'

	export let variant: ButtonType.Variant = 'contained'
	export let color: ButtonType.Color = 'light'
	export let direction: 'left' | 'right' | 'bottom' = 'right'
	export let hidden: boolean = false
	export let shortcut: string | undefined = undefined
	export let panelName: string | undefined = undefined
	export let customHiddenIcon: ComponentType<Icon> | undefined = undefined
	export let usePopoverOverride: boolean = false

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
	<svelte:fragment slot="text">
		{#if usePopoverOverride && $$slots.popoverOverride}
			<slot name="popoverOverride" />
		{:else}
			<div class="flex flex-row gap-1">
				{hidden ? 'Show' : 'Hide '} the {panelName ?? direction} panel.

				<div class="flex flex-row items-center !text-md opacity-60 gap-0 font-normal">
					{getModifierKey()}{shortcut ?? shortcuts[direction]}
				</div>
			</div>
		{/if}
	</svelte:fragment>
	<Button
		iconOnly
		startIcon={{
			icon: hidden ? customHiddenIcon ?? OpenIconMap[direction] : CloseIconMap[direction]
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
