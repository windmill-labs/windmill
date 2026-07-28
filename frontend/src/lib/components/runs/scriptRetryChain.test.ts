import { describe, it, expect } from 'vitest'
import type { Job } from '$lib/gen'
import { canSkipRetryChainQuery } from './scriptRetryChain'

// Minimal CompletedJob/QueuedJob factories — canSkipRetryChainQuery only reads
// type/parent_job/success/schedule_path, so the rest is cast away.
function completed(overrides: Partial<Job> = {}): Job {
	return {
		type: 'CompletedJob',
		id: 'j1',
		job_kind: 'script',
		success: true,
		...overrides
	} as Job
}

function queued(overrides: Partial<Job> = {}): Job {
	return { type: 'QueuedJob', id: 'j1', job_kind: 'script', ...overrides } as Job
}

describe('canSkipRetryChainQuery', () => {
	it('skips a successful, non-scheduled, top-level script (nothing to show)', () => {
		expect(canSkipRetryChainQuery(completed())).toBe(true)
	})

	it('does NOT skip a failed script — it may have retry attempts', () => {
		expect(canSkipRetryChainQuery(completed({ success: false }))).toBe(false)
	})

	it('does NOT skip a chain member (parent_job set) — e.g. a successful final retry', () => {
		expect(canSkipRetryChainQuery(completed({ parent_job: 'root' }))).toBe(false)
	})

	it('does NOT skip a schedule-triggered success — it may have a recovery handler', () => {
		expect(canSkipRetryChainQuery(completed({ schedule_path: 'f/s/daily' }))).toBe(false)
	})

	it('does NOT skip a still-running (queued) job — outcome not yet known', () => {
		expect(canSkipRetryChainQuery(queued({ running: true }))).toBe(false)
	})
})
