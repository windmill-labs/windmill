<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import { ButtonType } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { EyeOff } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	export let btnClasses: string | undefined = undefined
	export let size: ButtonType.Size = 'xs'
</script>

<Popover>
	<svelte:fragment slot="text">
		<div class="flex flex-row gap-1"> Hide this panel </div>
	</svelte:fragment>
	<Button
		iconOnly
		startIcon={{
			icon: EyeOff
		}}
		{size}
		btnClasses={twMerge(
			'p-1 text-gray-300 hover:!text-gray-600 dark:text-gray-500 dark:hover:!text-gray-200 bg-transparent',
			btnClasses
		)}
		on:click
		color="light"
	/>
</Popover>
