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
		responsive: true,
		animation: false
	}

	let nextColor = 0

	// TODO: Replace with nicer windmill branded color pallet.
	const colors = ['#3b82f6', '#ff6384', '#4bc0c0', '#ff9f40', '#9966ff', '#ffcd56', '#c9cbcf']

	function generateColor() {
		const col = colors[nextColor]
		nextColor = (nextColor + 1) % colors.length
		return col
	}

	let result: { name: string; value: number; color: string | undefined }[] | undefined = undefined
	let data: ChartData<'pie', number[], string> | undefined = undefined

	$: if (Array.isArray(result)) {
		nextColor = 0
		data = {
			datasets: [
				{
					data: result.map((x) => x.value),
					backgroundColor: result.map((x) => x.color ?? generateColor())
				}
			],
			labels: result.map((x) => x.name)
		}
	} else {
		data = undefined
	}
</script>

<RunnableComponent {id} {path} {runType} {inlineScriptName} bind:inputs bind:result>
	{#if result}
		<Pie {data} {options} />
	{/if}
</RunnableComponent>
