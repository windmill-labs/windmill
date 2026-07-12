import { beforeEach, describe, expect, it, vi } from 'vitest'

// Shared holders the mocked `./dapClient` writes into, so tests can control the
// singleton client and the store, and fire the reset hook the module registers.
const holders = vi.hoisted(() => ({
	state: null as any,
	client: null as any,
	resetHooks: [] as Array<() => void>,
	initial: () => ({
		connected: false,
		initialized: false,
		running: false,
		stopped: false,
		stoppedReason: undefined,
		currentLine: undefined,
		currentFile: undefined,
		stackFrames: [],
		scopes: [],
		variables: new Map(),
		breakpoints: new Map(),
		output: [],
		logs: '',
		result: undefined,
		error: undefined
	})
}))

vi.mock('./dapClient', async () => {
	const { writable } = await import('svelte/store')
	holders.state = writable(holders.initial())
	return {
		debugState: holders.state,
		peekDAPClient: () => holders.client,
		onDAPReset: (fn: () => void) => {
			holders.resetHooks.push(fn)
			return () => {}
		}
	}
})

vi.mock('./debugUtils', () => ({
	getDebugErrorMessage: (e: unknown) => String((e as any)?.message ?? e)
}))

import {
	agentDebugContinue,
	agentDebugEvaluate,
	agentDebugGetState,
	agentDebugStart,
	agentDebugStep,
	agentDebugStop,
	agentDebugWait,
	debugOwner,
	getDebugOwner,
	onAgentTurnEnd,
	preemptToHuman,
	resetDebugBudget,
	type DebugController
} from './agentDebugSession'

type FakeClient = ReturnType<typeof makeClient>

// Faithful fake: mirrors dapClient.handleEvent's store mutations on each event so the
// tier-1 shaping paths and the isStopped/isRunning guards see production-like state.
function makeClient() {
	const listeners = new Set<(e: any) => void>()
	const patch = (p: any) => holders.state.update((s: any) => ({ ...s, ...p }))
	return {
		stopped: false,
		connected: true,
		running: false,
		evalCalls: [] as any[],
		emit(event: string, body?: any) {
			if (event === 'stopped') {
				this.stopped = true
				this.running = false
				patch({
					stopped: true,
					running: false,
					stoppedReason: body?.reason,
					currentLine: body?.line
				})
			} else if (event === 'continued') {
				this.stopped = false
				this.running = true
				patch({ stopped: false, running: true, stoppedReason: undefined })
			} else if (event === 'terminated') {
				this.stopped = false
				this.running = false
				patch({ running: false, stopped: false, result: body?.result, error: body?.error })
			} else if (event === 'closed') {
				this.connected = false
				this.running = false
				patch({ connected: false })
			}
			for (const l of listeners) l({ seq: 0, type: 'event', event, body })
		},
		onEvent(l: (e: any) => void) {
			listeners.add(l)
			return () => listeners.delete(l)
		},
		isConnected() {
			return this.connected
		},
		isStopped() {
			return this.stopped
		},
		isRunning() {
			return this.running
		},
		async continue_() {
			this.stopped = false
			this.running = true
			patch({ stopped: false, running: true })
		},
		async stepOver() {},
		async stepIn() {},
		async stepOut() {},
		async terminate() {
			this.connected = false
			patch({ connected: false })
		},
		async getStackTrace() {
			const frames = [
				{ id: 1, name: 'main', source: { path: '/tmp/script.ts' }, line: 6, column: 0 }
			]
			patch({ stackFrames: frames, currentLine: 6, currentFile: '/tmp/script.ts' })
			return frames
		},
		async getScopes() {
			const scopes = [{ name: 'Locals', variablesReference: 10, expensive: false }]
			patch({ scopes })
			return scopes
		},
		async evaluate(expression: string, frameId?: number, context?: string, token?: string) {
			this.evalCalls.push({ expression, frameId, context, token })
			return { result: '42', variablesReference: 0 }
		}
	}
}

function makeController(client: FakeClient): DebugController {
	return {
		language: () => 'bun',
		start: async (_args, onClientReady) => {
			holders.client = client
			client.connected = true
			holders.state.update((s: any) => ({ ...s, connected: true }))
			onClientReady(client as any)
		},
		stop: async () => {
			await client.terminate()
		},
		signExpression: async () => 'signed-token',
		setBreakpoints: async () => {}
	}
}

