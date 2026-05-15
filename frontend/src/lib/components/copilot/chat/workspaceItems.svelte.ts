import { AppService, FlowService, ScriptService } from '$lib/gen'
import { findAndReplace } from 'mdast-util-find-and-replace'
import type { Root } from 'mdast'

export type WindmillItemKind = 'script' | 'flow' | 'app'

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

const itemKindToRoute: Record<WindmillItemKind, string> = {
	script: '/scripts/get',
	flow: '/flows/get',
	app: '/apps/get'
}

/** Build the in-app URL for a resolved workspace item. */
export function itemHref(entry: WorkspaceItemEntry, workspace?: string): string {
	const base = `${itemKindToRoute[entry.kind]}/${entry.path}`
	return workspace ? `${base}?workspace=${workspace}` : base
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
		const [scripts, flows, apps] = await Promise.all([
			ScriptService.listScripts({ workspace }).catch(() => []),
			FlowService.listFlows({ workspace }).catch(() => []),
			AppService.listApps({ workspace }).catch(() => [])
		])

		const map = new Map<string, WorkspaceItemEntry>()
		for (const s of scripts) {
			map.set(s.path, { kind: 'script', path: s.path, summary: s.summary })
		}
		for (const f of flows) {
			map.set(f.path, { kind: 'flow', path: f.path, summary: f.summary })
		}
		for (const a of apps) {
			map.set(a.path, { kind: 'app', path: a.path, summary: a.summary })
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

/**
 * Remark plugin that rewrites Windmill path tokens (`u/...`, `f/...`) into link nodes,
 * but only when the path resolves to a known workspace item.
 *
 * Skips text inside link / inline code / code nodes (find-and-replace only visits Text nodes,
 * which already excludes inlineCode/code; the explicit `ignore` covers links).
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
					return {
						type: 'link',
						url: itemHref(entry, options.workspace),
						title: entry.summary || null,
						data: {
							hProperties: {
								'data-wm-kind': entry.kind,
								'data-wm-path': entry.path,
								target: '_blank',
								rel: 'noopener noreferrer'
							}
						},
						children: [{ type: 'text', value: path }]
					}
				}
			],
			{ ignore: ['link', 'linkReference'] }
		)
	}
}
