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
	import { getDbClockNow } from '$lib/forLater'

	export let jobs: CompletedJob[] | undefined = []
	export let maxIsNow: boolean = false
	export let minTimeSet: string | undefined = undefined
	export let maxTimeSet: string | undefined = undefined

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
						x: job.started_at as any,
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
						x: job.started_at as any,
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
				dispatch('zoom', {
					min: addSeconds(new Date(chart.scales.x.min), -1),
					max: addSeconds(new Date(chart.scales.x.max), 1)
				})
			}
		},
		zoom: {
			drag: {
				enabled: true
			},
			mode: 'x' as 'x',
			onZoom: ({ chart }) => {
				dispatch('zoom', {
					min: addSeconds(new Date(chart.scales.x.min), -1),
					max: addSeconds(new Date(chart.scales.x.max), 1)
				})
			}
		}
	}

	function getPath(x: any): string {
		return x.path
	}

	let minTime = addSeconds(new Date(), -300)
	let maxTime = getDbClockNow()

	$: computeMinMaxTime(jobs, minTimeSet, maxTimeSet)

	function minJobTime(jobs: CompletedJob[]): Date {
		let min: Date = new Date(jobs[0].started_at)
		for (const job of jobs) {
			if (new Date(job.started_at) < min) {
				min = new Date(job.started_at)
			}
		}
		return min
	}

	function maxJobTime(jobs: CompletedJob[]): Date {
		let max: Date = new Date(jobs[0].started_at)
		for (const job of jobs) {
			if (new Date(job.started_at) > max) {
				max = new Date(job.started_at)
			}
		}
		return max
	}
	function computeMinMaxTime(
		jobs: CompletedJob[] | undefined,
		minTimeSet: string | undefined,
		maxTimeSet: string | undefined
	) {
		console.log(minTimeSet, maxTimeSet)
		let minTimeSetDate = minTimeSet ? new Date(minTimeSet) : undefined
		let maxTimeSetDate = maxTimeSet ? new Date(maxTimeSet) : undefined
		if (minTimeSetDate && maxTimeSetDate) {
			minTime = minTimeSetDate
			maxTime = maxTimeSetDate
			return
		}

		if (jobs == undefined || jobs?.length == 0) {
			minTime = minTimeSetDate ?? addSeconds(new Date(), -300)
			maxTime = maxTimeSetDate ?? getDbClockNow()
			return
		}

		const maxJob = maxIsNow ? getDbClockNow() : maxJobTime(jobs)
		const minJob = minJobTime(jobs)

		const diff = (maxJob.getTime() - minJob.getTime()) / 20000

		minTime = minTimeSetDate ?? addSeconds(minJob, -diff)
		if (maxIsNow) {
			maxTime = maxTimeSetDate ?? maxJob
		} else {
			maxTime = maxTimeSetDate ?? addSeconds(maxJob, diff)
		}
	}

	function addSeconds(date: Date, seconds: number): Date {
		date.setTime(date.getTime() + seconds * 1000)
		return date
	}

	$: scatterOptions = {
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
				type: 'time',
				grid: {
					display: false
				},
				min: minTime,
				max: maxTime
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
	} as any
</script>

<!-- {JSON.stringify(minTime)}
{JSON.stringify(maxTime)}

{JSON.stringify(jobs?.map((x) => x.started_at))} -->
<!-- {minTime}
{maxTime} -->
<!-- {JSON.stringify(jobs?.map((x) => x.started_at))} -->
<div class="relative max-h-40">
	<Scatter {data} options={scatterOptions} />
</div>
