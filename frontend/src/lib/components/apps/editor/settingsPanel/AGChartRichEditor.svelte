<script lang="ts">
	import type { RichConfiguration } from '../../types'
	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import PanelSection from './common/PanelSection.svelte'

	interface Props {
		datasets?: RichConfiguration | undefined
		xData?: RichConfiguration | undefined
		id: string
	}

	let { datasets = $bindable(undefined), xData = $bindable(undefined), id }: Props = $props()
</script>

<PanelSection
	title={`AG Chart configuration`}
	tooltip="The configuration is divided into two parts: X-axis data and an array of datasets. Each dataset hold the data for the Y-axis and the configuration for the plot (type, name, etc)."
>
	<div class="w-full flex flex-col gap-4">
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
				tooltip="For each dataset, you can specify the data for the Y-axis and the configuration for the plot (type, color, etc). If you want to have an eval for every data point, you can switch to JSON mode."
				fileUpload={datasets?.['fileUpload']}
				placeholder={datasets?.['placeholder']}
				customTitle={datasets?.['customTitle']}
				displayType={false}
				shouldFormatExpression={true}
				allowTypeChange={false}
			/>
		{/if}
	</div>
</PanelSection>
