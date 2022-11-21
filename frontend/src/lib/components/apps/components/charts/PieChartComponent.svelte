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
	import ComponentInputValue from '../helpers/ComponentInputValue.svelte'
	import type { ComponentInputsSpec } from '../../types'

	export let componentInputs: ComponentInputsSpec
	export const staticOutputs: string[] = []

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

	/*
	let data = {
		labels: ['Red', 'Green', 'Yellow', 'Grey', 'Dark Grey'],
		datasets: [
			{
				data: [300, 50, 100, 40, 120],
				backgroundColor: ['#F7464A', '#46BFBD', '#FDB45C', '#949FB1', '#4D5360', '#AC64AD'],
				hoverBackgroundColor: ['#FF5A5E', '#5AD3D1', '#FFC870', '#A8B3C5', '#616774', '#DA92DB']
			}
		]
	}
	*/

	let options = {
		responsive: true
	}

	let dataSetValue: Record<string, number> | undefined = undefined

	let data: ChartData<'pie', number[], unknown> = { datasets: [], labels: [] }

	function populateDataSet() {
		if (dataSetValue) {
			Object.keys(dataSetValue).map((key) => {
				// remove string properties
				if (dataSetValue && typeof dataSetValue[key] === 'string') {
					delete dataSetValue[key]
				}
			})

			data.datasets = [
				{
					data: Object.values(dataSetValue),
					backgroundColor: ['#F7464A', '#46BFBD', '#FDB45C', '#949FB1', '#4D5360', '#AC64AD'],
					hoverBackgroundColor: ['#FF5A5E', '#5AD3D1', '#FFC870', '#A8B3C5', '#616774', '#DA92DB']
				}
			]
			data.labels = Object.keys(dataSetValue).map((s) =>
				s.replace(/([A-Z]+)*([A-Z][a-z])/g, '$1 $2')
			)
			data = data
		}
	}

	$: dataSetValue && populateDataSet()
</script>

<ComponentInputValue input={componentInputs.dataset} bind:value={dataSetValue} />
{#if data.datasets.length > 0}
	<Pie {data} {options} />
{:else}
	<span>No dataset</span>
{/if}
