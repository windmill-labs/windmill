// Pure logic for the "project = folder" Hub bundle. A project is one folder
// `f/<slug>/...`. Bundling: collect the transitive closure, relocate external
// refs (`u/<user>/<name>`, `f/<other>/<name>` -> `f/<slug>/<name>`, `_2`/`_3`…
// on collision) and rewrite them. Hub refs stay external; runtime string-concat
// paths are out of scope. No API/Svelte deps so it's unit-testable.

import { getAllModules } from '$lib/components/flows/flowExplorer'
import { isRunnableByPath } from '$lib/components/apps/inputType'

export type RefKind = 'resource' | 'script' | 'flow'

// Who an agent runs as and who may use it belong to the workspace it is deployed in: an
// export must not carry them, and an install must not trust one that does, or the agent
// would run its tools as whoever installed it for anyone who can read it.
const AGENT_WORKSPACE_KEYS = ['on_behalf_of', 'run_on_behalf_of', 'slack_anyone']

export function withoutAgentWorkspaceKeys(value: unknown): Record<string, unknown> {
	const out = { ...((value ?? {}) as Record<string, unknown>) }
	for (const key of AGENT_WORKSPACE_KEYS) delete out[key]
	return out
}

export interface Ref {
	kind: RefKind
	/** Bare path, without the `$res:` / `res://` prefix for resources. */
	path: string
}

export type PathClass = 'internal' | 'hub' | 'external'

/** A single `$res:PATH` / `res://PATH` token (path captured in group 1). */
const RES_TOKEN_RE = /(?:\$res:|res:\/\/)([\w\-./]+)/g

// A whole-string `$var:PATH` / `$jsonvar:PATH` value. The worker substitutes these
// only when an argument value *is* the reference (walking nested JSON), never a
// token embedded in inline code, so the whole value must match. `_KIND` captures
// the prefix (group 1) and path (group 2) so a rewrite can preserve `var`/`jsonvar`.
const VAR_VALUE_RE = /^\$(?:json)?var:([\w\-./]+)$/
const VAR_VALUE_RE_KIND = /^\$(var|jsonvar):([\w\-./]+)$/

// Variable paths a value will resolve at runtime (flow static inputs, flow_env,
// app runnable inputs, trigger config fields). Walk the parsed structure and match
// whole string values so inline code carrying a literal `$var:` string is ignored.
export function extractVarRefsFromValue(value: any): string[] {
	const out = new Set<string>()
	const walk = (v: any) => {
		if (typeof v === 'string') {
			const m = VAR_VALUE_RE.exec(v)
			if (m) out.add(m[1])
		} else if (Array.isArray(v)) {
			for (const x of v) walk(x)
		} else if (v && typeof v === 'object') {
			for (const k of Object.keys(v)) walk(v[k])
		}
	}
	walk(value)
	return [...out]
}

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

