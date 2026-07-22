// Pure logic for the "project = folder" Hub bundle. A project is one folder
// `f/<slug>/...`. Bundling: collect the transitive closure, relocate external
// refs (`u/<user>/<name>`, `f/<other>/<name>` -> `f/<slug>/<name>`, `_2`/`_3`…
// on collision) and rewrite them. Hub refs stay external; runtime string-concat
// paths are out of scope. No API/Svelte deps so it's unit-testable.

import { getAllModules } from '$lib/components/flows/flowExplorer'
import { isRunnableByPath } from '$lib/components/apps/inputType'

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
	// failure module) so each module only needs local inspection; the
	// preprocessor module sits outside `modules` and is walked the same way.
	for (const mod of allFlowModules(value)) {
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
	// flow_env values support `$res:path` references.
	const env = value?.flow_env
	if (env && typeof env === 'object') {
		for (const key of Object.keys(env)) {
			if (typeof env[key] !== 'string') continue
			for (const r of extractScriptRefs(env[key])) add('resource', r.path)
		}
	}
	return out
}

// Every module of a flow value: the tree under `modules`, the failure module,
// and the preprocessor module (which lives outside `modules`).
function allFlowModules(value: any) {
	return getAllModules(
		[...(value?.modules ?? []), ...(value?.preprocessor_module ? [value.preprocessor_module] : [])],
		value?.failure_module
	)
}

// Visit every object node in an app value tree (JSON-safe, no cycles).
function walkAppNodes(value: any, visit: (node: Record<string, any>) => void): void {
	if (value == null || typeof value !== 'object') return
	if (Array.isArray(value)) {
		for (const v of value) walkAppNodes(v, visit)
		return
	}
	visit(value)
	for (const k of Object.keys(value)) walkAppNodes(value[k], visit)
}

// `runnableByPath`/`path` nodes reference a workspace runnable by path.
function runnableRef(node: Record<string, any>): Ref | undefined {
	if (!isRunnableByPath(node as any) || typeof node.path !== 'string') return undefined
	if (node.runType === 'flow') return { kind: 'flow', path: node.path }
	if (node.runType === 'script') return { kind: 'script', path: node.path }
	return undefined // hubscript -> external hub, ignored
}

// App refs: `$res:` resources anywhere in the value, plus script/flow runnables
// referenced by path in components.
export function extractAppRefs(value: any): Ref[] {
	const out: Ref[] = []
	const seen = new Set<string>()
	const add = (kind: RefKind, path: string) => {
		const key = `${kind}:${path}`
		if (!seen.has(key)) {
			seen.add(key)
			out.push({ kind, path })
		}
	}
	walkAppNodes(value, (node) => {
		const r = runnableRef(node)
		if (r) add(r.kind, r.path)
	})
	for (const r of extractScriptRefs(JSON.stringify(value ?? {}))) add('resource', r.path)
	return out
}

/**
 * Build the relocation map. Internal paths (`f/<slug>/...`) map to themselves
 * and are reserved first; external paths relocate to `f/<slug>/<name>` (`_2`/`_3`…
 * on collision). Input is sorted so suffix assignment is deterministic.
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

/**
 * Trigger configs reference resources as plain path strings (e.g.
 * `kafka_resource_path: "f/slug/db"`), not `$res:` tokens, so token rewriting
 * misses them. Deep-walk the config and remap any string that exact-matches a
 * map key (map keys are full bundle paths, so an exact match is a reference),
 * or a `script/<path>`/`flow/<path>` handler reference (schedules' on_failure
 * et al.), falling back to `$res:` token rewriting for embedded refs.
 */
/**
 * `$res:`/`res://` tokens anywhere in a trigger config — schedule args,
 * on_*_extra_args, error_handler_args, … (e.g. the built-in Slack handler
 * stores its channel resource this way). These must enter the bundle path map
 * so `rewriteTriggerConfig` relocates them and a stub is exported.
 */
export function extractTriggerConfigResourceRefs(config: any): string[] {
	return extractScriptRefs(JSON.stringify(config ?? {})).map((r) => r.path)
}

export function rewriteTriggerConfig(config: any, map: Map<string, string>): any {
	if (typeof config === 'string') {
		const direct = map.get(config)
		if (direct) return direct
		const handler = /^(script|flow)\/(.+)$/.exec(config)
		if (handler && map.has(handler[2])) return `${handler[1]}/${map.get(handler[2])}`
		// Websocket URLs can be runnables: $script:<path> / $flow:<path>.
		const urlRunnable = /^\$(script|flow):(.+)$/.exec(config)
		if (urlRunnable && map.has(urlRunnable[2]))
			return `$${urlRunnable[1]}:${map.get(urlRunnable[2])}`
		return rewriteContent(config, map)
	}
	if (Array.isArray(config)) return config.map((v) => rewriteTriggerConfig(v, map))
	if (config && typeof config === 'object') {
		return Object.fromEntries(
			Object.entries(config).map(([k, v]) => [k, rewriteTriggerConfig(v, map)])
		)
	}
	return config
}

