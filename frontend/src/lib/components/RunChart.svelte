<script lang="ts">
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
		LogarithmicScale,
		type ChartOptions
	} from 'chart.js'
	import type { CompletedJob } from '$lib/gen'
	import { getDbClockNow } from '$lib/forLater'
	import { Scatter } from '$lib/components/chartjs-wrappers/chartJs'
	import DarkModeObserver from './DarkModeObserver.svelte'

	interface Props {
		jobs?: CompletedJob[] | undefined
		maxIsNow?: boolean
		minTimeSet?: string | null
		maxTimeSet?: string | null
		selectedIds?: string[]
		canSelect?: boolean
		onPointClicked: (ids: string[]) => void
		onZoom: (zoom: { min: Date; max: Date }) => void
	}

	let {
		jobs = [],
		maxIsNow = false,
		minTimeSet = null,
		maxTimeSet = null,
		selectedIds = $bindable([]),
		canSelect = true,
		onPointClicked,
		onZoom
	}: Props = $props()

	const SUCCESS_COLOR = '#4ade80'
	// const SUCCESS_COLOR_TRANSPARENT = '#c9b638'
	const SUCCESS_COLOR_TRANSPARENT = $derived(mergeColors(SUCCESS_COLOR, getBackgroundColor(), 0.8))
	const FAIL_COLOR = '#f87171'
	const FAIL_COLOR_TRANSPARENT = $derived(mergeColors(FAIL_COLOR, getBackgroundColor(), 0.8))

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

	let darkMode = $state(false)
	$effect(() => {
		ChartJS.defaults.color = darkMode ? '#ccc' : '#666'
		ChartJS.defaults.borderColor = darkMode ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'
	})

	function getBackgroundColor(): string {
		return darkMode ? '#2e3440' : '#ffffff'
	}
	function hexToRgb(hexColor: string): number[] {
		hexColor = hexColor.replace(/^#/, '')

		const r = parseInt(hexColor.substring(0, 2), 16)
		const g = parseInt(hexColor.substring(2, 4), 16)
		const b = parseInt(hexColor.substring(4, 6), 16)

		return [r, g, b]
	}

	function rgbToHex(rgbColor: number[]): string {
		// Convert RGB components to hexadecimal string
		return (
			'#' +
			rgbColor
				.map((c) => {
					const hex = c.toString(16)
					return hex.length === 1 ? '0' + hex : hex
				})
				.join('')
		)
	}

	function mergeColors(color1: string, color2: string, slider: number): string {
		const rgb1 = hexToRgb(color1)
		const rgb2 = hexToRgb(color2)

		// Blend colors based on percentage
		const blendedRgb = rgb1.map((c1, i) => Math.round(c1 * (1 - slider) + rgb2[i] * slider))

		return rgbToHex(blendedRgb)
	}

	function getPath(x: any): string {
		return x.path
	}

	function minJobTime(jobs: CompletedJob[]): Date {
		let min: Date = new Date(jobs[0].completed_at!)
		for (const job of jobs) {
			if (job.completed_at != undefined) {
				const date = new Date(job.completed_at)
				if (date < min) {
					min = date
				}
			}
		}
		return min
	}

	function maxJobTime(jobs: CompletedJob[]): Date {
		let max: Date = new Date(jobs[0].completed_at!)
		for (const job of jobs) {
			if (new Date(job.completed_at!) > max) {
				max = new Date(job.completed_at!)
			}
		}
		return max
	}

	function computeMinMaxTime(
		jobs: CompletedJob[] | undefined,
		minTimeSet: string | null,
		maxTimeSet: string | null
	) {
		let minTimeSetDate = minTimeSet ? new Date(minTimeSet) : undefined
		let maxTimeSetDate = maxTimeSet ? new Date(maxTimeSet) : undefined
		if (minTimeSetDate && maxTimeSetDate) {
			return { minTime: minTimeSetDate, maxTime: maxTimeSetDate }
		}

		if (jobs == undefined || jobs?.length == 0) {
			const computedMinTime = minTimeSetDate ?? addSeconds(new Date(), -300)
			const computedMaxTime = maxTimeSetDate ?? getDbClockNow()
			return { minTime: computedMinTime, maxTime: computedMaxTime }
		}

		const maxJob = maxIsNow ? getDbClockNow() : maxJobTime(jobs)
		const minJob = minJobTime(jobs)

		const diff = (maxJob.getTime() - minJob.getTime()) / 20000

		let computedMinTime = minTimeSetDate ?? addSeconds(minJob, -diff)
		let computedMaxTime = maxIsNow
			? (maxTimeSetDate ?? maxJob)
			: (maxTimeSetDate ?? addSeconds(maxJob, diff))
		return { minTime: computedMinTime, maxTime: computedMaxTime }
	}

	function addSeconds(date: Date, seconds: number): Date {
		date.setTime(date.getTime() + seconds * 1000)
		return date
	}

	let success = $derived(jobs?.filter((x) => x.success))
	let failed = $derived(jobs?.filter((x) => !x.success))
	let data = $derived.by(() => {
		const data = {
			datasets: [
				{
					borderColor: 'rgba(99,0,125, 0)',
					backgroundColor: FAIL_COLOR as string | string[],
					radius: 3,
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
					borderColor: 'rgba(99,0,125, 0)',
					backgroundColor: SUCCESS_COLOR as string | string[],
					radius: 3,
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
		if (!canSelect || selectedIds.length === 0) {
			data.datasets[0].backgroundColor = FAIL_COLOR
			data.datasets[1].backgroundColor = SUCCESS_COLOR
		} else {
			data.datasets[0].backgroundColor = data.datasets[0].data.map((p) =>
				selectedIds.includes(p.id) ? FAIL_COLOR : FAIL_COLOR_TRANSPARENT
			)
			data.datasets[1].backgroundColor = data.datasets[1].data.map((p) =>
				selectedIds.includes(p.id) ? SUCCESS_COLOR : SUCCESS_COLOR_TRANSPARENT
			)
		}
		return data
	})

	const minMaxTime = $derived.by(() => computeMinMaxTime(jobs, minTimeSet, maxTimeSet))

	let scatterOptions: ChartOptions<'scatter'> = $derived({
		responsive: true,
		maintainAspectRatio: false,
		plugins: {
			zoom: zoomOptions,
			legend: {
				display: false
			},
			tooltip: {
				callbacks: {
					label: function (context: any) {
						return getPath(context.raw)
					}
				}
			}
		},
		onClick: (_e: any, u: any) => {
			if (canSelect) {
				const ids = u.map((j: any) => data.datasets[j.datasetIndex].data[j.index].id)
				selectedIds = ids
				onPointClicked(ids)
			}
		},

		scales: {
			x: {
				type: 'time',
				grid: {
					display: false
				},
				min: minMaxTime.minTime.getTime(),
				max: minMaxTime.maxTime.getTime(),
				ticks: { maxRotation: 0, minRotation: 0 }
			},
			y: {
				grid: {
					display: false
				},
				title: {
					display: true,
					text: 'job duration (ms)'
				},
				type: 'logarithmic',
				afterFit: function (axis) {
					axis.width = Math.max(axis.width, 65) // min width to prevent layout flickering
				}
			}
		},
		animation: false
	})
</script>

<DarkModeObserver bind:darkMode />

<div class="relative h-44">
	<Scatter {data} options={scatterOptions} />
</div>
