<script lang="ts">
	import type { AppComponent } from '../component'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import PlotlyRichEditor from './PlotlyRichEditor.svelte'
	import ChartJSRichEditor from './ChartJSRichEditor.svelte'
	import AGChartRichEditor from './AGChartRichEditor.svelte'
	import { getContext, onMount } from 'svelte'
	import type { AppViewerContext, RichConfiguration } from '../../types'
	import type { InputConnectionEval } from '../../inputType'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'

	interface Props {
		component: AppComponent
		children?: import('svelte').Snippet
	}

	let { component = $bindable(), children }: Props = $props()
	let convertToUIEditorCallback: (() => void) | undefined = $state(undefined)

	let selected = $state('ui-editor')
	let renderCount = $state(0)

	onMount(() => {
		if (
			(component.type === 'plotlycomponentv2' ||
				component.type === 'chartjscomponentv2' ||
				component.type === 'agchartscomponent' ||
				component.type === 'agchartscomponentee') &&
			component.componentInput === undefined &&
			component.datasets === undefined
		) {
			setUpUIEditor()
			renderCount++
		} else if (component.componentInput !== undefined) {
			selected = 'json'
		}
	})

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')
	interface Dataset {
		value: any // Define more specific type if possible
		name: string
		aggregation_method: string
		type: string
		tooltip: string
		color: string
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

	function handleSelected(selected: string) {
		if (component.type === 'plotlycomponentv2') {
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
				convertToJson()
			}
		} else if (component.type === 'chartjscomponentv2') {
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
				convertChartJSToJson()
			}
		} else if (isAgChartsComponent()) {
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
				convertAgChartToJson()
			}
		}
	}

	function convertAgChartToJson() {
		if (component.type !== 'agchartscomponent' && component.type !== 'agchartscomponentee') {
			return
		}

		const connections: InputConnectionEval[] = []

		if (component.datasets === undefined || component.datasets.type !== 'static') return

		const datasetsAsString = datasetToAgChartJson()

		component.componentInput = {
			type: 'evalv2',
			fieldType: 'object',
			noStatic: true,
			expr: datasetsAsString,
			connections: connections.filter(Boolean)
		}

		component.datasets = undefined
		component.xData = undefined
	}

	function setUpUIEditor() {
		if (component.type === 'plotlycomponentv2') {
			component.datasets = createPlotlyComponentDataset()
			component.xData = createXData()
		} else if (component.type === 'chartjscomponentv2') {
			component.datasets = createChartjsComponentDataset()
			component.xData = createXData()
		} else if (component.type === 'agchartscomponent' || component.type === 'agchartscomponentee') {
			component.datasets = createAgChartsComponentDataset()
			component.xData = createXData()
		}
	}

	function createAgChartsComponentDataset(): RichConfiguration {
		return {
			type: 'static',
			fieldType: 'array',
			subFieldType: 'ag-chart',

			value: [
				{
					value: {
						type: 'oneOf',
						selected: 'bar',
						labels: {
							bar: 'Bar',
							scatter: 'Scatter',
							line: 'Line',
							area: 'Area',
							['range-bar']: 'Range Bar',
							['range-area']: 'Range Area'
						},
						configuration: {
							bar: {
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number',
									value: [25, 25, 50]
								}
							},
							scatter: {
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number',
									value: [25, 25, 50]
								}
							},
							line: {
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number',
									value: [25, 25, 50]
								}
							},
							area: {
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number',
									value: [25, 25, 50]
								}
							},
							['range-bar']: {
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number-tuple',
									value: [
										[10, 15],
										[20, 25],
										[18, 27]
									]
								}
							},
							['range-area']: {
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number-tuple',
									value: [
										[10, 15],
										[20, 25],
										[18, 27]
									]
								}
							}
						}
					} as const,
					name: 'Dataset 1',
					type: 'bar'
				}
			]
		}
	}

	function createChartjsComponentDataset(): RichConfiguration {
		return {
			type: 'static',
			fieldType: 'array',
			subFieldType: 'chartjs',
			value: [
				{
					value: {
						type: 'static',
						fieldType: 'array',
						subFieldType: 'number',
						value: [25, 25, 50]
					},
					name: 'Dataset 1'
				}
			]
		}
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

	function isAgChartsComponent(): boolean {
		return component.type === 'agchartscomponent' || component.type === 'agchartscomponentee'
	}

	function convertToJson() {
		if (
			component.type !== 'plotlycomponentv2' &&
			component.type !== 'agchartscomponent' &&
			component.type !== 'agchartscomponentee'
		) {
			return
		}

		const connections: InputConnectionEval[] = []
		const xDataResolved = resolveConfiguration(component.xData, connections)

		if (component.datasets === undefined || component.datasets.type !== 'static') return

		const datasets = component.datasets?.value

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

	function convertChartJSToJson() {
		if (component.type !== 'chartjscomponentv2') {
			return
		}

		const connections: InputConnectionEval[] = []
		const xDataResolved = resolveConfiguration(component.xData, connections)

		if (component.datasets === undefined || component.datasets.type !== 'static') return

		const datasets = component.datasets?.value

		component.componentInput = {
			type: 'evalv2',
			fieldType: 'object',
			noStatic: true,
			expr: `({
\t"labels": ${xDataResolved},
\t"datasets": [\n${
				datasets
					.map(
						(rawDataset: Dataset) => `\t\t{
\t\t\tdata:${resolveConfiguration(rawDataset.value, connections)},
\t\t\tlabel: "${rawDataset.name}"
\t\t}`
					)
					.join(',\n') || ''
			}\n\t]
})`,
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

	function datasetToAgChartJson(): string {
		const outputs = $worldStore.outputsById[component.id]
		const result = outputs?.result.peak()

		if (!result.data && !result.series) {
			throw new Error('Invalid result')
		}

		return (
			'(' +
			JSON.stringify(
				{
					data: result.data,
					series: result.series
				},
				null,
				'\t'
			) +
			')'
		)
	}
</script>

{#if component.type === 'plotlycomponentv2' || component.type === 'chartjscomponentv2' || component.type === 'agchartscomponent' || component.type === 'agchartscomponentee'}
	<div class="p-2">
		<ToggleButtonGroup
			bind:selected
			on:selected={({ detail }) => {
				handleSelected(detail)
			}}
		>
			{#snippet children({ item })}
				<ToggleButton
					value="ui-editor"
					label="UI Editor"
					tooltip="Use the UI editor to quickly create a plotly chart."
					{item}
				/>
				<ToggleButton
					value="json"
					label="JSON"
					tooltip="Switch to JSON mode for complete customization of Plotly settings."
					{item}
				/>
			{/snippet}
		</ToggleButtonGroup>
	</div>

	{#if selected === 'ui-editor'}
		{#key renderCount}
			{#if component.type === 'plotlycomponentv2'}
				<PlotlyRichEditor
					id={component.id}
					bind:datasets={component.datasets}
					bind:xData={component.xData}
				/>
			{:else if isAgChartsComponent()}
				<AGChartRichEditor
					id={component.id}
					bind:datasets={component.datasets}
					bind:xData={component.xData}
				/>
			{:else if component.type === 'chartjscomponentv2'}
				<ChartJSRichEditor
					id={component.id}
					bind:datasets={component.datasets}
					bind:xData={component.xData}
				/>
			{/if}
		{/key}
	{:else}
		{@render children?.()}
	{/if}
{:else}
	{@render children?.()}
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
