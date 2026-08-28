import { afterEach, describe, expect, it, vi } from 'vitest'
import {
	clearRunPosition,
	currentRunId,
	noteDriverAlive,
	noteRemoteTurnEnded,
	onDriverLost,
	withSessionRunLock
} from './sessionRunOwner.svelte'

const SESSIONS = [
	'session-watching',
	'session-catching-up',
	'session-idle',
	'session-unheard',
	'session-runs'
]

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
		onDriverLost(() => {})
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

	// A control message names the run it was pressed for, so a Stop delivered
	// after its turn ended is dropped rather than landing on the turn that
	// followed — which the queue flush and job auto-resume start immediately.
	// Reusing an id across turns would defeat that, so pin that they differ.
	it('gives each turn its own run id', async () => {
		const seen: (string | undefined)[] = []
		await withSessionRunLock('session-runs', async () => {
			seen.push(currentRunId('session-runs'))
		})
		await withSessionRunLock('session-runs', async () => {
			seen.push(currentRunId('session-runs'))
		})

		expect(seen[0]).toBeTruthy()
		expect(seen[1]).toBeTruthy()
		expect(seen[0]).not.toBe(seen[1])
		// And nothing is left claiming a run once the turn is over.
		expect(currentRunId('session-runs')).toBeUndefined()
	})

	// A tab that opened between the driver's last status message and its turn-end
	// never saw the run start, so it is idle — holding the conversation from
	// before the turn, and idle is the position that permits driving on it.
	it('refuses a turn-end it never saw start, rather than driving on stale history', async () => {
		onDriverLost(() => {})
		noteRemoteTurnEnded('session-unheard')
		const body = vi.fn(async () => 'ran')

		expect(await withSessionRunLock('session-unheard', body)).toBe('busy')
		expect(body).not.toHaveBeenCalled()
	})
})

// A fresh module instance is the only honest way to test this: the runtime's
// registration is process-wide and one-way, exactly as it is in a real tab.
describe('a turn ending in a tab with no session runtime', () => {
	it('settles to idle rather than waiting for a re-read nobody can do', async () => {
		vi.resetModules()
		const owner = await import('./sessionRunOwner.svelte')

		owner.noteDriverAlive('session-no-runtime', false)
		owner.noteRemoteTurnEnded('session-no-runtime')

		expect(owner.isCatchingUp('session-no-runtime')).toBe(false)
		expect(owner.runHeldElsewhere('session-no-runtime')).toBe(false)
		owner.clearRunPosition('session-no-runtime')
	})
})
