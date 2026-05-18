import { describe, it, expect } from 'vitest'
import { getJobStatusKind } from './favicon'
import type { Job } from '$lib/gen'

describe('getJobStatusKind', () => {
	it('returns undefined when there is no job', () => {
		expect(getJobStatusKind(undefined)).toBeUndefined()
	})

	it('maps a successful completed job to success', () => {
		expect(getJobStatusKind({ type: 'CompletedJob', success: true } as Job)).toBe('success')
	})

	it('maps a failed completed job to failure', () => {
		expect(getJobStatusKind({ type: 'CompletedJob', success: false } as Job)).toBe('failure')
	})

	it('maps a canceled completed job to failure', () => {
		expect(getJobStatusKind({ type: 'CompletedJob', success: false, canceled: true } as Job)).toBe(
			'failure'
		)
	})

	it('maps a canceled queued job to failure', () => {
		expect(getJobStatusKind({ type: 'QueuedJob', running: false, canceled: true } as Job)).toBe(
			'failure'
		)
	})

	it('maps a running queued job to running', () => {
		expect(getJobStatusKind({ type: 'QueuedJob', running: true } as Job)).toBe('running')
	})

	it('maps a not-yet-started queued job to running', () => {
		expect(getJobStatusKind({ type: 'QueuedJob', running: false } as Job)).toBe('running')
	})
})
