<script lang="ts">
	import 'chartjs-adapter-date-fns'
	import zoomPlugin from 'chartjs-plugin-zoom'
	import Tooltip2 from '$lib/components/Tooltip.svelte'
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
	import Button from './common/button/Button.svelte'
	import { Scatter } from '$lib/components/chartjs-wrappers/chartJs'

	export let jobs: CompletedJob[] | undefined = []
	export let maxIsNow: boolean = false
	export let minTimeSet: string | undefined = undefined
	export let maxTimeSet: string | undefined = undefined
	export let selectedIds: string[] = []
	export let canSelect: boolean = true
	export let lastFetchWentToEnd: boolean = false

	const dispatch = createEventDispatcher()
	const SUCCESS_COLOR = '#4ade80'
	// const SUCCESS_COLOR_TRANSPARENT = '#c9b638'
	const SUCCESS_COLOR_TRANSPARENT = mergeColors(SUCCESS_COLOR, getBackgorundColor(), 0.8)
	const FAIL_COLOR = '#f87171'
	const FAIL_COLOR_TRANSPARENT = mergeColors(FAIL_COLOR, getBackgorundColor(), 0.8)

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

	function isDark(): boolean {
		return document.documentElement.classList.contains('dark')
	}

	ChartJS.defaults.color = isDark() ? '#ccc' : '#666'
	ChartJS.defaults.borderColor = isDark() ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'

	function getBackgorundColor(): string {
		return isDark() ? '#2e3440' : '#ffffff'
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

	function highlightSelectedPoints(ids: string[]) {
		if (!canSelect || ids.length === 0) {
			data.datasets[0].backgroundColor = FAIL_COLOR
			data.datasets[1].backgroundColor = SUCCESS_COLOR
		} else {
			data.datasets[0].backgroundColor = data.datasets[0].data.map((p) =>
				ids.includes(p.id) ? FAIL_COLOR : FAIL_COLOR_TRANSPARENT
			)
			data.datasets[1].backgroundColor = data.datasets[1].data.map((p) =>
				ids.includes(p.id) ? SUCCESS_COLOR : SUCCESS_COLOR_TRANSPARENT
			)
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
			if (job.started_at != undefined) {
				const date = new Date(job.started_at)
				if (date < min) {
					min = date
				}
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
		onClick: (e, u) => {
			if (canSelect) {
				const ids = u.map((j) => data.datasets[j.datasetIndex].data[j.index].id)
				selectedIds = ids
				dispatch('pointClicked', ids)
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

	$: data && scatterOptions && highlightSelectedPoints(selectedIds)
</script>

<!-- {JSON.stringify(minTime)}
{JSON.stringify(maxTime)}

{JSON.stringify(jobs?.map((x) => x.started_at))} -->
<!-- {minTime}
{maxTime} -->
<!-- {JSON.stringify(jobs?.map((x) => x.started_at))} -->
<div class="relative max-h-40">
	{#if !lastFetchWentToEnd}
		<div class="absolute top-[-10px] left-[60px]"
			><Button
				size="xs"
				color="transparent"
				variant="contained"
				on:click={() => dispatch('loadExtra')}
				>Load more <Tooltip2
					>There are more jobs to load but only the first 1000 were fetched</Tooltip2
				></Button
			></div
		>
	{/if}
	<Scatter {data} options={scatterOptions} />
</div>
