<script lang="ts">
	import { Chart } from '$lib/components/chartjs-wrappers/chartJs'
	import { Chart as ChartJS, registerables, type ChartOptions } from 'chart.js'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		RichConfiguration,
		RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { deepMergeWithPriority } from '$lib/utils'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'chartjscomponent'> | undefined = undefined
	export let render: boolean
	export let datasets: RichConfiguration | undefined
	export let xData: RichConfiguration | undefined

	type Dataset = {
		value: RichConfiguration
		name: string
	}

	let resolvedDatasets: Dataset[]
	let resolvedDatasetsValues: Array<number[]> = []
	let resolvedXData: any[] = []

	const { app, worldStore, darkMode } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	ChartJS.register(...registerables)

	let result: undefined = undefined

	const resolvedConfig = initConfig(
		components['chartjscomponent'].initialData.configuration,
		configuration
	)

	function hasScales() {
		const type = resolvedConfig?.type as string
		return type === 'bar' || type === 'line' || type === 'scatter' || type === 'bubble'
	}

	$: options = deepMergeWithPriority(resolvedConfig.options, {
		responsive: true,
		animation: false,
		maintainAspectRatio: false,
		plugins: {
			legend: {
				labels: {
					color: $darkMode ? '#fff' : '#000'
				}
			}
		},
		...(hasScales()
			? {
					scales: {
						y: {
							ticks: { color: $darkMode ? '#eee' : '#333' },
							grid: { color: $darkMode ? '#555' : '#ddd' }
						},
						x: {
							ticks: { color: $darkMode ? '#eee' : '#333' },
							grid: { color: $darkMode ? '#555' : '#ddd' }
						}
					}
				}
			: {})
	}) as ChartOptions

	$: data =
		datasets && xData && resolvedDatasets
			? {
					labels: resolvedXData,
					datasets: resolvedDatasets?.map((d, index) => ({
						label: d.name,
						data: Array.isArray(resolvedDatasetsValues[index]) ? resolvedDatasetsValues[index] : []
					}))
				}
			: result

	let css = initCss(app.val.css?.chartjscomponent, customCss)
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

{#each Object.keys(components['chartjscomponentv2'].initialData.configuration) as key (key)}
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
		componentStyle={app.val.css?.chartjscomponent}
	/>
{/each}

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	<div
		class={twMerge('w-full h-full', css?.container?.class, 'wm-chartjs')}
		style={css?.container?.style ?? ''}
	>
		{#if data && resolvedConfig.type}
			{#key resolvedConfig.type}
				{#key options}
					<Chart type={resolvedConfig.type} {data} {options} />
				{/key}
			{/key}
		{/if}
	</div>
</RunnableWrapper>
