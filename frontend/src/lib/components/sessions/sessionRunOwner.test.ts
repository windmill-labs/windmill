import { afterEach, describe, expect, it, vi } from 'vitest'
import {
	clearRunPosition,
	noteDriverAnswered,
	setDriverProbe,
	withSessionRunLock
} from './sessionRunOwner.svelte'

const SESSIONS = ['session-throttled-driver', 'session-driver-gone']

// `positions` is module state and a watching entry keeps the reaper interval
// armed, so leave nothing behind for the next suite.
afterEach(() => SESSIONS.forEach(clearRunPosition))

// Under the test env there is no Web Locks API, which is the same footing a
// self-hosted instance served over plain HTTP runs on — so these exercise the
// path that has nothing but the channel to arbitrate with.
//
// A driving tab that is hidden has its timers throttled to as little as once a
// minute, so its heartbeat stops long before its turn does and the reaper
// retires it. Concluding from that silence that the run is over starts a second
// turn against the same chat id: duplicate tool calls against the workspace,
// and whichever save lands last discards the other's transcript.
describe('withSessionRunLock with no lock to take', () => {
	it('refuses the run when a driver answers the probe', async () => {
		setDriverProbe((sessionId) => noteDriverAnswered(sessionId))
		const body = vi.fn(async () => 'ran')

		const outcome = await withSessionRunLock('session-throttled-driver', body)

		expect(outcome).toBe('busy')
		expect(body).not.toHaveBeenCalled()
	})

	it('takes the run when nothing answers', async () => {
		setDriverProbe(() => {})
		const body = vi.fn(async () => 'ran')

		const outcome = await withSessionRunLock('session-driver-gone', body)

		expect(outcome).toBe('ran')
		expect(body).toHaveBeenCalledTimes(1)
	})
})
