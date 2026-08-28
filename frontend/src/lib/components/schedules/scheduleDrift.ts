/**
 * Whether a schedule's runs are taking longer than the gap between its slots.
 *
 * A plain script schedule queues its next run only once the previous one has
 * completed, so a run that outlasts the interval necessarily pushes the next
 * one to a later slot: the schedule quietly runs less often than its cron says.
 * Schedules that queue the next run as the previous one starts are exempt, and
 * the server says which those are.
 *
 * Reads the runs the schedules page has already loaded, and asks for a few of
 * them so that one slow run is not read as a change of cadence.
 */
const MIN_RUNS = 3

export function scheduleOutlastsItsInterval(schedule: {
	queues_next_run_at_start?: boolean
	enabled?: boolean
	interval_s?: number
	jobs?: Array<{ duration_ms: number }>
}): boolean {
	const { queues_next_run_at_start, enabled, interval_s, jobs } = schedule
	if (queues_next_run_at_start || !enabled || !interval_s || (jobs?.length ?? 0) < MIN_RUNS)
		return false
	const durations = jobs!.map((j) => j.duration_ms).sort((a, b) => a - b)
	return durations[durations.length >> 1] > interval_s * 1000
}
