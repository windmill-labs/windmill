import {
	AppService,
	AzureTriggerService,
	EmailTriggerService,
	FlowService,
	GcpTriggerService,
	HttpTriggerService,
	KafkaTriggerService,
	MqttTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ResourceService,
	ScheduleService,
	ScriptService,
	SqsTriggerService,
	VariableService,
	WebsocketTriggerService
} from '$lib/gen'
import { findAndReplace } from 'mdast-util-find-and-replace'
import { visit } from 'unist-util-visit'
import type { Root, InlineCode, Link } from 'mdast'

export type WindmillItemKind =
	| 'script'
	| 'flow'
	| 'app'
	| 'variable'
	| 'resource'
	| 'schedule'
	| 'http_trigger'
	| 'websocket_trigger'
	| 'kafka_trigger'
	| 'nats_trigger'
	| 'postgres_trigger'
	| 'mqtt_trigger'
	| 'sqs_trigger'
	| 'gcp_trigger'
	| 'azure_trigger'
	| 'email_trigger'

export interface WorkspaceItemEntry {
	kind: WindmillItemKind
	path: string
	summary?: string
}

/**
 * Matches Windmill paths of the form `u/<owner>/<path>` or `f/<folder>/<path>`.
 *
 * Owners may contain `[A-Za-z0-9._-]`; the trailing path can include dots and slashes
 * (sub-paths, version segments) but must end on an alphanumeric / underscore / hyphen —
 * this prevents the regex from gobbling sentence punctuation like the period in
 * "look at f/foo/bar."
 *
 * The negative lookbehind prevents matches embedded in URLs or longer identifiers.
 */
export const WINDMILL_PATH_REGEX =
	/(?<![A-Za-z0-9/_.\-])([uf]\/[A-Za-z0-9_.\-]+\/[A-Za-z0-9_./\-]*[A-Za-z0-9_\-])/g

/**
 * Anchored variant of {@link WINDMILL_PATH_REGEX} for use against a whole string —
 * matches when the entire input is exactly a path (after trimming).
 */
const WINDMILL_PATH_EXACT_REGEX = /^[uf]\/[A-Za-z0-9_.\-]+\/[A-Za-z0-9_./\-]*[A-Za-z0-9_\-]$/

/**
 * URL builder per kind.
 *
 * - For scripts/flows/apps we link to the dedicated `/get/{path}` page.
 * - For everything else, link to the relevant list page with a hash fragment that the
 *   list page reads on mount to pop the editor drawer open. Hash format mirrors what
 *   each list page already expects (`#<path>` for most, `#/resource/<path>` for the
 *   resources page).
 */
const itemKindToHref: Record<WindmillItemKind, (path: string) => string> = {
	script: (p) => `/scripts/get/${p}`,
	flow: (p) => `/flows/get/${p}`,
	app: (p) => `/apps/get/${p}`,
	variable: (p) => `/variables#${p}`,
	resource: (p) => `/resources#/resource/${p}`,
	schedule: (p) => `/schedules#${p}`,
	http_trigger: (p) => `/routes#${p}`,
	websocket_trigger: (p) => `/websocket_triggers/#${p}`,
	kafka_trigger: (p) => `/kafka_triggers/#${p}`,
	nats_trigger: (p) => `/nats_triggers/#${p}`,
	postgres_trigger: (p) => `/postgres_triggers/#${p}`,
	mqtt_trigger: (p) => `/mqtt_triggers/#${p}`,
	sqs_trigger: (p) => `/sqs_triggers/#${p}`,
	gcp_trigger: (p) => `/gcp_triggers/#${p}`,
	azure_trigger: (p) => `/azure_triggers/#${p}`,
	email_trigger: (p) => `/email_triggers/#${p}`
}

/** Kinds whose items can be opened in an inline drawer from the chat. */
const DRAWERABLE_KINDS = new Set<WindmillItemKind>(['variable', 'resource'])

/** Whether the chat pill should expose an "open in drawer" affordance for this kind. */
export function hasInlineDrawer(kind: WindmillItemKind): boolean {
	return DRAWERABLE_KINDS.has(kind)
}

/**
 * Build the in-app URL for a resolved workspace item.
 *
 * The workspace query param goes before the hash so the SvelteKit router still applies
 * it; the hash fragment is consumed client-side by the list page on mount to open the
 * matching drawer.
 */
