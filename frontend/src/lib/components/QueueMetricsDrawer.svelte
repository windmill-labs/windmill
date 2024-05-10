<script lang="ts">
	import { Drawer, DrawerContent } from './common'
	import 'chartjs-adapter-date-fns'
	import { Scatter } from 'svelte-chartjs'

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
		type Point,
		type ChartData
	} from 'chart.js'
	import { WorkerService } from '$lib/gen'
	import { onDestroy } from 'svelte'
	import { superadmin } from '$lib/stores'
	export let drawer: Drawer

	function randomColorTuple(seedString: string) {
		let seed = 0
		for (let i = 0; i < seedString.length; i++) {
			seed += seedString.charCodeAt(i)
		}
		function random() {
			const x = Math.sin(seed++) * 10000
			console.log(x)
			return x - Math.floor(x)
		}
		console.log('random', random())
		const r = Math.floor(random() * 255)
		const g = Math.floor(random() * 255)
		const b = Math.floor(random() * 255)
		return ['rgb(' + r + ',' + g + ',' + b + ')', 'rgba(' + r + ',' + g + ',' + b + ', .2)']
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

	let countData: ChartData<'scatter', Point[], undefined> | undefined = undefined
	let delayData: ChartData<'scatter', Point[], undefined> | undefined = undefined

	let minDate = new Date()

	let noMetrics = false

	async function loadMetrics() {
		const metrics = await WorkerService.getQueueMetrics()

		if (metrics.length == 0) {
			noMetrics = true
			return
		}

		countData = {
			datasets: metrics
				.filter((m) => m.id.startsWith('queue_count_'))
				.map((m) => {
					const [color, bgColor] = randomColorTuple(m.id.slice(12))
					return {
						label: m.id.slice(12),
						backgroundColor: bgColor,
						borderColor: color,
						data: m.values.map((v) => ({ x: v.created_at, y: v.value as number }))
					}
				})
		}

		delayData = {
			datasets: metrics
				.filter((m) => m.id.startsWith('queue_delay_'))
				.map((m) => {
					const [color, bgColor] = randomColorTuple(m.id.slice(12))
					return {
						label: m.id.slice(12),
						borderColor: color,
						backgroundColor: bgColor,
						data: m.values.map((v) => ({ x: v.created_at, y: v.value as number }))
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
					<Scatter
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
					<Scatter
						data={delayData}
						options={{
							plugins: {
								title: {
									display: true,
									text: 'Queue delay per tag (> 3s)'
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
