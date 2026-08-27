import { afterEach, describe, expect, it, vi } from 'vitest'
import {
	clearRunPosition,
	noteDriverAlive,
	noteRemoteTurnEnded,
	withSessionRunLock
} from './sessionRunOwner.svelte'

const SESSIONS = ['session-watching', 'session-catching-up', 'session-idle']

// `positions` is module state and a watching entry keeps the reaper interval
// armed, so leave nothing behind for the next suite.
afterEach(() => SESSIONS.forEach(clearRunPosition))

// The test env has no Web Locks API, which is the footing an origin served over
// plain HTTP runs on, so these exercise the fallback rather than the lock.
describe('withSessionRunLock with no lock to take', () => {
	it('refuses while another tab is driving', async () => {
		noteDriverAlive('session-watching', false)
		const body = vi.fn(async () => 'ran')

		expect(await withSessionRunLock('session-watching', body)).toBe('busy')
		expect(body).not.toHaveBeenCalled()
	})

	// The window after the driver's turn ends and before this tab has re-read what
	// it left behind. The run is over, so a check for "someone is driving" says
	// go — but the transcript on screen is still paired with the history from
	// before that turn, and sending would put that pair to the model.
	it('refuses while still catching up on a finished turn', async () => {
		noteDriverAlive('session-catching-up', false)
		noteRemoteTurnEnded('session-catching-up')
		const body = vi.fn(async () => 'ran')

		expect(await withSessionRunLock('session-catching-up', body)).toBe('busy')
		expect(body).not.toHaveBeenCalled()
	})

	it('takes a session no other tab holds', async () => {
		const body = vi.fn(async () => 'ran')

		expect(await withSessionRunLock('session-idle', body)).toBe('ran')
		expect(body).toHaveBeenCalledTimes(1)
	})
})
