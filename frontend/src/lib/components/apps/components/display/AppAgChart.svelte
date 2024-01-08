<script lang="ts">
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		RichConfiguration,
		RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import { getContext, onMount } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { AgCharts, type AgChartOptions, type AgChartInstance } from 'ag-charts-community'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'agchartcomponent'> | undefined = undefined
	export let render: boolean
	export let datasets: RichConfiguration | undefined
	export let xData: RichConfiguration | undefined

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	type Dataset = {
		value: RichConfiguration
		name: string
		type: 'bar' | 'line'
	}

	let resolvedDatasets: Dataset[]
	let resolvedDatasetsValues: Array<number[]> = []
	let resolvedXData: number[] = []

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let result: undefined = undefined

	const resolvedConfig = initConfig(
		components['agchartcomponent'].initialData.configuration,
		configuration
	)

	let css = initCss($app.css?.agchartcomponent, customCss)

	let chartInstance: AgChartInstance | undefined = undefined

	function updateChart() {
		if (!chartInstance) {
			return
		}

		let data = [] as any[]

		for (let i = 0; i < resolvedXData.length; i++) {
			const o = {
				x: resolvedXData[i]
			}

			for (let j = 0; j < resolvedDatasets.length; j++) {
				if (!resolvedDatasetsValues[j]) {
					continue
				}

				o[`y-${j}`] = resolvedDatasetsValues[j][i]
			}

			data.push(o)
		}

		const options = {
			container: document.getElementById('myChart') as HTMLElement,
			data: data,
			series:
				(resolvedDatasets?.map((d, index) => ({
					type: d.type,
					xKey: 'x',
					yKey: `y-${index}`,
					yName: d.name
				})) as any[]) ?? []
		}

		AgCharts.update(chartInstance, options)
	}

	$: resolvedXData && resolvedDatasets && resolvedDatasetsValues && chartInstance && updateChart()
	$: result && updateChartByResult()

	function updateChartByResult() {
		if (!result || !chartInstance) {
			return
		}

		debugger

		const options = {
			container: document.getElementById('myChart') as HTMLElement,
			data: result?.['data'],
			series: result?.['series']
		}

		AgCharts.update(chartInstance, options)
	}

	onMount(() => {
		try {
			// Chart Options
			const options: AgChartOptions = {
				container: document.getElementById('myChart') as HTMLElement,
				data: [],
				series: []
			}

			chartInstance = AgCharts.create(options)
		} catch (error) {
			console.error(error)
		}
	})
</script>

{#if datasets}
	<ResolveConfig
		{id}
		key={'datasets'}
		bind:resolvedConfig={resolvedDatasets}
		configuration={datasets}
	/>
{/if}

{#if xData}
	<ResolveConfig {id} key={'xData'} bind:resolvedConfig={resolvedXData} configuration={xData} />
{/if}

{#if resolvedDatasets}
	{#each resolvedDatasets as resolvedDataset, index (resolvedDataset.name + index)}
		<ResolveConfig
			{id}
			key={'datasets' + index}
			bind:resolvedConfig={resolvedDatasetsValues[index]}
			configuration={resolvedDataset.value}
		/>
	{/each}
{/if}

{#each Object.keys(components['agchartcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.chartjscomponent}
	/>
{/each}

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	<div
		class={twMerge('w-full h-full', css?.container?.class, 'wm-agchart')}
		style={css?.container?.style ?? ''}
	>
		<div id="myChart" class="h-full w-full" />
	</div>
</RunnableWrapper>
