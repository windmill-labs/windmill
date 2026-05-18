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
import { itemHref as offboardingItemHref } from '$lib/components/offboarding-utils'
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

const itemKindToOffboardingKind: Record<WindmillItemKind, string> = {
	script: 'scripts',
	flow: 'flows',
	app: 'apps',
	variable: 'variables',
	resource: 'resources',
	schedule: 'schedules',
	http_trigger: 'http_trigger',
	websocket_trigger: 'websocket_trigger',
	kafka_trigger: 'kafka_trigger',
	nats_trigger: 'nats_trigger',
	postgres_trigger: 'postgres_trigger',
	mqtt_trigger: 'mqtt_trigger',
	sqs_trigger: 'sqs_trigger',
	gcp_trigger: 'gcp_trigger',
	azure_trigger: 'azure_trigger',
	email_trigger: 'email_trigger'
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
	const raw = offboardingItemHref(itemKindToOffboardingKind[entry.kind], entry.path) ?? '#'
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

type WorkspaceItemListResult = Array<{ path: string; summary?: string | null }>

const workspaceItemLoaders: Array<{
	kind: WindmillItemKind
	list: (workspace: string) => Promise<WorkspaceItemListResult>
}> = [
	// First writer wins on path collisions. Keep resources before variables because
	// Windmill creates a companion variable for each resource at the same path.
	{ kind: 'script', list: (workspace) => ScriptService.listScripts({ workspace }) },
	{ kind: 'flow', list: (workspace) => FlowService.listFlows({ workspace }) },
	{ kind: 'app', list: (workspace) => AppService.listApps({ workspace }) },
	{ kind: 'resource', list: (workspace) => ResourceService.listResource({ workspace }) },
	{ kind: 'variable', list: (workspace) => VariableService.listVariable({ workspace }) },
	{ kind: 'schedule', list: (workspace) => ScheduleService.listSchedules({ workspace }) },
	{ kind: 'http_trigger', list: (workspace) => HttpTriggerService.listHttpTriggers({ workspace }) },
	{
		kind: 'websocket_trigger',
		list: (workspace) => WebsocketTriggerService.listWebsocketTriggers({ workspace })
	},
	{
		kind: 'kafka_trigger',
		list: (workspace) => KafkaTriggerService.listKafkaTriggers({ workspace })
	},
	{ kind: 'nats_trigger', list: (workspace) => NatsTriggerService.listNatsTriggers({ workspace }) },
	{
		kind: 'postgres_trigger',
		list: (workspace) => PostgresTriggerService.listPostgresTriggers({ workspace })
	},
	{ kind: 'mqtt_trigger', list: (workspace) => MqttTriggerService.listMqttTriggers({ workspace }) },
	{ kind: 'sqs_trigger', list: (workspace) => SqsTriggerService.listSqsTriggers({ workspace }) },
	{ kind: 'gcp_trigger', list: (workspace) => GcpTriggerService.listGcpTriggers({ workspace }) },
	{
		kind: 'azure_trigger',
		list: (workspace) => AzureTriggerService.listAzureTriggers({ workspace })
	},
	{
		kind: 'email_trigger',
		list: (workspace) => EmailTriggerService.listEmailTriggers({ workspace })
	}
]

/**
 * Reactive registry that caches workspace item paths per workspace.
 *
 * - Loads lazily on first call to `ensureLoaded`
 * - Dedups concurrent in-flight loads
 * - Exposes a reactive map (`$state`) so consumers re-render once data lands
 */
class WorkspaceItemRegistry {
	#byWorkspace: Map<string, Map<string, WorkspaceItemEntry>> = $state(new Map())
	#inflight: Map<string, Promise<void>> = new Map()

	private async load(workspace: string): Promise<void> {
		const loadedItems = await Promise.all(
			workspaceItemLoaders.map(async ({ kind, list }) => ({
				kind,
				items: await list(workspace).catch(() => [])
			}))
		)

		const map = new Map<string, WorkspaceItemEntry>()
		for (const { kind, items } of loadedItems) {
			for (const it of items) {
				if (!map.has(it.path)) {
					map.set(it.path, { kind, path: it.path, summary: it.summary ?? undefined })
				}
			}
		}

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
