<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import { Badge, ButtonType } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { PanelBottomClose, PanelLeftClose, PanelRightClose } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	export let btnClasses: string | undefined = undefined
	export let size: ButtonType.Size = 'xs'

	export let direction: 'left' | 'right' | 'bottom' = 'right'
	export let hidden: boolean = false

	const IconMap = {
		left: PanelLeftClose,
		right: PanelRightClose,
		bottom: PanelBottomClose
	}

	const shortcuts = {
		left: 'j',
		right: 'k',
		bottom: 'l'
	}
</script>

<Popover>
	<svelte:fragment slot="text">
		<div class="flex flex-row gap-1">
			{hidden ? 'Show' : 'Hide '} the {direction} panel. Shortcut: <Badge>
				{shortcuts[direction]}
			</Badge>
		</div>
	</svelte:fragment>
	<Button
		iconOnly
		startIcon={{
			icon: IconMap[direction]
		}}
		{size}
		btnClasses={twMerge(
			'p-1 text-gray-300 hover:!text-gray-600 dark:text-gray-500 dark:hover:!text-gray-200 bg-transparent',
			hidden ? 'bg-surface-selected !text-primary' : '',
			btnClasses
		)}
		on:click
		color="light"
	/>
</Popover>
