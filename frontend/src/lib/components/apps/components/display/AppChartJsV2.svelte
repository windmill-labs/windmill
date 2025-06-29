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

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'chartjscomponent'> | undefined
		render: boolean
		datasets: RichConfiguration | undefined
		xData: RichConfiguration | undefined
	}

	let {
		id,
		componentInput,
		configuration,
		initializing = $bindable(undefined),
		customCss = undefined,
		render,
		datasets,
		xData
	}: Props = $props()

	type Dataset = {
		value: RichConfiguration
		name: string
	}

	let resolvedDatasets = $state(undefined) as Dataset[] | undefined
	let resolvedDatasetsValues: Array<number[]> = $state([])
	let resolvedXData: any[] = $state([])

	const { app, worldStore, darkMode } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	ChartJS.register(...registerables)

	let result: undefined = $state(undefined)

	const resolvedConfig = $state(
		initConfig(components['chartjscomponent'].initialData.configuration, configuration)
	)

	function hasScales() {
		const type = resolvedConfig?.type as string
		return type === 'bar' || type === 'line' || type === 'scatter' || type === 'bubble'
	}

	let options = $derived(
		deepMergeWithPriority(resolvedConfig.options, {
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
	)

	let data = $derived(
		datasets && xData && resolvedDatasets
			? {
					labels: resolvedXData,
					datasets: resolvedDatasets?.map((d, index) => ({
						label: d.name,
						data: Array.isArray(resolvedDatasetsValues[index]) ? resolvedDatasetsValues[index] : []
					}))
				}
			: result
	)

	let css = $state(initCss($app.css?.chartjscomponent, customCss))
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
		componentStyle={$app.css?.chartjscomponent}
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
