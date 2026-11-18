import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import * as Y from 'yjs'

let signed = 0
vi.mock('$lib/components/debug/debugUtils', () => ({
	signMultiplayerRequest: vi.fn(async () => `token-${++signed}`)
}))
vi.mock('$lib/wsUrl', () => ({ buildWsUrl: (path: string) => `ws://mp.test${path}` }))

import { MultiplayerConnection } from './multiplayerConnection'

const attempts: string[] = []

// What the multiplayer server does with a token it rejects: accept the upgrade,
// then close with 4403.
class RejectingWebSocket {
	binaryType = ''
	onopen: (() => void) | null = null
	onclose: ((e: { code: number }) => void) | null = null
	onerror: (() => void) | null = null
	onmessage: (() => void) | null = null
	constructor(url: string) {
		attempts.push(url)
		setTimeout(() => {
			this.onopen?.()
			this.onclose?.({ code: 4403 })
		}, 0)
	}
	send() {}
	close() {}
}

describe('MultiplayerConnection', () => {
	beforeEach(() => {
		vi.useFakeTimers()
		vi.stubGlobal('WebSocket', RejectingWebSocket)
		attempts.length = 0
		signed = 0
	})
	afterEach(() => {
		vi.unstubAllGlobals()
		vi.useRealTimers()
	})

	it('backs off on rejected connections and signs a fresh token for each attempt', async () => {
		const connection = await MultiplayerConnection.open('ws', 'ws/room', new Y.Doc())
		await vi.advanceTimersByTimeAsync(5 * 60_000)

		// 1s doubling to a 60s cap, halved by jitter at worst: at most 15 in 5 minutes,
		// where y-websocket's own reconnect makes thousands.
		expect(attempts.length).toBeGreaterThan(3)
		expect(attempts.length).toBeLessThanOrEqual(15)
		const tokens = attempts.map((u) => new URL(u).searchParams.get('token'))
		expect(new Set(tokens).size).toBe(attempts.length)

		connection.destroy()
		const after = attempts.length
		await vi.advanceTimersByTimeAsync(5 * 60_000)
		expect(attempts.length).toBe(after)
	})
})
