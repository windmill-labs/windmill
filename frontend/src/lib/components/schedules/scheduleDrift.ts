const MIN_RUNS = 3

/** What the schedules page has already loaded about how a schedule is running. */
export type ScheduleRunsSample = {
	queues_next_run_at_start?: boolean
	enabled?: boolean
	interval_s?: number
	jobs?: Array<{ duration_ms: number }>
}

/**
 * How long a schedule's runs have been taking, when that is longer than the gap
 * between its slots, and `undefined` otherwise.
 *
 * A plain script schedule queues its next run only once the previous one has
 * completed, so a run that outlasts the interval necessarily pushes the next one
 * to a later slot: the schedule quietly runs less often than its cron says.
 * Schedules that queue the next run as the previous one starts are exempt, and
 * the server says which those are.
 */
export function runsOutlastingInterval(sample: ScheduleRunsSample): number | undefined {
	const { queues_next_run_at_start, enabled, interval_s, jobs } = sample
	if (queues_next_run_at_start || !enabled || !interval_s || (jobs?.length ?? 0) < MIN_RUNS) {
		return undefined
	}
	const durations = jobs!.map((j) => j.duration_ms).sort((a, b) => a - b)
	const median = durations[durations.length >> 1]
	return median > interval_s * 1000 ? median : undefined
}
