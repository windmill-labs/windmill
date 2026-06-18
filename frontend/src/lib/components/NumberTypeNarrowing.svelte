<script lang="ts">
	import Label from './Label.svelte'
	import Tooltip from './Tooltip.svelte'
	import { selectOptions } from './apps/editor/component'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Select from './select/Select.svelte'
	import TextInput from './text_input/TextInput.svelte'

	interface Props {
		min: number | undefined
		max: number | undefined
		currency: string | undefined
		currencyLocale: string | undefined
		seconds: boolean | undefined
	}

	let {
		min = $bindable(),
		max = $bindable(),
		currency = $bindable(),
		currencyLocale = $bindable(),
		seconds = $bindable()
	}: Props = $props()

	let mode: string | undefined = $state(seconds ? 'seconds' : currency ? 'currency' : undefined)

	function onModeChange(newMode: string | undefined) {
		mode = newMode
		if (newMode === 'seconds') {
			seconds = true
			currency = undefined
			currencyLocale = undefined
		} else if (newMode === 'currency') {
			seconds = undefined
		} else {
			seconds = undefined
			currency = undefined
			currencyLocale = undefined
		}
	}
</script>

<div class="grid grid-cols-2 gap-4 p-4 border rounded-md">
	<Label label="Min" class="w-full">
		{#snippet header()}
			<Tooltip light small>
				Set a minimum value for the number. If both min and max are set, the input will render as a
				range slider.
			</Tooltip>
		{/snippet}
		<TextInput
			inputProps={{ type: 'number' }}
			bind:value={() => min?.toString(), (v) => (min = v !== '' && v != null ? parseInt(v) : undefined)}
		/>
	</Label>

	<Label label="Max" class="w-full">
		{#snippet header()}
			<Tooltip light small>
				Set a maximum value for the number. If both min and max are set, the input will render as a
				range slider.
			</Tooltip>
		{/snippet}
		<TextInput
			inputProps={{ type: 'number' }}
			bind:value={() => max?.toString(), (v) => (max = v !== '' && v != null ? parseInt(v) : undefined)}
		/>
	</Label>

	<Label label="Format" class="w-full col-span-2">
		{#snippet header()}
			<Tooltip light small>Display the number as a currency or as a duration in seconds.</Tooltip>
		{/snippet}
		<ToggleButtonGroup
			selected={mode ?? 'none'}
			onSelected={(v) => onModeChange(v === 'none' ? undefined : v)}
		>
			{#snippet children({ item })}
				<ToggleButton value="none" label="None" {item} size="sm" />
				<ToggleButton value="currency" label="Currency" {item} size="sm" />
				<ToggleButton value="seconds" label="Seconds" {item} size="sm" />
			{/snippet}
		</ToggleButtonGroup>
	</Label>

	{#if mode === 'currency'}
		<Label label="Currency" class="w-full">
			<Select
				bind:value={
					() => currency,
					(v) => {
						currency = v
						if (!v) currencyLocale = undefined
					}
				}
				items={selectOptions.currencyOptions.map((c) => ({ label: c, value: c }))}
				placeholder="No currency"
				clearable
			/>
		</Label>

		<Label label="Currency locale" class="w-full">
			<Select
				bind:value={currencyLocale}
				items={selectOptions.localeOptions.map((c) => ({ label: c, value: c }))}
				placeholder="No locale"
				disabled={!currency}
				clearable
			/>
		</Label>
	{/if}
</div>
