<script lang="ts">
	import { Drawer, DrawerContent } from './common'
	import 'chartjs-adapter-date-fns'
	import { Line } from 'svelte-chartjs'

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
	import { onDestroy } from 'svelte'
	import { superadmin } from '$lib/stores'
	export let drawer: Drawer

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
		zero: number = 0
	) {
		let last = -1
		const newElements: typeof data = []
		for (const el of [
			...data,
			{
				created_at: new Date().toISOString(),
				value: zero
			}
		]) {
			const currentTs = new Date(el.created_at).getTime()
			if (last > -1 && currentTs - last > 1000 * 60 * 2) {
				const numElements = Math.floor((currentTs - last) / (1000 * 30))
				for (let i = 1; i < numElements; i++) {
					newElements.push({
						created_at: new Date(last + i * (1000 * 30)).toISOString(),
						value: zero
					})
				}
			}
			last = currentTs
		}

		return [...data, ...newElements].sort(
			(a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
		)
	}

	async function loadMetrics() {
		let metrics = await WorkerService.getQueueMetrics()

		if (metrics.length == 0) {
			noMetrics = true
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
						data: fillData(m.values, 1).map((v) => ({ x: v.created_at as any, y: v.value }))
					}
				})
		}

		minDate = new Date(Math.min(...countData.datasets.map((d) => new Date(d.data[0].x).getTime())))
	}

	let interval: NodeJS.Timeout | undefined = undefined

	$: if ($superadmin) {
		loadMetrics()
		if (interval) {
			clearInterval(interval)
		}
		interval = setInterval(loadMetrics, 35000)
	} else {
		if (interval) {
			clearInterval(interval)
		}
	}

	onDestroy(() => {
		clearInterval(interval)
	})
</script>

<Drawer bind:this={drawer}>
	<DrawerContent title="Queue Metrics" on:close={drawer.closeDrawer}>
		{#if noMetrics}
			<p class="text-secondary">No jobs delayed by more than 3 seconds in the last 14 days</p>
		{:else}
			<div class="flex flex-col gap-4">
				{#if countData}
					<Line
						data={countData}
						options={{
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
			</div>
		{/if}
	</DrawerContent>
</Drawer>
