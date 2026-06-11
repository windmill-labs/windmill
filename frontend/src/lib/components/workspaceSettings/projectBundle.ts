// Pure logic for the "project = folder" Hub bundle.
//
// A project is a single folder `f/<slug>/...`. Bundling it means:
//  1. collect the transitive closure of what the folder references,
//  2. relocate anything referenced from OUTSIDE the folder into it
//     (`u/<user>/<name>` and `f/<other>/<name>` -> `f/<slug>/<name>`, with
//     `_2`, `_3`… on name collisions),
//  3. rewrite every reference to its new in-folder path.
//
// Hub references (`hub/...`) are external dependencies and are left untouched.
// Paths built dynamically at runtime (string concat) can't be detected and are
// out of scope by design.
//
// This module is intentionally free of API/Svelte deps so it can be unit-tested.
// The async closure orchestrator that fetches items lives in the component.

import { getAllModules } from '$lib/components/flows/flowExplorer'

export type RefKind = 'resource' | 'script' | 'flow'

export interface Ref {
	kind: RefKind
	/** Bare path, without the `$res:` / `res://` prefix for resources. */
	path: string
}

export type PathClass = 'internal' | 'hub' | 'external'

/** A single `$res:PATH` / `res://PATH` token (path captured in group 1). */
const RES_TOKEN_RE = /(?:\$res:|res:\/\/)([\w\-./]+)/g

export function classifyPath(path: string, slug: string): PathClass {
	if (path.startsWith(`f/${slug}/`) || path === `f/${slug}`) return 'internal'
	if (path.startsWith('hub/')) return 'hub'
	return 'external'
}

export function extractScriptRefs(content: string): Ref[] {
	const out: Ref[] = []
	const seen = new Set<string>()
	let m: RegExpExecArray | null
	RES_TOKEN_RE.lastIndex = 0
	while ((m = RES_TOKEN_RE.exec(content)) !== null) {
		if (!seen.has(m[1])) {
			seen.add(m[1])
			out.push({ kind: 'resource', path: m[1] })
		}
	}
	return out
}

/**
 * References inside a flow value:
 *  - inline rawscript code with `$res:` (resource)
 *  - static step inputs whose value is a `$res:` literal (resource)
 *  - `type: script` steps that reference a script by path (script)
 *  - `type: flow` steps that reference a sub-flow by path (flow)
 */
export function extractFlowRefs(value: any): Ref[] {
	const out: Ref[] = []
	const seen = new Set<string>()
	const add = (kind: RefKind, path: string) => {
		const key = `${kind}:${path}`
		if (!seen.has(key)) {
			seen.add(key)
			out.push({ kind, path })
		}
	}
	// getAllModules flattens the whole tree (loops, branches, aiagent tools,
	// failure module) so each module only needs local inspection.
	for (const mod of getAllModules(value?.modules ?? [], value?.failure_module)) {
		const v: any = (mod as any)?.value
		if (!v || typeof v !== 'object') continue
		if (v.type === 'script' && typeof v.path === 'string') add('script', v.path)
		if (v.type === 'flow' && typeof v.path === 'string') add('flow', v.path)
		if (typeof v.content === 'string') {
			for (const r of extractScriptRefs(v.content)) add('resource', r.path)
		}
		const it = v.input_transforms
		if (it && typeof it === 'object') {
			for (const key of Object.keys(it)) {
				const t = it[key]
				if (t?.type === 'static' && typeof t.value === 'string') {
					const sm = /^\s*(?:\$res:|res:\/\/)([\w\-./]+)\s*$/.exec(t.value)
					if (sm) add('resource', sm[1])
				}
			}
		}
	}
	return out
}

export function extractAppRefs(value: any): Ref[] {
	return extractScriptRefs(JSON.stringify(value ?? {}))
}

/**
 * Build the relocation map. Paths already inside the project folder
 * (`f/<slug>/...`) map to themselves — their structure and subfolder depth are
 * preserved. Only external paths (`u/<user>/<name>` or `f/<other>/<name>`) are
 * relocated to `f/<slug>/<name>`; on collision the later entry gets a `_2`,
 * `_3`… suffix. Internal paths are reserved first so an external one can never
 * land on an occupied internal path. Input is sorted so suffix assignment is
 * deterministic regardless of discovery order.
 */
export function buildPathMap(paths: Iterable<string>, slug: string): Map<string, string> {
	const map = new Map<string, string>()
	const used = new Set<string>()
	const sorted = [...new Set(paths)].sort()
	for (const p of sorted) {
		if (classifyPath(p, slug) === 'internal') {
			map.set(p, p)
			used.add(p)
		}
	}
	for (const old of sorted) {
		if (map.has(old)) continue
		const name = old.split('/').filter(Boolean).pop() ?? old
		let candidate = `f/${slug}/${name}`
		let n = 2
		while (used.has(candidate)) candidate = `f/${slug}/${name}_${n++}`
		used.add(candidate)
		map.set(old, candidate)
	}
	return map
}

// Both ref forms normalize to `$res:` on rewrite.
export function rewriteContent(content: string, map: Map<string, string>): string {
	return content.replace(RES_TOKEN_RE, (whole, path) => {
		const next = map.get(path)
		return next ? `$res:${next}` : whole
	})
}

