<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Popover from '$lib/components/Popover.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	type TogglableItem = {
		label: string
		value: string
		tooltip?: string
	}

	interface Props {
		disabled?: boolean
		small?: boolean
		light?: boolean
		id?: string | undefined
		item?: any | undefined
		selected?: string | undefined
		togglableItems: TogglableItem[]
		onSelected?: (v: string) => void
	}

	let {
		disabled = false,
		small = false,
		light = false,
		id = undefined,
		item = undefined,
		selected = $bindable(undefined),
		togglableItems,
		onSelected
	}: Props = $props()

	function select(v: string) {
		if (v !== selected) {
			onSelected?.(v)
		}
		selected = v
	}

	let items = togglableItems.map((i) => ({
		displayName: i.label,
		action: () => select(i.value),
		tooltip: i.tooltip
	}))

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
			{@const tooltip = togglableItems.find((i) => i.value === selected)?.tooltip}
			<ToggleButton
				{disabled}
				value={selected ?? ''}
				{item}
				{small}
				{light}
				{id}
				label={togglableItems.find((i) => i.value === selected)?.label}
				{tooltip}
				showTooltipIcon={!!tooltip}
			/>
		{/if}
		<div class="flex items-center">
			<DropdownV2 {items} />
		</div>
	</div>
</Popover>
