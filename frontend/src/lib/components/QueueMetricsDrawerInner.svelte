<script lang="ts">
	import { run } from 'svelte/legacy'

	import 'chartjs-adapter-date-fns'
	import { Line } from '$lib/components/chartjs-wrappers/chartJs'

	import {
		Chart as ChartJS,
		Title,
		Tooltip,
		Legend,
		LineElement,
		CategoryScale,
		LinearScale,
		PointElement,
		LogarithmicScale,
		TimeScale,
		type ChartData,
		type Point
	} from 'chart.js'
	import { WorkerService } from '$lib/gen'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { Section } from './common'

	let loading: boolean = $state(true)

	const colorTuples = [
		['#7EB26D', 'rgba(126, 178, 109, 0.2)'],
		['#EAB839', 'rgba(234, 184, 57, 0.2)'],
		['#6ED0E0', 'rgba(110, 208, 224, 0.2)'],
		['#EF843C', 'rgba(239, 132, 60, 0.2)'],
		['#E24D42', 'rgba(226, 77, 66, 0.2)'],
		['#1F78C1', 'rgba(31, 120, 193, 0.2)'],
		['#BA43A9', 'rgba(186, 67, 169, 0.2)'],
		['#705DA0', 'rgba(112, 93, 160, 0.2)'],
		['#508642', 'rgba(80, 134, 66, 0.2)'],
		['#CCA300', 'rgba(204, 163, 0, 0.2)'],
		['#447EBC', 'rgba(68, 126, 188, 0.2)'],
		['#C15C17', 'rgba(193, 92, 23, 0.2)'],
		['#890F02', 'rgba(137, 15, 2, 0.2)'],
		['#666666', 'rgba(102, 102, 102, 0.2)'],
		['#44AA99', 'rgba(68, 170, 153, 0.2)'],
		['#6D8764', 'rgba(109, 135, 100, 0.2)'],
		['#555555', 'rgba(85, 85, 85, 0.2)'],
		['#B3B3B3', 'rgba(179, 179, 179, 0.2)'],
		['#008C9E', 'rgba(0, 140, 158, 0.2)'],
		['#6BBA70', 'rgba(107, 186, 112, 0.2)']
	]

	function getColors(labels: string[]) {
		const colors = labels.map((_, i) => colorTuples[i % colorTuples.length])
		return Object.fromEntries(colors.map((c, i) => [labels[i], c]))
	}

	ChartJS.register(
		Title,
		Tooltip,
		Legend,
		LineElement,
		LinearScale,
		PointElement,
		CategoryScale,
		TimeScale,
		LogarithmicScale
	)

	let countData: ChartData<'line', Point[], undefined> | undefined = $state(undefined)
	let delayData: ChartData<'line', Point[], undefined> | undefined = $state(undefined)

	let minDate = $state(new Date())

	let noMetrics = $state(false)

	// The sampler only records a queue metric when its value moves, plus a heartbeat while a
	// tag stays backlogged and a closing zero once it drains, so a gap means "unchanged". A
	// series silent for longer than the heartbeat allows never got its closing zero (no server
	// was up when the tag drained), so it reads as zero from there on.
	const HEARTBEAT_MS = 5 * 60 * 1000 // QUEUE_METRIC_HEARTBEAT_SECS in backend/src/monitor.rs
	const STALE_AFTER_MS = 2 * HEARTBEAT_MS
	// Hold each value until the next point. `'after'` starts a value at the previous point
	// instead, which draws a whole backlog at the height of the zero that closes it.
	const STEPPED = 'before' as const

	function toPoints(
		data: {
			created_at: string
			value: number
		}[],
		tolerance: number
	): Point[] {
		const points: Point[] = []
		let lastSampleTs: number | undefined
		let lastValue: number | undefined

		function push(x: number, y: number) {
			points.push({ x, y })
			lastValue = y
		}
		function bridge(until: number) {
			if (lastSampleTs != undefined && until - lastSampleTs > STALE_AFTER_MS && lastValue !== 0) {
				push(lastSampleTs + STALE_AFTER_MS, 0)
			}
		}

		for (const el of data) {
			const ts = new Date(el.created_at).getTime()
			bridge(ts)
			// Keep only points that move the line: a heartbeat repeats the value in force, which
			// stepped drawing renders identically.
			if (
				lastValue == undefined ||
				Math.abs(el.value - lastValue) > Math.abs(lastValue) * tolerance
			) {
				push(ts, el.value)
			}
			lastSampleTs = ts
		}

		if (lastSampleTs != undefined) {
			const now = Date.now()
			bridge(now)
			// Carry the value in force to the right edge of the chart.
			push(now, lastValue!)
		}
		return points
	}

	// Delay is drawn on a log scale, which cannot plot 0, so a drained tag is pinned to 1 and
	// the tooltip reads it back as 0.
	function asLogSafe(points: Point[]): Point[] {
		return points.map((p) => ({ x: p.x, y: p.y === 0 ? 1 : p.y }))
	}

	async function loadMetrics() {
		loading = true
		let metrics = await WorkerService.getQueueMetrics()

		if (metrics.length == 0) {
			noMetrics = true
			loading = false
			return
		}

		const labels = metrics
			.map((m) => m.id.slice(12))
			.filter((v, i, a) => a.indexOf(v) === i)
			.sort()
		const labelColors = getColors(labels)

		countData = {
			datasets: metrics
				.filter((m) => m.id.startsWith('queue_count_'))
				.map((m) => {
					const [color, bgColor] = labelColors[m.id.slice(12)]
					return {
						label: m.id.slice(12),
						backgroundColor: bgColor,
						borderColor: color,
						stepped: STEPPED,
						data: toPoints(m.values, 0)
					}
				})
		}

		delayData = {
			datasets: metrics
				.filter((m) => m.id.startsWith('queue_delay_'))
				.map((m) => {
					const [color, bgColor] = labelColors[m.id.slice(12)]
					return {
						label: m.id.slice(12),
						borderColor: color,
						backgroundColor: bgColor,
						stepped: STEPPED,
						// Delay climbs on its own while a tag stays backlogged; the sampler stores a
						// new row only once it has moved by 10%, so hold the chart to the same step.
						data: asLogSafe(toPoints(m.values, 0.1))
					}
				})
		}

		const firstTs = [...countData.datasets, ...delayData.datasets]
			.map((d) => d.data[0]?.x)
			.filter((x) => x != undefined)
		minDate = firstTs.length > 0 ? new Date(Math.min(...firstTs)) : new Date()

		loading = false
	}

	loadMetrics()

	let darkMode = $state(false)

	run(() => {
		ChartJS.defaults.color = darkMode ? '#ccc' : '#666'
	})
	run(() => {
		ChartJS.defaults.borderColor = darkMode ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'
	})
