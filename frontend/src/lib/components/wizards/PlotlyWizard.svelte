<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Popup } from '../common'
	import Label from '../Label.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import ColorInput from '../apps/editor/settingsPanel/inputEditor/ColorInput.svelte'
	import Button from '../common/button/Button.svelte'
	import InputsSpecEditor from '../apps/editor/settingsPanel/InputsSpecEditor.svelte'
	import type { RichConfiguration } from '../apps/types'

	type Dataset = {
		value: RichConfiguration
		name: string
		aggregation_method: string
		tooltip: string
		color: string
		type: 'bar' | 'line' | 'scatter' | 'pie'
	}

	export let value: Dataset | undefined = undefined
	export let id: string

	const dispatch = createEventDispatcher()

	function removeDataset() {
		dispatch('remove')
	}
</script>

<Popup
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
	containerClasses="border rounded-lg shadow-lg bg-surface p-4"
>
	<svelte:fragment slot="button">
		<slot name="trigger" />
	</svelte:fragment>
	{#if value}
		<div class="flex flex-col w-96 p-2 gap-4">
			<Label label="Name">
				<input type="text" bind:value={value.name} />
			</Label>

			<Label label="Type">
				<select bind:value={value.type}>
					<option value="bar">Bar</option>
					<option value="line">Line</option>
					<option value="scatter">Scatter</option>
					<option value="pie">Pie</option>
				</select>
			</Label>

			<Label label="Aggregation method">
				<select bind:value={value.aggregation_method}>
					<option value="sum">Sum</option>
					<option value="mean">Mean</option>
					<option value="median">Median</option>
					<option value="min">Min</option>
					<option value="max">Max</option>
				</select>
			</Label>

			<Label label="Tooltip">
				<input type="text" bind:value={value.tooltip} />
			</Label>

			<Label label="Color">
				<ColorInput bind:value={value.color} />
			</Label>

			<InputsSpecEditor
				key={'Data'}
				bind:componentInput={value.value}
				{id}
				userInputEnabled={false}
				shouldCapitalize={true}
				resourceOnly={false}
				fieldType={value.value?.['fieldType']}
				subFieldType={value.value?.['subFieldType']}
				format={value.value?.['format']}
				selectOptions={value.value?.['selectOptions']}
				tooltip={value.value?.['tooltip']}
				fileUpload={value.value?.['fileUpload']}
				placeholder={value.value?.['placeholder']}
				customTitle={value.value?.['customTitle']}
				displayType={false}
			/>

			<Button color="red" size="xs" on:click={removeDataset}>Remove dataset</Button>
		</div>
	{/if}
</Popup>
