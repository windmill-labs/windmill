<script lang="ts">
	import { getContext } from 'svelte'
	import { Tab } from '@rgossiaux/svelte-headlessui'
	import type { ToggleButtonContext } from './ToggleButtonGroup.svelte'
	import { twMerge } from 'tailwind-merge'
	import Popover from '$lib/components/Popover.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'

	export let disabled: boolean = false
	export let small = false
	export let light = false
	export let id: string | undefined = undefined

	type TogglableItem = {
		label: string
		value: string
	}

	export let togglableItems: TogglableItem[]

	const { select, selected } = getContext<ToggleButtonContext>('ToggleButtonGroup')

	let items = togglableItems.map((i) => ({ displayName: i.label, action: () => select(i.value) }))

	function isAnOptionSelected(selected: string) {
		return togglableItems.some((i) => i.value === selected)
	}
</script>

<Popover
	disablePopup={true}
	notClickable
	class={twMerge('flex', disabled ? 'cursor-not-allowed' : 'cursor-pointer')}
	disappearTimeout={0}
>
	<div {id} class="flex">
		<Tab
			{disabled}
			class={twMerge(
				' rounded-md transition-all text-xs flex gap-1 flex-row items-center',
				small ? 'px-1.5 py-0.5 text-2xs' : 'px-2 py-1',
				light ? 'font-medium' : '',
				isAnOptionSelected($selected)
					? 'bg-surface shadow-md'
					: 'bg-surface-secondary hover:bg-surface-hover',
				$$props.class
			)}
		>
			<DropdownV2 {items} />
		</Tab>
	</div>
</Popover>
