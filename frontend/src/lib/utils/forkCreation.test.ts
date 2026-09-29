import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/gen', () => ({
	WorkspaceService: {
		createWorkspaceFork: vi.fn(),
		getForkCreationStatus: vi.fn()
	}
}))

import { WorkspaceService } from '$lib/gen'
import { createWorkspaceForkAndWait } from './forkCreation'

const svc = vi.mocked(WorkspaceService)
const fork = { id: 'wm-fork-x', name: 'x' }

async function run(onStep?: (step: string) => void) {
	const p = createWorkspaceForkAndWait('ws', fork, onStep)
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
		svc.createWorkspaceFork.mockResolvedValue('5c3e1b1e-0000-4000-8000-000000000000')
	})
	afterEach(() => vi.useRealTimers())

	it('reports each step while the fork runs, then how it ended', async () => {
		svc.getForkCreationStatus
			.mockResolvedValueOnce({ status: 'running' })
			.mockResolvedValueOnce({ status: 'running', step: 'Copying flows' })
			.mockResolvedValueOnce({ status: 'failed', error: 'boom' })
		const steps: string[] = []
		expect(await run((s) => steps.push(s))).toBe('failed: boom')
		expect(steps).toEqual(['Copying flows'])
	})

	it('returns at once when the server created the fork synchronously', async () => {
		// A server without background forks ignores the flag.
		svc.createWorkspaceFork.mockResolvedValue('Created forked workspace wm-fork-x')
		expect(await run()).toBe('completed')
		expect(svc.getForkCreationStatus).not.toHaveBeenCalled()
	})

	it('keeps polling through failed polls', async () => {
		svc.getForkCreationStatus
			.mockRejectedValueOnce(Object.assign(new Error('Not Found'), { status: 404 }))
			.mockRejectedValueOnce(new Error('upstream request timeout'))
			.mockResolvedValueOnce({ status: 'completed' })
		expect(await run()).toBe('completed')
	})

	it('gives up once polls keep failing for a fork that never appears', async () => {
		svc.getForkCreationStatus.mockRejectedValue(new Error('upstream request timeout'))
		expect(await run()).toBe('failed: upstream request timeout')
	})
})
