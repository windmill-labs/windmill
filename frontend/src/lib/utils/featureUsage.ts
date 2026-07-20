import { get } from 'svelte/store'
import { OpenAPI } from '$lib/gen'
import { workspaceStore } from '$lib/stores'

// Anonymous product-usage counters (e.g. AI session activity), batched into the
// backend `feature_usage` accumulator. Only aggregated counts ever leave the
// instance, and only when telemetry is enabled and not in minimal mode — never
// log paths, prompts, code, or user identifiers here (entity ids must be
// random UUIDs).

export interface FeatureUsageOpts {
	key?: string
	entityId?: string
	value?: number
	/** Workspace whose API route carries the batch; defaults to the active workspace. */
	workspace?: string
}

type SendFn = (workspace: string, events: FeatureUsageEventPayload[]) => Promise<void>

export interface FeatureUsageEventPayload {
	feature: string
	kind: string
	key?: string
	entity_id?: string
	value?: number
}

const FLUSH_INTERVAL_MS = 30_000
// Backend caps a batch at 50 events; chunk larger flushes.
const MAX_EVENTS_PER_REQUEST = 50

export function createFeatureUsageBuffer(
	send: SendFn,
	getDefaultWorkspace: () => string | undefined,
	flushIntervalMs = FLUSH_INTERVAL_MS
) {
	// One accumulator per (workspace, feature, kind, key, entityId): repeated
	// events sum locally so a chatty UI still produces one upsert per flush.
	const pending = new Map<string, { workspace: string; event: FeatureUsageEventPayload }>()
	let timer: ReturnType<typeof setTimeout> | undefined

	function log(feature: string, kind: string, opts: FeatureUsageOpts = {}): void {
		const workspace = opts.workspace ?? getDefaultWorkspace()
		if (!workspace) return
		const key = opts.key ?? ''
		const entityId = opts.entityId ?? ''
		const value = Math.max(1, Math.round(opts.value ?? 1))
		const mapKey = `${workspace}\u0000${feature}\u0000${kind}\u0000${key}\u0000${entityId}`
		const existing = pending.get(mapKey)
		if (existing) {
			existing.event.value = (existing.event.value ?? 1) + value
		} else {
			pending.set(mapKey, {
				workspace,
				event: { feature, kind, key, entity_id: entityId, value }
			})
		}
		if (timer === undefined) {
			timer = setTimeout(() => {
				timer = undefined
				void flush()
			}, flushIntervalMs)
		}
	}

	async function flush(): Promise<void> {
		if (timer !== undefined) {
			clearTimeout(timer)
			timer = undefined
		}
		if (pending.size === 0) return
		const byWorkspace = new Map<string, FeatureUsageEventPayload[]>()
		for (const { workspace, event } of pending.values()) {
			let events = byWorkspace.get(workspace)
			if (!events) {
				events = []
				byWorkspace.set(workspace, events)
			}
			events.push(event)
		}
		pending.clear()
		for (const [workspace, events] of byWorkspace) {
			for (let i = 0; i < events.length; i += MAX_EVENTS_PER_REQUEST) {
				try {
					await send(workspace, events.slice(i, i + MAX_EVENTS_PER_REQUEST))
				} catch {
					// Telemetry is best-effort: drop the batch rather than retry.
				}
			}
		}
	}

	return { log, flush }
}

const buffer = createFeatureUsageBuffer(
	async (workspace, events) => {
		// Raw fetch instead of the generated client: `keepalive` lets the request
		// finish after tab close/navigation, which is when the final flush runs.
		// Auth rides on the token cookie (WITH_CREDENTIALS app setup).
		await fetch(`${OpenAPI.BASE}/w/${encodeURIComponent(workspace)}/workspaces/log_feature_usage`, {
			method: 'POST',
			credentials: 'include',
			keepalive: true,
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ events })
		})
	},
	() => get(workspaceStore) ?? undefined
)

if (typeof document !== 'undefined') {
	// Flush what's buffered before the tab goes away. pagehide covers
	// close/navigation paths where visibilitychange is not delivered.
	document.addEventListener('visibilitychange', () => {
		if (document.visibilityState === 'hidden') {
			void buffer.flush()
		}
	})
	window.addEventListener('pagehide', () => {
		void buffer.flush()
	})
}

/**
 * Record an anonymous feature-usage event. Fire-and-forget: events are summed
 * locally per (feature, kind, key, entityId) and flushed in batches.
 */
export function logFeatureUsage(feature: string, kind: string, opts: FeatureUsageOpts = {}): void {
	buffer.log(feature, kind, opts)
}
