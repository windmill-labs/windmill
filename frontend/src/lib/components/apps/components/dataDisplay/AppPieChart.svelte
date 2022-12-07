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
		<Pie {data} {options} />
	{/if}
</RunnableWrapper>
