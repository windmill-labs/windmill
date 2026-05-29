// Pure logic for the "project = folder" Hub bundle (Stage 1).
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

export type RefKind = 'resource' | 'script'

export interface Ref {
	kind: RefKind
	/** Bare path, without the `$res:` / `res://` prefix for resources. */
	path: string
}

export type PathClass = 'internal' | 'hub' | 'external'

/** A single `$res:PATH` / `res://PATH` token (path captured in group 1). */
const RES_TOKEN_RE = /(?:\$res:|res:\/\/)([\w\-./]+)/g

/** Classify a path relative to the project folder. */
export function classifyPath(path: string, slug: string): PathClass {
	if (path.startsWith(`f/${slug}/`) || path === `f/${slug}`) return 'internal'
	if (path.startsWith('hub/')) return 'hub'
	return 'external'
}

/** Resource references inside a script's code. */
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
	const visitModules = (modules: any[]) => {
		if (!Array.isArray(modules)) return
		for (const mod of modules) visitModule(mod)
	}
	const visitModule = (mod: any) => {
		const v = mod?.value
		if (!v || typeof v !== 'object') return
		if (v.type === 'script' && typeof v.path === 'string') add('script', v.path)
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
		if (Array.isArray(v.modules)) visitModules(v.modules)
		if (Array.isArray(v.branches)) for (const b of v.branches) visitModules(b?.modules)
		if (Array.isArray(v.default)) visitModules(v.default)
	}
	visitModules(value?.modules)
	return out
}

/** References inside an app value (resources only). */
export function extractAppRefs(value: any): Ref[] {
	return extractScriptRefs(JSON.stringify(value ?? {}))
}

/**
 * Build the relocation map for external paths. Each `u/<user>/<name>` or
 * `f/<other>/<name>` maps to `f/<slug>/<name>`; on collision the later entry
 * gets a `_2`, `_3`… suffix. Input is sorted first so the suffix assignment is
 * deterministic regardless of discovery order.
 */
export function buildPathMap(externalPaths: Iterable<string>, slug: string): Map<string, string> {
	const map = new Map<string, string>()
	const used = new Set<string>()
	const sorted = [...new Set(externalPaths)].sort()
	for (const old of sorted) {
		const name = old.split('/').filter(Boolean).pop() ?? old
		let candidate = `f/${slug}/${name}`
		let n = 2
		while (used.has(candidate)) candidate = `f/${slug}/${name}_${n++}`
		used.add(candidate)
		map.set(old, candidate)
	}
	return map
}

/** Rewrite `$res:` / `res://` tokens in code using the map (normalized to `$res:`). */
export function rewriteContent(content: string, map: Map<string, string>): string {
	return content.replace(RES_TOKEN_RE, (whole, path) => {
		const next = map.get(path)
		return next ? `$res:${next}` : whole
	})
}

/** Deep-rewrite a flow value: inline code, static `$res:` inputs, and script paths. */
export function rewriteFlowValue(value: any, map: Map<string, string>): any {
	const cloned = JSON.parse(JSON.stringify(value ?? {}))
	const visitModules = (modules: any[]) => {
		if (!Array.isArray(modules)) return
		for (const mod of modules) visitModule(mod)
	}
	const visitModule = (mod: any) => {
		const v = mod?.value
		if (!v || typeof v !== 'object') return
		if (v.type === 'script' && typeof v.path === 'string' && map.has(v.path)) {
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
		if (Array.isArray(v.modules)) visitModules(v.modules)
		if (Array.isArray(v.branches)) for (const b of v.branches) visitModules(b?.modules)
		if (Array.isArray(v.default)) visitModules(v.default)
	}
	visitModules(cloned?.modules)
	return cloned
}

/** Rewrite resource tokens inside an app value (round-trips via JSON). */
export function rewriteAppValue(value: any, map: Map<string, string>): any {
	if (value == null) return value
	const json = JSON.stringify(value)
	return JSON.parse(rewriteContent(json, map))
}

// ---------------------------------------------------------------------------
// Closure orchestrator (Stage 2)
// ---------------------------------------------------------------------------

export type ItemKind = 'script' | 'flow' | 'app' | 'raw_app'

export interface ItemRef {
	kind: ItemKind
	path: string
}

/** Everything needed to extract refs from, rewrite, and later push an item. */
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

/**
 * Build a self-contained project bundle from a set of seed items.
 *
 * Walks the transitive closure of references: scripts referenced by path are
 * pulled in as items (recursively), resources are pulled in as empty stubs.
 * `hub/...` references are external dependencies and left untouched. Every
 * collected path (items + resources) is relocated under `f/<slug>/...` and all
 * references are rewritten to the new paths.
 */
export async function buildProjectBundle(
	seed: ItemRef[],
	slug: string,
	deps: BundleDeps
): Promise<ProjectBundle> {
	const fetched = new Map<string, FetchedItem>()
	const queued = new Set<string>(seed.map((s) => s.path))
	const queue: ItemRef[] = [...seed]
	const resourcePaths = new Set<string>()
	const unresolved: string[] = []

	while (queue.length > 0) {
		const ref = queue.shift()!
		if (fetched.has(ref.path)) continue
		const item = await deps.fetchItem(ref)
		if (!item) {
			unresolved.push(ref.path)
			continue
		}
		fetched.set(ref.path, item)
		for (const r of refsForFetched(item)) {
			if (classifyPath(r.path, slug) === 'hub') continue
			if (r.kind === 'resource') {
				resourcePaths.add(r.path)
			} else if (r.kind === 'script') {
				if (!fetched.has(r.path) && !queued.has(r.path)) {
					queued.add(r.path)
					queue.push({ kind: 'script', path: r.path })
				}
			}
		}
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
	for (const path of resourcePaths) {
		const type = await deps.resolveResourceType(path)
		if (!type) {
			unresolved.push(path)
			continue
		}
		resourceStubs.push({ originalPath: path, newPath: map.get(path) ?? path, resource_type: type })
	}

	return { items, resourceStubs, unresolved }
}
