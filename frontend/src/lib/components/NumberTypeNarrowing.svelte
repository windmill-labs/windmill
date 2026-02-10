<script lang="ts">
	import Label from './Label.svelte'
	import Tooltip from './Tooltip.svelte'
	import { selectOptions } from './apps/editor/component'
	import Select from './select/Select.svelte'
	import TextInput from './text_input/TextInput.svelte'

	interface Props {
		min: number | undefined
		max: number | undefined
		currency: string | undefined
		currencyLocale: string | undefined
	}

	let {
		min = $bindable(),
		max = $bindable(),
		currency = $bindable(),
		currencyLocale = $bindable()
	}: Props = $props()
</script>

<div class="grid grid-cols-2 gap-4 p-4 border rounded-md">
	<Label label="Min" class="w-full">
		{#snippet header()}
			<Tooltip light small>
				Set a minimum value for the number. If both min and max are set, the input will render as
				a range slider.
			</Tooltip>
		{/snippet}
		<TextInput
			inputProps={{ type: 'number' }}
			bind:value={() => min?.toString(), (v) => (min = v ? parseInt(v) : undefined)}
		/>
	</Label>

	<Label label="Max" class="w-full">
		{#snippet header()}
			<Tooltip light small>
				Set a maximum value for the number. If both min and max are set, the input will render as
				a range slider.
			</Tooltip>
		{/snippet}
		<TextInput
			inputProps={{ type: 'number' }}
			bind:value={() => max?.toString(), (v) => (max = v ? parseInt(v) : undefined)}
		/>
	</Label>

	<Label label="Currency" class="w-full">
		{#snippet header()}
			<div class="-my-1">
				<Tooltip light small>
					Select a currency to display the number in. If a currency is selected, you can also
					select a locale to format the number according to that locale.
				</Tooltip>
			</div>
		{/snippet}
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
</div>
