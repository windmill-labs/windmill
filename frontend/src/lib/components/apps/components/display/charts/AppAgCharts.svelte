<script lang="ts">
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../../inputType'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		RichConfiguration,
		RichConfigurations
	} from '../../../types'
	import { initCss } from '../../../utils'
	import { getContext, tick, untrack } from 'svelte'
	import { initConfig, initOutput } from '../../../editor/appUtils'
	import { components } from '../../../editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../../helpers/ResolveStyle.svelte'
	import type { AgChartOptions, AgChartInstance } from 'ag-charts-community'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'agchartscomponent'> | undefined
		render: boolean
		datasets: RichConfiguration | undefined
		xData: RichConfiguration | undefined
		license?: string | undefined
		ee?: boolean
	}

	let {
		id,
		componentInput,
		configuration,
		initializing = $bindable(undefined),
		customCss = undefined,
		render,
		datasets,
		xData,
		license = undefined,
		ee = false
	}: Props = $props()

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	type Dataset = {
		value: RichConfiguration
		name: string
		type: 'bar' | 'line' | 'scatter' | 'area' | 'range-bar' | 'range-area'
	}

	let resolvedDatasets = $state(undefined) as Dataset[] | undefined
	let resolvedXData: number[] = $state([])

	const outputs = initOutput($worldStore, id, {
		result: undefined as
			| {
					data: any[]
					series: any[]
			  }
			| undefined,
		loading: false
	})

	let result: undefined | any = $state(undefined)

	const resolvedConfig = $state(
		initConfig(components['agchartscomponent'].initialData.configuration, configuration)
	)

	let css = $state(initCss($app.css?.agchartscomponent, customCss))
	let chartInstance: AgChartInstance | undefined = $state(undefined)

	function getChartStyleByTheme() {
		const gridColor = darkMode ? '#555555' : '#dddddd'
		const axisColor = darkMode ? '#555555' : '#dddddd'
		const textColor = darkMode ? '#eeeeee' : '#333333'

		return {
			axes: [
				{
					type: 'category',
					position: 'bottom',
					label: { color: textColor },
					line: { color: axisColor },
					tick: { color: axisColor },
					gridLine: {
						style: [
							{
								stroke: gridColor
							}
						]
					}
				},
				{
					type: 'number',
					position: 'left',
					label: { color: textColor },
					line: { color: axisColor },
					tick: { color: axisColor },
					gridLine: {
						style: [
							{
								stroke: gridColor
							}
						]
					}
				}
			],
			background: {
				visible: false
			}
		}
	}

	function updateChart() {
		if (!chartInstance) {
			return
		}

		let data = [] as any[]
		if (!resolvedDatasets) {
			return
		}
		for (let i = 0; i < resolvedXData.length; i++) {
			const o = {
				x: resolvedXData[i]
			}
			if (!resolvedDatasetsValues) {
				return
			}
			for (let j = 0; j < resolvedDatasets.length; j++) {
				if (!resolvedDatasetsValues[j]) {
					continue
				}

				if (
					resolvedDatasetsValues[j].type === 'range-bar' ||
					resolvedDatasetsValues[j].type === 'range-area'
				) {
					o[`y-${j}-low`] = resolvedDatasetsValues[j].value[i]?.[0]
					o[`y-${j}-high`] = resolvedDatasetsValues[j].value[i]?.[1]
				} else {
					o[`y-${j}`] = resolvedDatasetsValues[j].value[i]
				}
			}

			data.push(o)
		}

		const options = {
			container: document.getElementById(`agchart-${id}`) as HTMLElement,
			data: data,
			series:
				(resolvedDatasets?.map((d, index) => {
					if (!resolvedDatasetsValues) {
						return
					}
					const type = resolvedDatasetsValues[index].type
					if (type === 'range-bar' || type === 'range-area') {
						return {
							type: type,
							xKey: 'x',
							yLowKey: `y-${index}-low`,
							yHighKey: `y-${index}-high`,
							yName: d.name
						}
					} else {
						return {
							type: type,
							xKey: 'x',
							yKey: `y-${index}`,
							yName: d.name
						}
					}
				}) as any[]) ?? [],
			...getChartStyleByTheme()
		}

		outputs.result.set({
			data: options.data,
			series: options.series
		})
		AgChartsInstance?.update(chartInstance, options)
	}

	function updateChartByResult() {
		if (!result || !chartInstance) {
			return
		}

		if (typeof result !== 'object') {
			return
		}
		const options = {
			container: document.getElementById(`agchart-${id}`) as HTMLElement,
			...getChartStyleByTheme(),
			...result
		}

		outputs.result.set({
			data: result?.['data'],
			series: result?.['series']
		})
		AgChartsInstance?.update(chartInstance, options)
	}

	let AgChartsInstance: any | undefined = undefined

	async function loadLibrary() {
		if (ee) {
			const enterprise = await import('ag-charts-enterprise')
			AgChartsInstance = enterprise.AgCharts

			AgChartsInstance.setLicenseKey(license)
		} else {
			const community = await import('ag-charts-community')
			AgChartsInstance = community.AgCharts
		}
	}

	function initChart() {
		try {
			// Chart Options
			const options: AgChartOptions = {
				container: document.getElementById(`agchart-${id}`) as HTMLElement,
				data: [],
				series: [],
				...getChartStyleByTheme()
			}

			chartInstance = AgChartsInstance?.create(options)
		} catch (error) {
			console.error(error)
		}
	}

	function destroyChart() {
		if (chartInstance) {
			chartInstance.destroy()
			chartInstance = undefined
		}
	}

	let darkMode = $state(false)
	let resolvedDatasetsValues = $derived(
		resolvedDatasets?.map((d) => {
			const config = initConfig(
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
					}
				},
				{
					value: d.value
				}
			)

			return {
				type: d.value['selected'],
				// @ts-ignore
				value: config.value?.['configuration']?.[d.value['selected']].value
			}
		})
	)
	$effect(() => {
		resolvedXData &&
			resolvedDatasets &&
			resolvedDatasetsValues &&
			chartInstance &&
			untrack(() => updateChart())
	})
	$effect(() => {
		result && chartInstance && untrack(() => updateChartByResult())
	})
	$effect(() => {
		license && untrack(() => loadLibrary())
	})
	$effect(() => {
		if (render) {
			untrack(() => {
				destroyChart()
				loadLibrary().then(() => {
					initChart()
				})
			})
		}
	})
</script>

<DarkModeObserver
	bind:darkMode
	on:change={(e) => {
		tick().then(() => {
			updateChart()
		})
	}}
/>

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
		{#if resolvedDatasetsValues}
			<ResolveConfig
				{id}
				key={'datasets' + index}
				extraKey={resolvedDataset.name}
				bind:resolvedConfig={resolvedDatasetsValues[index]}
				configuration={resolvedDataset.value}
			/>
		{/if}
	{/each}
{/if}

{#each Object.keys(components['agchartscomponent'].initialData.configuration) as key (key)}
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
		<!-- {JSON.stringify(result)} -->
		<div id={`agchart-${id}`} class="h-full w-full"></div>
	</div>
</RunnableWrapper>
