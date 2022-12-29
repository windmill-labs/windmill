<script lang="ts">
	import { Pie } from 'svelte-chartjs'

	import {
		Chart as ChartJS,
		Title,
		Tooltip,
		Legend,
		LineElement,
		LinearScale,
		PointElement,
		CategoryScale,
		ArcElement
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
		ArcElement
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
		animation: false,
		maintainAspectRatio: false
	}

	$: data = {
		labels,
		datasets: [
			{
				data: result,
				backgroundColor: backgroundColor
			}
		]
	}
</script>

<InputValue {id} input={configuration.theme} bind:value={theme} />
<InputValue {id} input={configuration.labels} bind:value={labels} />

<RunnableWrapper autoRefresh bind:componentInput {id} bind:result>
	{#if result}
		<Pie {data} {options} />
	{/if}
</RunnableWrapper>
