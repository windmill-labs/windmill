<script lang="ts">
	import { Alert } from '$lib/components/common'
	import { getContext, onMount } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, RichConfiguration, RichConfigurations } from '../../types'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let datasets: RichConfiguration | undefined
	export let xData: RichConfiguration | undefined

	export let initializing: boolean | undefined = undefined
	export let render: boolean

	const { worldStore, darkMode } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['plotlycomponentv2'].initialData.configuration,
		configuration
	)

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let result: object | undefined = undefined
	let divEl: HTMLDivElement | null = null

	let Plotly
	onMount(async () => {
		//@ts-ignore
		await import(
			/* webpackIgnore: true */
			//@ts-ignorez
			'https://cdn.jsdelivr.net/npm/plotly.js-dist@2.18.0/plotly.min.js'
		)

		Plotly = window['Plotly']
	})

	let h: number | undefined = undefined
	let w: number | undefined = undefined

	let shouldeUpdate = 1

	darkMode.subscribe(() => {
		shouldeUpdate++
	})

	$: Plotly && render && resolvedConfig.layout && divEl && h && w && shouldeUpdate && plot(data)

	$: data =
		datasets && xData && resolvedDatasets
			? resolvedDatasets.map((d, index) => {
					const fields =
						d.type === 'pie'
							? {
									values: resolvedDatasetsValues[index]
							  }
							: {
									x: resolvedXData,
									y: resolvedDatasetsValues[index]
							  }

					return {
						type: d.type,
						color: d.color,
						text: d.tooltip,
						...fields,
						marker: {
							color: d.color
						},
						transforms: [
							{
								type: 'aggregate',
								groups: resolvedXData,
								aggregations: [{ target: 'y', func: d.aggregation_method, enabled: true }]
							}
						],
						...(d?.extraOptions ?? {})
					}
			  })
			: Array.isArray(result)
			? result
			: [result]

	let error = ''
	function plot(data) {
		try {
			Plotly.newPlot(
				divEl,
				data,
				{
					width: w,
					height: h,
					margin: { l: 50, r: 40, b: 40, t: 40, pad: 4 },
					paper_bgcolor: $darkMode ? '#2e3440' : '#fff',
					plot_bgcolor: $darkMode ? '#2e3440' : '#fff',
					...resolvedConfig.layout,
					xaxis: {
						color: $darkMode ? '#f3f6f8' : '#000',
						...(resolvedConfig?.layout?.['xaxis'] ?? {})
					},
					yaxis: {
						color: $darkMode ? '#f3f6f8' : '#000',
						...(resolvedConfig?.layout?.['yaxis'] ?? {})
					}
				},
				{ responsive: true, displayModeBar: false }
			)
			error = ''
		} catch (e) {
			error = e.message
			console.error(e)
		}
	}

	type Dataset = {
		value: RichConfiguration
		name: string
		aggregation_method: string
		tooltip: string
		color: string
		type: 'bar' | 'line' | 'scatter' | 'pie'
		extraOptions?: { mode: 'markers' | 'lines' | 'lines+markers' } | undefined
	}

	let resolvedDatasets: Dataset[]
	let resolvedDatasetsValues: Array<number[]> = []
	let resolvedXData: number[] = []
</script>

{#each Object.keys(components['plotlycomponentv2'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

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

{#if render}
	<div class="w-full h-full" bind:clientHeight={h} bind:clientWidth={w}>
		<RunnableWrapper {outputs} {render} {componentInput} {id} bind:initializing bind:result>
			{#if error != ''}
				<div class="flex flex-col h-full w-full overflow-auto">
					<Alert title="Plotly error" type="error" size="xs" class="h-full w-full ">
						<pre class="w-full bg-surface p-2 rounded-md whitespace-pre-wrap">{error}</pre>
					</Alert>
				</div>
			{/if}
			<div on:pointerdown bind:this={divEl}></div>
		</RunnableWrapper>
	</div>
{:else}
	<RunnableWrapper {outputs} {render} {componentInput} {id} />
{/if}
