<script lang="ts">
	import zoomPlugin from 'chartjs-plugin-zoom'
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
		type Point
	} from 'chart.js'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import Scatter from 'svelte-chartjs/Scatter.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import type { ChartOptions, ChartData } from 'chart.js'
	import { concatCustomCss } from '../../utils'
	import { getContext } from 'svelte'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined

	let zoomable = false
	let pannable = false

	export const staticOutputs: string[] = ['loading', 'result']
	const { app } = getContext<AppEditorContext>('AppEditorContext')

	ChartJS.register(
		Title,
		Tooltip,
		Legend,
		LineElement,
		LinearScale,
		PointElement,
		CategoryScale,
		BarElement,
		zoomPlugin
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
		}
	} as ChartOptions<'scatter'>

	$: data = {
		datasets: result ?? []
	} as ChartData<'scatter', (number | Point)[], unknown>

	$: css = concatCustomCss($app.css?.scatterchartcomponent, customCss)
</script>

<InputValue {id} input={configuration.zoomable} bind:value={zoomable} />
<InputValue {id} input={configuration.pannable} bind:value={pannable} />

<RunnableWrapper flexWrap autoRefresh {componentInput} {id} bind:initializing bind:result>
	<div class="w-full h-full {css?.container?.class ?? ''}" style={css?.container?.style ?? ''}>
		{#if result}
			<Scatter {data} {options} />
		{/if}
	</div>
</RunnableWrapper>
