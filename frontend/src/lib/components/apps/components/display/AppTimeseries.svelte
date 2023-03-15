<script lang="ts">
	import zoomPlugin from 'chartjs-plugin-zoom'
	import 'chartjs-adapter-date-fns'
	import {
		Chart as ChartJS,
		Title,
		Tooltip,
		Legend,
		LineElement,
		LinearScale,
		PointElement,
		CategoryScale,
		BarElement,
		TimeScale,
		LogarithmicScale,
		type Point
	} from 'chart.js'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import { Scatter } from 'svelte-chartjs'
	import InputValue from '../helpers/InputValue.svelte'
	import type { ChartOptions, ChartData } from 'chart.js'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { getContext } from 'svelte'
	import { concatCustomCss } from '../../utils'
	import { initOutput } from '../../editor/appUtils'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let logarithmicScale = false
	let zoomable = false
	let pannable = false

	ChartJS.register(
		Title,
		Tooltip,
		Legend,
		LineElement,
		LinearScale,
		PointElement,
		CategoryScale,
		BarElement,
		zoomPlugin,
		TimeScale,
		LogarithmicScale
	)

	let result: { data: { x: any[]; y: string[] } } | undefined = undefined

	$: options = {
		responsive: true,
		animation: false,
		maintainAspectRatio: false,
		plugins: {
			zoom: {
				pan: {
					enabled: pannable
				},
				zoom: {
					drag: {
						enabled: false
					},
					wheel: {
						enabled: zoomable
					}
				}
			}
		},
		scales: {
			x: {
				type: 'time'
			},
			y: {
				type: logarithmicScale ? 'logarithmic' : 'linear'
			}
		}
	} as ChartOptions<'scatter'>

	$: data = {
		datasets: result ?? []
	} as ChartData<'scatter', (number | Point)[], unknown>

	$: css = concatCustomCss($app.css?.timeseriescomponent, customCss)
</script>

<InputValue {id} input={configuration.logarithmicScale} bind:value={logarithmicScale} />
<InputValue {id} input={configuration.zoomable} bind:value={zoomable} />
<InputValue {id} input={configuration.pannable} bind:value={pannable} />

<RunnableWrapper {render} flexWrap autoRefresh {componentInput} {id} bind:initializing bind:result>
	<div class="w-full h-full {css?.container?.class ?? ''}" style={css?.container?.style ?? ''}>
		{#if result}
			<Scatter {data} {options} />
		{/if}
	</div>
</RunnableWrapper>
