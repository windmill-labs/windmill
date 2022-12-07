<script lang="ts">
	import { Bar } from 'svelte-chartjs'

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

	import type { ChartData } from 'chart.js'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { ComponentInput, ComponentParameter } from '../../inputType'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let componentInput: ComponentInput | undefined
	export let configuration: Record<string, ComponentParameter>

	export const staticOutputs: string[] = ['loading', 'result']

	ChartJS.register(
		Title,
		Tooltip,
		Legend,
		LineElement,
		LinearScale,
		PointElement,
		CategoryScale,
		BarElement
	)

	let result: ChartData<'bar', number[], unknown> | undefined = undefined
	let labels: string[] = []
	let theme: string[] = []

	const options = {
		responsive: true,
		animation: false
	}

	$: data = {
		labels,
		datasets: [
			{
				data: result,
				backgroundColor: theme
			}
		]
	}
</script>

<InputValue input={configuration.theme} bind:value={theme} />
<InputValue input={configuration.labels} bind:value={labels} />

<RunnableWrapper {componentInput} {id} bind:result>
	{#if data}
		<Bar {data} {options} />
	{/if}
</RunnableWrapper>
