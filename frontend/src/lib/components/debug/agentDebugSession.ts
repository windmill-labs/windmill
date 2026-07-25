/**
 * Agent-facing control layer over the singleton debug session (`dapClient` +
 * `debugState`), shared by the copilot and the human. It adds the three things the
 * raw DAP client lacks: an ownership token (DAP has no actor attribution), a
 * transition-wait state machine (a control tool must not return until execution
 * settles), and a per-turn budget (one `continue` in a hot loop is thousands of
 * stops). Operations return short model-facing strings so the tools stay thin.
 */

import { get, writable } from 'svelte/store'
import {
	debugState,
	peekDAPClient,
	onDAPReset,
	type DAPClient,
	type DAPMessage,
	type StackFrame,
	type Variable
} from './dapClient'
import { getDebugErrorMessage } from './debugUtils'

export interface DebugController {
	language: () => string
	// `onClientReady` MUST run once the client is connected but BEFORE launch, so the
	// transition listener is attached in time to catch the first breakpoint stop.
	start: (
		agentArgs: Record<string, unknown> | undefined,
		onClientReady: (client: DAPClient) => void
	) => Promise<void>
	stop: () => Promise<void>
	signExpression: (expression: string) => Promise<string | undefined>
	setBreakpoints: (lines: number[]) => Promise<void>
}

// --- Ownership ---

export type DebugOwner = 'human' | 'agent' | null

export const debugOwner = writable<DebugOwner>(null)

export function getDebugOwner(): DebugOwner {
	return get(debugOwner)
}

// Called from every human debug entry point. Claiming for the human and settling the
// agent's pending wait is what makes ownership a hand-off rather than a deadlock.
export function preemptToHuman(): void {
	if (get(debugOwner) !== 'human') debugOwner.set('human')
	if (tracker) tracker.latched = null
	settleWaiter({ kind: 'preempted' })
}

function acquireAgentOwner(force: boolean): boolean {
	if (get(debugOwner) === 'human' && !force) return false
	debugOwner.set('agent')
	return true
}

function releaseAgentOwner(): void {
	if (get(debugOwner) === 'agent') debugOwner.set(null)
}

// --- Transition-wait state machine (one slot per session) ---

export type DebugLanding =
	| { kind: 'stopped'; reason?: string }
	| { kind: 'terminated'; result?: unknown; error?: string }
	| { kind: 'disconnected' }
	| { kind: 'timeout' }
	| { kind: 'preempted' }
	| { kind: 'torn_down' }
	| { kind: 'turn_ended' }
	| { kind: 'error'; message: string }

type Tracker = {
	client: DAPClient
	unsub: () => void
	latched: DebugLanding | null
	waiter: { promise: Promise<DebugLanding>; resolve: (l: DebugLanding) => void; timer: any } | null
}

let tracker: Tracker | null = null

function eventToLanding(event: DAPMessage): DebugLanding | null {
	const body = event.body as Record<string, unknown> | undefined
	switch (event.event) {
		case 'stopped':
			return { kind: 'stopped', reason: body?.reason as string }
		case 'terminated':
			return { kind: 'terminated', result: body?.result, error: body?.error as string | undefined }
		case 'closed':
			return { kind: 'disconnected' }
		default:
			return null
	}
}

function onTrackerEvent(event: DAPMessage): void {
	if (!tracker) return
	const landing = eventToLanding(event)
	if (!landing) return
	if (tracker.waiter) {
		const waiter = tracker.waiter
		tracker.waiter = null
		clearTimeout(waiter.timer)
		waiter.resolve(landing)
	} else {
		tracker.latched = landing
	}
	// After a terminal landing the listener is useless; keep the tracker only while a
	// landing is still latched for a pending await to read.
	if (landing.kind === 'terminated' || landing.kind === 'disconnected') {
		tracker.unsub()
		if (!tracker.latched) tracker = null
	}
}

function ensureTracker(client: DAPClient): Tracker {
	if (tracker && tracker.client === client) return tracker
	tracker?.unsub()
	tracker = { client, unsub: client.onEvent(onTrackerEvent), latched: null, waiter: null }
	return tracker
}

function settleWaiter(landing: DebugLanding): void {
	if (!tracker?.waiter) return
	const waiter = tracker.waiter
	tracker.waiter = null
	clearTimeout(waiter.timer)
	waiter.resolve(landing)
}

