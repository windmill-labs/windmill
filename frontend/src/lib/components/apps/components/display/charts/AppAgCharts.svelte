<script lang="ts">
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../../types'
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

		license = undefined,
		ee = false
	}: Props = $props()

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

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
			chartInstance && updateChartByResult()
		})
	}}
/>

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