</script>

<DarkModeObserver bind:darkMode />

<Section label="Queue metrics">
	{#if loading}
		<Skeleton layout={[[20]]} />
	{:else if noMetrics}
		<p class="text-secondary">No jobs delayed by more than 3 seconds in the last 14 days</p>
	{:else}
		<div class="flex flex-col gap-4">
			{#if countData}
				<Line
					data={countData}
					options={{
						animation: false,
						plugins: {
							title: {
								display: true,
								text: 'Number of delayed jobs per tag (> 3s)'
							}
						},
						scales: {
							x: {
								type: 'time',
								min: minDate.toISOString(),
								max: new Date().toISOString()
							},
							y: {
								title: {
									display: true,
									text: 'count'
								}
							}
						}
					}}
				/>
			{/if}
			{#if delayData}
				<Line
					data={delayData}
					options={{
						animation: false,
						plugins: {
							title: {
								display: true,
								text: 'Queue delay per tag (> 3s)'
							},
							tooltip: {
								callbacks: {
									label: function (context) {
										// @ts-ignore
										if (context.raw.y === 1) {
											return context.dataset.label + ': 0'
										} else {
											// @ts-ignore
											return context.dataset.label + ': ' + context.raw.y
										}
									}
								}
							}
						},
						scales: {
							x: {
								type: 'time',
								min: minDate.toISOString(),
								max: new Date().toISOString()
							},

							y: {
								type: 'logarithmic',
								title: {
									display: true,
									text: 'delay (s)'
								},
								ticks: {
									callback: (value, _) => (value === 1 ? '0' : value)
								}
							}
						}
					}}
				/>
			{/if}
			<Alert title="Info">
				Only tags for jobs that have been delayed by more than 3 seconds in the last 14 days are
				included in the graph.
			</Alert>
		</div>
	{/if}
</Section>
