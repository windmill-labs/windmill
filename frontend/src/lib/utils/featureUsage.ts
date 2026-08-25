import { get } from 'svelte/store'
import { OpenAPI } from '$lib/gen'
import { workspaceStore } from '$lib/stores'
import { PRIVATE_HUB_MIN_VERSION } from '$lib/hub'

// Anonymous product-usage counters (e.g. AI session activity), batched into the
// backend `feature_usage` accumulator. Only aggregated counts ever leave the
// instance, and only when telemetry is enabled and not in minimal mode — never
// log paths, prompts, code, or user identifiers here (entity ids must be
// opaque random ids).

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
		// Start every chunk request synchronously before awaiting: the pagehide
		// flush only protects requests that were already issued (keepalive can't
		// help a fetch that never started).
		const inflight: Promise<void>[] = []
		for (const [workspace, events] of byWorkspace) {
			for (let i = 0; i < events.length; i += MAX_EVENTS_PER_REQUEST) {
				inflight.push(
					send(workspace, events.slice(i, i + MAX_EVENTS_PER_REQUEST)).catch(() => {
						// Telemetry is best-effort: drop the batch rather than retry.
					})
				)
			}
		}
		await Promise.all(inflight)
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

/**
 * Record a hub script the user or the AI settled on.
 *
 * Takes the structured fields the hub API returned rather than a
 * `hub/<version>/<app>/<slug>` path. A path stored in a flow is workspace-authored
 * text that nothing validates against the hub, so its segments could hold any name
 * a user wrote; these fields came from the hub itself and are safe to report.
 *
 * `considered` rather than `picked` for the AI: the AI pulls a handful of
 * candidates' content before choosing between them, and nothing downstream records
 * which one it went on to use.
 *
 * A script from a private hub is still the customer's own content, so at or above
 * `PRIVATE_HUB_MIN_VERSION` only the fact that one was used is recorded.
 */
export function logHubScriptPick(
	script: { version_id: number; app: string; summary: string },
	origin: 'picker' | 'ai'
): void {
	logFeatureUsage('hub_script', origin === 'ai' ? 'considered_ai' : 'picked', {
		key: hubScriptUsageKey(script)
	})
}

const PRIVATE_HUB_KEY = 'private'

/** Lowercase, with every run of other characters collapsed to a single `_`. */
function slugify(value: string): string {
	return value
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '_')
		.replace(/^_+|_+$/g, '')
}

export function hubScriptUsageKey(script: {
	version_id: number
	app: string
	summary: string
}): string {
	if (!Number.isInteger(script.version_id) || script.version_id >= PRIVATE_HUB_MIN_VERSION) {
		return PRIVATE_HUB_KEY
	}
	// Slugified rather than shape-checked: hub summaries carry commas, apostrophes
	// and parentheses, and rejecting those would file real public scripts under
	// `private` and undercount exactly the integrations this is meant to surface.
	const app = slugify(script.app)
	const summary = slugify(script.summary)
	if (!app) return PRIVATE_HUB_KEY
	return (summary ? `${app}/${summary}` : app).slice(0, 100)
}