const tick = () => new Promise((r) => setTimeout(r, 0))

/** Drive continue → immediate stop so the op resolves fast (for budget loops). */
async function continueThenStop(client: FakeClient, timeoutMs = 5000): Promise<string> {
	client.stopped = true // paused so continue's guard passes
	const p = agentDebugContinue(timeoutMs)
	await tick()
	client.emit('stopped', { reason: 'breakpoint' })
	return p
}

beforeEach(() => {
	for (const fn of holders.resetHooks) fn() // simulate resetDAPClient(): settle waiters, drop tracker, clear owner
	holders.client = null
	holders.state.set(holders.initial())
	debugOwner.set(null)
	resetDebugBudget()
})

describe('ownership gate', () => {
	it('refuses agent driving/tier-2/stop tools while the human owns the session', async () => {
		holders.client = makeClient()
		preemptToHuman()
		expect(getDebugOwner()).toBe('human')

		const controller = makeController(holders.client)
		expect(await agentDebugContinue(50)).toMatch(/control was taken by the user/i)
		expect(await agentDebugStep('over', 50)).toMatch(/control was taken by the user/i)
		expect(await agentDebugEvaluate(controller, 'x')).toMatch(/control was taken by the user/i)
		expect(await agentDebugStop(controller)).toMatch(/control was taken by the user/i)
	})

	it('keeps tier-1 getState live regardless of owner and reports the owner', () => {
		holders.client = makeClient()
		holders.state.update((s: any) => ({ ...s, connected: true }))
		preemptToHuman()
		expect(agentDebugGetState()).toMatch(/controlled by: human/i)
	})
})

describe('no-session / not-paused guards', () => {
	it('getState reports no session when there is no client', () => {
		expect(agentDebugGetState()).toMatch(/no active debug session/i)
	})

	it('continue fast-fails when the session is not paused', async () => {
		const client = makeClient()
		client.stopped = false
		holders.client = client
		expect(await agentDebugContinue(50)).toMatch(/not paused/i)
	})
})

