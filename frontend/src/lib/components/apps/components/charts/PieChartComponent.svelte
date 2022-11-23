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
		ArcElement
	)

	let options = {
		responsive: true
	}

	let result: ChartData<'pie', number[], unknown> | undefined = undefined
</script>

<RunnableComponent {id} {path} {runType} {inlineScriptName} bind:inputs bind:result>
	{#if result}
		<Pie data={result} {options} />
	{/if}
</RunnableComponent>
