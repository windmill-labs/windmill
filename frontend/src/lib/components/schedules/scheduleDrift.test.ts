import { describe, it, expect } from 'vitest'
import { scheduleOutlastsItsInterval } from './scheduleDrift'

const runs = (duration_ms: number) => Array.from({ length: 5 }, () => ({ duration_ms }))

// Each exemption below is a schedule that is genuinely running less often than
// its cron reads, and is still not something to report. Losing one of them turns
// the badge into noise on a correctly configured schedule.
describe('scheduleOutlastsItsInterval', () => {
	it('flags runs that outlast the gap between slots', () => {
		expect(scheduleOutlastsItsInterval({ enabled: true, interval_s: 20, jobs: runs(50_000) })).toBe(
			true
		)
	})

	it('says nothing while the runs still fit', () => {
		expect(scheduleOutlastsItsInterval({ enabled: true, interval_s: 20, jobs: runs(5_000) })).toBe(
			false
		)
	})

	it('exempts a schedule that queues its next run as the previous one starts', () => {
		expect(
			scheduleOutlastsItsInterval({
				enabled: true,
				queues_next_run_at_start: true,
				interval_s: 20,
				jobs: runs(50_000)
			})
		).toBe(false)
	})

	it('exempts a disabled schedule, which is not running at all', () => {
		expect(
			scheduleOutlastsItsInterval({ enabled: false, interval_s: 20, jobs: runs(50_000) })
		).toBe(false)
	})

	it('waits for more than one run before calling it a pattern', () => {
		expect(
			scheduleOutlastsItsInterval({
				enabled: true,
				interval_s: 20,
				jobs: [{ duration_ms: 50_000 }]
			})
		).toBe(false)
	})
})
