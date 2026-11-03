import { logFeatureUsage } from '$lib/utils/featureUsage'
import type { DurationScale, DurationUnit } from './durationAxis'

// Anonymous counters for the Runs page chart's display controls. The unit, scale and resize
// unions are the whole key vocabulary: nothing about the runs on the chart is logged.

const FEATURE = 'runs_chart'

export type ChartResizeEvent = 'drag' | 'keyboard' | 'reset'

export function logDurationUnit(unit: DurationUnit) {
	logFeatureUsage(FEATURE, 'duration_unit', { key: unit })
}

export function logDurationScale(scale: DurationScale) {
	logFeatureUsage(FEATURE, 'duration_scale', { key: scale })
}

export function logChartResize(event: ChartResizeEvent) {
	logFeatureUsage(FEATURE, 'resize', { key: event })
}