// On timeout the listener stays attached so a later `debug_wait` reads the latch
// instead of racing a stop that fired in the gap.
function awaitTransition(timeoutMs: number): Promise<DebugLanding> {
	if (!tracker) return Promise.resolve({ kind: 'error', message: 'no active debug session' })
	if (tracker.latched) {
		const landing = tracker.latched
		tracker.latched = null
		if (landing.kind === 'terminated' || landing.kind === 'disconnected') tracker = null
		return Promise.resolve(landing)
	}
	if (tracker.waiter) return tracker.waiter.promise
	let resolveFn!: (l: DebugLanding) => void
	const promise = new Promise<DebugLanding>((resolve) => {
		resolveFn = resolve
	})
	const timer = setTimeout(() => {
		if (tracker?.waiter) {
			const resolve = tracker.waiter.resolve
			tracker.waiter = null
			resolve({ kind: 'timeout' })
		}
	}, timeoutMs)
	tracker.waiter = { promise, resolve: resolveFn, timer }
	return promise
}

// --- Per-turn budget ---

const MAX_TIER2_PER_TURN = 25
const MAX_TRANSITIONS_PER_TURN = 40

let tier2Count = 0
let transitionCount = 0

export function resetDebugBudget(): void {
	tier2Count = 0
	transitionCount = 0
}

function chargeTransition(): boolean {
	transitionCount += 1
	return transitionCount <= MAX_TRANSITIONS_PER_TURN
}

// Check without charging: a start that fails to launch is not a transition.
function hasTransitionBudget(): boolean {
	return transitionCount < MAX_TRANSITIONS_PER_TURN
}

function chargeTier2(): boolean {
	tier2Count += 1
	return tier2Count <= MAX_TIER2_PER_TURN
}

const BUDGET_TRANSITION_MESSAGE =
	`Per-turn debug budget reached (${MAX_TRANSITIONS_PER_TURN} run/step actions). A breakpoint may be inside a hot loop — ` +
	'remove or narrow it, or ask the user to continue in a new turn.'

const BUDGET_TIER2_MESSAGE =
	`Per-turn debug evaluation budget reached (${MAX_TIER2_PER_TURN} evaluations). ` +
	'Read the state you already have, or ask the user to continue in a new turn.'

// --- Result shaping (token-frugal, frame-scoped, depth-limited) ---

const MAX_VALUE_LEN = 200
const MAX_LOG_TAIL = 2000
const MAX_FRAMES = 8

function truncate(value: string, max = MAX_VALUE_LEN): string {
	if (value.length <= max) return value
	return value.slice(0, max) + `… (${value.length} chars)`
}

function safeStringify(value: unknown): string {
	if (typeof value === 'string') return value
	try {
		return JSON.stringify(value)
	} catch {
		return String(value)
	}
}

function formatFrames(frames: StackFrame[]): string {
	if (frames.length === 0) return '  (no stack frames)'
	return frames
		.slice(0, MAX_FRAMES)
		.map((f, i) => `  #${i} ${f.name} @ line ${f.line}`)
		.join('\n')
}

function formatVariables(vars: Variable[]): string {
	if (vars.length === 0) return '  (none)'
	return vars
		.map((v) => `  ${v.name}${v.type ? `: ${v.type}` : ''} = ${truncate(v.value)}`)
		.join('\n')
}

export function shapeState(): string {
	const s = get(debugState)
	const owner = get(debugOwner)
	const lines: string[] = []

	let status: string
	if (!s.connected) status = 'not connected'
	else if (s.stopped) status = `stopped (${s.stoppedReason ?? 'unknown'})`
	else if (s.running) status = 'running'
	else status = 'connected/idle'
	lines.push(`Session: ${status}${owner ? ` — controlled by: ${owner}` : ''}`)

	if (s.currentFile || s.currentLine !== undefined) {
		lines.push(`Position: ${s.currentFile ?? 'script'}:${s.currentLine ?? '?'}`)
	}

	if (s.stopped && s.stackFrames.length > 0) {
		lines.push('Stack (top first):')
		lines.push(formatFrames(s.stackFrames))
		if (s.scopes.length > 0) {
			lines.push(
				`Scopes at top frame: ${s.scopes.map((sc) => sc.name).join(', ')} ` +
					'(use debug_evaluate to read values)'
			)
			for (const scope of s.scopes) {
				const cached = s.variables.get(scope.variablesReference)
				if (cached && cached.length > 0) {
					lines.push(`${scope.name}:`)
					lines.push(formatVariables(cached))
				}
			}
		}
	}

	if (s.breakpoints.size > 0) {
		const allLines = Array.from(s.breakpoints.values())
			.flat()
			.map((b) => b.line)
			.sort((a, b) => a - b)
		if (allLines.length > 0) lines.push(`Breakpoints at lines: ${allLines.join(', ')}`)
	}

	if (s.logs) {
		const tail = s.logs.length > MAX_LOG_TAIL ? '…' + s.logs.slice(-MAX_LOG_TAIL) : s.logs
		lines.push(`Output so far:\n${tail}`)
	}

	if (!s.running && !s.stopped) {
		if (s.error) lines.push(`Error: ${truncate(s.error, MAX_LOG_TAIL)}`)
		if (s.result !== undefined)
			lines.push(`Result: ${truncate(safeStringify(s.result), MAX_LOG_TAIL)}`)
	}

	return lines.join('\n')
}

