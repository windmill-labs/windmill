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

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>

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

	const zoomOptions = {
		pan: {
			enabled: true
		},
		zoom: {
			drag: {
				enabled: false
			},
			wheel: {
				enabled: true
			}
		}
	}

	let result: { data: { x: any[]; y: string[] } } | undefined = undefined

	const options = {
		responsive: true,
		animation: false,
		maintainAspectRatio: false,
		plugins: {
			zoom: zoomOptions
		}
	}

	$: data = {
		datasets: result ?? []
	}
</script>

<RunnableWrapper autoRefresh bind:componentInput {id} bind:result>
	{#if result}
		<Scatter {data} {options} />
	{/if}
</RunnableWrapper>
