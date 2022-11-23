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

	import type { InputsSpec } from '../../types'
	import RunnableComponent from '../helpers/RunnableComponent.svelte'

	export let id: string
	export let inputs: InputsSpec
	export let path: string | undefined = undefined
	export let runType: 'script' | 'flow' | undefined = undefined
	export let inlineScriptName: string | undefined = undefined

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
</script>

<RunnableComponent {id} {path} {runType} {inlineScriptName} bind:inputs bind:result>
	{#if result}
		<Bar data={result} options={{ responsive: true, animation: false }} />
	{/if}
</RunnableComponent>
