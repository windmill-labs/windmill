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
	import type { AppInput } from '../../inputType'
	import InputValue from '../helpers/InputValue.svelte'

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
		BarElement
	)

	let result: ChartData<'bar', number[], unknown> | undefined = undefined
	let labels: string[] = []
	let theme: string = 'theme1'

	$: backgroundColor = {
		theme1: ['#FF6384', '#4BC0C0', '#FFCE56', '#E7E9ED', '#36A2EB'],
		// blue theme
		theme2: ['#4e73df', '#1cc88a', '#36b9cc', '#f6c23e', '#e74a3b'],
		// red theme
		theme3: ['#e74a3b', '#4e73df', '#1cc88a', '#36b9cc', '#f6c23e']
	}[theme]

	const options = {
		responsive: true,
		animation: false
	}

	$: data = {
		labels,
		datasets: [
			{
				data: result,
				backgroundColor
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
