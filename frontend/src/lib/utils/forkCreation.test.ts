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
		svc.createWorkspaceFork.mockResolvedValue('started')
		svc.existsWorkspace.mockResolvedValue(false)
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

	it('takes a failed poll for success once the fork exists', async () => {
		// A server without background forks created it synchronously and has no status route.
		svc.getForkCreationStatus.mockRejectedValue(
			Object.assign(new Error('Not Found'), { status: 404 })
		)
		svc.existsWorkspace.mockResolvedValue(true)
		expect(await run()).toBe('completed')
	})

	it('gives up once polls keep failing for a fork that never appears', async () => {
		svc.getForkCreationStatus.mockRejectedValue(new Error('upstream request timeout'))
		expect(await run()).toBe('failed: upstream request timeout')
			})
})
