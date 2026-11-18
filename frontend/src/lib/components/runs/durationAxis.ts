import type { ChartOptions, Scale } from 'chart.js'

export type DurationUnit = 'auto' | 'ms' | 's' | 'min' | 'h'
export type DurationScale = 'log' | 'linear'
type FixedUnit = Exclude<DurationUnit, 'auto'>

export const DURATION_UNITS: readonly DurationUnit[] = ['auto', 'ms', 's', 'min', 'h']
export const DURATION_SCALES: readonly DurationScale[] = ['log', 'linear']

const SECOND = 1_000
const MINUTE = 60 * SECOND
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

const UNIT_MS: Record<FixedUnit, number> = { ms: 1, s: SECOND, min: MINUTE, h: HOUR }
// The runs table prints durations with `msToReadableTime` ("12m 3s"), so Auto's ticks spell
// minutes `m` as well, while a fixed unit's axis title keeps `min`.
const SHORT: Record<FixedUnit, string> = { ms: 'ms', s: 's', min: 'm', h: 'h' }

type Candidate = [ms: number, rank: 0 | 1 | 2]

// Round durations an Auto log axis labels. 10s→1m and 10m→1h are closer than a decade, so when
// labels crowd, the unit boundaries (rank 0) are kept over the 10 and 100 steps (rank 1). Rank 2
// fills in only when fewer than three rank 0 or 1 ticks fall in range.
const AUTO_LOG_TICKS: Candidate[] = [
	[1, 1],
	[2, 2],
	[5, 2],
	[10, 1],
	[20, 2],
	[50, 2],
	[100, 1],
	[200, 2],
	[500, 2],
	[SECOND, 0],
	[2 * SECOND, 2],
	[5 * SECOND, 2],
	[10 * SECOND, 1],
	[20 * SECOND, 2],
	[30 * SECOND, 2],
	[MINUTE, 0],
	[2 * MINUTE, 2],
	[5 * MINUTE, 2],
	[10 * MINUTE, 1],
	[20 * MINUTE, 2],
	[30 * MINUTE, 2],
	[HOUR, 0],
	[2 * HOUR, 2],
	[6 * HOUR, 2],
	[12 * HOUR, 2],
	[DAY, 0],
	[2 * DAY, 2],
	[7 * DAY, 2]
]

const AUTO_LINEAR_STEPS: Record<FixedUnit, number[]> = {
	ms: [1, 2, 5, 10, 20, 25, 50, 100, 200, 250, 500],
	s: [1, 2, 5, 10, 15, 20, 30, 60],
	min: [1, 2, 5, 10, 15, 20, 30, 60],
	h: [1, 2, 3, 6, 12, 24, 48]
}

const MIN_LABEL_GAP_PX = 18
const LINEAR_TICK_SPACING_PX = 40
const LOG_PADDING = 1.4
// chart.js runs the tick hooks once before the axis has a height.
const FALLBACK_HEIGHT_PX = 140

const round2 = (n: number) => String(Math.round(n * 100) / 100)

export function formatAutoDuration(ms: number): string {
	if (ms < SECOND) return `${round2(ms)}ms`
	if (ms < MINUTE) return `${round2(ms / SECOND)}s`
	if (ms < HOUR) return `${round2(ms / MINUTE)}m`
	if (ms < DAY) return `${round2(ms / HOUR)}h`
	return `${round2(ms / DAY)}d`
}

function formatInUnit(ms: number, unit: FixedUnit): string {
	return (ms / UNIT_MS[unit]).toLocaleString(undefined, { maximumFractionDigits: 6 })
}

function logCandidates(unit: DurationUnit): Candidate[] {
	if (unit === 'auto') return AUTO_LOG_TICKS
	const out: Candidate[] = []
	for (let k = -6; k <= 9; k++) {
		const p = UNIT_MS[unit] * 10 ** k
		out.push([p, 1], [2 * p, 2], [5 * p, 2])
	}
	return out
}

/** The axis bounds for the visible runs, padded so no dot sits on an edge. */
export function logRange(unit: DurationUnit, minMs: number, maxMs: number) {
	let min = Math.max(minMs, 1) / LOG_PADDING
	let max = Math.max(maxMs, minMs, 1) * LOG_PADDING
	const values = logCandidates(unit).map(([v]) => v)
	// Runs of similar length can leave no round duration in range at all: widen to the round
	// durations around them so the axis keeps at least two labels.
	if (values.filter((v) => v >= min && v <= max).length < 2) {
		min = values.filter((v) => v <= min).at(-1) ?? min
		max = values.find((v) => v >= max) ?? max
	}
	return { min, max }
}

