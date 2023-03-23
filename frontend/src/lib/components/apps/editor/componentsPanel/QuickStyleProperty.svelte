<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import { Button, ClearableInput } from '../../../common'
	import Popover from '../../../Popover.svelte'
	import ColorInput from '../settingsPanel/inputEditor/ColorInput.svelte'
	import {
		StylePropertyType,
		StylePropertyUnits,
		STYLE_STORE_KEY,
		type StyleStore,
		type StyleStoreValue
	} from './quickStyleProperties'

	export let prop: StyleStoreValue['style'][number]['prop']
	export let value: string | undefined
	export let inline = false
	const styleStore = getContext<StyleStore>(STYLE_STORE_KEY)
	const dispatch = createEventDispatcher()
	const key = prop.key
	const type = prop.value?.['type']
	let unit: (typeof StylePropertyUnits)[number] = StylePropertyUnits[0]
	let internalValue: number | string

	$: internalValue = value ? +value.replace(unit, '') : ''
	$: dispatch('change', value)

	function updateValue(next: number) {
		value = next ? next + unit : ''
	}

	function updateUnit(next: (typeof StylePropertyUnits)[number]) {
		value = value?.replace(unit, next) || ''
		unit = next
	}
</script>

<div class={inline && type !== StylePropertyType.color ? '' : 'w-full'}>
	{#if prop.value['title']}
		<div class="font-medium text-xs text-gray-500">
			{prop.value['title']}
		</div>
	{/if}
	<div class="flex gap-1">
		{#if type === StylePropertyType.color}
			<ColorInput bind:value />
			{#each $styleStore.topColors as color}
				<Popover placement="bottom" notClickable disapperTimoout={0} class="flex">
					<Button
						color="light"
						size="xs"
						variant="border"
						btnClasses="!p-0 !w-[34px] !h-[34px]"
						aria-label="Set {key} to {color}"
						style={`background-color: ${color};`}
						on:click={() => (value = color)}
					/>
					<svelte:fragment slot="text">{color}</svelte:fragment>
				</Popover>
			{/each}
		{:else if type === StylePropertyType.number}
			<ClearableInput type="number" bind:value />
		{:else if type === StylePropertyType.unit}
			<ClearableInput
				wrapperClass="flex items-center gap-1 {inline ? '!grow-0' : ''}"
				inputClass={inline ? '!w-20' : '!w-[calc(100%-64px)]'}
				buttonClass="!right-[68px]"
				type="number"
				value={internalValue}
				on:change={({ detail }) => updateValue(detail)}
			>
				<select
					on:change={({ currentTarget }) => updateUnit(currentTarget.value)}
					class="!w-[60px]"
				>
					{#each StylePropertyUnits as unit}
						<option value={unit}>{unit}</option>
					{/each}
				</select>
			</ClearableInput>
		{:else if type === StylePropertyType.text}
			{#each prop.value?.['options'] || [] as option}
				<Popover placement="bottom" notClickable disapperTimoout={0}>
					<Button
						color={value === option.text ? 'dark' : 'light'}
						size="xs"
						variant={value === option.text ? 'contained' : 'border'}
						btnClasses="!p-1 !min-w-[34px] !h-[34px]"
						aria-label="Set {key} to {option.text}"
						on:click={() => {
							if (value === option.text) {
								value = ''
							} else {
								value = option.text
							}
						}}
					>
						{#if typeof option.icon === 'string'}
							{option.icon}
						{:else}
							<svelte:component this={option.icon} size={18} />
						{/if}
					</Button>
					<svelte:fragment slot="text">{option.text}</svelte:fragment>
				</Popover>
			{:else}
				<ClearableInput inputClass="min-w-[32px]" bind:value />
			{/each}
		{/if}
	</div>
</div>
