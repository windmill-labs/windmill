<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Label from '../Label.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import ColorInput from '../apps/editor/settingsPanel/inputEditor/ColorInput.svelte'
	import Button from '../common/button/Button.svelte'
	import InputsSpecEditor from '../apps/editor/settingsPanel/InputsSpecEditor.svelte'
	import type { AppViewerContext, RichConfiguration } from '../apps/types'
	import Tooltip from '../Tooltip.svelte'

	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	let closeOnOutsideClick = true

	type Dataset = {
		value: RichConfiguration
		name: string
		aggregation_method: string
		tooltip: string
		color: string
		type: 'bar' | 'scatter'
		extraOptions?: { mode: 'markers' | 'lines' | 'lines+markers' } | undefined
	}

	export let value: Dataset | undefined = undefined

	const dispatch = createEventDispatcher()

	function removeDataset() {
		dispatch('remove')
	}
</script>

<Popover
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
	{closeOnOutsideClick}
>
	<svelte:fragment slot="trigger">
		<slot name="trigger" />
	</svelte:fragment>
	<svelte:fragment slot="content">
		{#if value}
			<div class="flex flex-col w-96 p-4 gap-4 max-h-[70vh] overflow-y-auto">
				<Label label="Name">
					<input type="text" bind:value={value.name} />
				</Label>

				<Label label="Type">
					<select bind:value={value.type}>
						<option value="bar">Bar</option>
						<option
							value="scatter"
							on:click={() => {
								if (value && value?.extraOptions === undefined) {
									value.extraOptions = { mode: 'markers' }
								}
							}}
						>
							Scatter
						</option>
					</select>
				</Label>

				{#if value.type === 'scatter' && value.extraOptions}
					<Label label="Mode">
						<select bind:value={value.extraOptions.mode}>
							<option value="markers">Markers</option>
							<option value="lines">Lines</option>
							<option value="lines+markers">Lines + Markers</option>
						</select>
					</Label>
				{/if}

				<Label label="Aggregation method">
					<svelte:fragment slot="header">
						<Tooltip>
							A method to aggregate the data. For example, if you have multiple x data points with
							the same value, you can choose to sum them up or take the mean. If you don't have
							multiple x data points with the same value, this option will have no effect.
						</Tooltip>
					</svelte:fragment>
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
					id={$selectedComponent?.[0] ?? ''}
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
					on:openConnection={() => {
						closeOnOutsideClick = false
					}}
					on:closeConnection={() => {
						closeOnOutsideClick = true
					}}
				/>

				<Button color="red" size="xs" on:click={removeDataset}>Remove dataset</Button>
			</div>
		{/if}
	</svelte:fragment>
</Popover>
