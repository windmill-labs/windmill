import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
	__receiveForTest,
	__resetForTest,
	onRemoteTurnEnd,
	runHeldElsewhere
} from './sessionSync.svelte'

// Exercises the receive-side state machine directly (the module opens no
// BroadcastChannel outside the browser). The channel itself is glue that only
// a real browser can prove; what these pin are the invariants a refactor could
// silently break: the identity-token cleanup, the staleness prune, and the
// turn-end hold.

beforeEach(() => {
	__resetForTest()
	vi.useFakeTimers()
})

afterEach(() => {
	__resetForTest()
	vi.useRealTimers()
})

describe('sessionSync receive-side state', () => {
	it('locks on a heartbeat and unlocks by staleness when the driver dies silently', async () => {
		__receiveForTest({ kind: 'run-heartbeat', sessionId: 's1' })
		expect(runHeldElsewhere('s1')).toBe(true)

		// Refreshed heartbeats keep the lock past the original entry's window.
		await vi.advanceTimersByTimeAsync(60_000)
		__receiveForTest({ kind: 'run-heartbeat', sessionId: 's1' })
		await vi.advanceTimersByTimeAsync(60_000)
		expect(runHeldElsewhere('s1')).toBe(true)

		// Silence past STALE_MS (90s) prunes the entry.
		await vi.advanceTimersByTimeAsync(40_000)
		expect(runHeldElsewhere('s1')).toBe(false)
	})

	it('holds the lock through the turn-end catch-up and releases when it settles', async () => {
		let releaseCatchUp: (() => void) | undefined
		onRemoteTurnEnd(() => new Promise<void>((resolve) => (releaseCatchUp = resolve)))

		__receiveForTest({ kind: 'run-heartbeat', sessionId: 's1' })
		__receiveForTest({ kind: 'turn-end', sessionId: 's1', chatId: 'c1' })
		await vi.advanceTimersByTimeAsync(0)
		// Unlocking on receipt would let a send here start from history missing
		// the turn that just ended; the lock must outlive the re-read.
		expect(runHeldElsewhere('s1')).toBe(true)

		releaseCatchUp?.()
		await vi.advanceTimersByTimeAsync(0)
		expect(runHeldElsewhere('s1')).toBe(false)
	})

	it("keeps the lock when the driver's next turn arrives during the catch-up", async () => {
		let releaseCatchUp: (() => void) | undefined
		onRemoteTurnEnd(() => new Promise<void>((resolve) => (releaseCatchUp = resolve)))

		__receiveForTest({ kind: 'turn-end', sessionId: 's1', chatId: 'c1' })
		// The queued-follow-up sequence: the next turn's first heartbeat lands
		// while this tab's catch-up is still reading — in the same millisecond,
		// which is why the cleanup must compare identity, not timestamps.
		__receiveForTest({ kind: 'run-heartbeat', sessionId: 's1' })

		releaseCatchUp?.()
		await vi.advanceTimersByTimeAsync(0)
		expect(runHeldElsewhere('s1')).toBe(true)
	})
})