export function rewriteFlowValue(value: any, map: Map<string, string>): any {
	const cloned = JSON.parse(JSON.stringify(value ?? {}))
	for (const mod of getAllModules(cloned?.modules ?? [], cloned?.failure_module)) {
		const v: any = (mod as any)?.value
		if (!v || typeof v !== 'object') continue
		if (
			(v.type === 'script' || v.type === 'flow') &&
			typeof v.path === 'string' &&
			map.has(v.path)
		) {
			v.path = map.get(v.path)
		}
		if (typeof v.content === 'string') v.content = rewriteContent(v.content, map)
		const it = v.input_transforms
		if (it && typeof it === 'object') {
			for (const key of Object.keys(it)) {
				const t = it[key]
				if (t?.type === 'static' && typeof t.value === 'string') {
					const sm = /^(\s*)(?:\$res:|res:\/\/)([\w\-./]+)(\s*)$/.exec(t.value)
					if (sm && map.has(sm[2])) t.value = `${sm[1]}$res:${map.get(sm[2])}${sm[3]}`
				}
			}
		}
	}
	return cloned
}

// Round-trips through JSON since app values are opaque.
function rewriteAppValue(value: any, map: Map<string, string>): any {
	if (value == null) return value
	const json = JSON.stringify(value)
	return JSON.parse(rewriteContent(json, map))
}

export type ItemKind = 'script' | 'flow' | 'app' | 'raw_app'

export interface ItemRef {
	kind: ItemKind
	path: string
}

export interface FetchedItem {
	kind: ItemKind
	path: string
	summary?: string
	description?: string
	/** scripts + raw_apps */
	content?: string
	/** flows + apps */
	value?: any
	/** scripts */
	language?: string
	schema?: any
	lock?: string
	scriptKind?: string
}

export interface BundleDeps {
	/** Fetch a workspace item by ref, or undefined if it doesn't exist. */
	fetchItem: (ref: ItemRef) => Promise<FetchedItem | undefined>
	/** Resolve a resource path to its type, or undefined if missing. */
	resolveResourceType: (path: string) => Promise<string | undefined>
}

export interface BundledItem extends FetchedItem {
	/** Path the item takes inside the project folder. */
	newPath: string
}

export interface ResourceStub {
	originalPath: string
	newPath: string
	resource_type: string
}

export interface ProjectBundle {
	items: BundledItem[]
	resourceStubs: ResourceStub[]
	/** Original -> relocated path for every item and resource (incl. unresolved). */
	pathMap: Map<string, string>
	/** External paths we couldn't fetch/resolve (missing items or untyped resources). */
	unresolved: string[]
}

function refsForFetched(item: FetchedItem): Ref[] {
	if (item.kind === 'script') return extractScriptRefs(item.content ?? '')
	if (item.kind === 'flow') return extractFlowRefs(item.value)
	if (item.kind === 'app') return extractAppRefs(item.value)
	if (item.kind === 'raw_app') return extractScriptRefs(item.content ?? '')
	return []
}

// Walks the transitive closure: scripts referenced by path are pulled in
// recursively, resources become empty stubs, hub refs stay external.
export async function buildProjectBundle(
	seed: ItemRef[],
	slug: string,
	deps: BundleDeps,
	extraResourcePaths: string[] = []
): Promise<ProjectBundle> {
	const fetched = new Map<string, FetchedItem>()
	const queued = new Set<string>()
	const resourcePaths = new Set<string>()
	const unresolved: string[] = []

	// Resources referenced by triggers (by config path, not `$res:` in code).
	for (const p of extraResourcePaths) {
		if (classifyPath(p, slug) !== 'hub') resourcePaths.add(p)
	}

	// Refs at the same BFS depth are independent: fetch each level concurrently.
	let level: ItemRef[] = []
	for (const s of seed) {
		if (!queued.has(s.path)) {
			queued.add(s.path)
			level.push(s)
		}
	}
	while (level.length > 0) {
		const results = await Promise.all(
			level.map(async (ref) => ({ ref, item: await deps.fetchItem(ref) }))
		)
		const next: ItemRef[] = []
		for (const { ref, item } of results) {
			if (!item) {
				unresolved.push(ref.path)
				continue
			}
			fetched.set(ref.path, item)
			for (const r of refsForFetched(item)) {
				if (classifyPath(r.path, slug) === 'hub') continue
				if (r.kind === 'resource') {
					resourcePaths.add(r.path)
				} else if (r.kind === 'script' || r.kind === 'flow') {
					if (!queued.has(r.path)) {
						queued.add(r.path)
						next.push({ kind: r.kind, path: r.path })
					}
				}
			}
		}
		level = next
	}

	const itemPaths = [...fetched.keys()]
	const map = buildPathMap([...itemPaths, ...resourcePaths], slug)

	const items: BundledItem[] = itemPaths.map((path) => {
		const it = fetched.get(path)!
		const rewritten: BundledItem = { ...it, newPath: map.get(path) ?? path }
		if (it.kind === 'script' || it.kind === 'raw_app') {
			rewritten.content = rewriteContent(it.content ?? '', map)
		} else if (it.kind === 'flow') {
			rewritten.value = rewriteFlowValue(it.value, map)
		} else if (it.kind === 'app') {
			rewritten.value = rewriteAppValue(it.value, map)
		}
		return rewritten
	})

	const resourceStubs: ResourceStub[] = []
	const resolved = await Promise.all(
		[...resourcePaths].map(async (path) => ({ path, type: await deps.resolveResourceType(path) }))
	)
	for (const { path, type } of resolved) {
		if (!type) {
			unresolved.push(path)
			continue
		}
		resourceStubs.push({ originalPath: path, newPath: map.get(path) ?? path, resource_type: type })
	}

	return { items, resourceStubs, pathMap: map, unresolved }
}
