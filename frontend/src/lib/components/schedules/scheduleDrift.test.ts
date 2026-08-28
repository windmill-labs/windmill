import { describe, it, expect } from 'vitest'
import { runsOutlastingInterval } from './scheduleDrift'

const runs = (duration_ms: number) => Array.from({ length: 5 }, () => ({ duration_ms }))

// Each exemption below is a schedule that is genuinely running less often than
// its cron reads, and is still not something to report. Losing one of them turns
// the badge into noise on a correctly configured schedule.
describe('runsOutlastingInterval', () => {
	it('reports how long runs that outlast the gap between slots are taking', () => {
		expect(runsOutlastingInterval({ enabled: true, interval_s: 20, jobs: runs(50_000) })).toBe(
			50_000
		)
	})

	it('says nothing while the runs still fit', () => {
		expect(
			runsOutlastingInterval({ enabled: true, interval_s: 20, jobs: runs(5_000) })
		).toBeUndefined()
	})

	it('exempts a schedule that queues its next run as the previous one starts', () => {
		expect(
			runsOutlastingInterval({
				enabled: true,
				queues_next_run_at_start: true,
				interval_s: 20,
				jobs: runs(50_000)
			})
		).toBeUndefined()
	})

	it('exempts a disabled schedule, which is not running at all', () => {
		expect(
			runsOutlastingInterval({ enabled: false, interval_s: 20, jobs: runs(50_000) })
		).toBeUndefined()
	})

	it('waits for more than one run before calling it a pattern', () => {
		expect(
			runsOutlastingInterval({
				enabled: true,
				interval_s: 20,
				jobs: [{ duration_ms: 50_000 }]
			})
		).toBeUndefined()
	})
})
