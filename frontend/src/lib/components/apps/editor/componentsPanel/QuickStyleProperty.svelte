<script lang="ts">
	import { getContext } from 'svelte'
	import { Button, ClearableInput, Menu } from '../../../common'
	import Popover from '../../../Popover.svelte'
	import ColorInput from '../settingsPanel/inputEditor/ColorInput.svelte'
	import {
		StylePropertyType,
		StylePropertyUnits,
		STYLE_STORE_KEY,
		type StyleStore,
		type StyleStoreValue,
		type StylePropertyValue
	} from './quickStyleProperties'

	interface Props {
		prop: StyleStoreValue['style'][number]['prop']
		value: string | undefined
		inline?: boolean
		onChange?: (value: string) => void
	}

	let { prop, value = $bindable(), inline = false, onChange }: Props = $props()
	const styleStore = getContext<StyleStore>(STYLE_STORE_KEY)
	const key = prop.key
	const type = prop.value?.['type']
	let unit: (typeof StylePropertyUnits)[number] = $state(StylePropertyUnits[0])
	let internalValue: number | string = $derived(
		getInteralValue(value, prop.value as StylePropertyValue)
	)

	function getInteralValue(value: string | undefined, propValue: StylePropertyValue) {
		if (!value) {
			return ''
		}
		if (propValue.type === StylePropertyType.number) {
			return value
		}
		if (propValue.type === StylePropertyType.text) {
			return +value.replace(unit, '')
		}
		return ''
	}

	function updateValue(next: number) {
		value = next ? next + unit : ''
		onChange?.(value ?? '')
	}

	function updateUnit(next: (typeof StylePropertyUnits)[number]) {
		value = value?.replace(unit, next) || ''
		unit = next
		onChange?.(value ?? '')
	}
</script>

<div class={inline && type !== StylePropertyType.color ? '' : 'w-full'}>
	{#if prop.value['title']}
		<div class="font-medium text-xs text-tertiary">
			{prop.value['title']}
		</div>
	{/if}
	<div class="flex gap-1">
		{#if type === StylePropertyType.color}
			<ColorInput bind:value />
			{#each $styleStore.topColors as color}
				<Popover placement="bottom" notClickable disappearTimeout={0} class="flex">
					<Button
						color="light"
						size="xs"
						variant="border"
						btnClasses="!p-0 !w-[34px] !h-[34px]"
						aria-label="Set {key} to {color}"
						style={`background-color: ${color};`}
						on:click={() => (value = color)}
					/>
					{#snippet text()}
						{color}
					{/snippet}
				</Popover>
			{/each}
		{:else if type === StylePropertyType.number}
			<ClearableInput
				type="number"
				inputClass={inline ? '!w-[90px]' : ''}
				bind:value
				step={prop.value?.['step'] || 1}
				min={prop.value?.['min']}
				max={prop.value?.['max']}
			/>
		{:else if type === StylePropertyType.unit}
			<ClearableInput
				wrapperClass="flex items-center {inline ? '!grow-0' : ''}"
				inputClass="!border-r-0 !rounded-r-none {inline ? '!w-[90px]' : ''}"
				buttonClass="!right-9"
				type="number"
				value={internalValue}
				on:change={({ detail }) => updateValue(detail)}
			>
				<Menu
					noMinW
					wrapperClasses="h-full bg-surface rounded-r-md border-y border-r pr-0.5"
					popupClasses="!mt-0"
				>
					{#snippet trigger()}
						<button
							type="button"
							class="font-normal text-xs px-1 py-1.5 w-8 rounded mt-0.5 duration-200 hover:bg-gray-200/90"
						>
							{unit}
						</button>
					{/snippet}
					{#snippet children({ close })}
						<ul class="bg-surface rounded border py-1 overflow-auto">
							{#each StylePropertyUnits as u}
								<li class="w-full">
									<Button
										type="button"
										color="light"
										size="xs"
										variant="contained"
										btnClasses="!justify-start !rounded-none !w-full !px-3 !py-1.5"
										on:click={() => {
											updateUnit(u)
											close()
										}}
									>
										{u}
									</Button>
								</li>
							{/each}
						</ul>
					{/snippet}
				</Menu>
			</ClearableInput>
		{:else if type === StylePropertyType.text}
			{#each prop.value?.['options'] || [] as option}
				<Popover placement="bottom" notClickable disappearTimeout={0}>
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
							<option.icon size={18} />
						{/if}
					</Button>
					{#snippet text()}
						{option.text}
					{/snippet}
				</Popover>
			{:else}
				<ClearableInput inputClass="min-w-[32px]" bind:value />
			{/each}
		{/if}
	</div>
</div>
