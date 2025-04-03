<script lang="ts">
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

	let loading: boolean = true

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

	let countData: ChartData<'line', Point[], undefined> | undefined = undefined
	let delayData: ChartData<'line', Point[], undefined> | undefined = undefined

	let minDate = new Date()

	let noMetrics = false

	function fillData(
		data: {
			created_at: string
			value: number
		}[],
		zero = 0
	) {
		// fill holes with 0
		const sorted: typeof data = []
		for (const el of [
			...data,
			{
				created_at: new Date().toISOString(),
				value: zero
			}
		]) {
			const last =
				sorted.length > 0 ? new Date(sorted[sorted.length - 1].created_at).getTime() : undefined
			const currentTs = new Date(el.created_at).getTime()
			if (last && currentTs - last > 1000 * 60 * 2) {
				const numElements = Math.floor((currentTs - last) / (1000 * 30))
				for (let i = 1; i < numElements; i++) {
					sorted.push({
						created_at: new Date(last + i * (1000 * 30)).toISOString(),
						value: zero
					})
				}
			}
			sorted.push(el)
		}

		// remove high frequency data points for similar values
		const light: typeof sorted = []
		for (const el of sorted) {
			const last = light.length > 0 ? light[light.length - 1] : undefined
			if (
				!last ||
				Math.abs((el.value - last.value) / last.value) > 0.1 ||
				new Date(el.created_at).getTime() - new Date(last.created_at).getTime() > 1000 * 60 * 15
			) {
				light.push(el)
			}
		}

		return light
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
						data: fillData(m.values).map((v) => ({ x: v.created_at as any, y: v.value }))
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
						data: fillData(m.values, 1).map((v) => ({
							x: v.created_at as any,
							y: v.value
						}))
					}
				})
		}

		minDate = new Date(Math.min(...countData.datasets.map((d) => new Date(d.data[0].x).getTime())))

		loading = false
	}

	loadMetrics()

	let darkMode = false

	$: ChartJS.defaults.color = darkMode ? '#ccc' : '#666'
	$: ChartJS.defaults.borderColor = darkMode ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'
</script>

<DarkModeObserver bind:darkMode />

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