export function rewriteFlowValue(value: any, map: Map<string, string>): any {
	const cloned = JSON.parse(JSON.stringify(value ?? {}))
	for (const mod of allFlowModules(cloned)) {
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
	const env = cloned?.flow_env
	if (env && typeof env === 'object') {
		for (const key of Object.keys(env)) {
			if (typeof env[key] === 'string') env[key] = rewriteContent(env[key], map)
		}
	}
	return cloned
}

// Relocate `$res:` tokens (one round-trip, also produces a fresh clone) then
// runnable-by-path refs structurally. Incidental `f/<slug>/` strings stay intact.
export function rewriteAppValue(value: any, map: Map<string, string>): any {
	if (value == null) return value
	const cloned = JSON.parse(rewriteContent(JSON.stringify(value), map))
	walkAppNodes(cloned, (node) => {
		if (runnableRef(node) && map.has(node.path)) node.path = map.get(node.path)
	})
	return cloned
}

// Raw/compiled apps store their structure as a JSON string (`{ runnables, files }`).
// Parse it so runnable-by-path refs in the runnables map are seen, reusing the
// same walk; fall back to plain `$res:` scanning if it isn't valid JSON.
export function extractRawAppRefs(content: string): Ref[] {
	let parsed: any
	try {
		parsed = JSON.parse(content)
	} catch {
		return extractScriptRefs(content)
	}
	return extractAppRefs(parsed)
}

export function rewriteRawAppContent(content: string, map: Map<string, string>): string {
	let parsed: any
	try {
		parsed = JSON.parse(content)
	} catch {
		return rewriteContent(content, map)
	}
	return JSON.stringify(rewriteAppValue(parsed, map))
}

// ---------------------------------------------------------------------------
// Hub project export format (what /projects/{slug}/export returns) and its
// retargeting into a destination folder. Kept here, next to the rewriters,
// so the bundle format is defined in one module for both publish and install.
// ---------------------------------------------------------------------------

export type ExportItem = Record<string, any>
export interface ProjectMigration {
	datatable_name: string
	sql: string
	sql_down?: string
	enabled: boolean
}
export interface ProjectExport {
	project: { slug: string; name: string; summary: string; readme: string | null }
	scripts: ExportItem[]
	flows: ExportItem[]
	apps: ExportItem[]
	resources: ExportItem[]
	triggers: ExportItem[]
	migrations?: ProjectMigration[]
}

// Map bundled paths `f/<fromSlug>/...` -> `f/<folder>/...`. Only enumerated
// paths go in, so rewriters touch real refs, never incidental text.
export function buildRetargetMap(
	bundle: ProjectExport,
	fromSlug: string,
	folder: string
): Map<string, string> {
	const map = new Map<string, string>()
	const prefix = `f/${fromSlug}/`
	const add = (p: unknown) => {
		if (typeof p === 'string' && p.startsWith(prefix)) {
			map.set(p, `f/${folder}/${p.slice(prefix.length)}`)
		}
	}
	for (const s of bundle.scripts) add(s.path)
	for (const f of bundle.flows) add(f.path)
	for (const a of bundle.apps) add(a.path)
	for (const r of bundle.resources) add(r.path)
	for (const t of bundle.triggers) {
		add(t.path)
		add(t.runnable_path)
	}
	return map
}

// Structural retarget: rewrite each item's path and its internal refs,
// leaving Hub refs and arbitrary content untouched.
export function retargetProjectExport(
	bundle: ProjectExport,
	fromSlug: string,
	folder: string
): ProjectExport {
	if (folder === fromSlug) return bundle
	const map = buildRetargetMap(bundle, fromSlug, folder)
	const remap = (p: unknown) => (typeof p === 'string' ? (map.get(p) ?? p) : p)
	return {
		...bundle,
		scripts: bundle.scripts.map((s) => ({
			...s,
			path: remap(s.path),
			content: rewriteContent(s.content ?? '', map)
		})),
		flows: bundle.flows.map((f) => ({
			...f,
			path: remap(f.path),
			value: rewriteFlowValue(f.value, map)
		})),
		apps: bundle.apps.map((a) => ({
			...a,
			path: remap(a.path),
			// Raw apps keep their structure in the `value.raw` JSON string.
			value:
				a.app_type === 'raw'
					? { ...a.value, raw: rewriteRawAppContent(a.value?.raw ?? '', map) }
					: rewriteAppValue(a.value, map)
		})),
		resources: bundle.resources.map((r) => ({ ...r, path: remap(r.path) })),
		triggers: bundle.triggers.map((t) => ({
			...t,
			path: remap(t.path),
			runnable_path: remap(t.runnable_path),
			// Configs hold both `$res:` tokens and plain resource paths
			// (kafka_resource_path etc.) — rewrite both.
			config: t.config ? rewriteTriggerConfig(t.config, map) : t.config
		}))
	}
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
	if (item.kind === 'raw_app') return extractRawAppRefs(item.content ?? '')
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

	// Key by `${kind}:${path}`, not bare path: a script and flow can share a path,
	// and keying by path alone would silently drop one.
	const refKey = (kind: string, path: string) => `${kind}:${path}`

	// Refs at the same BFS depth are independent: fetch each level concurrently.
	let level: ItemRef[] = []
	for (const s of seed) {
		const key = refKey(s.kind, s.path)
		if (!queued.has(key)) {
			queued.add(key)
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
			fetched.set(refKey(ref.kind, ref.path), item)
			for (const r of refsForFetched(item)) {
				if (classifyPath(r.path, slug) === 'hub') continue
				if (r.kind === 'resource') {
					resourcePaths.add(r.path)
				} else if (r.kind === 'script' || r.kind === 'flow') {
					const key = refKey(r.kind, r.path)
					if (!queued.has(key)) {
						queued.add(key)
						next.push({ kind: r.kind, path: r.path })
					}
				}
			}
		}
		level = next
	}

	const fetchedItems = [...fetched.values()]
	const itemPaths = fetchedItems.map((it) => it.path)
	const map = buildPathMap([...itemPaths, ...resourcePaths], slug)

	const items: BundledItem[] = fetchedItems.map((it) => {
		const rewritten: BundledItem = { ...it, newPath: map.get(it.path) ?? it.path }
		if (it.kind === 'script') {
			rewritten.content = rewriteContent(it.content ?? '', map)
		} else if (it.kind === 'raw_app') {
			rewritten.content = rewriteRawAppContent(it.content ?? '', map)
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