/** Tick values in ms for a log axis spanning `min`–`max` drawn `heightPx` tall. */
export function logTicks(unit: DurationUnit, min: number, max: number, heightPx: number) {
	const inRange = logCandidates(unit).filter(([v]) => v >= min * 0.999 && v <= max * 1.001)
	const majors = inRange.filter(([, rank]) => rank <= 1)
	const pool = (majors.length >= 3 ? majors : inRange).sort((a, b) => a[1] - b[1] || a[0] - b[0])
	const start = Math.log10(min)
	const span = Math.log10(max) - start || 1
	const px = (v: number) => ((Math.log10(v) - start) / span) * heightPx
	const kept: number[] = []
	for (const [v] of pool) {
		if (kept.every((k) => Math.abs(px(k) - px(v)) >= MIN_LABEL_GAP_PX)) kept.push(v)
	}
	return kept.sort((a, b) => a - b)
}

function nice125(raw: number): number {
	const p = 10 ** Math.floor(Math.log10(raw))
	const f = raw / p
	return (f <= 1 ? 1 : f <= 2 ? 2 : f <= 5 ? 5 : 10) * p
}

/** A linear axis from 0: the unit its labels use, its step and its top, all in ms. */
export function linearLayout(unit: DurationUnit, maxMs: number, heightPx: number) {
	const target = Math.max(2, Math.floor(heightPx / LINEAR_TICK_SPACING_PX))
	const longest = Math.max(maxMs, 1)
	const display: FixedUnit =
		unit !== 'auto'
			? unit
			: longest < SECOND
				? 'ms'
				: longest < 2 * MINUTE
					? 's'
					: longest < 2 * HOUR
						? 'min'
						: 'h'
	const m = longest / UNIT_MS[display]
	const step =
		(unit === 'auto' ? AUTO_LINEAR_STEPS[display].find((s) => m / s <= target) : undefined) ??
		nice125(m / target)
	return {
		unit: display,
		stepMs: step * UNIT_MS[display],
		maxMs: Math.ceil(m / step - 1e-9) * step * UNIT_MS[display]
	}
}

type YScaleOptions = NonNullable<ChartOptions<'scatter'>['scales']>[string]

/** The y axis of the Duration chart for `durations` (ms), in `unit` on a `scale` axis. */
export function durationAxisOptions(
	unit: DurationUnit,
	scale: DurationScale,
	durations: number[]
): YScaleOptions {
	let minMs = Infinity
	let maxMs = 0
	for (const d of durations) {
		if (d > 0 && d < minMs) minMs = d
		if (d > maxMs) maxMs = d
	}
	if (maxMs === 0) {
		minMs = 10
		maxMs = 10 * SECOND
	}
	const title = {
		display: true,
		text: unit === 'auto' ? 'job duration' : `job duration (${unit})`
	}

	if (scale === 'log') {
		const { min, max } = logRange(unit, minMs, maxMs)
		return {
			type: 'logarithmic',
			min,
			max,
			bounds: 'data',
			title,
			afterBuildTicks: (axis: Scale) => {
				axis.ticks = logTicks(unit, min, max, axis.height || FALLBACK_HEIGHT_PX).map((value) => ({
					value
				}))
			},
			ticks: {
				autoSkip: false,
				callback: (value) =>
					unit === 'auto' ? formatAutoDuration(Number(value)) : formatInUnit(Number(value), unit)
			}
		}
	}

	// The step depends on the axis height, which chart.js knows only once it lays the axis out.
	let layout = linearLayout(unit, maxMs, FALLBACK_HEIGHT_PX)
	return {
		type: 'linear',
		bounds: 'data',
		title,
		afterDataLimits: (axis: Scale) => {
			layout = linearLayout(unit, maxMs, axis.height || FALLBACK_HEIGHT_PX)
			axis.min = 0
			axis.max = layout.maxMs
		},
		afterBuildTicks: (axis: Scale) => {
			const ticks: { value: number }[] = []
			for (let i = 0; i * layout.stepMs <= layout.maxMs * (1 + 1e-9); i++) {
				ticks.push({ value: i * layout.stepMs })
			}
			axis.ticks = ticks
		},
		ticks: {
			autoSkip: false,
			callback: (value) => {
				const v = Number(value)
				if (v === 0) return '0'
				return unit === 'auto'
					? `${round2(v / UNIT_MS[layout.unit])}${SHORT[layout.unit]}`
					: formatInUnit(v, unit)
			}
		}
	}
}
