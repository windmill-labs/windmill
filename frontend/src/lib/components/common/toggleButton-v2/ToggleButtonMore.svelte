<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Popover from '$lib/components/Popover.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	export let disabled: boolean = false
	export let small = false
	export let light = false
	export let id: string | undefined = undefined
	export let item: any | undefined = undefined
	export let selected: string | undefined = undefined
	type TogglableItem = {
		label: string
		value: string
	}

	export let togglableItems: TogglableItem[]

	function select(v: string) {
		selected = v
	}

	let items = togglableItems.map((i) => ({ displayName: i.label, action: () => select(i.value) }))

	function isAnOptionSelected(selected: string | undefined) {
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
		{#if isAnOptionSelected(selected)}
			<ToggleButton
				{disabled}
				value={selected ?? ''}
				{item}
				{small}
				{light}
				{id}
				label={togglableItems.find((i) => i.value === selected)?.label}
			/>
		{/if}
		<div class="flex items-center">
			<DropdownV2 {items} />
		</div>
	</div>
</Popover>
