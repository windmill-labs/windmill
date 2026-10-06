import { describe, it, expect } from 'vitest'
import { CancelablePromise, type Job } from '$lib/gen'
import { scanJobWindows, type JobWindow } from './jobsWindowScan'

const HOUR = 60 * 60 * 1000
const NOW = Date.parse('2026-01-10T00:00:00.000Z')

function job(at: number): Job {
	return { type: 'CompletedJob', id: `j${at}`, created_at: new Date(at).toISOString() } as Job
}

// A history the scan can only read through its windows: `at` are the creation times of the
// matching jobs, and a window wider than `slowOver` never answers.
function history(at: number[], slowOver = Infinity) {
	const windows: JobWindow[] = []
	const fetchWindow = (w: JobWindow) => {
		windows.push(w)
		const lo = Date.parse(w.after)
		const hi = w.before ? Date.parse(w.before) : Infinity
		if (hi - lo > slowOver) return new CancelablePromise<Job[]>(() => {})
		const rows = at
			.filter((t) => t >= lo && t <= hi)
			.sort((a, b) => b - a)
			.slice(0, w.limit)
			.map(job)
		return new CancelablePromise<Job[]>((resolve) => resolve(rows))
	}
	return { windows, fetchWindow }
}

describe('scanJobWindows', () => {
	it('lists every match down to the oldest job, in order, without a gap between windows', async () => {
		const at = [NOW - 2 * HOUR, NOW - 30 * HOUR, NOW - 200 * HOUR]
		const { windows, fetchWindow } = history(at)
		const seen: Job[] = []
		const wentToEnd = await scanJobWindows({
			before: new Date(NOW).toISOString(),
			after: null,
			pageSize: 10,
			oldest: async () => new Date(NOW - 240 * HOUR).toISOString(),
			fetchWindow,
			onWindow: (jobs) => seen.push(...jobs)
		})
		expect(wentToEnd).toBe(true)
		expect(seen.map((j) => j.id)).toEqual(at.map((t) => `j${t}`))
		expect(windows.at(-1)!.after).toBe(new Date(NOW - 240 * HOUR).toISOString())
	})

	it('counts a job created exactly on a window bound once', async () => {
		// The first window is an hour wide, so its lower bound is the first job's creation time.
		const at = [NOW - HOUR, NOW - 2 * HOUR, NOW - 100 * HOUR]
		const { fetchWindow } = history(at)
		const seen: Job[] = []
		const wentToEnd = await scanJobWindows({
			before: new Date(NOW).toISOString(),
			after: new Date(NOW - 240 * HOUR).toISOString(),
			pageSize: 2,
			oldest: async () => undefined,
			fetchWindow,
			onWindow: (jobs) => seen.push(...jobs)
		})
		expect(wentToEnd).toBe(false)
		expect(seen.map((j) => j.id)).toEqual([`j${NOW - HOUR}`, `j${NOW - 2 * HOUR}`])
	})

	it('still lists the window of a cursor that sits on the oldest job', async () => {
		const oldest = NOW - 240 * HOUR
		const { windows, fetchWindow } = history([oldest])
		const seen: Job[] = []
		const before = new Date(oldest).toISOString().replace('Z', '500Z')
		const wentToEnd = await scanJobWindows({
			before,
			after: null,
			pageSize: 10,
			oldest: async () => new Date(oldest).toISOString(),
			fetchWindow,
			onWindow: (jobs) => seen.push(...jobs)
		})
		expect(wentToEnd).toBe(true)
		expect(windows).toEqual([{ before, after: new Date(oldest).toISOString(), limit: 10 }])
		expect(seen.map((j) => j.id)).toEqual([`j${oldest}`])
	})

	it('stops as soon as the page is full, asking each window only for what is missing', async () => {
		const at = [NOW - 0.5 * HOUR, NOW - 2 * HOUR, NOW - 3 * HOUR, NOW - 100 * HOUR]
		const { windows, fetchWindow } = history(at)
		const seen: Job[] = []
		const wentToEnd = await scanJobWindows({
			before: new Date(NOW).toISOString(),
			after: new Date(NOW - 240 * HOUR).toISOString(),
			pageSize: 3,
			oldest: async () => undefined,
			fetchWindow,
			onWindow: (jobs) => seen.push(...jobs)
		})
		expect(wentToEnd).toBe(false)
		expect(seen).toHaveLength(3)
		expect(windows.map((w) => w.limit)).toEqual([3, 2])
	})

	it('retries a window that does not answer in time with a narrower one', async () => {
		const at = [NOW - 0.2 * HOUR, NOW - 0.9 * HOUR]
		const { windows, fetchWindow } = history(at, 0.5 * HOUR)
		const seen: Job[] = []
		const wentToEnd = await scanJobWindows({
			before: new Date(NOW).toISOString(),
			after: new Date(NOW - HOUR).toISOString(),
			pageSize: 10,
			oldest: async () => undefined,
			fetchWindow,
			onWindow: (jobs) => seen.push(...jobs),
			patienceMs: 5
		})
		expect(wentToEnd).toBe(true)
		expect(seen.map((j) => j.id)).toEqual(at.map((t) => `j${t}`))
		expect(windows.length).toBeGreaterThan(2)
	})
})
