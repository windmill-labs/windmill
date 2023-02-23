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

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let initializing: boolean | undefined = undefined

	export const staticOutputs: string[] = ['loading', 'result']

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
</script>

<InputValue {id} input={configuration.logarithmicScale} bind:value={logarithmicScale} />
<InputValue {id} input={configuration.zoomable} bind:value={zoomable} />
<InputValue {id} input={configuration.pannable} bind:value={pannable} />

<RunnableWrapper flexWrap autoRefresh bind:componentInput {id} bind:initializing bind:result>
	{#if result}
		<Scatter {data} {options} />
	{/if}
</RunnableWrapper>
