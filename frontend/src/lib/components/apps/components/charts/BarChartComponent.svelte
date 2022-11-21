<script lang="ts">
	import type { Schema } from '$lib/common'
	import { emptySchema } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
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

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')
	export const schema: Schema = emptySchema()
	export const staticOutputs: string[] = []

	const data = {
		labels: ['Red', 'Blue', 'Yellow'],
		datasets: [
			{
				label: '% of Votes',
				data: [12, 19, 3],
				backgroundColor: [
					'rgba(255, 134,159,0.4)',
					'rgba(98,  182, 239,0.4)',
					'rgba(255, 218, 128,0.4)'
				],
				borderWidth: 2,
				borderColor: ['rgba(255, 134, 159, 1)', 'rgba(98,  182, 239, 1)', 'rgba(255, 218, 128, 1)']
			}
		]
	}
</script>

{#if $worldStore}
	<Bar {data} options={{ responsive: true }} />
{/if}