async function shapeLanding(landing: DebugLanding, client: DAPClient): Promise<string> {
	switch (landing.kind) {
		case 'stopped': {
			// The auto-fetch fired inside handleEvent is still in flight; re-fetch (tier-1
			// reads) so shapeState reads a populated snapshot.
			try {
				const frames = await client.getStackTrace()
				if (frames[0]) await client.getScopes(frames[0].id)
			} catch (error) {
				console.error('[debug] failed to refresh stop snapshot:', error)
			}
			const reason = landing.reason ? ` (${landing.reason})` : ''
			return `Execution stopped${reason}.\n${shapeState()}`
		}
		case 'terminated':
			if (landing.error) return `Execution errored out:\n${truncate(landing.error, MAX_LOG_TAIL)}`
			return `Execution ran to completion.\n${shapeState()}`
		case 'disconnected':
			return 'The debug session disconnected (socket closed).'
		case 'timeout':
			return 'Still running after the wait window. The code has not stopped or finished yet. Call debug_wait to keep waiting, or debug_stop to abort.'
		case 'preempted':
			return CONTROL_TAKEN_MESSAGE
		case 'torn_down':
			return 'The debug session was torn down.'
		case 'turn_ended':
			return 'This turn ended while waiting; the debug session is left intact. Call debug_get_state to check on it or debug_wait to keep waiting.'
		case 'error':
			return `Debug operation failed: ${landing.message}`
	}
}

const CONTROL_TAKEN_MESSAGE =
	'Control was taken by the user, who is now driving this debug session manually. Stop issuing debug control commands unless the user explicitly asks you to resume. To resume when they ask, call debug_start to take over (they will be asked to confirm).'

const NO_SESSION_MESSAGE =
	'No active debug session. Start one with debug_start before continuing, stepping, evaluating, or reading state.'

// --- Guards shared by the driving (tier-2 / control) operations ---

function requireLiveClient(): DAPClient | null {
	const client = peekDAPClient()
	if (!client || !client.isConnected()) return null
	return client
}

function ensureAgentControl(): { ok: true } | { ok: false; message: string } {
	if (get(debugOwner) === 'human') return { ok: false, message: CONTROL_TAKEN_MESSAGE }
	acquireAgentOwner(false)
	return { ok: true }
}

// --- High-level agent operations (called by the copilot helpers) ---

export function agentDebugGetState(): string {
	if (!peekDAPClient()) return NO_SESSION_MESSAGE
	return shapeState()
}

export async function agentDebugStart(
	controller: DebugController,
	agentArgs: Record<string, unknown> | undefined,
	timeoutMs: number
): Promise<string> {
	// Taking over a human session is allowed: debug_start is confirmation-gated in the UI.
	if (!hasTransitionBudget()) return BUDGET_TRANSITION_MESSAGE
	try {
		// `start` resets the client and the reset hook clears ownership, so claim the
		// session in the ready callback: after the reset, before launch.
		await controller.start(agentArgs, (client) => {
			ensureTracker(client).latched = null
			acquireAgentOwner(true)
		})
	} catch (error) {
		if (tracker) {
			tracker.unsub()
			tracker = null
		}
		releaseAgentOwner()
		return `Could not start the debugger: ${getDebugErrorMessage(error)}`
	}
	chargeTransition()
	const client = peekDAPClient()
	if (!client) {
		releaseAgentOwner()
		return 'Debugger did not initialize a session.'
	}
	return shapeLanding(await awaitTransition(timeoutMs), client)
}

