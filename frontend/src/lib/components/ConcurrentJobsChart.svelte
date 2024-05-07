<script lang="ts">
	import { Scatter } from 'svelte-chartjs'
	import 'chartjs-adapter-date-fns'
	import zoomPlugin from 'chartjs-plugin-zoom'
	import {
		Chart as ChartJS,
		CategoryScale,
		Legend,
		LineElement,
		LinearScale,
		PointElement,
		TimeScale,
		Title,
		Tooltip
	} from 'chart.js'
	import type { ConcurrencyIntervals } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import { getDbClockNow } from '$lib/forLater'

	export let concurrencyIntervals: ConcurrencyIntervals | undefined = undefined
	export let maxIsNow: boolean = false
	export let minTimeSet: string | undefined = undefined
	export let maxTimeSet: string | undefined = undefined

	const dispatch = createEventDispatcher()

	function calculateTimeSeries(concurrencyIntervals: ConcurrencyIntervals): AggregatedInterval[] {
		const timeline = new Map<number, number>() // Holds count of concurrent processes at each moment
		concurrencyIntervals.completed_jobs?.forEach(({ started_at, ended_at }) => {
			if (started_at != undefined && ended_at != undefined) {
				const startTime = new Date(started_at).getTime()
				const endTime = new Date(ended_at).getTime()
				timeline.set(startTime, (timeline.get(startTime) || 0) + 1)
				timeline.set(endTime, (timeline.get(endTime) || 0) - 1)
			}
		})

		concurrencyIntervals.running_jobs.forEach(({ started_at }) => {
			if (started_at != undefined) {
				const startTime = new Date(started_at).getTime()
				timeline.set(startTime, (timeline.get(startTime) || 0) + 1)
			}
		})

		let count = 0
		const result: AggregatedInterval[] = []
		for (const [time, change] of [...timeline.entries()].sort(
			([time1], [time2]) => time1 - time2
		)) {
			count += change
			result.push({ time: new Date(time), count } as AggregatedInterval)
		}

		// Add points to continue the line towards the extremities
		if (result.length > 0) {
			let start_time = addSeconds(new Date(result[0].time), -300)
			let start_count = 0
			let end_count = result[result.length - 1].count
			result.unshift({
				time: start_time,
				count: start_count
			} as AggregatedInterval)
			result.push({
				time: new Date(),
				count: end_count
			} as AggregatedInterval)
		}

		return result
	}
	type AggregatedInterval = { time: Date; count: number }

	let intervals: AggregatedInterval[] | undefined = undefined
	$: intervals = concurrencyIntervals ? calculateTimeSeries(concurrencyIntervals) : undefined

	ChartJS.register(
		Title,
		Tooltip,
		Legend,
		zoomPlugin,
		LineElement,
		CategoryScale,
		LinearScale,
		PointElement,
		TimeScale
	)

	$: data = {
		datasets: [
			{
				borderColor: '#4ade80',
				backgroundColor: '#f8717100',
				pointRadius: 0,
				label: 'running',
				showLine: true,
				stepped: true,
				data:
					intervals?.map((job) => ({
						x: job.time as any,
						y: job.count
						// id: job.id,
						// path: job.script_path
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
	let minTime = addSeconds(new Date(), -300)
	let maxTime = getDbClockNow()

	$: computeMinMaxTime(intervals, minTimeSet, maxTimeSet)

	function minJobTime(intervals: AggregatedInterval[]): Date {
		return intervals[0].time
	}

	function maxJobTime(intervals: AggregatedInterval[]): Date {
		return intervals[intervals?.length - 1].time
	}
	function computeMinMaxTime(
		intervals: AggregatedInterval[] | undefined,
		minTimeSet: string | undefined,
		maxTimeSet: string | undefined
	) {
		let minTimeSetDate = minTimeSet ? new Date(minTimeSet) : undefined
		let maxTimeSetDate = maxTimeSet ? new Date(maxTimeSet) : undefined
		if (minTimeSetDate && maxTimeSetDate) {
			minTime = minTimeSetDate
			maxTime = maxTimeSetDate
			return
		}

		if (intervals == undefined || intervals?.length == 0) {
			minTime = minTimeSetDate ?? addSeconds(new Date(), -300)
			maxTime = maxTimeSetDate ?? getDbClockNow()
			return
		}

		const maxJob = maxIsNow ? getDbClockNow() : maxJobTime(intervals)
		const minJob = minJobTime(intervals)

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

	$: options = {
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
						return context.raw.x
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
					text: 'concurrent jobs'
				}
			}
		},
		animation: false
	} as any
</script>

<div class="relative max-h-40">
	<Scatter {data} {options} />
</div>