export function itemHref(entry: WorkspaceItemEntry, workspace?: string): string {
	const raw = itemKindToHref[entry.kind](entry.path)
	if (!workspace) return raw
	const hashIdx = raw.indexOf('#')
	if (hashIdx === -1) {
		return raw.includes('?') ? `${raw}&workspace=${workspace}` : `${raw}?workspace=${workspace}`
	}
	const pathPart = raw.slice(0, hashIdx)
	const hashPart = raw.slice(hashIdx)
	const sep = pathPart.includes('?') ? '&' : '?'
	return `${pathPart}${sep}workspace=${workspace}${hashPart}`
}

/**
 * Reactive registry that caches script/flow/app paths per workspace.
 *
 * - Loads lazily on first call to `ensureLoaded`
 * - Dedups concurrent in-flight loads
 * - Exposes a reactive map (`$state`) so consumers re-render once data lands
 */
class WorkspaceItemRegistry {
	#byWorkspace: Map<string, Map<string, WorkspaceItemEntry>> = $state(new Map())
	#inflight: Map<string, Promise<void>> = new Map()

	private async load(workspace: string): Promise<void> {
		// Fire every list endpoint in parallel. `.catch(() => [])` keeps a single failing
		// endpoint from poisoning the whole snapshot — the registry just records empty
		// for that kind, the rest still resolve.
		const [
			scripts,
			flows,
			apps,
			variables,
			resources,
			schedules,
			httpTriggers,
			wsTriggers,
			kafkaTriggers,
			natsTriggers,
			pgTriggers,
			mqttTriggers,
			sqsTriggers,
			gcpTriggers,
			azureTriggers,
			emailTriggers
		] = await Promise.all([
			ScriptService.listScripts({ workspace }).catch(() => []),
			FlowService.listFlows({ workspace }).catch(() => []),
			AppService.listApps({ workspace }).catch(() => []),
			VariableService.listVariable({ workspace }).catch(() => []),
			ResourceService.listResource({ workspace }).catch(() => []),
			ScheduleService.listSchedules({ workspace }).catch(() => []),
			HttpTriggerService.listHttpTriggers({ workspace }).catch(() => []),
			WebsocketTriggerService.listWebsocketTriggers({ workspace }).catch(() => []),
			KafkaTriggerService.listKafkaTriggers({ workspace }).catch(() => []),
			NatsTriggerService.listNatsTriggers({ workspace }).catch(() => []),
			PostgresTriggerService.listPostgresTriggers({ workspace }).catch(() => []),
			MqttTriggerService.listMqttTriggers({ workspace }).catch(() => []),
			SqsTriggerService.listSqsTriggers({ workspace }).catch(() => []),
			GcpTriggerService.listGcpTriggers({ workspace }).catch(() => []),
			AzureTriggerService.listAzureTriggers({ workspace }).catch(() => []),
			EmailTriggerService.listEmailTriggers({ workspace }).catch(() => [])
		])

		const map = new Map<string, WorkspaceItemEntry>()
		const add = (
			items: Array<{ path: string; summary?: string | null }>,
			kind: WindmillItemKind
		) => {
			for (const it of items) {
				// First writer wins — scripts/flows/apps are checked first so they win over
				// triggers if a path collides (extremely unlikely but possible across kinds).
				if (!map.has(it.path)) {
					map.set(it.path, { kind, path: it.path, summary: it.summary ?? undefined })
				}
			}
		}
		// Order matters because of first-writer-wins on path collisions. Resources are
		// added before variables: Windmill auto-creates a companion variable at the same
		// path for every resource, and a path written by the user almost always means the
		// resource, not the hidden variable.
		add(scripts, 'script')
		add(flows, 'flow')
		add(apps, 'app')
		add(resources, 'resource')
		add(variables, 'variable')
		add(schedules, 'schedule')
		add(httpTriggers, 'http_trigger')
		add(wsTriggers, 'websocket_trigger')
		add(kafkaTriggers, 'kafka_trigger')
		add(natsTriggers, 'nats_trigger')
		add(pgTriggers, 'postgres_trigger')
		add(mqttTriggers, 'mqtt_trigger')
		add(sqsTriggers, 'sqs_trigger')
		add(gcpTriggers, 'gcp_trigger')
		add(azureTriggers, 'azure_trigger')
		add(emailTriggers, 'email_trigger')

		// Build a new outer map to trigger reactivity on consumers using $derived.
		const next = new Map(this.#byWorkspace)
		next.set(workspace, map)
		this.#byWorkspace = next
	}

	/** Ensure the workspace items are loaded. Returns the in-flight promise if any. */
	ensureLoaded(workspace: string): Promise<void> {
		if (!workspace) return Promise.resolve()
		if (this.#byWorkspace.has(workspace)) return Promise.resolve()
		let pending = this.#inflight.get(workspace)
		if (!pending) {
			pending = this.load(workspace).finally(() => this.#inflight.delete(workspace))
			this.#inflight.set(workspace, pending)
		}
		return pending
	}

	/** Synchronously resolve a path. Returns undefined if the workspace isn't loaded yet. */
	resolve(workspace: string, path: string): WorkspaceItemEntry | undefined {
		if (!workspace) return undefined
		return this.#byWorkspace.get(workspace)?.get(path)
	}

	/** Whether the registry has data for the given workspace. */
	isLoaded(workspace: string): boolean {
		return this.#byWorkspace.has(workspace)
	}

	/** Drop the cached data for a workspace (or all workspaces). */
	invalidate(workspace?: string): void {
		if (workspace) {
			if (!this.#byWorkspace.has(workspace)) return
			const next = new Map(this.#byWorkspace)
			next.delete(workspace)
			this.#byWorkspace = next
		} else {
			this.#byWorkspace = new Map()
		}
	}

	/** All items known for the workspace; empty if not yet loaded. */
	items(workspace: string): WorkspaceItemEntry[] {
		const map = this.#byWorkspace.get(workspace)
		return map ? [...map.values()] : []
	}
}

export const workspaceItemRegistry = new WorkspaceItemRegistry()

/** Extract every Windmill-looking path from raw text (no resolution against the registry). */
export function extractCandidatePaths(text: string | undefined | null): string[] {
	if (!text) return []
	const seen = new Set<string>()
	for (const match of text.matchAll(WINDMILL_PATH_REGEX)) {
		seen.add(match[1])
	}
	return [...seen]
}

/**
 * Resolve every Windmill path mentioned in the given texts against the registry.
 * Returns deduped entries (one per unique path).
 */
export function resolveMentionedItems(
	texts: Array<string | undefined | null>,
	workspace: string
): WorkspaceItemEntry[] {
	if (!workspace) return []
	const seen = new Map<string, WorkspaceItemEntry>()
	for (const text of texts) {
		for (const candidate of extractCandidatePaths(text)) {
			if (seen.has(candidate)) continue
			const entry = workspaceItemRegistry.resolve(workspace, candidate)
			if (entry) seen.set(candidate, entry)
		}
	}
	return [...seen.values()]
}

/** Build the link node used to replace a resolved path token. */
function buildPathLinkNode(
	entry: WorkspaceItemEntry,
	displayPath: string,
	workspace: string | undefined
): Link {
	return {
		type: 'link',
		url: itemHref(entry, workspace),
		title: entry.summary || null,
		data: {
			hProperties: {
				'data-wm-kind': entry.kind,
				'data-wm-path': entry.path,
				target: '_blank',
				rel: 'noopener noreferrer'
			}
		},
		children: [{ type: 'text', value: displayPath }]
	}
}

/**
 * Remark plugin that rewrites Windmill path tokens (`u/...`, `f/...`) into link nodes,
 * but only when the path resolves to a known workspace item.
 *
 * Handles two cases:
 * 1. Bare path tokens in regular text — handled by `findAndReplace`, which only visits Text
 *    nodes (so fenced code and inline code are naturally skipped). We additionally `ignore`
 *    existing `link` nodes so we don't break autolinked URLs.
 * 2. Inline-code spans whose entire content is a single path — handled by a second pass via
 *    `unist-util-visit`. LLMs often wrap identifiers in backticks (`` `u/admin/foo` ``);
 *    when the inline code is *just* a path we treat the backticks as styling and replace
 *    the node with a link pill. Mixed inline-code content (e.g. `` `f/foo + extra text` ``)
 *    is left untouched.
 */
export function remarkWindmillPaths(options: {
	resolve: (path: string) => WorkspaceItemEntry | undefined
	workspace?: string
}) {
	return () => (tree: Root) => {
		findAndReplace(
			tree,
			[
				WINDMILL_PATH_REGEX,
				(_match: string, path: string) => {
					const entry = options.resolve(path)
					if (!entry) return false
					return buildPathLinkNode(entry, path, options.workspace)
				}
			],
			{ ignore: ['link', 'linkReference'] }
		)

		visit(tree, 'inlineCode', (node: InlineCode, index, parent) => {
			if (!parent || typeof index !== 'number') return
			const value = node.value.trim()
			if (!WINDMILL_PATH_EXACT_REGEX.test(value)) return
			const entry = options.resolve(value)
			if (!entry) return
			parent.children[index] = buildPathLinkNode(entry, value, options.workspace) as any
		})
	}
}
