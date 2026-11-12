<script lang="ts">
	import type { ChartData, ChartOptions } from 'chart.js'
	import { Bar, Line, Pie, Scatter } from '$lib/components/chartjs-wrappers/chartJs'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'
	import type { ChartBody } from './displayCatalog'

	let { chart }: { chart: ChartBody } = $props()

	const isDarkMode = useIsDarkMode()

	// Categorical slots in fixed order, stepped separately for each surface (validated for
	// colour-vision deficiency). Never cycled: the catalog caps series and pie slices to their count.
	const LIGHT = [
		'#2a78d6',
		'#eb6834',
		'#1baf7a',
		'#eda100',
		'#e87ba4',
		'#008300',
		'#6250d6',
		'#e34948'
	]
	const DARK = [
		'#3987e5',
		'#d95926',
		'#199e70',
		'#c98500',
		'#d55181',
		'#008300',
		'#9085e9',
		'#e66767'
	]
	const OTHER_LIGHT = '#9b9a93'
	const OTHER_DARK = '#6f6e68'

	let palette = $derived(isDarkMode.val ? DARK : LIGHT)
	let ink = $derived(isDarkMode.val ? '#c3c2b7' : '#55544f')
	let grid = $derived(isDarkMode.val ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.07)')

	let ys = $derived(typeof chart.y === 'string' ? [chart.y] : chart.y)

	// The gap between adjacent fills (stacked bars, pie slices) is a border in the colour of
	// whatever surface the chart sits on, which differs between run pages, chat and artifacts.
	let container: HTMLDivElement | undefined = $state()
	let surface = $state('#ffffff')
	$effect(() => {
		void isDarkMode.val
		let el: Element | null = container ?? null
		while (el) {
			const bg = getComputedStyle(el).backgroundColor
			if (bg && bg !== 'transparent' && !/rgba\(.*,\s*0\)$/.test(bg)) {
				surface = bg
				return
			}
			el = el.parentElement
		}
		surface = isDarkMode.val ? '#1a1a19' : '#ffffff'
	})

	function num(v: unknown): number | null {
		return typeof v === 'number' ? v : null
	}

	// A pie keeps its largest slices and folds the rest into one, so no slice reuses a hue.
	let pieSlices = $derived.by(() => {
		if (chart.type !== 'pie') return []
		const slices: { label: string; value: number; other?: boolean }[] = chart.data.map((r) => ({
			label: String(r[chart.x] ?? ''),
			value: num(r[ys[0]]) ?? 0
		}))
		if (slices.length <= palette.length) return slices
		const sorted = [...slices].sort((a, b) => b.value - a.value)
		const kept = sorted.slice(0, palette.length - 1)
		const rest = sorted.slice(palette.length - 1).reduce((s, x) => s + x.value, 0)
		return [...kept, { label: 'Other', value: rest, other: true }]
	})

	let common = $derived({
		responsive: true,
		maintainAspectRatio: false,
		animation: false as const,
		plugins: {
			legend: {
				display: chart.type === 'pie' || ys.length > 1,
				position: 'bottom' as const,
				labels: { color: ink, boxWidth: 10, boxHeight: 10 }
			},
			title: {
				display: !!chart.title,
				text: chart.title ?? '',
				color: ink,
				align: 'start' as const,
				font: { size: 13, weight: 600 as const }
			}
		}
	})

	function axis(label: string | undefined, stacked: boolean) {
		return {
			stacked,
			title: { display: !!label, text: label ?? '', color: ink },
			ticks: { color: ink },
			grid: { color: grid },
			border: { color: grid }
		}
	}

	let labels = $derived(chart.data.map((r) => String(r[chart.x] ?? '')))

	let barData: ChartData<'bar'> = $derived({
		labels,
		datasets: ys.map((y, i) => ({
			label: y,
			data: chart.data.map((r) => num(r[y])),
			backgroundColor: palette[i],
			borderRadius: 4,
			borderSkipped: chart.stacked ? false : ('start' as const),
			borderColor: surface,
			borderWidth: chart.stacked ? { top: 2 } : 0,
			maxBarThickness: 48
		}))
	})

	let barOptions: ChartOptions<'bar'> = $derived({
		...common,
		indexAxis: chart.horizontal ? 'y' : 'x',
		scales: chart.horizontal
			? { y: axis(chart.x_label, !!chart.stacked), x: axis(chart.y_label, !!chart.stacked) }
			: { x: axis(chart.x_label, !!chart.stacked), y: axis(chart.y_label, !!chart.stacked) }
	})

	let lineData: ChartData<'line'> = $derived({
		labels,
		datasets: ys.map((y, i) => ({
			label: y,
			data: chart.data.map((r) => num(r[y])),
			borderColor: palette[i],
			backgroundColor: palette[i],
			borderWidth: 2,
			pointRadius: chart.data.length > 60 ? 0 : 3,
			pointHoverRadius: 5,
			fill: chart.stacked ? 'stack' : false,
			tension: 0
		}))
	})

	let lineOptions: ChartOptions<'line'> = $derived({
		...common,
		interaction: { mode: 'index', intersect: false },
		scales: { x: axis(chart.x_label, false), y: axis(chart.y_label, !!chart.stacked) }
	})

	let scatterData: ChartData<'scatter'> = $derived({
		datasets: ys.map((y, i) => ({
			label: y,
			data: chart.data
				.map((r) => ({ x: num(r[chart.x]), y: num(r[y]) }))
				.filter((p): p is { x: number; y: number } => p.x !== null && p.y !== null),
			backgroundColor: palette[i],
			borderColor: surface,
			borderWidth: 2,
			pointRadius: 5,
			pointHoverRadius: 7
		}))
	})

	let scatterOptions: ChartOptions<'scatter'> = $derived({
		...common,
		scales: { x: axis(chart.x_label, false), y: axis(chart.y_label, false) }
	})

	let pieData: ChartData<'pie'> = $derived({
		labels: pieSlices.map((s) => s.label),
		datasets: [
			{
				label: ys[0],
				data: pieSlices.map((s) => s.value),
				backgroundColor: pieSlices.map((s, i) =>
					s.other ? (isDarkMode.val ? OTHER_DARK : OTHER_LIGHT) : palette[i]
				),
				borderColor: surface,
				borderWidth: 2
			}
		]
	})

	let pieOptions: ChartOptions<'pie'> = $derived(common)
</script>

<div
	bind:this={container}
	class="relative w-full h-72 p-2"
	data-interactive
	data-chart-type={chart.type}
>
	{#if chart.type === 'bar'}
		<Bar data={barData} options={barOptions} />
	{:else if chart.type === 'line'}
		<Line data={lineData} options={lineOptions} />
	{:else if chart.type === 'scatter'}
		<Scatter data={scatterData} options={scatterOptions} />
	{:else}
		<Pie data={pieData} options={pieOptions} />
	{/if}
</div>
