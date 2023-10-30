<script lang="ts">
	import type { AppComponent } from '../component'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import PlotlyRichEditor from './PlotlyRichEditor.svelte'
	import { onMount } from 'svelte'
	import type { RichConfiguration } from '../../types'
	import type { InputConnectionEval } from '../../inputType'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'

	export let component: AppComponent
	let convertToUIEditorCallback: (() => void) | undefined = undefined

	let selected = 'ui-editor'
	let renderCount = 0

	onMount(() => {
		if (
			component.type === 'plotlycomponentv2' &&
			component.componentInput === undefined &&
			component.datasets === undefined
		) {
			setUpUIEditor()
			renderCount++
		} else if (component.componentInput !== undefined) {
			selected = 'json'
		}
	})

	interface Dataset {
		value: any // Define more specific type if possible
		name: string
		aggregation_method: string
		type: string
		tooltip: string
		color: string
	}

	function handleSelected(selected: string) {
		if (component.type !== 'plotlycomponentv2') return

		if (selected === 'ui-editor') {
			convertToUIEditorCallback = () => {
				component.componentInput = undefined
				setUpUIEditor()
			}

			setTimeout(() => {
				const activeElement = document.activeElement as HTMLElement
				activeElement?.blur()
				document.body.focus()
			})
		} else if (selected === 'json') {
			convertToJson(component)
		}
	}

	function setUpUIEditor() {
		if (component.type !== 'plotlycomponentv2') return

		component.datasets = createPlotlyComponentDataset()
		component.xData = createXData()
	}

	function createPlotlyComponentDataset(): RichConfiguration {
		return {
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
					tooltip: '',
					color: '#C8A2C8'
				}
			]
		}
	}

	function createXData(): RichConfiguration {
		return {
			type: 'evalv2',
			fieldType: 'array',
			expr: '[1, 2, 3, 4]',
			connections: []
		}
	}

	function resolveConfiguration(
		x: RichConfiguration | undefined,
		connections: InputConnectionEval[]
	): string | undefined {
		if (!x) return undefined
		if (x.type === 'evalv2') {
			connections.push(...x.connections)
			return x.expr
		}
		if (x.type === 'static') return `[${x.value}]`
		return undefined
	}

	function convertToJson(component: any) {
		const connections: InputConnectionEval[] = []
		const xDataResolved = resolveConfiguration(component.xData, connections)

		const datasets =
			(component.datasets?.type === 'static'
				? component.datasets?.value
				: JSON.parse(component.datasets?.expr)) ?? []

		const datasetsAsString =
			datasets
				.map((rawDataset: Dataset) => datasetToJson(rawDataset, xDataResolved, connections))
				.join(',\n') || ''

		component.componentInput = {
			type: 'evalv2',
			fieldType: 'object',
			noStatic: true,
			expr: `[\n${datasetsAsString}\n]`,
			connections: connections.filter(Boolean)
		}

		component.datasets = undefined
		component.xData = undefined
	}

	function datasetToJson(
		dataset: Dataset,
		xDataExpr: string | undefined,
		connections: InputConnectionEval[]
	): string {
		return `\t{
\t\t"type": "${dataset.type}",
\t\t"x": ${xDataExpr},
\t\t"y": ${resolveConfiguration(dataset.value, connections)},
\t\t"text": "${dataset.tooltip || ''}",
\t\t"aggregation_method": "${dataset.aggregation_method}",
\t\t"marker": {
\t\t\t"color": "${dataset.color}"
\t\t}
\t}`
	}
</script>

{#if component.type === 'plotlycomponentv2'}
	<div class="p-2">
		<ToggleButtonGroup
			bind:selected
			on:selected={() => {
				handleSelected(selected)
			}}
		>
			<ToggleButton
				value="ui-editor"
				label="UI Editor"
				tooltip="Use the UI editor to quickly create a plotly chart."
			/>
			<ToggleButton
				value="json"
				label="JSON"
				tooltip="Switch to JSON mode for complete customization of Plotly settings."
			/>
		</ToggleButtonGroup>
	</div>

	{#if selected === 'ui-editor'}
		{#key renderCount}
			<PlotlyRichEditor
				id={component.id}
				bind:datasets={component.datasets}
				bind:xData={component.xData}
			/>
		{/key}
	{:else}
		<slot />
	{/if}
{:else}
	<slot />
{/if}

<ConfirmationModal
	open={Boolean(convertToUIEditorCallback)}
	title="Convert to UI Editor"
	confirmationText="Remove"
	on:canceled={() => {
		convertToUIEditorCallback = undefined
		selected = 'json'
	}}
	on:confirmed={() => {
		if (convertToUIEditorCallback) {
			convertToUIEditorCallback()
		}
		convertToUIEditorCallback = undefined
		selected = 'ui-editor'
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>
			Are you sure you want to convert this component to the UI Editor? The UI Editor does not
			support all the features of the JSON editor.
		</span>
	</div>
</ConfirmationModal>
