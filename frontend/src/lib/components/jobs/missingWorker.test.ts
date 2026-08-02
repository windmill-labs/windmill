import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'

const getCompletedJobResultMaybe = vi.fn()
const getJob = vi.fn()
const existsWorkersWithTags = vi.fn()

vi.mock('$lib/gen', () => ({
	JobService: {
		getCompletedJobResultMaybe: (...a: unknown[]) => getCompletedJobResultMaybe(...(a as [])),
		getJob: (...a: unknown[]) => getJob(...(a as []))
	},
	WorkerService: {
		existsWorkersWithTags: (...a: unknown[]) => existsWorkersWithTags(...(a as []))
	}
}))

import { pollJobResult } from './utils'
import { NoWorkerForTagError } from './missingWorker'

beforeEach(() => {
	getCompletedJobResultMaybe.mockReset()
	getJob.mockReset()
	existsWorkersWithTags.mockReset()
	getCompletedJobResultMaybe.mockResolvedValue({ completed: false })
	getJob.mockResolvedValue({ type: 'QueuedJob', running: false, tag: 'postgresql' })
	vi.useFakeTimers()
})

afterEach(() => {
	vi.useRealTimers()
})

describe('pollJobResult', () => {
	it('reports the tag of a queued job no worker serves instead of polling forever', async () => {
		existsWorkersWithTags.mockResolvedValue({ postgresql: false })

		const promise = pollJobResult('job-1', 'ws')
		const rejects = expect(promise).rejects.toBeInstanceOf(NoWorkerForTagError)
		await vi.advanceTimersByTimeAsync(15_000)
		await rejects
	})

	it('keeps waiting on a job queued behind a busy worker that serves its tag', async () => {
		existsWorkersWithTags.mockResolvedValue({ postgresql: true })

		const promise = pollJobResult('job-1', 'ws')
		let settled = false
		promise.then(
			() => (settled = true),
			() => (settled = true)
		)

		await vi.advanceTimersByTimeAsync(60_000)
		expect(settled).toBe(false)

		getCompletedJobResultMaybe.mockResolvedValue({ completed: true, success: true, result: 42 })
		await vi.advanceTimersByTimeAsync(1_000)
		await expect(promise).resolves.toBe(42)
	})
})
