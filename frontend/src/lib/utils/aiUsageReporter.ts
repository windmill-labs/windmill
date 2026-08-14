import { get } from 'svelte/store'
import { OpenAPI } from '$lib/gen'
import { workspaceStore } from '$lib/stores'

// Per-workspace AI token spend, batched into the backend `ai_token_usage`
// accumulator that powers the workspace and per-user usage views.
//
// Deliberately separate from `featureUsage.ts`: that buffer carries anonymous
// product telemetry that leaves the instance, and its events must not identify a
// user. These events are attributed to the caller (server-side, from the session)
// and never leave the instance, so the two must not share a transport.
//
// Only token counts are sent. Money is derived when the usage is read, from the
// price table plus the workspace's overrides, so correcting a rate also corrects
// history. The one exception is a cost the provider itself billed back.

export interface AiUsageEvent {
	provider: string
	model: string
	/** Empty for chats not attached to an AI session. */
	sessionId?: string
	inputTokens: number
	cacheReadTokens: number
	cacheWriteTokens: number
	outputTokens: number
	/** Set only where the provider reports what it actually charged, in USD. */
	costUsd?: number
	/** Workspace whose API route carries the batch; defaults to the active workspace. */
	workspace?: string
}

interface AiUsageEventPayload {
	provider: string
	model: string
	session_id: string
	input_tokens: number
	cache_read_tokens: number
	cache_write_tokens: number
	output_tokens: number
	reported_cost_nano_usd?: number
	requests: number
}

const FLUSH_INTERVAL_MS = 15_000
// Backend caps a batch at 50 events; chunk larger flushes.
const MAX_EVENTS_PER_REQUEST = 50

const NANO_USD_PER_USD = 1_000_000_000

// One accumulator per (workspace, provider, model, session): a chat that sends
// several turns before a flush produces one upsert instead of one per turn.
const pending = new Map<string, { workspace: string; event: AiUsageEventPayload }>()
let timer: ReturnType<typeof setTimeout> | undefined

/**
 * Record AI token spend. Fire-and-forget: events are summed locally and flushed
 * in batches.
 */
export function logAiUsage(event: AiUsageEvent): void {
	const workspace = event.workspace ?? get(workspaceStore) ?? undefined
	if (!workspace) return
	const sessionId = event.sessionId ?? ''
	const mapKey = JSON.stringify([workspace, event.provider, event.model, sessionId])
	const existing = pending.get(mapKey)?.event
	const target: AiUsageEventPayload = existing ?? {
		provider: event.provider,
		model: event.model,
		session_id: sessionId,
		input_tokens: 0,
		cache_read_tokens: 0,
		cache_write_tokens: 0,
		output_tokens: 0,
		requests: 0
	}
	target.input_tokens += Math.max(0, Math.round(event.inputTokens))
	target.cache_read_tokens += Math.max(0, Math.round(event.cacheReadTokens))
	target.cache_write_tokens += Math.max(0, Math.round(event.cacheWriteTokens))
	target.output_tokens += Math.max(0, Math.round(event.outputTokens))
	target.requests += 1
	if (event.costUsd !== undefined) {
		target.reported_cost_nano_usd =
			(target.reported_cost_nano_usd ?? 0) +
			Math.max(0, Math.round(event.costUsd * NANO_USD_PER_USD))
	}
	pending.set(mapKey, { workspace, event: target })

	if (timer === undefined) {
		timer = setTimeout(() => {
			timer = undefined
			void flushAiUsage()
		}, FLUSH_INTERVAL_MS)
	}
}

export async function flushAiUsage(): Promise<void> {
	if (timer !== undefined) {
		clearTimeout(timer)
		timer = undefined
	}
	if (pending.size === 0) return

	const byWorkspace = new Map<string, AiUsageEventPayload[]>()
	for (const { workspace, event } of pending.values()) {
		let events = byWorkspace.get(workspace)
		if (!events) {
			events = []
			byWorkspace.set(workspace, events)
		}
		events.push(event)
	}
	pending.clear()

	// Start every chunk request synchronously before awaiting: the pagehide flush
	// only protects requests that were already issued (keepalive can't help a fetch
	// that never started).
	const inflight: Promise<void>[] = []
	for (const [workspace, events] of byWorkspace) {
		for (let i = 0; i < events.length; i += MAX_EVENTS_PER_REQUEST) {
			inflight.push(send(workspace, events.slice(i, i + MAX_EVENTS_PER_REQUEST)))
		}
	}
	await Promise.all(inflight)
}

async function send(workspace: string, events: AiUsageEventPayload[]): Promise<void> {
	try {
		// Raw fetch instead of the generated client: `keepalive` lets the request
		// finish after tab close/navigation, which is when the final flush runs.
		// Auth rides on the token cookie (WITH_CREDENTIALS app setup).
		await fetch(`${OpenAPI.BASE}/w/${encodeURIComponent(workspace)}/ai/usage`, {
			method: 'POST',
			credentials: 'include',
			keepalive: true,
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ events })
		})
	} catch {
		// Accounting is best-effort: a dropped batch under-reports spend, which is
		// better than surfacing a network error in the middle of a chat.
	}
}

if (typeof document !== 'undefined') {
	// Flush what's buffered before the tab goes away. pagehide covers
	// close/navigation paths where visibilitychange is not delivered.
	document.addEventListener('visibilitychange', () => {
		if (document.visibilityState === 'hidden') {
			void flushAiUsage()
		}
	})
	window.addEventListener('pagehide', () => {
		void flushAiUsage()
	})
}