export async function agentDebugContinue(timeoutMs: number): Promise<string> {
	const control = ensureAgentControl()
	if (!control.ok) return control.message
	const client = requireLiveClient()
	if (!client) return NO_SESSION_MESSAGE
	if (!client.isStopped()) {
		return client.isRunning()
			? 'Execution is already running, not paused — use debug_wait to wait for the next stop.'
			: 'Execution is not paused (it has finished or not started). Start a new session with debug_start.'
	}
	if (!chargeTransition()) return BUDGET_TRANSITION_MESSAGE
	ensureTracker(client).latched = null
	try {
		await client.continue_()
	} catch (error) {
		return `Continue failed: ${getDebugErrorMessage(error)}`
	}
	return shapeLanding(await awaitTransition(timeoutMs), client)
}

export async function agentDebugStep(
	kind: 'over' | 'in' | 'out',
	timeoutMs: number
): Promise<string> {
	const control = ensureAgentControl()
	if (!control.ok) return control.message
	const client = requireLiveClient()
	if (!client) return NO_SESSION_MESSAGE
	if (!client.isStopped()) return 'Cannot step: execution is not paused at a breakpoint.'
	if (!chargeTransition()) return BUDGET_TRANSITION_MESSAGE
	ensureTracker(client).latched = null
	try {
		if (kind === 'over') await client.stepOver()
		else if (kind === 'in') await client.stepIn()
		else await client.stepOut()
	} catch (error) {
		return `Step failed: ${getDebugErrorMessage(error)}`
	}
	return shapeLanding(await awaitTransition(timeoutMs), client)
}

export async function agentDebugWait(timeoutMs: number): Promise<string> {
	const control = ensureAgentControl()
	if (!control.ok) return control.message
	const client = requireLiveClient()
	if (!client) return NO_SESSION_MESSAGE
	if (client.isStopped()) return `Execution is already paused.\n${shapeState()}`
	if (!client.isRunning()) return `Execution has finished or has not started.\n${shapeState()}`
	if (!chargeTransition()) return BUDGET_TRANSITION_MESSAGE
	return shapeLanding(await awaitTransition(timeoutMs), client)
}

export async function agentDebugEvaluate(
	controller: DebugController,
	expression: string,
	frameId?: number
): Promise<string> {
	const control = ensureAgentControl()
	if (!control.ok) return control.message
	const client = requireLiveClient()
	if (!client) return NO_SESSION_MESSAGE
	if (!chargeTier2()) return BUDGET_TIER2_MESSAGE
	const effectiveFrame = frameId ?? get(debugState).stackFrames[0]?.id
	try {
		const token = await controller.signExpression(expression)
		const { result } = await client.evaluate(expression, effectiveFrame, 'repl', token)
		return truncate(result ?? 'undefined', MAX_LOG_TAIL)
	} catch (error) {
		return `Evaluation failed: ${getDebugErrorMessage(error)}`
	}
}

export async function agentDebugSetBreakpoints(
	controller: DebugController,
	lines: number[]
): Promise<string> {
	const control = ensureAgentControl()
	if (!control.ok) return control.message
	try {
		await controller.setBreakpoints(lines)
	} catch (error) {
		return `Could not set breakpoints: ${getDebugErrorMessage(error)}`
	}
	return lines.length === 0
		? 'Cleared all breakpoints.'
		: `Breakpoints set at lines: ${[...lines].sort((a, b) => a - b).join(', ')}.`
}

export async function agentDebugStop(controller: DebugController): Promise<string> {
	if (getDebugOwner() === 'human') return CONTROL_TAKEN_MESSAGE
	try {
		await controller.stop()
	} catch (error) {
		return `Could not stop the session: ${getDebugErrorMessage(error)}`
	}
	settleWaiter({ kind: 'torn_down' })
	if (tracker) {
		tracker.unsub()
		tracker = null
	}
	releaseAgentOwner()
	return 'Debug session stopped.'
}

// --- Turn / teardown lifecycle ---

// End of a chat turn: settle any pending wait and hand ownership back. The live
// session (socket + token) is left intact — the same as a human pausing mid-debug.
export function onAgentTurnEnd(): void {
	settleWaiter({ kind: 'turn_ended' })
	releaseAgentOwner()
}

onDAPReset(() => {
	settleWaiter({ kind: 'torn_down' })
	if (tracker) {
		tracker.unsub()
		tracker = null
	}
	debugOwner.set(null)
})
