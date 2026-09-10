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
		type ChartOptions,
		type Point
	} from 'chart.js'
	import { WorkerService } from '$lib/gen'
	import { resource } from 'runed'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { Section } from './common'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'

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

	const WINDOWS = {
		'1h': { secs: 3600, label: 'hour' },
		'24h': { secs: 24 * 3600, label: '24 hours' },
		'7d': { secs: 7 * 24 * 3600, label: '7 days' },
		'14d': { secs: 14 * 24 * 3600, label: '14 days' }
	}
	let windowKey: keyof typeof WINDOWS = $state('24h')

	const metrics = resource(
		() => windowKey,
		async (key, _, { signal }) => {
			const res = await WorkerService.getQueueMetricsSeries({ windowSecs: WINDOWS[key].secs })
			// A slower response for a window no longer selected must not replace the current one.
			signal.throwIfAborted()
			return res
		}
	)

	function datasets(
		kind: 'count' | 'delay',
		toPoint: (vertex: number[]) => Point
	): ChartData<'line', Point[], undefined> {
		const tags = metrics.current?.tags ?? []
		return {
			datasets: tags
				.map((t, i) => ({ t, colors: colorTuples[i % colorTuples.length] }))
				.filter(({ t }) => t[kind].length > 0)
				.map(({ t, colors: [color, bgColor] }) => ({
					label: t.tag,
					borderColor: color,
					backgroundColor: bgColor,
					data: t[kind].map(toPoint)
				}))
		}
	}

	// The server draws each line (a value holds until the next sample, and a spike keeps its
	// height when a slot aggregates many samples), so its vertices are joined as they are.
	const countData = $derived(datasets('count', ([x, y]) => ({ x, y })))
	// Delay is drawn on a log scale, which cannot plot 0, so a drained tag is pinned to 1 and the
	// tooltip reads it back as 0.
	const delayData = $derived(datasets('delay', ([x, y]) => ({ x, y: y === 0 ? 1 : y })))

	function chartOptions(title: string, y: ChartOptions<'line'>['scales']): ChartOptions<'line'> {
		return {
			animation: false,
			elements: { point: { radius: 0, hoverRadius: 4 } },
			interaction: { mode: 'nearest', axis: 'x', intersect: false },
			plugins: { title: { display: true, text: title } },
			scales: {
				x: { type: 'time', min: metrics.current?.from, max: metrics.current?.to },
				...y
			}
		}
	}

	const countOptions = $derived(
		chartOptions('Number of delayed jobs per tag (> 3s)', {
			y: { title: { display: true, text: 'count' } }
		})
	)

	const delayOptions = $derived.by(() => {
		const options = chartOptions('Queue delay per tag (> 3s)', {
			y: {
				type: 'logarithmic',
				title: { display: true, text: 'delay (s)' },
				ticks: { callback: (value) => (value === 1 ? '0' : value) }
			}
		})
		options.plugins!.tooltip = {
			callbacks: {
				label: (context) => {
					const y = (context.raw as Point).y
					return `${context.dataset.label}: ${y === 1 ? 0 : y}`
				}
			}
		}
		return options
	})

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
	{#snippet action()}
		<ToggleButtonGroup bind:selected={windowKey} noWFull>
			{#snippet children({ item })}
				{#each Object.keys(WINDOWS) as key (key)}
					<ToggleButton value={key} label={key} size="sm" {item} />
				{/each}
			{/snippet}
		</ToggleButtonGroup>
	{/snippet}

	{#if metrics.error}
		<Alert type="error" title="Failed to load the queue metrics">{metrics.error.message}</Alert>
	{:else if metrics.current === undefined}
		<Skeleton layout={[[20]]} />
	{:else if metrics.current.tags.length === 0}
		<p class="text-secondary text-xs">
			No jobs delayed by more than 3 seconds in the last {WINDOWS[windowKey].label}
		</p>
	{:else}
		<div class="flex flex-col gap-4">
			<Line data={countData} options={countOptions} />
			<Line data={delayData} options={delayOptions} />
			<Alert title="Info">
				Only tags with jobs delayed by more than 3 seconds in this window are included. At wide
				windows a line shows the highest value of each time slot, so short spikes stay visible.
			</Alert>
		</div>
	{/if}
</Section>
