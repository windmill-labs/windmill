<script lang="ts">
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import { getContext, onMount } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { AgCharts, type AgChartOptions, type AgBarSeriesOptions } from 'ag-charts-community'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'agchartcomponent'> | undefined = undefined
	export let render: boolean

	interface IData {
		// Chart Data Interface
		month:
			| 'Jan'
			| 'Feb'
			| 'Mar'
			| 'Apr'
			| 'May'
			| 'Jun'
			| 'Jul'
			| 'Aug'
			| 'Sep'
			| 'Oct'
			| 'Nov'
			| 'Dec'
		avgTemp: number
		iceCreamSales: number
	}

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

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

	onMount(() => {
		try {
			// Chart Options
			const options: AgChartOptions = {
				container: document.getElementById('myChart') as HTMLElement, // Container: HTML Element to hold the chart
				// Data: Data to be displayed in the chart
				data: [
					{ month: 'Jan', avgTemp: 2.3, iceCreamSales: 162000 },
					{ month: 'Mar', avgTemp: 6.3, iceCreamSales: 302000 },
					{ month: 'May', avgTemp: 16.2, iceCreamSales: 800000 },
					{ month: 'Jul', avgTemp: 22.8, iceCreamSales: 1254000 },
					{ month: 'Sep', avgTemp: 14.5, iceCreamSales: 950000 },
					{ month: 'Nov', avgTemp: 8.9, iceCreamSales: 200000 }
				] as IData[],
				// Series: Defines which chart type and data to use
				series: [{ type: 'bar', xKey: 'month', yKey: 'iceCreamSales' } as AgBarSeriesOptions]
			}
			AgCharts.create(options)
		} catch (error) {
			console.error(error)
		}
	})
</script>

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

<div
	class={twMerge('w-full h-full', css?.container?.class, 'wm-agchart')}
	style={css?.container?.style ?? ''}
>
	<div id="myChart" />
</div>
