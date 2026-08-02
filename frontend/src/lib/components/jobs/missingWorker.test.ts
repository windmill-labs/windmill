import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'

const getCompletedJobResultMaybe = vi.fn()
const getJob = vi.fn()
const cancelQueuedJob = vi.fn()
const existsWorkersWithTags = vi.fn()

vi.mock('$lib/gen', () => ({
	JobService: {
		getCompletedJobResultMaybe: (...a: unknown[]) => getCompletedJobResultMaybe(...(a as [])),
		getJob: (...a: unknown[]) => getJob(...(a as [])),
		cancelQueuedJob: (...a: unknown[]) => cancelQueuedJob(...(a as []))
	},
	WorkerService: {
		existsWorkersWithTags: (...a: unknown[]) => existsWorkersWithTags(...(a as []))
	}
}))

import { pollJobResult } from './utils'
import { NoWorkerForTagError } from './missingWorker'

function settlementTracker(promise: Promise<unknown>) {
	const state = { settled: false }
	promise.then(
		() => (state.settled = true),
		() => (state.settled = true)
	)
	return state
}

beforeEach(() => {
	getCompletedJobResultMaybe.mockReset()
	getJob.mockReset()
	cancelQueuedJob.mockReset()
	existsWorkersWithTags.mockReset()
	getCompletedJobResultMaybe.mockResolvedValue({ completed: false })
	getJob.mockResolvedValue({ type: 'QueuedJob', running: false, tag: 'postgresql' })
	cancelQueuedJob.mockResolvedValue(undefined)
	vi.useFakeTimers()
})

afterEach(() => {
	vi.useRealTimers()
})

describe('pollJobResult', () => {
	it('cancels and reports a queued job whose tag stays unserved', async () => {
		existsWorkersWithTags.mockResolvedValue({ postgresql: false })

		const promise = pollJobResult('job-1', 'ws')
		const rejects = expect(promise).rejects.toBeInstanceOf(NoWorkerForTagError)
		await vi.advanceTimersByTimeAsync(30_000)
		await rejects
		expect(cancelQueuedJob).toHaveBeenCalledWith(
			expect.objectContaining({ id: 'job-1', workspace: 'ws' })
		)
	})

	it('does not give up on a single unserved reading, as workers may be scaling up', async () => {
		existsWorkersWithTags.mockResolvedValueOnce({ postgresql: false })
		existsWorkersWithTags.mockResolvedValue({ postgresql: true })

		const promise = pollJobResult('job-1', 'ws')
		const tracker = settlementTracker(promise)

		await vi.advanceTimersByTimeAsync(60_000)
		expect(tracker.settled).toBe(false)
		expect(cancelQueuedJob).not.toHaveBeenCalled()
	})

	it('keeps waiting on a job queued behind a busy worker that serves its tag', async () => {
		existsWorkersWithTags.mockResolvedValue({ postgresql: true })

		const promise = pollJobResult('job-1', 'ws')
		const tracker = settlementTracker(promise)

		await vi.advanceTimersByTimeAsync(60_000)
		expect(tracker.settled).toBe(false)

		getCompletedJobResultMaybe.mockResolvedValue({ completed: true, success: true, result: 42 })
		await vi.advanceTimersByTimeAsync(3_000)
		await expect(promise).resolves.toBe(42)
	})
})
