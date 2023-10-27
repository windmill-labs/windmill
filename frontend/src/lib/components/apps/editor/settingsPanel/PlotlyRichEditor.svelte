<script lang="ts">
	import type { RichConfiguration } from '../../types'
	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import PanelSection from './common/PanelSection.svelte'

	export let datasets: RichConfiguration | undefined = undefined
	export let kind: RichConfiguration | undefined = undefined
	export let xData: RichConfiguration | undefined = undefined
	export let id: string

	$: if (datasets == undefined) {
		datasets = {
			type: 'static',
			fieldType: 'array',
			subFieldType: 'plotly',
			value: [
				{
					value: {
						type: 'static',
						fieldType: 'array',
						subFieldType: 'number',
						value: [1, 2, 3, 4]
					},
					name: 'Dataset 1',
					aggregation_method: 'sum',
					type: 'bar',
					toolip: 'This is an example',
					color: '#FF0000'
				}
			]
		}
	}

	$: if (kind == undefined) {
		kind = {
			type: 'static',
			fieldType: 'select',
			value: 'bar',
			selectOptions: ['bar', 'line', 'scatter', 'pie']
		}
	}

	$: if (xData == undefined) {
		xData = {
			type: 'static',
			fieldType: 'array',
			value: [1, 2, 3, 4]
		}
	}
</script>

<PanelSection title={`Plotly configuration`} tooltip="s">
	<div class="w-full flex flex-col gap-2">
		{#if xData}
			<InputsSpecEditor
				key={`X-axis data`}
				bind:componentInput={xData}
				{id}
				userInputEnabled={false}
				shouldCapitalize={true}
				resourceOnly={false}
				fieldType={xData?.['fieldType']}
				subFieldType={xData?.['subFieldType']}
				format={xData?.['format']}
				selectOptions={xData?.['selectOptions']}
				tooltip={xData?.['tooltip']}
				fileUpload={xData?.['fileUpload']}
				placeholder={xData?.['placeholder']}
				customTitle={xData?.['customTitle']}
				displayType={false}
			/>
		{/if}

		{#if datasets}
			<InputsSpecEditor
				key={`Dataset`}
				bind:componentInput={datasets}
				{id}
				userInputEnabled={false}
				shouldCapitalize={true}
				resourceOnly={false}
				fieldType={datasets?.['fieldType']}
				subFieldType={datasets?.['subFieldType']}
				format={datasets?.['format']}
				selectOptions={datasets?.['selectOptions']}
				tooltip={datasets?.['tooltip']}
				fileUpload={datasets?.['fileUpload']}
				placeholder={datasets?.['placeholder']}
				customTitle={datasets?.['customTitle']}
				displayType={false}
			/>
		{/if}</div
	>
</PanelSection>
