<script lang="ts">
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
	import type { CompletedJob, ExtendedJobs } from '$lib/gen'
	import { getDbClockNow } from '$lib/forLater'
	import { Line } from '$lib/components/chartjs-wrappers/chartJs'

	interface Props {
		extendedJobs?: ExtendedJobs | undefined
		maxIsNow?: boolean
		minTimeSet?: string | null
		maxTimeSet?: string | null
		onZoom: (zoom: { min: Date; max: Date }) => void
	}

	let {
		extendedJobs = undefined,
		maxIsNow = false,
		minTimeSet = null,
		maxTimeSet = null,
		onZoom
	}: Props = $props()

	function calculateTimeSeries(extendedJobs: ExtendedJobs): AggregatedInterval[] {
		const timeline = new Map<number, { count: number; id_started: string[]; id_ended: string[] }>()

		extendedJobs.jobs.forEach((j) => {
			if (j.started_at != undefined) {
				const startTime = new Date(j.started_at).getTime()
				if (!timeline.has(startTime)) {
					timeline.set(startTime, { count: 0, id_started: [], id_ended: [] })
				}
				const s = timeline.get(startTime)!
				s.count += 1
				s.id_started.push(j.id)
				if (j.type === 'CompletedJob') {
					const jc = j as CompletedJob
					const endTime = startTime + jc.duration_ms
					if (!timeline.has(endTime)) {
						timeline.set(endTime, { count: 0, id_started: [], id_ended: [] })
					}
					const e = timeline.get(endTime)!
					e.count -= 1
					e.id_ended.push(j.id)
				}
			}
		})

		extendedJobs.obscured_jobs.forEach((j) => {
			if (j.started_at != undefined) {
				const startTime = new Date(j.started_at).getTime()
				if (!timeline.has(startTime)) {
					timeline.set(startTime, { count: 0, id_started: [], id_ended: [] })
				}
				const s = timeline.get(startTime)!
				s.count += 1
				s.id_started.push('unknown')
				if (j.duration_ms != undefined) {
					const jc = j as CompletedJob
					const endTime = startTime + jc.duration_ms
					if (!timeline.has(endTime)) {
						timeline.set(endTime, { count: 0, id_started: [], id_ended: [] })
					}
					const e = timeline.get(endTime)!
					e.count -= 1
					e.id_ended.push('unknown')
				}
			}
		})

		let count = 0
		const result: AggregatedInterval[] = []
		for (const [time, change] of [...timeline.entries()].sort(
			([time1], [time2]) => time1 - time2
		)) {
			count += change.count
			let msg = ''
			msg += change.id_started.length != 0 ? `${change.id_started.join(',')} started` : ''
			msg += change.id_started.length != 0 && change.id_ended.length != 0 ? '\n' : ''
			msg += change.id_ended.length != 0 ? `${change.id_ended.join(',')} ended` : ''
			result.push({ time: new Date(time), count, msg } as AggregatedInterval)
		}

		// Add points to continue the line towards the extremities
		if (result.length > 0) {
			let start_time = addSeconds(new Date(result[0].time), -1)
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
	type AggregatedInterval = { time: Date; count: number; msg?: string }

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

	const zoomOptions = {
		pan: {
			enabled: true,
			modifierKey: 'ctrl' as 'ctrl',
			onPanComplete: ({ chart }) => {
				onZoom({
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
				onZoom({
					min: addSeconds(new Date(chart.scales.x.min), -1),
					max: addSeconds(new Date(chart.scales.x.max), 1)
				})
			}
		}
	}

	function minJobTime(intervals: AggregatedInterval[]): Date {
		return intervals[0].time
	}

	function maxJobTime(intervals: AggregatedInterval[]): Date {
		return intervals[intervals?.length - 1].time
	}
	function computeMinMaxTime(
		intervals: AggregatedInterval[] | undefined,
		minTimeSet: string | null,
		maxTimeSet: string | null
	) {
		let minTimeSetDate = minTimeSet ? new Date(minTimeSet) : undefined
		let maxTimeSetDate = maxTimeSet ? new Date(maxTimeSet) : undefined
		if (minTimeSetDate && maxTimeSetDate) {
			return { min: minTimeSetDate, max: maxTimeSetDate }
		}

		if (intervals == undefined || intervals?.length == 0) {
			const minTime = minTimeSetDate ?? addSeconds(new Date(), -300)
			const maxTime = maxTimeSetDate ?? getDbClockNow()
			return { min: minTime, max: maxTime }
		}

		const maxJob = maxIsNow ? getDbClockNow() : maxJobTime(intervals)
		const minJob = minJobTime(intervals)

		const diff = (maxJob.getTime() - minJob.getTime()) / 20000

		const minTime = minTimeSetDate ?? addSeconds(minJob, -diff)
		const maxTime = maxIsNow
			? (maxTimeSetDate ?? maxJob)
			: (maxTimeSetDate ?? addSeconds(maxJob, diff))
		return { min: minTime, max: maxTime }
	}

	function addSeconds(date: Date, seconds: number): Date {
		date.setTime(date.getTime() + seconds * 1000)
		return date
	}

	const intervals = $derived(extendedJobs ? calculateTimeSeries(extendedJobs) : undefined)

	let data = $derived({
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
						y: job.count,
						id: job.msg
					})) ?? []
			}
		]
	})

	const minMaxTimes = $derived(computeMinMaxTime(intervals, minTimeSet, maxTimeSet))

	let options = $derived({
		responsive: true,
		maintainAspectRatio: false,
		plugins: {
			zoom: zoomOptions,
			legend: {
				display: false
			},
			tooltip: {
				callbacks: {
					footer: function (context) {
						return context[context.length - 1].raw.id
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
				min: minMaxTimes.min,
				max: minMaxTimes.max,
				ticks: { maxRotation: 0, minRotation: 0 }
			},
			y: {
				grid: {
					display: false
				},
				title: {
					display: true,
					text: 'concurrent jobs'
				},
				beginAtZero: true,
				ticks: {
					stepSize: 1
				}
			}
		},
		animation: false,
		interaction: {
			intersect: false,
			mode: 'index'
		}
	} as any)
</script>

<div class="relative h-44">
	<Line {data} {options} />
</div>
