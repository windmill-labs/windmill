<script lang="ts">
	import Label from './Label.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import { selectOptions } from './apps/editor/component'

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

	let minChecked: boolean = $state(min != undefined)
	let maxChecked: boolean = $state(max != undefined)
</script>

<div class="flex flex-col gap-2">
	<div class="grid grid-cols-2 gap-4">
		<Label label="Min" class="w-full col-span-1">
			{#snippet header()}
				<Tooltip light small>
					Set a minimum value for the number. If both min and max are set, the input will render as
					a range slider.
				</Tooltip>
			{/snippet}
			{#snippet action()}
				<Toggle
					bind:checked={minChecked}
					on:change={(e) => {
						if (e.detail) {
							min = 0
						} else {
							min = undefined
						}
					}}
					options={{ right: 'Enabled' }}
					size="xs"
				/>
			{/snippet} <input type="number" bind:value={min} disabled={!minChecked} />
		</Label>

		<Label label="Max" class="w-full col-span-1 ">
			{#snippet header()}
				<Tooltip light small>
					Set a maximum value for the number. If both min and max are set, the input will render as
					a range slider.
				</Tooltip>
			{/snippet}
			{#snippet action()}
				<Toggle
					bind:checked={maxChecked}
					on:change={(e) => {
						if (e.detail) {
							max = 42
						} else {
							max = undefined
						}
					}}
					options={{ right: 'Enabled' }}
					size="xs"
				/>
			{/snippet}
			<input type="number" bind:value={max} disabled={!maxChecked} />
		</Label>
	</div>
	<div class="grid grid-cols-3 gap-4">
		<Label label="Currency" class="w-full col-span-2">
			{#snippet header()}
				<Tooltip light small>
					Select a currency to display the number in. If a currency is selected, you can also select
					a locale to format the number according to that locale.
				</Tooltip>
			{/snippet}
			<select bind:value={currency}>
				<option value={undefined}> No currency </option>
				{#each selectOptions.currencyOptions as c}
					<option value={c}>{c}</option>
				{/each}
			</select>
		</Label>
		<Label label="Currency locale" class="w-full">
			<select bind:value={currencyLocale} disabled={!currency}>
				<option value={undefined}> No locale </option>
				{#each selectOptions.localeOptions as c}
					<option value={c}>{c}</option>
				{/each}
			</select>
		</Label>
	</div>
</div>