describe('transition-wait state machine', () => {
	it('resolves a pending continue when a stop fires, and shapes the stop state', async () => {
		const client = makeClient()
		client.stopped = true
		holders.client = client

		const pending = agentDebugContinue(5000)
		await tick()
		client.emit('stopped', { reason: 'breakpoint', line: 6 })

		const res = await pending
		expect(res).toMatch(/stopped \(breakpoint\)/i)
		// tier-1 shaping actually ran against populated store state
		expect(res).toMatch(/#0 main @ line 6/)
		expect(res).toMatch(/Scopes at top frame: Locals/)
	})

	it('returns "still running" on timeout, then resolves on the next wait', async () => {
		const client = makeClient()
		client.stopped = true
		holders.client = client

		const timedOut = await agentDebugContinue(10)
		expect(timedOut).toMatch(/still running/i)
		expect(client.isRunning()).toBe(true)

		const waiting = agentDebugWait(5000)
		await tick()
		client.emit('stopped', { reason: 'breakpoint' })
		expect(await waiting).toMatch(/stopped \(breakpoint\)/i)
	})

	it('reports the paused state if a stop fired before the next wait was issued', async () => {
		const client = makeClient()
		client.stopped = true
		holders.client = client

		const timedOut = await agentDebugContinue(10)
		expect(timedOut).toMatch(/still running/i)
		client.emit('stopped', { reason: 'breakpoint' }) // fires in the gap -> isStopped true
		expect(await agentDebugWait(10)).toMatch(/already paused/i)
	})

	it('reports completion on terminated, and a follow-up wait says finished (not failed)', async () => {
		const client = makeClient()
		client.stopped = true
		holders.client = client

		const pending = agentDebugContinue(5000)
		await tick()
		client.emit('terminated', { result: { ok: true } })
		expect(await pending).toMatch(/ran to completion/i)

		expect(await agentDebugWait(10)).toMatch(/finished or has not started/i)
	})

	it('preemptToHuman settles a pending wait as preempted', async () => {
		const client = makeClient()
		client.stopped = true
		holders.client = client

		const pending = agentDebugContinue(5000)
		await tick()
		preemptToHuman()

		expect(await pending).toMatch(/control was taken by the user/i)
		expect(getDebugOwner()).toBe('human')
	})
})

describe('evaluate', () => {
	it('threads the signed token, frame, and repl context through to the client', async () => {
		const client = makeClient()
		const controller = makeController(client)
		holders.client = client

		const res = await agentDebugEvaluate(controller, 'a + b', 3)
		expect(res).toBe('42')
		expect(client.evalCalls).toHaveLength(1)
		expect(client.evalCalls[0]).toEqual({
			expression: 'a + b',
			frameId: 3,
			context: 'repl',
			token: 'signed-token'
		})
	})
})

describe('per-turn budget', () => {
	it('caps tier-2 evaluations and resets on a new turn', async () => {
		const client = makeClient()
		holders.client = client
		const controller = makeController(client)

		for (let i = 0; i < 25; i++) {
			expect(await agentDebugEvaluate(controller, 'x')).toBe('42')
		}
		expect(await agentDebugEvaluate(controller, 'x')).toMatch(/evaluation budget reached/i)

		resetDebugBudget()
		expect(await agentDebugEvaluate(controller, 'x')).toBe('42')
	})

	it('caps transitions (continue/wait/step) at the per-turn limit', async () => {
		const client = makeClient()
		holders.client = client

		for (let i = 0; i < 40; i++) {
			expect(await continueThenStop(client)).toMatch(/stopped/i)
		}
		client.stopped = true
		expect(await agentDebugContinue(5000)).toMatch(/run\/step actions|budget reached/i)
	})

	it('does not spend transition budget on a failed launch', async () => {
		const failing: DebugController = {
			language: () => 'bun',
			start: async () => {
				throw new Error('boom')
			},
			stop: async () => {},
			signExpression: async () => undefined,
			setBreakpoints: async () => {}
		}
		expect(await agentDebugStart(failing, undefined, 50)).toMatch(/could not start the debugger/i)

		// budget intact: 40 transitions still succeed after the failed start
		const client = makeClient()
		holders.client = client
		for (let i = 0; i < 40; i++) {
			expect(await continueThenStop(client)).toMatch(/stopped/i)
		}
	})
})

describe('start', () => {
	it('claims agent ownership and resolves at the first stop', async () => {
		const client = makeClient()
		client.stopped = false
		const controller = makeController(client)

		const startPromise = agentDebugStart(controller, { a: 1 }, 5000)
		await tick()
		client.emit('stopped', { reason: 'breakpoint', line: 2 })

		expect(await startPromise).toMatch(/stopped \(breakpoint\)/i)
		expect(getDebugOwner()).toBe('agent')
	})

	it('takes over a human-owned session (debug_start is confirmation-gated in the UI)', async () => {
		const client = makeClient()
		client.stopped = false
		const controller = makeController(client)
		preemptToHuman()
		expect(getDebugOwner()).toBe('human')

		const startPromise = agentDebugStart(controller, undefined, 5000)
		await tick()
		client.emit('stopped', { reason: 'breakpoint', line: 2 })

		expect(await startPromise).toMatch(/stopped \(breakpoint\)/i)
		expect(getDebugOwner()).toBe('agent')
	})
})

describe('stop', () => {
	it('tears down and releases ownership on success', async () => {
		const client = makeClient()
		holders.client = client
		const controller = makeController(client)
		debugOwner.set('agent')

		expect(await agentDebugStop(controller)).toMatch(/stopped/i)
		expect(getDebugOwner()).toBe(null)
		expect(client.connected).toBe(false)
	})
})

describe('turn / teardown lifecycle', () => {
	it('onAgentTurnEnd settles a pending wait as turn-ended and releases ownership', async () => {
		const client = makeClient()
		client.stopped = true
		holders.client = client

		const pending = agentDebugContinue(5000)
		await tick()
		onAgentTurnEnd()

		expect(await pending).toMatch(/turn ended/i)
		expect(getDebugOwner()).toBe(null)
	})

	it('the reset hook settles a pending wait and clears ownership', async () => {
		const client = makeClient()
		client.stopped = true
		holders.client = client

		const pending = agentDebugContinue(5000)
		await tick()
		for (const fn of holders.resetHooks) fn()

		expect(await pending).toMatch(/torn down/i)
		expect(getDebugOwner()).toBe(null)
	})
})
