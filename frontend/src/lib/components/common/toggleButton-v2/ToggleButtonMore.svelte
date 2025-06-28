<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Popover from '$lib/components/Popover.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	type TogglableItem = {
		label: string
		value: string
	}

	interface Props {
		disabled?: boolean
		small?: boolean
		light?: boolean
		id?: string | undefined
		item?: any | undefined
		selected?: string | undefined
		togglableItems: TogglableItem[]
	}

	let {
		disabled = false,
		small = false,
		light = false,
		id = undefined,
		item = undefined,
		selected = $bindable(undefined),
		togglableItems
	}: Props = $props()

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
