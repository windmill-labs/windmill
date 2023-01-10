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
		BarElement
	} from 'chart.js'

	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import Scatter from 'svelte-chartjs/Scatter.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>

	let zoomable = false
	let pannable = false

	export const staticOutputs: string[] = ['loading', 'result']

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
	}

	$: data = {
		datasets: result ?? []
	}
</script>

<InputValue {id} input={configuration.zoomable} bind:value={zoomable} />
<InputValue {id} input={configuration.pannable} bind:value={pannable} />

<RunnableWrapper flexWrap autoRefresh bind:componentInput {id} bind:result>
	{#if result}
		<Scatter {data} {options} />
	{/if}
</RunnableWrapper>
