import type { ChartConfiguration } from 'chart.js'
import type { ChartBody } from './displayCatalog'

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

export interface ChartTheme {
	dark: boolean
	/** The colour behind the chart: gaps between adjacent fills are borders drawn in it. */
	surface: string
}

function num(v: unknown): number | null {
	return typeof v === 'number' ? v : null
}

/** The chart.js configuration a catalog chart draws with, on screen and in exports alike. */
export function chartConfig(chart: ChartBody, { dark, surface }: ChartTheme): ChartConfiguration {
	const palette = dark ? DARK : LIGHT
	const ink = dark ? '#c3c2b7' : '#55544f'
	const grid = dark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.07)'
	const ys = typeof chart.y === 'string' ? [chart.y] : chart.y
	const stacked = !!chart.stacked
	const labels = chart.data.map((r) => String(r[chart.x] ?? ''))

	const axis = (label: string | undefined, stack: boolean) => ({
		stacked: stack,
		title: { display: !!label, text: label ?? '', color: ink },
		ticks: { color: ink },
		grid: { color: grid },
		border: { color: grid }
	})
	const common = {
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
	}

	switch (chart.type) {
		case 'bar':
			return {
				type: 'bar',
				data: {
					labels,
					datasets: ys.map((y, i) => ({
						label: y,
						data: chart.data.map((r) => num(r[y])),
						backgroundColor: palette[i],
						borderRadius: 4,
						borderSkipped: stacked ? false : ('start' as const),
						borderColor: surface,
						borderWidth: stacked ? { top: 2 } : 0,
						maxBarThickness: 48
					}))
				},
				options: {
					...common,
					indexAxis: chart.horizontal ? 'y' : 'x',
					scales: chart.horizontal
						? { y: axis(chart.x_label, stacked), x: axis(chart.y_label, stacked) }
						: { x: axis(chart.x_label, stacked), y: axis(chart.y_label, stacked) }
				}
			}
		case 'line':
			return {
				type: 'line',
				data: {
					labels,
					datasets: ys.map((y, i) => ({
						label: y,
						data: chart.data.map((r) => num(r[y])),
						borderColor: palette[i],
						backgroundColor: palette[i],
						borderWidth: 2,
						pointRadius: chart.data.length > 60 ? 0 : 3,
						pointHoverRadius: 5,
						fill: stacked ? 'stack' : false,
						tension: 0
					}))
				},
				options: {
					...common,
					interaction: { mode: 'index', intersect: false },
					scales: { x: axis(chart.x_label, false), y: axis(chart.y_label, stacked) }
				}
			}
		case 'scatter':
			return {
				type: 'scatter',
				data: {
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
				},
				options: {
					...common,
					scales: { x: axis(chart.x_label, false), y: axis(chart.y_label, false) }
				}
			}
		case 'pie': {
			// A pie keeps its largest slices and folds the rest into one, so no slice reuses a hue.
			let slices: { label: string; value: number; other?: boolean }[] = chart.data.map((r) => ({
				label: String(r[chart.x] ?? ''),
				value: num(r[ys[0]]) ?? 0
			}))
			if (slices.length > palette.length) {
				const sorted = [...slices].sort((a, b) => b.value - a.value)
				const rest = sorted.slice(palette.length - 1).reduce((s, x) => s + x.value, 0)
				slices = [
					...sorted.slice(0, palette.length - 1),
					{ label: 'Other', value: rest, other: true }
				]
			}
			return {
				type: 'pie',
				data: {
					labels: slices.map((s) => s.label),
					datasets: [
						{
							label: ys[0],
							data: slices.map((s) => s.value),
							backgroundColor: slices.map((s, i) =>
								s.other ? (dark ? '#6f6e68' : '#9b9a93') : palette[i]
							),
							borderColor: surface,
							borderWidth: 2
						}
					]
				},
				options: common
			}
		}
	}
}