/** A resource path written bare or as a `$res:` / `res://` token, without its prefix. */
function bareResourcePath(ref: unknown): string | undefined {
	if (typeof ref !== 'string') return undefined
	return ref.replace(/^\$res:/, '').replace(/^res:\/\//, '') || undefined
}

/** `next` written the way `ref` was: bare, `$res:` or `res://`. */
function withRefPrefix(ref: string, next: string): string {
	const prefix = /^(\$res:|res:\/\/)/.exec(ref)?.[1] ?? ''
	return prefix + next
}

/** A whole-string `$res:PATH` / `res://PATH` value (prefix in group 1, path in group 2). */
const RES_VALUE_RE = /^(\$res:|res:\/\/)(.+)$/

type AgentRefResolver = (kind: RefKind, path: string) => string | undefined
type AgentWalkContext = 'config' | 'inputs' | 'toolInputs' | 'input' | 'data'

// The single definition of what an agent's configuration references, for reading
// (`resolve` returns nothing) and rewriting (it returns the new path) alike. No list of
// fields: a saved agent is read unvalidated and tools nest, so shapes are matched at any depth.
// The install containment check relies on this walk seeing everything the worker acts on.
function walkAgentConfig(value: any, resolve: AgentRefResolver): any {
	const walk = (v: any, ctx: AgentWalkContext): any => {
		if (typeof v === 'string') {
			// Whole-string only, as the worker resolves arguments: a token in prose is inert.
			const m = RES_VALUE_RE.exec(v)
			const next = m ? resolve('resource', m[2]) : undefined
			return next !== undefined ? m![1] + next : v
		}
		if (Array.isArray(v)) return v.map((x) => walk(x, ctx === 'data' ? 'data' : 'config'))
		if (!v || typeof v !== 'object') return v

		const out: Record<string, any> = {}
		for (const k of Object.keys(v)) {
			// Only an entry of `input_transforms` (or of a tool's map under `tool_inputs`) is an
			// input: its static `value` is argument data and its JavaScript `expr` is never
			// resolved. The same keys anywhere else are configuration, or a crafted export
			// could hide a tool beside them.
			let child: AgentWalkContext = 'config'
			if (ctx === 'data') child = 'data'
			else if (ctx === 'inputs') child = 'input'
			else if (ctx === 'toolInputs') child = 'inputs'
			else if (ctx === 'input' && v.type === 'static' && k === 'value') child = 'data'
			else if (ctx === 'input' && v.type === 'javascript' && k === 'expr') {
				out[k] = v[k]
				continue
			} else if (k === 'input_transforms') child = 'inputs'
			else if (k === 'tool_inputs') child = 'toolInputs'
			out[k] = walk(v[k], child)
		}
		if (ctx !== 'config' && ctx !== 'input') return out

		if ((v.type === 'script' || v.type === 'flow') && typeof v.path === 'string') {
			out.path = resolve(v.type, v.path) ?? v.path
		}
		if (v.type === 'aiagent') {
			// Linked to a saved agent, by bare path or `$res:`.
			const linked = bareResourcePath(v.agent)
			const next = linked ? resolve('resource', linked) : undefined
			if (next !== undefined) out.agent = withRefPrefix(v.agent, next)
		}
		if (v.tool_type === 'mcp') {
			const mcp = bareResourcePath(v.resource_path)
			const next = mcp ? resolve('resource', mcp) : undefined
			if (next !== undefined) out.resource_path = withRefPrefix(v.resource_path, next)
		}
		if (v.type === 'rawscript' && typeof v.content === 'string') {
			out.content = v.content.replace(RES_TOKEN_RE, (whole: string, path: string) => {
				const next = resolve('resource', path)
				return next !== undefined ? `$res:${next}` : whole
			})
		}
		return out
	}
	return walk(value, 'config')
}

function collectAgentConfigRefs(value: any, add: (kind: RefKind, path: string) => void): void {
	walkAgentConfig(value, (kind, path) => {
		add(kind, path)
		return undefined
	})
}

/** Returns a rewritten copy. */
function rewriteAgentConfig(value: any, map: Map<string, string>, only?: RefKind): any {
	const out = walkAgentConfig(value, (kind, path) =>
		only === undefined || kind === only ? map.get(path) : undefined
	)
	// `enabled_tools` names an MCP server by its bare resource path and the worker matches
	// names exactly: the list must follow the server, or it silently leaves the roster.
	const fixEnabled = (v: any) => {
		if (Array.isArray(v)) v.forEach(fixEnabled)
		else if (v && typeof v === 'object') {
			const enabled = v.type === 'aiagent' ? v.input_transforms?.enabled_tools : undefined
			if (enabled?.type === 'static' && Array.isArray(enabled.value)) {
				enabled.value = enabled.value.map((name: unknown) =>
					typeof name === 'string' ? (map.get(name) ?? name) : name
				)
			}
			for (const k of Object.keys(v)) fixEnabled(v[k])
		}
	}
	fixEnabled(out)
	return out
}

// What `walkAgentConfig` must read on an `aiagent` flow step. Its module tools are left
// out (`null` keeps the indexes): the flow walk visits those as modules.
function agentStepConfig(v: any): any {
	return {
		type: 'aiagent',
		agent: v.agent,
		tools: (Array.isArray(v.tools) ? v.tools : []).map((t: any) =>
			t?.value?.tool_type === 'mcp' ? t : null
		),
		tool_inputs: v.tool_inputs,
		input_transforms: { enabled_tools: v.input_transforms?.enabled_tools }
	}
}

/**
 * References inside a flow value:
 *  - inline rawscript code with `$res:` (resource)
 *  - static step inputs whose value is a `$res:` literal (resource)
 *  - `type: script` steps that reference a script by path (script)
 *  - `type: flow` steps that reference a sub-flow by path (flow)
 *  - on an `aiagent` step or tool: the saved agent it is linked to (resource, shipped whole
 *    as an agent), its MCP servers (resource), and the tool wiring a linked step lays over
 *    the agent's tools (`tool_inputs`)
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
		if (v.type === 'aiagent') collectAgentConfigRefs(agentStepConfig(v), add)
		if (typeof v.content === 'string') {
			for (const r of extractScriptRefs(v.content)) add('resource', r.path)
		}
		const it = v.input_transforms
		if (it && typeof it === 'object') {
			for (const key of Object.keys(it)) {
				const t = it[key]
				// Static values can be a bare `$res:` string or arbitrary JSON with
				// refs nested anywhere — the worker resolves both, so scan the full
				// serialization.
				if (t?.type === 'static' && t.value !== undefined) {
					const text = typeof t.value === 'string' ? t.value : JSON.stringify(t.value)
					for (const r of extractScriptRefs(text)) add('resource', r.path)
				}
			}
		}
	}
	// flow_env values support `$res:path` references — as whole string values or
	// nested inside JSON values (the worker resolves both), so scan the full
	// serialization.
	if (value?.flow_env && typeof value.flow_env === 'object') {
		for (const r of extractScriptRefs(JSON.stringify(value.flow_env))) add('resource', r.path)
	}
	return out
}

// Every module of a flow value: the tree under `modules`, the failure module,
// and the preprocessor module (which lives outside `modules`). Any walk over a
// flow's modules must go through this — a walk that misses a module class
// silently drops its dependencies from bundles or migrations. All three go in
// the root list (not getAllModules' failure_module parameter, which appends
// the module without expanding its descendants) so nested children of a
// failure or preprocessor module are walked too.
export function allFlowModules(value: any) {
	return getAllModules([
		...(value?.modules ?? []),
		...(value?.preprocessor_module ? [value.preprocessor_module] : []),
		...(value?.failure_module ? [value.failure_module] : [])
	])
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
	// A saved agent is an `ai_agent` resource.
	if (node.runType === 'agent') return { kind: 'resource', path: node.path }
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
 * Build the relocation map. Paths under the source folder (`f/<sourceFolder>/...`,
 * possibly nested like `hub/billing`) keep their sub-path under `f/<slug>/` and are
 * reserved first; other paths relocate to `f/<slug>/<name>` (`_2`/`_3`… on
 * collision). Input is sorted so suffix assignment is deterministic.
 */
export function buildPathMap(
	paths: Iterable<string>,
	slug: string,
	sourceFolder: string = slug
): Map<string, string> {
	const map = new Map<string, string>()
	const used = new Set<string>()
	const sorted = [...new Set(paths)].sort()
	const sourcePrefix = `f/${sourceFolder}/`
	for (const p of sorted) {
		if (p.startsWith(sourcePrefix)) {
			const kept = `f/${slug}/${p.slice(sourcePrefix.length)}`
			map.set(p, kept)
			used.add(kept)
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

// Structurally relocate whole-string `$var:`/`$jsonvar:` values — the only form the
// worker resolves. Walks the parsed value so an inert token embedded in inline code
// or arbitrary text is left untouched, unlike token replacement over serialized
// strings. Only paths present in the map move (the retarget map carries variables).
export function rewriteVarRefsInValue(value: any, map: Map<string, string>): any {
	if (typeof value === 'string') {
		const m = VAR_VALUE_RE_KIND.exec(value)
		if (m) {
			const next = map.get(m[2])
			if (next) return `$${m[1]}:${next}`
		}
		return value
	}
	if (Array.isArray(value)) return value.map((v) => rewriteVarRefsInValue(v, map))
	if (value && typeof value === 'object') {
		const out: Record<string, any> = {}
		for (const k of Object.keys(value)) out[k] = rewriteVarRefsInValue(value[k], map)
		return out
	}
	return value
}

/**
 * `$res:`/`res://` tokens anywhere in a trigger config — schedule args,
 * on_*_extra_args, error_handler_args, … (e.g. the built-in Slack handler
 * stores its channel resource this way). These must enter the bundle path map
 * so `rewriteTriggerConfig` relocates them and a stub is exported.
 */
export function extractTriggerConfigResourceRefs(config: any): string[] {
	return extractScriptRefs(JSON.stringify(config ?? {})).map((r) => r.path)
}

/**
 * Whether a value reaches a resource, in either spelling a rewrite has to handle: a
 * `$res:`/`res://` token embedded in content, or the bare path standing alone as a string
 * the way trigger configs hold it (`kafka_resource_path: "f/slug/db"`).
 *
 * Matching the parsed structure rather than its serialization is what keeps
 * `f/slug/db` out of `$res:f/slug/db_prod`.
 */
export function referencesResourcePath(value: unknown, path: string): boolean {
	const walk = (v: any): boolean => {
		if (typeof v === 'string') {
			if (v === path) return true
			RES_TOKEN_RE.lastIndex = 0
			let m: RegExpExecArray | null
			while ((m = RES_TOKEN_RE.exec(v)) !== null) if (m[1] === path) return true
			return false
		}
		if (Array.isArray(v)) return v.some(walk)
		if (v && typeof v === 'object') return Object.values(v).some(walk)
		return false
	}
	return walk(value)
}

/**
 * Whether a `$res:`/`res://` token for this path survives anywhere in the value — the one
 * spelling the rewriters relocate, and so the only one whose survival means a rewrite did
 * not take. A bare path is deliberately not matched: nothing here moves one, so its presence
 * says nothing about whether the rewrite worked.
 */
export function holdsResourceToken(value: unknown, path: string): boolean {
	const walk = (v: any): boolean => {
		if (typeof v === 'string') {
			RES_TOKEN_RE.lastIndex = 0
			let m: RegExpExecArray | null
			while ((m = RES_TOKEN_RE.exec(v)) !== null) if (m[1] === path) return true
			return false
		}
		if (Array.isArray(v)) return v.some(walk)
		if (v && typeof v === 'object') return Object.values(v).some(walk)
		return false
	}
	return walk(value)
}

/**
 * Whether the text names the resource somewhere no rewriter reaches — a path written on its
 * own rather than inside a `$res:` token, the way `getResource("f/proj/db")` does. The
 * tokens are stripped first so the ones a rewrite would move do not count, and the match is
 * bounded so `f/proj/db` is not found inside `f/proj/db_prod`.
 */
export function textHoldsBarePath(text: string, path: string): boolean {
	const withoutTokens = text.replace(RES_TOKEN_RE, '')
	const boundary = /[\w\-./]/
	for (let i = withoutTokens.indexOf(path); i !== -1; i = withoutTokens.indexOf(path, i + 1)) {
		const before = withoutTokens[i - 1] ?? ''
		const after = withoutTokens[i + path.length] ?? ''
		if (!boundary.test(before) && !boundary.test(after)) return true
	}
	return false
}

/**
 * Whether anything the project ships points at one of its own resources.
 *
 * A project declares two kinds of resource. One is referenced — an app pins `$res:` for a
 * script argument, a trigger names it — and the project does not work until it holds a
 * credential. The other is minted from a `resource-<type>` input schema: it names a type
 * a script accepts, nothing points at it, and a standalone run picks a resource in the
 * argument picker instead. Only the first kind is worth asking anyone to fill in.
 *
 * The `resources` list is excluded from the walk because a stub's own declaration carries
 * its path, which would make every stub look referenced.
 */
export function projectReferencesResource(bundle: ProjectExport, path: string): boolean {
	const { resources: _resources, ...rest } = bundle as any
	if (referencesResourcePath(rest, path)) return true
	// A script that reads the resource by name rather than through a `$res:` token still needs
	// it filled in. Asked about is the safe side of this answer: the cost of a wrong yes is a
	// row nobody had to act on, and of a wrong no a credential nobody was told to set up.
	return textHoldsBarePath(JSON.stringify(rest ?? {}), path)
}

/**
 * Trigger configs reference resources as plain path strings (e.g.
 * `kafka_resource_path: "f/slug/db"`), not `$res:` tokens, so token rewriting
 * misses them. Deep-walk the config and remap any string that exact-matches a
 * map key (map keys are full bundle paths, so an exact match is a reference),
 * or a `script/<path>`/`flow/<path>` handler reference (schedules' on_failure
 * et al.), falling back to `$res:` token rewriting for embedded refs.
 */
// Top-level config fields whose string values are prefixed runnable refs.
// Prefixed forms are remapped ONLY in these known positions: deciding meaning
// from string shape alone rewrote literal payloads that merely looked like
// refs. Bare-path exact matches and $res: tokens stay position-independent.
const HANDLER_REF_FIELDS = new Set(['on_failure', 'on_recovery', 'on_success'])

export function rewriteTriggerConfig(config: any, map: Map<string, string>, depth = 0): any {
	if (typeof config === 'string') {
		const direct = map.get(config)
		if (direct) return direct
		return rewriteContent(config, map)
	}
	if (Array.isArray(config)) return config.map((v) => rewriteTriggerConfig(v, map, depth + 1))
	if (config && typeof config === 'object') {
		return Object.fromEntries(
			Object.entries(config).map(([k, v]) => {
				if (depth === 0 && typeof v === 'string') {
					// Websocket url: $script:<path> / $flow:<path>.
					if (k === 'url') {
						const m = /^\$(script|flow):(.+)$/.exec(v)
						if (m && map.has(m[2])) return [k, `$${m[1]}:${map.get(m[2])}`]
					}
					// Schedule handlers: script/<path> / flow/<path>.
					if (HANDLER_REF_FIELDS.has(k)) {
						const m = /^(script|flow)\/(.+)$/.exec(v)
						if (m && map.has(m[2])) return [k, `${m[1]}/${map.get(m[2])}`]
					}
				}
				return [k, rewriteTriggerConfig(v, map, depth + 1)]
			})
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
		if (v.type === 'aiagent') {
			const next = rewriteAgentConfig(agentStepConfig(v), map)
			if (v.agent !== undefined) v.agent = next.agent
			next.tools.forEach((tool: any, i: number) => {
				if (tool) v.tools[i] = tool
			})
			if (v.tool_inputs !== undefined) v.tool_inputs = next.tool_inputs
			if (v.input_transforms?.enabled_tools !== undefined) {
				v.input_transforms.enabled_tools = next.input_transforms.enabled_tools
			}
		}
		if (typeof v.content === 'string') v.content = rewriteContent(v.content, map)
		const it = v.input_transforms
		if (it && typeof it === 'object') {
			for (const key of Object.keys(it)) {
				const t = it[key]
				// Mirror extraction: rewrite refs wherever they sit, preserving the
				// value's type (a string stays a string, JSON round-trips).
				if (t?.type === 'static' && t.value !== undefined) {
					if (typeof t.value === 'string') {
						t.value = rewriteContent(t.value, map)
					} else {
						t.value = JSON.parse(rewriteContent(JSON.stringify(t.value), map))
					}
				}
			}
		}
	}
	if (cloned?.flow_env && typeof cloned.flow_env === 'object') {
		// Tokens can sit inside nested JSON values, not just string values; the
		// serialize→rewrite→parse round-trip reaches all of them (paths contain
		// no characters that would break JSON string literals).
		cloned.flow_env = JSON.parse(rewriteContent(JSON.stringify(cloned.flow_env), map))
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

// A saved agent (`ai_agent` resource value), walked whole: see `walkAgentConfig`.
export function extractAgentRefs(value: any): Ref[] {
	const out: Ref[] = []
	const seen = new Set<string>()
	collectAgentConfigRefs(value, (kind, path) => {
		const key = `${kind}:${path}`
		if (!seen.has(key)) {
			seen.add(key)
			out.push({ kind, path })
		}
	})
	return out
}

// Moves resource references only. Scripts, flows and resources share a namespace, so a map
// holding one resource path must not repoint a tool that runs a script sharing that path.
export function rewriteAgentResourceRefs(value: any, map: Map<string, string>): any {
	if (!value || typeof value !== 'object') return value
	return rewriteAgentConfig(value, map, 'resource')
}

export function rewriteAgentValue(value: any, map: Map<string, string>): any {
	if (!value || typeof value !== 'object') return value
	return rewriteAgentConfig(value, map)
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
	/** AI agents the project ships: `ai_agent` resources published with their value.
	 * Absent from exports made before agents were published. */
	agents?: ExportItem[]
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
	for (const a of bundle.agents ?? []) add(a.path)
	for (const t of bundle.triggers) {
		add(t.path)
		add(t.runnable_path)
	}
	// Variables aren't enumerated in the export; their `$var:`/`$jsonvar:` refs live
	// inside item values. Relocate the internal ones so a renamed-folder import
	// rewrites them into the target folder instead of retaining the old prefix.
	for (const p of collectExportVarPaths(bundle)) add(p)
	return map
}

// Internal-or-external variable paths referenced by the export's flows, apps,
// triggers and agents. Scripts carry no variable args. Raw apps hold their structure in the
// `value.raw` JSON string.
export function collectExportVarPaths(bundle: ProjectExport): string[] {
	const out = new Set<string>()
	const collect = (value: any) => {
		for (const p of extractVarRefsFromValue(value)) out.add(p)
	}
	for (const f of bundle.flows) collect(f.value)
	for (const a of bundle.apps) collect(a.app_type === 'raw' ? safeParseRaw(a.value?.raw) : a.value)
	for (const t of bundle.triggers) collect(t.config)
	for (const a of bundle.agents ?? []) collect(a.value)
	return [...out]
}

function safeParseRaw(raw: unknown): any {
	if (typeof raw !== 'string') return undefined
	try {
		return JSON.parse(raw)
	} catch {
		return undefined
	}
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
			value: rewriteVarRefsInValue(rewriteFlowValue(f.value, map), map)
		})),
		apps: bundle.apps.map((a) => ({
			...a,
			path: remap(a.path),
			// Raw apps keep their structure in the `value.raw` JSON string.
			value:
				a.app_type === 'raw'
					? {
							...a.value,
							raw: rewriteRawVarRefs(rewriteRawAppContent(a.value?.raw ?? '', map), map)
						}
					: rewriteVarRefsInValue(rewriteAppValue(a.value, map), map)
		})),
		resources: bundle.resources.map((r) => ({ ...r, path: remap(r.path) })),
		...(bundle.agents
			? {
					agents: bundle.agents.map((a) => ({
						...a,
						path: remap(a.path),
						value: rewriteVarRefsInValue(rewriteAgentValue(a.value, map), map)
					}))
				}
			: {}),
		triggers: bundle.triggers.map((t) => ({
			...t,
			path: remap(t.path),
			runnable_path: remap(t.runnable_path),
			// Configs hold `$res:` tokens, plain resource paths (kafka_resource_path
			// etc.) and whole-string `$var:` values — rewrite all three.
			config: t.config ? rewriteVarRefsInValue(rewriteTriggerConfig(t.config, map), map) : t.config
		}))
	}
}

// Var relocation for a raw app's `value.raw` JSON string: parse, structurally
// rewrite whole-string var values, re-serialize; leave invalid JSON untouched.
function rewriteRawVarRefs(raw: string, map: Map<string, string>): string {
	const parsed = safeParseRaw(raw)
	if (parsed === undefined) return raw
	return JSON.stringify(rewriteVarRefsInValue(parsed, map))
}

// `agent` is a saved AI agent: an `ai_agent` resource, published whole rather than as a stub.
export type ItemKind = 'script' | 'flow' | 'app' | 'raw_app' | 'agent'

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
	/** flows + apps + agents */
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
	if (item.kind === 'agent') return extractAgentRefs(item.value)
	return []
}

// Whole-string `$var:`/`$jsonvar:` paths an item resolves at runtime. Scripts carry
// no variable args; raw apps hold their structure in the `content` JSON string.
function varRefsForFetched(item: FetchedItem): string[] {
	if (item.kind === 'flow' || item.kind === 'app' || item.kind === 'agent')
		return extractVarRefsFromValue(item.value)
	if (item.kind === 'raw_app') return extractVarRefsFromValue(safeParseRaw(item.content))
	return []
}

// Walks the transitive closure: scripts referenced by path are pulled in
// recursively, resources become empty stubs, hub refs stay external. A referenced
// `ai_agent` resource is the exception: it is pulled in as an agent, whole, with what
// its own tools reference.
export async function buildProjectBundle(
	seed: ItemRef[],
	slug: string,
	deps: BundleDeps,
	extraResourcePaths: string[] = [],
	extraVarPaths: string[] = [],
	sourceFolder: string = slug
): Promise<ProjectBundle> {
	const fetched = new Map<string, FetchedItem>()
	const queued = new Set<string>()
	const resourcePaths = new Set<string>()
	const varPaths = new Set<string>()
	const unresolved: string[] = []

	// Resources and variables referenced by triggers (by config value, not `$res:`
	// in code) — relocated through the same map so the export stays slug-relative.
	for (const p of extraResourcePaths) {
		if (classifyPath(p, slug) !== 'hub') resourcePaths.add(p)
	}
	for (const p of extraVarPaths) varPaths.add(p)

	// Key by `${kind}:${path}`, not bare path: a script and flow can share a path,
	// and keying by path alone would silently drop one.
	const refKey = (kind: string, path: string) => `${kind}:${path}`
	// Resource types, looked up once: a referenced `ai_agent` resource is an agent the
	// project ships whole (with what its tools run), not an empty stub.
	const types = new Map<string, Promise<string | undefined>>()
	const typeOf = (path: string) => {
		if (!types.has(path)) types.set(path, deps.resolveResourceType(path))
		return types.get(path)!
	}
	// Agents among `paths`, as items to fetch. Asked once per path, whoever named it first:
	// a trigger's resource is checked like one an item references.
	const promoted = new Set<string>()
	const agentsAmong = async (paths: string[]): Promise<ItemRef[]> => {
		const fresh = [...new Set(paths)].filter((p) => !promoted.has(p))
		for (const p of fresh) promoted.add(p)
		const found = await Promise.all(fresh.map(typeOf))
		const out: ItemRef[] = []
		fresh.forEach((path, i) => {
			const key = refKey('agent', path)
			if (found[i] === 'ai_agent' && !queued.has(key)) {
				queued.add(key)
				out.push({ kind: 'agent', path })
			}
		})
		return out
	}

	// Refs at the same BFS depth are independent: fetch each level concurrently.
	let level: ItemRef[] = []
	for (const s of seed) {
		const key = refKey(s.kind, s.path)
		if (!queued.has(key)) {
			queued.add(key)
			level.push(s)
		}
	}
	level.push(...(await agentsAmong([...resourcePaths])))
	while (level.length > 0) {
		const results = await Promise.all(
			level.map(async (ref) => ({ ref, item: await deps.fetchItem(ref) }))
		)
		const next: ItemRef[] = []
		const newResources: string[] = []
		for (const { ref, item } of results) {
			if (!item) {
				unresolved.push(ref.path)
				continue
			}
			fetched.set(refKey(ref.kind, ref.path), item)
			for (const r of refsForFetched(item)) {
				if (classifyPath(r.path, slug) === 'hub') continue
				if (r.kind === 'resource') {
					newResources.push(r.path)
					resourcePaths.add(r.path)
				} else if (r.kind === 'script' || r.kind === 'flow') {
					const key = refKey(r.kind, r.path)
					if (!queued.has(key)) {
						queued.add(key)
						next.push({ kind: r.kind, path: r.path })
					}
				}
			}
			// Relocate the item's runtime variable refs into the project folder too, so
			// the export is slug-relative regardless of the source folder (import then
			// materializes them as placeholders). Variables are never hub-hosted.
			for (const p of varRefsForFetched(item)) varPaths.add(p)
		}
		next.push(...(await agentsAmong(newResources)))
		level = next
	}

	const fetchedItems = [...fetched.values()]
	const itemPaths = fetchedItems.map((it) => it.path)
	const map = buildPathMap([...itemPaths, ...resourcePaths, ...varPaths], slug, sourceFolder)

	const items: BundledItem[] = fetchedItems.map((it) => {
		const rewritten: BundledItem = { ...it, newPath: map.get(it.path) ?? it.path }
		if (it.kind === 'script') {
			rewritten.content = rewriteContent(it.content ?? '', map)
		} else if (it.kind === 'raw_app') {
			rewritten.content = rewriteRawVarRefs(rewriteRawAppContent(it.content ?? '', map), map)
		} else if (it.kind === 'flow') {
			rewritten.value = rewriteVarRefsInValue(rewriteFlowValue(it.value, map), map)
		} else if (it.kind === 'app') {
			rewritten.value = rewriteVarRefsInValue(rewriteAppValue(it.value, map), map)
		} else if (it.kind === 'agent') {
			rewritten.value = rewriteVarRefsInValue(rewriteAgentValue(it.value, map), map)
		}
		return rewritten
	})

	const resourceStubs: ResourceStub[] = []
	// An agent the bundle ships is published whole: an app or a flow step naming it must not
	// also turn it into an empty stub at the same path.
	for (const it of fetchedItems) if (it.kind === 'agent') resourcePaths.delete(it.path)
	const resolved = await Promise.all(
		[...resourcePaths].map(async (path) => ({ path, type: await typeOf(path) }))
	)
	for (const { path, type } of resolved) {
		if (!type) {
			unresolved.push(path)
			continue
		}
		resourceStubs.push({ originalPath: path, newPath: map.get(path) ?? path, resource_type: type })
	}

	// `unresolved` keys missing items by kind:path but stores the bare path, so a
	// missing script and flow (or a runnable and resource) sharing a path can push
	// the same string twice. Dedupe: callers use it as a display/blocker list where
	// duplicate keys would break keyed rendering.
	return { items, resourceStubs, pathMap: map, unresolved: [...new Set(unresolved)] }
}
