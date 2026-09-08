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
import { hasWorkerForTag, NoWorkerForTagError } from './missingWorker'

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

// Long enough for the whole confirmation window (first probe + 2 intervals).
const PAST_CONFIRMATION_WINDOW_MS = 120_000

describe('pollJobResult', () => {
	it('reports a queued read whose tag stays unserved without cancelling it', async () => {
		existsWorkersWithTags.mockResolvedValue({ postgresql: false })

		const promise = pollJobResult('job-1', 'ws')
		const rejects = expect(promise).rejects.toBeInstanceOf(NoWorkerForTagError)
		await vi.advanceTimersByTimeAsync(PAST_CONFIRMATION_WINDOW_MS)
		await rejects
		// The backlog is what the autoscaler scales up on: cancelling would stop a
		// group coming back from zero from ever recovering.
		expect(cancelQueuedJob).not.toHaveBeenCalled()
	})

	it('never abandons a write, and reports once why it is waiting', async () => {
		existsWorkersWithTags.mockResolvedValue({ postgresql: false })
		const onNoWorkerForTag = vi.fn()

		const promise = pollJobResult('job-1', 'ws', { sideEffecting: true, onNoWorkerForTag })
		const tracker = settlementTracker(promise)

		await vi.advanceTimersByTimeAsync(PAST_CONFIRMATION_WINDOW_MS * 3)
		// Reporting failure while the write stays executable would let it apply after
		// the caller gave up and duplicate on retry; cancelling it first cannot be
		// done atomically from the client.
		expect(tracker.settled).toBe(false)
		expect(cancelQueuedJob).not.toHaveBeenCalled()
		expect(onNoWorkerForTag).toHaveBeenCalledTimes(1)
		expect(onNoWorkerForTag).toHaveBeenCalledWith('postgresql')

		getCompletedJobResultMaybe.mockResolvedValue({ completed: true, success: true, result: 7 })
		await vi.advanceTimersByTimeAsync(3_000)
		await expect(promise).resolves.toBe(7)
	})

	it('does not give up while a worker group could still be coming up', async () => {
		// A worker group booting is absent from worker_ping exactly like an unserved
		// tag; only a run of empty lookups distinguishes them.
		existsWorkersWithTags.mockResolvedValueOnce({ postgresql: false })
		existsWorkersWithTags.mockResolvedValueOnce({ postgresql: false })
		existsWorkersWithTags.mockResolvedValue({ postgresql: true })

		const promise = pollJobResult('job-1', 'ws')
		const tracker = settlementTracker(promise)

		await vi.advanceTimersByTimeAsync(PAST_CONFIRMATION_WINDOW_MS)
		expect(tracker.settled).toBe(false)
		expect(cancelQueuedJob).not.toHaveBeenCalled()
	})

	it('keeps waiting on a job queued behind a busy worker that serves its tag', async () => {
		existsWorkersWithTags.mockResolvedValue({ postgresql: true })

		const promise = pollJobResult('job-1', 'ws')
		const tracker = settlementTracker(promise)

		await vi.advanceTimersByTimeAsync(PAST_CONFIRMATION_WINDOW_MS)
		expect(tracker.settled).toBe(false)

		getCompletedJobResultMaybe.mockResolvedValue({ completed: true, success: true, result: 42 })
		await vi.advanceTimersByTimeAsync(3_000)
		await expect(promise).resolves.toBe(42)
	})
})

describe('hasWorkerForTag', () => {
	it('treats an answer it did not get as a worker being there', async () => {
		// `existsWorkersWithTags` returns an empty map when TAGS_ARE_SENSITIVE hides
		// the tag from the caller. Reading that as "unserved" would diagnose a
		// perfectly healthy instance.
		existsWorkersWithTags.mockResolvedValue({})
		await expect(hasWorkerForTag('ws', 'postgresql')).resolves.toBe(true)
	})
})
