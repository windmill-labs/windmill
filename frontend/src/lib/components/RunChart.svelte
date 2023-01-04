<script lang="ts">
	import { Scatter } from 'svelte-chartjs'
	import 'chartjs-adapter-date-fns'
	import zoomPlugin from 'chartjs-plugin-zoom'
	import {
		Chart as ChartJS,
		Title,
		Tooltip,
		Legend,
		LineElement,
		CategoryScale,
		LinearScale,
		PointElement,
		TimeScale,
		LogarithmicScale
	} from 'chart.js'
	import type { CompletedJob } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'

	export let jobs: CompletedJob[] | undefined = []

	const dispatch = createEventDispatcher()

	$: success = jobs?.filter((x) => x.success)
	$: failed = jobs?.filter((x) => !x.success)
	ChartJS.register(
		Title,
		Tooltip,
		Legend,
		zoomPlugin,
		LineElement,
		CategoryScale,
		LinearScale,
		LogarithmicScale,
		PointElement,
		TimeScale
	)

	$: data = {
		labels: ['Duration'],
		datasets: [
			{
				// borderColor: 'rgba(99,0,125, .2)',
				backgroundColor: '#f87171',
				label: 'Failed',
				data:
					failed?.map((job) => ({
						x: job.created_at as any,
						y: job.duration_ms,
						id: job.id,
						path: job.script_path
					})) ?? []
			},
			{
				// borderColor: 'rgba(99,0,125, .2)',
				backgroundColor: '#4ade80',
				label: 'Successful',
				data:
					success?.map((job) => ({
						x: job.created_at as any,
						y: job.duration_ms,
						id: job.id,
						path: job.script_path
					})) ?? []
			}
		]
	}

	const zoomOptions = {
		pan: {
			enabled: true,
			modifierKey: 'ctrl' as 'ctrl',
			onPanComplete: ({ chart }) => {
				dispatch('zoom', { min: new Date(chart.scales.x.min), max: new Date(chart.scales.x.max) })
			}
		},
		zoom: {
			drag: {
				enabled: true
			},
			mode: 'x' as 'x',
			onZoom: ({ chart }) => {
				dispatch('zoom', { min: new Date(chart.scales.x.min), max: new Date(chart.scales.x.max) })
			}
		}
	}

	function getPath(x: any): string {
		return x.path
	}
</script>

<Scatter
	{data}
	options={{
		responsive: true,
		maintainAspectRatio: false,
		plugins: {
			zoom: zoomOptions,
			legend: {
				display: false
			},
			tooltip: {
				callbacks: {
					label: function (context) {
						return getPath(context.raw)
					}
				}
			}
		},

		scales: {
			x: {
				grid: {
					display: false
				},
				type: 'time',
				min: jobs?.[jobs?.length - 1]?.created_at ?? new Date().toString()
			},
			y: {
				grid: {
					display: false
				},
				title: {
					display: true,
					text: 'job duration (ms)'
				},
				type: 'logarithmic'
			}
		},
		animation: false
	}}
/>
