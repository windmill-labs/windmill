import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/gen', () => ({
	WorkspaceService: {
		createWorkspaceFork: vi.fn(),
		getForkCreationStatus: vi.fn(),
		existsWorkspace: vi.fn()
	}
}))

import { WorkspaceService } from '$lib/gen'
import { createWorkspaceForkAndWait } from './forkCreation'

const svc = vi.mocked(WorkspaceService)
const fork = { id: 'wm-fork-x', name: 'x' }

async function run() {
	const p = createWorkspaceForkAndWait('ws', fork)
	const settled = p.then(
		() => 'completed',
		(e) => `failed: ${e.message}`
	)
	await vi.runAllTimersAsync()
	return settled
}

describe('createWorkspaceForkAndWait', () => {
	beforeEach(() => {
		vi.useFakeTimers()
		vi.resetAllMocks()
		svc.createWorkspaceFork.mockResolvedValue('started')
		svc.existsWorkspace.mockResolvedValue(false)
	})
	afterEach(() => vi.useRealTimers())

	it('waits while the fork is running and reports how it ended', async () => {
		svc.getForkCreationStatus
			.mockResolvedValueOnce({ status: 'running' })
			.mockResolvedValueOnce({ status: 'failed', error: 'boom' })
		expect(await run()).toBe('failed: boom')
		expect(svc.getForkCreationStatus).toHaveBeenCalledTimes(2)
	})

	it('takes a failed poll for success once the fork exists', async () => {
		// A server without background forks created it synchronously and has no status route.
		svc.getForkCreationStatus.mockRejectedValue(
			Object.assign(new Error('Not Found'), { status: 404 })
		)
		svc.existsWorkspace.mockResolvedValue(true)
		expect(await run()).toBe('completed')
	})

	it('gives up after repeated failed polls for a fork that never appears', async () => {
		svc.getForkCreationStatus.mockRejectedValue(new Error('upstream request timeout'))
		expect(await run()).toBe('failed: upstream request timeout')
		expect(svc.getForkCreationStatus).toHaveBeenCalledTimes(10)
	})

	it('waits for a creation an earlier request already started', async () => {
		svc.createWorkspaceFork.mockRejectedValue({
			body: "Bad request: workspace 'wm-fork-x' is already being created"
		})
		svc.getForkCreationStatus.mockResolvedValue({ status: 'completed' })
		expect(await run()).toBe('completed')
	})
})
