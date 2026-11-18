import { describe, it, expect } from 'vitest'
import {
	classifyPath,
	extractScriptRefs,
	extractFlowRefs,
	extractAgentRefs,
	rewriteAgentValue,
	withoutAgentWorkspaceKeys,
	extractAppRefs,
	buildPathMap,
	rewriteContent,
	rewriteTriggerConfig,
	rewriteFlowValue,
	rewriteAppValue,
	extractRawAppRefs,
	rewriteRawAppContent,
	buildProjectBundle,
	retargetProjectExport,
	collectExportVarPaths,
	extractTriggerConfigResourceRefs,
	extractVarRefsFromValue,
	projectReferencesResource,
	textHoldsBarePath,
	type ProjectExport,
	type FetchedItem,
	type ItemRef
} from './projectBundle'

describe('classifyPath', () => {
	it('internal for paths under the project folder', () => {
		expect(classifyPath('f/proj/db', 'proj')).toBe('internal')
		expect(classifyPath('f/proj', 'proj')).toBe('internal')
	})
	it('hub for hub paths', () => {
		expect(classifyPath('hub/16043/discord/send', 'proj')).toBe('hub')
	})
	it('external for user and other folders', () => {
		expect(classifyPath('u/admin/db', 'proj')).toBe('external')
		expect(classifyPath('f/other/db', 'proj')).toBe('external')
	})
	it('does not treat a prefix-only match as internal', () => {
		expect(classifyPath('f/project2/db', 'proj')).toBe('external')
	})
})

describe('extractScriptRefs', () => {
	it('finds $res: and res:// resource refs, deduped', () => {
		const c = `const a = "$res:u/admin/db"; const b = "res://f/x/api"; const c2 = "$res:u/admin/db"`
		expect(extractScriptRefs(c)).toEqual([
			{ kind: 'resource', path: 'u/admin/db' },
			{ kind: 'resource', path: 'f/x/api' }
		])
	})
	it('returns nothing when no refs', () => {
		expect(extractScriptRefs('export async function main() {}')).toEqual([])
	})
})

describe('extractFlowRefs', () => {
	it('finds inline-code, static-input, and script-path refs', () => {
		const value = {
			modules: [
				{
					id: 'a',
					value: {
						type: 'rawscript',
						content: 'const db = "$res:u/admin/pg"',
						input_transforms: {
							other: { type: 'static', value: '$res:f/shared/api' },
							expr1: { type: 'javascript', expr: 'flow_input.x' }
						}
					}
				},
				{
					id: 'b',
					value: {
						type: 'branchone',
						branches: [
							{
								modules: [
									{ id: 'c', value: { type: 'script', path: 'u/admin/my_script' } },
									{ id: 'd', value: { type: 'script', path: 'hub/123/x/y' } }
								]
							}
						],
						default: [{ id: 'e', value: { type: 'rawscript', content: 'no refs' } }]
					}
				}
			]
		}
		const refs = extractFlowRefs(value)
		expect(refs).toContainEqual({ kind: 'resource', path: 'u/admin/pg' })
		expect(refs).toContainEqual({ kind: 'resource', path: 'f/shared/api' })
		expect(refs).toContainEqual({ kind: 'script', path: 'u/admin/my_script' })
		expect(refs).toContainEqual({ kind: 'script', path: 'hub/123/x/y' })
		// a javascript expr (flow_input) is not a hardcoded ref
		expect(refs.filter((r) => r.path === 'flow_input.x')).toEqual([])
	})
	it('finds sub-flow refs from type: flow steps', () => {
		const value = {
			modules: [
				{ id: 'a', value: { type: 'flow', path: 'u/admin/sub_flow' } },
				{ id: 'b', value: { type: 'flow', path: 'hub/9/x/y' } }
			]
		}
		const refs = extractFlowRefs(value)
		expect(refs).toContainEqual({ kind: 'flow', path: 'u/admin/sub_flow' })
		expect(refs).toContainEqual({ kind: 'flow', path: 'hub/9/x/y' })
	})
})

describe('buildPathMap', () => {
	it('reparents into the project folder keeping the leaf name', () => {
		const m = buildPathMap(['u/admin/db', 'f/other/api'], 'proj')
		expect(m.get('u/admin/db')).toBe('f/proj/db')
		expect(m.get('f/other/api')).toBe('f/proj/api')
	})
	it('suffixes collisions deterministically', () => {
		const m = buildPathMap(['u/alice/db', 'f/shared/db', 'u/bob/db'], 'proj')
		// sorted: f/shared/db, u/alice/db, u/bob/db
		expect(m.get('f/shared/db')).toBe('f/proj/db')
		expect(m.get('u/alice/db')).toBe('f/proj/db_2')
		expect(m.get('u/bob/db')).toBe('f/proj/db_3')
	})
	it('maps internal paths to themselves, preserving subfolder depth', () => {
		const m = buildPathMap(['f/proj/api', 'f/proj/sub/deep/script'], 'proj')
		expect(m.get('f/proj/api')).toBe('f/proj/api')
		expect(m.get('f/proj/sub/deep/script')).toBe('f/proj/sub/deep/script')
	})
	it('does not flatten two internal items sharing a leaf name', () => {
		const m = buildPathMap(['f/proj/a/x', 'f/proj/b/x'], 'proj')
		expect(m.get('f/proj/a/x')).toBe('f/proj/a/x')
		expect(m.get('f/proj/b/x')).toBe('f/proj/b/x')
	})
	it('keeps the sub-path of every item in a nested source folder', () => {
		const m = buildPathMap(
			['f/hub/billing/charge', 'f/hub/billing/utils/format', 'f/hub/billing/stripe/format'],
			'billing',
			'hub/billing'
		)
		expect(m.get('f/hub/billing/charge')).toBe('f/billing/charge')
		expect(m.get('f/hub/billing/utils/format')).toBe('f/billing/utils/format')
		expect(m.get('f/hub/billing/stripe/format')).toBe('f/billing/stripe/format')
		// The same for a top-level folder whose name is not its slug.
		expect(buildPathMap(['f/my_tools/a/x'], 'my-tools', 'my_tools').get('f/my_tools/a/x')).toBe(
			'f/my-tools/a/x'
		)
	})
	it('relocates an external onto a suffix when its leaf collides with an internal path', () => {
		const m = buildPathMap(['f/proj/db', 'u/admin/db'], 'proj')
		expect(m.get('f/proj/db')).toBe('f/proj/db')
		expect(m.get('u/admin/db')).toBe('f/proj/db_2')
	})
})

describe('rewriteContent', () => {
	it('rewrites mapped refs and leaves unmapped ones', () => {
		const map = new Map([['u/admin/db', 'f/proj/db']])
		expect(rewriteContent('x = "$res:u/admin/db"', map)).toBe('x = "$res:f/proj/db"')
		expect(rewriteContent('x = "res://u/admin/db"', map)).toBe('x = "$res:f/proj/db"')
		expect(rewriteContent('x = "$res:hub/1/a/b"', map)).toBe('x = "$res:hub/1/a/b"')
	})
	it('does not partial-match a longer path', () => {
		const map = new Map([['u/admin/db', 'f/proj/db']])
		// u/admin/db2 must not be rewritten by the u/admin/db entry
		expect(rewriteContent('x = "$res:u/admin/db2"', map)).toBe('x = "$res:u/admin/db2"')
	})
})

describe('rewriteTriggerConfig', () => {
	const map = new Map([
		['f/proj/kafka', 'f/target/kafka'],
		['f/proj/script', 'f/target/script']
	])
	it('remaps plain resource path fields', () => {
		expect(
			rewriteTriggerConfig({ kafka_resource_path: 'f/proj/kafka', group_id: 'g1' }, map)
		).toEqual({ kafka_resource_path: 'f/target/kafka', group_id: 'g1' })
	})
	it('remaps nested objects, arrays, and $res: tokens', () => {
		expect(
			rewriteTriggerConfig(
				{
					nested: { path: 'f/proj/script' },
					list: ['f/proj/kafka', 'unrelated'],
					code: 'x = "$res:f/proj/kafka"'
				},
				map
			)
		).toEqual({
			nested: { path: 'f/target/script' },
			list: ['f/target/kafka', 'unrelated'],
			code: 'x = "$res:f/target/kafka"'
		})
	})
	it('leaves non-matching strings and non-string values untouched', () => {
		const config = { url: 'wss://example.com', port: 9092, enabled: true, extra: null }
		expect(rewriteTriggerConfig(config, map)).toEqual(config)
	})
})

describe('rewriteFlowValue', () => {
	it('rewrites inline code, static inputs, and script paths; clones input', () => {
		const map = new Map([
			['u/admin/pg', 'f/proj/pg'],
			['f/shared/api', 'f/proj/api'],
			['u/admin/my_script', 'f/proj/my_script']
		])
		const value = {
			modules: [
				{
					id: 'a',
					value: {
						type: 'rawscript',
						content: 'const db = "$res:u/admin/pg"',
						input_transforms: { other: { type: 'static', value: '$res:f/shared/api' } }
					}
				},
				{ id: 'b', value: { type: 'script', path: 'u/admin/my_script' } },
				{ id: 'c', value: { type: 'script', path: 'hub/1/keep/me' } }
			]
		}
		const out = rewriteFlowValue(value, map)
		expect(out.modules[0].value.content).toBe('const db = "$res:f/proj/pg"')
		expect(out.modules[0].value.input_transforms.other.value).toBe('$res:f/proj/api')
		expect(out.modules[1].value.path).toBe('f/proj/my_script')
		expect(out.modules[2].value.path).toBe('hub/1/keep/me')
		// original untouched (deep clone)
		expect(value.modules[0].value.content).toBe('const db = "$res:u/admin/pg"')
	})
})

// A trimmed app value: a runnable-by-path component, a hub runnable, a $res in an
// inline script, and incidental `f/...` text that must NOT be rewritten.
const appValue = () => ({
	grid: [
		{
			data: {
				componentInput: {
					runnable: { type: 'runnableByPath', runType: 'script', path: 'u/admin/charts' }
				}
			}
		},
		{
			data: {
				componentInput: {
					runnable: { type: 'runnableByPath', runType: 'flow', path: 'f/shared/sync' }
				}
			}
		},
		{
			data: {
				componentInput: {
					runnable: { type: 'runnableByPath', runType: 'hubscript', path: 'hub/1/keep' }
				}
			}
		}
	],
	hiddenInlineScripts: [
		{ name: 'h', inlineScript: { content: 'x = "$res:u/admin/pg"', language: 'deno' } }
	],
	someLabel: 'see docs at f/shared/sync for details'
})

describe('extractAppRefs', () => {
	it('extracts runnable-by-path scripts/flows and $res resources, skips hub', () => {
		const refs = extractAppRefs(appValue())
		expect(refs).toContainEqual({ kind: 'script', path: 'u/admin/charts' })
		expect(refs).toContainEqual({ kind: 'flow', path: 'f/shared/sync' })
		expect(refs).toContainEqual({ kind: 'resource', path: 'u/admin/pg' })
		expect(refs.some((r) => r.path === 'hub/1/keep')).toBe(false)
	})
})

describe('rewriteAppValue', () => {
	it('relocates runnable paths and $res, leaves hub refs and incidental text intact', () => {
		const map = new Map([
			['u/admin/charts', 'f/proj/charts'],
			['f/shared/sync', 'f/proj/sync'],
			['u/admin/pg', 'f/proj/pg']
		])
		const value = appValue()
		const out = rewriteAppValue(value, map)
		expect(out.grid[0].data.componentInput.runnable.path).toBe('f/proj/charts')
		expect(out.grid[1].data.componentInput.runnable.path).toBe('f/proj/sync')
		expect(out.grid[2].data.componentInput.runnable.path).toBe('hub/1/keep')
		expect(out.hiddenInlineScripts[0].inlineScript.content).toBe('x = "$res:f/proj/pg"')
		// incidental text untouched
		expect(out.someLabel).toBe('see docs at f/shared/sync for details')
		// original untouched (deep clone)
		expect(value.grid[0].data.componentInput.runnable.path).toBe('u/admin/charts')
	})
})

describe('raw app (value.raw JSON string)', () => {
	const rawContent = () =>
		JSON.stringify({
			runnables: {
				a: { type: 'path', runType: 'flow', path: 'u/admin/sync' },
				b: { type: 'path', runType: 'script', path: 'f/shared/calc' },
				c: { type: 'path', runType: 'hubscript', path: 'hub/1/keep' }
			},
			files: { '/bundle.js': 'const conn = "$res:u/admin/pg"' }
		})

	it('extractRawAppRefs sees nested runnables and $res, skips hub', () => {
		const refs = extractRawAppRefs(rawContent())
		expect(refs).toContainEqual({ kind: 'flow', path: 'u/admin/sync' })
		expect(refs).toContainEqual({ kind: 'script', path: 'f/shared/calc' })
		expect(refs).toContainEqual({ kind: 'resource', path: 'u/admin/pg' })
		expect(refs.some((r) => r.path === 'hub/1/keep')).toBe(false)
	})

	it('rewriteRawAppContent relocates nested runnable paths and $res', () => {
		const map = new Map([
			['u/admin/sync', 'f/proj/sync'],
			['f/shared/calc', 'f/proj/calc'],
			['u/admin/pg', 'f/proj/pg']
		])
		const out = JSON.parse(rewriteRawAppContent(rawContent(), map))
		expect(out.runnables.a.path).toBe('f/proj/sync')
		expect(out.runnables.b.path).toBe('f/proj/calc')
		expect(out.runnables.c.path).toBe('hub/1/keep')
		expect(out.files['/bundle.js']).toBe('const conn = "$res:f/proj/pg"')
	})

	it('falls back to $res scan on non-JSON content', () => {
		expect(extractRawAppRefs('x = "$res:u/admin/pg"')).toContainEqual({
			kind: 'resource',
			path: 'u/admin/pg'
		})
		expect(
			rewriteRawAppContent('x = "$res:u/admin/pg"', new Map([['u/admin/pg', 'f/proj/pg']]))
		).toBe('x = "$res:f/proj/pg"')
	})
})

describe('buildProjectBundle', () => {
	// A flow that calls an external script which itself hardcodes a resource.
	const flow: FetchedItem = {
		kind: 'flow',
		path: 'u/admin/my_flow',
		summary: 'Flow',
		value: {
			modules: [
				{ id: 'a', value: { type: 'script', path: 'u/admin/helper' } },
				{
					id: 'b',
					value: {
						type: 'rawscript',
						content: 'const x = "$res:f/shared/api"',
						input_transforms: {}
					}
				}
			]
		}
	}
	const helper: FetchedItem = {
		kind: 'script',
		path: 'u/admin/helper',
		summary: 'Helper',
		language: 'bun',
		content: 'const db = "$res:u/admin/pg"; export async function main(){}'
	}

	const deps = {
		fetchItem: async (ref: ItemRef) => {
			if (ref.path === 'u/admin/my_flow') return flow
			if (ref.path === 'u/admin/helper') return helper
			return undefined
		},
		resolveResourceType: async (path: string) => {
			if (path === 'u/admin/pg') return 'postgresql'
			if (path === 'f/shared/api') return 'http_api'
			return undefined
		}
	}

	it('pulls in referenced scripts + resources and rewrites everything under the folder', async () => {
		const bundle = await buildProjectBundle(
			[{ kind: 'flow', path: 'u/admin/my_flow' }],
			'proj',
			deps
		)

		// flow + transitively-pulled helper script are both bundled
		const byPath = Object.fromEntries(bundle.items.map((i) => [i.path, i]))
		expect(Object.keys(byPath).sort()).toEqual(['u/admin/helper', 'u/admin/my_flow'])

		// items relocated under f/proj/
		expect(byPath['u/admin/my_flow'].newPath).toBe('f/proj/my_flow')
		expect(byPath['u/admin/helper'].newPath).toBe('f/proj/helper')

		// flow's script-path ref rewritten to the helper's new path
		expect(byPath['u/admin/my_flow'].value.modules[0].value.path).toBe('f/proj/helper')
		// flow inline + helper code resource refs rewritten
		expect(byPath['u/admin/my_flow'].value.modules[1].value.content).toBe(
			'const x = "$res:f/proj/api"'
		)
		expect(byPath['u/admin/helper'].content).toContain('"$res:f/proj/pg"')

		// resource stubs created at new paths with resolved types
		const stubs = Object.fromEntries(bundle.resourceStubs.map((s) => [s.originalPath, s]))
		expect(stubs['u/admin/pg'].newPath).toBe('f/proj/pg')
		expect(stubs['u/admin/pg'].resource_type).toBe('postgresql')
		expect(stubs['f/shared/api'].resource_type).toBe('http_api')

		expect(bundle.unresolved).toEqual([])
	})

	it('pulls in a sub-flow referenced by a type: flow step and rewrites its path', async () => {
		const parent: FetchedItem = {
			kind: 'flow',
			path: 'u/admin/parent_flow',
			value: { modules: [{ id: 'a', value: { type: 'flow', path: 'u/admin/sub_flow' } }] }
		}
		const sub: FetchedItem = {
			kind: 'flow',
			path: 'u/admin/sub_flow',
			value: {
				modules: [{ id: 'a', value: { type: 'script', path: 'hub/1/keep/me' } }]
			}
		}
		const d = {
			fetchItem: async (ref: ItemRef) => {
				if (ref.path === 'u/admin/parent_flow') return parent
				if (ref.path === 'u/admin/sub_flow') return sub
				return undefined
			},
			resolveResourceType: async () => undefined
		}
		const bundle = await buildProjectBundle(
			[{ kind: 'flow', path: 'u/admin/parent_flow' }],
			'proj',
			d
		)
		const byPath = Object.fromEntries(bundle.items.map((i) => [i.path, i]))
		// both flows bundled
		expect(Object.keys(byPath).sort()).toEqual(['u/admin/parent_flow', 'u/admin/sub_flow'])
		// parent's type: flow ref rewritten to the sub-flow's new path
		expect(byPath['u/admin/parent_flow'].value.modules[0].value.path).toBe('f/proj/sub_flow')
		expect(byPath['u/admin/sub_flow'].newPath).toBe('f/proj/sub_flow')
		// hub ref inside the sub-flow left untouched
		expect(byPath['u/admin/sub_flow'].value.modules[0].value.path).toBe('hub/1/keep/me')
		expect(bundle.unresolved).toEqual([])
	})

	it('leaves hub script references untouched and does not fetch them', async () => {
		const hubFlow: FetchedItem = {
			kind: 'flow',
			path: 'u/admin/hub_flow',
			value: { modules: [{ id: 'a', value: { type: 'script', path: 'hub/1/x/y' } }] }
		}
		const d = {
			fetchItem: async (ref: ItemRef) => (ref.path === 'u/admin/hub_flow' ? hubFlow : undefined),
			resolveResourceType: async () => undefined
		}
		const bundle = await buildProjectBundle([{ kind: 'flow', path: 'u/admin/hub_flow' }], 'proj', d)
		expect(bundle.items.map((i) => i.path)).toEqual(['u/admin/hub_flow'])
		expect(bundle.items[0].value.modules[0].value.path).toBe('hub/1/x/y')
		expect(bundle.unresolved).toEqual([])
	})

	it('reports a missing item and an unresolvable resource as unresolved', async () => {
		const root: FetchedItem = {
			kind: 'flow',
			path: 'u/admin/root',
			value: {
				modules: [
					{ id: 'a', value: { type: 'script', path: 'u/admin/gone' } },
					{
						id: 'b',
						value: {
							type: 'rawscript',
							content: 'const x = "$res:u/admin/untyped"',
							input_transforms: {}
						}
					}
				]
			}
		}
		const d = {
			fetchItem: async (ref: ItemRef) => (ref.path === 'u/admin/root' ? root : undefined),
			resolveResourceType: async () => undefined
		}
		const bundle = await buildProjectBundle([{ kind: 'flow', path: 'u/admin/root' }], 'proj', d)
		expect(bundle.unresolved.sort()).toEqual(['u/admin/gone', 'u/admin/untyped'])
	})

	it('relocates $var:/$jsonvar: refs into the slug when it differs from the source folder', async () => {
		const flow: FetchedItem = {
			kind: 'flow',
			path: 'f/source_folder/main',
			value: {
				flow_env: { CFG: '$jsonvar:f/source_folder/cfg' },
				modules: [
					{
						id: 'a',
						value: {
							type: 'rawscript',
							// Whole-value ref is relocated; the inline literal is not.
							content: 'return "$var:f/source_folder/key"',
							input_transforms: { k: { type: 'static', value: '$var:f/source_folder/key' } }
						}
					}
				]
			}
		}
		const d = {
			fetchItem: async (ref: ItemRef) => (ref.path === 'f/source_folder/main' ? flow : undefined),
			resolveResourceType: async () => undefined
		}
		const bundle = await buildProjectBundle(
			[{ kind: 'flow', path: 'f/source_folder/main' }],
			'kit',
			d
		)
		const v = bundle.items[0].value
		expect(v.modules[0].value.input_transforms.k.value).toBe('$var:f/kit/key')
		expect(v.flow_env.CFG).toBe('$jsonvar:f/kit/cfg')
		// Inline code literal is untouched.
		expect(v.modules[0].value.content).toBe('return "$var:f/source_folder/key"')
	})

	it('dedupes a path missing as both a script and a flow', async () => {
		// A missing script + flow sharing a path each push the bare path once; the
		// list must stay unique so a keyed UI render of it can't collide.
		const root: FetchedItem = {
			kind: 'flow',
			path: 'u/admin/root',
			value: {
				modules: [
					{ id: 'a', value: { type: 'script', path: 'u/admin/dup' } },
					{ id: 'b', value: { type: 'flow', path: 'u/admin/dup' } }
				]
			}
		}
		const d = {
			fetchItem: async (ref: ItemRef) => (ref.path === 'u/admin/root' ? root : undefined),
			resolveResourceType: async () => undefined
		}
		const bundle = await buildProjectBundle([{ kind: 'flow', path: 'u/admin/root' }], 'proj', d)
		expect(bundle.unresolved).toEqual(['u/admin/dup'])
	})
})

describe('extractVarRefsFromValue', () => {
	it('collects whole-value `$var:`/`$jsonvar:` refs, deduped, walking nested JSON', () => {
		const value = {
			flow_env: { API: '$var:u/admin/key' },
			modules: [
				{ value: { input_transforms: { a: { type: 'static', value: '$var:f/proj/token' } } } },
				{ value: { input_transforms: { b: { type: 'static', value: '$jsonvar:u/admin/cfg' } } } },
				{ value: { input_transforms: { c: { type: 'static', value: '$var:u/admin/key' } } } }
			]
		}
		expect(extractVarRefsFromValue(value).sort()).toEqual([
			'f/proj/token',
			'u/admin/cfg',
			'u/admin/key'
		])
	})
	it('ignores a `$var:` token embedded in inline code (not a whole value)', () => {
		// The worker only substitutes a value that *is* the reference, so an inline
		// script literal must not be treated as a variable arg.
		const value = {
			modules: [{ value: { type: 'rawscript', content: 'return "$var:u/example/template"' } }]
		}
		expect(extractVarRefsFromValue(value)).toEqual([])
	})
})

describe('retargetProjectExport', () => {
	const baseExport = (): ProjectExport => ({
		project: { slug: 'proj', name: 'Proj', summary: '', readme: null },
		scripts: [
			{
				path: 'f/proj/hello',
				content: 'const r = "$res:f/proj/db"',
				summary: 'hello'
			}
		],
		flows: [
			{
				path: 'f/proj/main_flow',
				value: {
					modules: [
						{ id: 'a', value: { type: 'script', path: 'f/proj/hello', input_transforms: {} } }
					]
				}
			}
		],
		apps: [
			{
				path: 'f/proj/dashboard',
				value: { grid: [{ data: { componentInput: { runnable: {} } } }] }
			},
			{
				path: 'f/proj/rawapp',
				app_type: 'raw',
				value: { raw: JSON.stringify({ files: {}, runnables: {} }) }
			}
		],
		resources: [{ path: 'f/proj/db', resource_type: 'postgresql' }],
		triggers: [
			{
				path: 'f/proj/every_day',
				kind: 'schedule',
				runnable_path: 'f/proj/hello',
				runnable_kind: 'script',
				config: { schedule: '0 0 12 * * *' }
			},
			{
				path: 'f/proj/kafka_in',
				kind: 'kafka',
				runnable_path: 'f/proj/hello',
				runnable_kind: 'script',
				config: { kafka_resource_path: 'f/proj/db' }
			}
		]
	})

	it('returns the bundle unchanged when the folder matches the slug', () => {
		const bundle = baseExport()
		expect(retargetProjectExport(bundle, 'proj', 'proj')).toBe(bundle)
	})

	it('relocates every item path and internal reference into the target folder', () => {
		const out = retargetProjectExport(baseExport(), 'proj', 'dest')
		expect(out.scripts[0].path).toBe('f/dest/hello')
		expect(out.scripts[0].content).toContain('$res:f/dest/db')
		expect(out.flows[0].path).toBe('f/dest/main_flow')
		expect(out.flows[0].value.modules[0].value.path).toBe('f/dest/hello')
		expect(out.apps.map((a) => a.path)).toEqual(['f/dest/dashboard', 'f/dest/rawapp'])
		expect(out.resources[0].path).toBe('f/dest/db')
		expect(out.triggers[0].path).toBe('f/dest/every_day')
		expect(out.triggers[0].runnable_path).toBe('f/dest/hello')
		// Plain-string resource path in a trigger config is remapped too.
		expect(out.triggers[1].config.kafka_resource_path).toBe('f/dest/db')
	})

	it('leaves external and hub paths untouched', () => {
		const bundle = baseExport()
		bundle.scripts[0].content = 'const a = "$res:u/admin/db"; const b = "$res:hub/1/x"'
		const out = retargetProjectExport(bundle, 'proj', 'dest')
		expect(out.scripts[0].content).toContain('$res:u/admin/db')
		expect(out.scripts[0].content).toContain('$res:hub/1/x')
	})

	it('retargets internal $var:/$jsonvar: refs but leaves external ones', () => {
		const bundle = baseExport()
		bundle.flows[0].value.modules[0].value.input_transforms = {
			key: { type: 'static', value: '$var:f/proj/api_key' },
			ext: { type: 'static', value: '$var:u/admin/personal' }
		}
		bundle.flows[0].value.flow_env = { CFG: '$jsonvar:f/proj/cfg' }
		bundle.triggers[1].config.queue_url = '$var:f/proj/sqs'
		const out = retargetProjectExport(bundle, 'proj', 'dest')
		const it = out.flows[0].value.modules[0].value.input_transforms
		expect(it.key.value).toBe('$var:f/dest/api_key')
		expect(it.ext.value).toBe('$var:u/admin/personal')
		expect(out.flows[0].value.flow_env.CFG).toBe('$jsonvar:f/dest/cfg')
		expect(out.triggers[1].config.queue_url).toBe('$var:f/dest/sqs')
	})

	it('leaves an inert $var: literal embedded in inline code unchanged', () => {
		const bundle = baseExport()
		// Same path as a real runtime ref, but here it is a literal inside code: it
		// must not be rewritten even once the path enters the retarget map.
		bundle.flows[0].value.modules[0].value = {
			type: 'rawscript',
			content: 'return "$var:f/proj/api_key"',
			input_transforms: { real: { type: 'static', value: '$var:f/proj/api_key' } }
		}
		const out = retargetProjectExport(bundle, 'proj', 'dest')
		const mod = out.flows[0].value.modules[0].value
		expect(mod.content).toBe('return "$var:f/proj/api_key"')
		expect(mod.input_transforms.real.value).toBe('$var:f/dest/api_key')
	})
})

describe('collectExportVarPaths', () => {
	it('gathers variable refs from flows, apps, and triggers (deduped)', () => {
		const bundle: ProjectExport = {
			project: { slug: 'proj', name: 'P', summary: '', readme: null },
			scripts: [],
			flows: [{ path: 'f/proj/f', value: { flow_env: { A: '$var:f/proj/a' }, modules: [] } }],
			apps: [
				{
					path: 'f/proj/raw',
					app_type: 'raw',
					value: { raw: JSON.stringify({ runnables: { r: { fields: { x: '$var:u/admin/b' } } } }) }
				}
			],
			triggers: [{ path: 'f/proj/t', kind: 'sqs', config: { queue_url: '$jsonvar:f/proj/a' } }],
			resources: []
		}
		expect(collectExportVarPaths(bundle).sort()).toEqual(['f/proj/a', 'u/admin/b'])
	})
})

describe('trigger handler relocation', () => {
	it('rewriteTriggerConfig remaps script/- and flow/-prefixed handler refs', () => {
		const map = new Map([
			['u/admin/handler', 'f/proj/handler'],
			['u/admin/recovery_flow', 'f/proj/recovery_flow']
		])
		const out = rewriteTriggerConfig(
			{
				error_handler_path: 'u/admin/handler',
				on_failure: 'script/u/admin/handler',
				on_recovery: 'flow/u/admin/recovery_flow',
				on_success: 'script/u/admin/unmapped'
			},
			map
		)
		expect(out.error_handler_path).toBe('f/proj/handler')
		expect(out.on_failure).toBe('script/f/proj/handler')
		expect(out.on_recovery).toBe('flow/f/proj/recovery_flow')
		expect(out.on_success).toBe('script/u/admin/unmapped')
	})

	it('remaps $script:/$flow: only in the url field, never in literal payloads', () => {
		const map = new Map([['u/admin/builder', 'f/proj/builder']])
		const out = rewriteTriggerConfig(
			{
				url: '$script:u/admin/builder',
				initial_messages: [{ raw_message: '$script:u/admin/builder' }]
			},
			map
		)
		expect(out.url).toBe('$script:f/proj/builder')
		expect(out.initial_messages[0].raw_message).toBe('$script:u/admin/builder')
	})

	it('leaves literal handler-shaped strings in args untouched', () => {
		const map = new Map([['f/proj/handler', 'f/dest/handler']])
		const out = rewriteTriggerConfig(
			{
				on_failure: 'script/f/proj/handler',
				args: { note: 'script/f/proj/handler' }
			},
			map
		)
		expect(out.on_failure).toBe('script/f/dest/handler')
		expect(out.args.note).toBe('script/f/proj/handler')
	})

	it('leaves nested url keys untouched, rewriting only the top-level websocket url', () => {
		const map = new Map([['u/admin/builder', 'f/proj/builder']])
		const out = rewriteTriggerConfig(
			{
				url: '$script:u/admin/builder',
				args: { url: '$script:u/admin/builder' }
			},
			map
		)
		expect(out.url).toBe('$script:f/proj/builder')
		expect(out.args.url).toBe('$script:u/admin/builder')
	})

	it('extracts and relocates $res refs nested in static input transform JSON', () => {
		const value = {
			modules: [
				{
					id: 'a',
					value: {
						type: 'script',
						path: 'f/proj/step',
						input_transforms: {
							provider: { type: 'static', value: { resource: '$res:u/admin/openai' } },
							note: { type: 'static', value: 'plain text' }
						}
					}
				}
			]
		}
		const refs = extractFlowRefs(value)
		expect(refs).toContainEqual({ kind: 'resource', path: 'u/admin/openai' })
		const out = rewriteFlowValue(value, new Map([['u/admin/openai', 'f/proj/openai']]))
		const it0 = out.modules[0].value.input_transforms
		expect(it0.provider.value).toEqual({ resource: '$res:f/proj/openai' })
		expect(typeof it0.note.value).toBe('string')
	})

	it('retargetProjectExport remaps trigger error handlers with the bundle', () => {
		const bundle: ProjectExport = {
			project: { slug: 'proj', name: 'P', summary: '', readme: null },
			scripts: [{ path: 'f/proj/handler', content: '' }],
			flows: [],
			apps: [],
			resources: [],
			triggers: [
				{
					path: 'f/proj/sched',
					kind: 'schedule',
					runnable_path: 'f/proj/handler',
					runnable_kind: 'script',
					config: { schedule: '0 0 * * * *', on_failure: 'script/f/proj/handler' }
				},
				{
					path: 'f/proj/mq',
					kind: 'mqtt',
					runnable_path: 'f/proj/handler',
					runnable_kind: 'script',
					config: { error_handler_path: 'f/proj/handler' }
				}
			]
		}
		const out = retargetProjectExport(bundle, 'proj', 'dest')
		expect(out.triggers[0].config.on_failure).toBe('script/f/dest/handler')
		expect(out.triggers[1].config.error_handler_path).toBe('f/dest/handler')
	})
})

describe('extractTriggerConfigResourceRefs', () => {
	it('collects $res: tokens nested anywhere in a trigger config', () => {
		expect(
			extractTriggerConfigResourceRefs({
				schedule: '0 0 * * * *',
				args: { channel: '$res:u/admin/slack' },
				on_failure_extra_args: { db: 'res://f/other/pg' },
				error_handler_args: { nested: { deep: '$res:u/admin/slack' } }
			})
		).toEqual(['u/admin/slack', 'f/other/pg'])
	})
})

describe('flow_env and preprocessor_module', () => {
	const flowValue = {
		modules: [],
		preprocessor_module: {
			id: 'pre',
			value: { type: 'script', path: 'u/admin/preproc', input_transforms: {} }
		},
		flow_env: { SLACK: '$res:u/admin/slack', PLAIN: 'not-a-ref' }
	}

	it('walks nested children of the failure module', () => {
		const refs = extractFlowRefs({
			modules: [],
			failure_module: {
				id: 'failure',
				value: {
					type: 'forloopflow',
					modules: [
						{ id: 'f-a', value: { type: 'script', path: 'u/admin/cleanup', input_transforms: {} } }
					]
				}
			}
		})
		expect(refs).toContainEqual({ kind: 'script', path: 'u/admin/cleanup' })
	})

	it('extractFlowRefs sees preprocessor scripts and flow_env resources', () => {
		const refs = extractFlowRefs(flowValue)
		expect(refs).toContainEqual({ kind: 'script', path: 'u/admin/preproc' })
		expect(refs).toContainEqual({ kind: 'resource', path: 'u/admin/slack' })
	})

	it('sees and relocates $res refs nested inside JSON flow_env values', () => {
		const value = {
			modules: [],
			flow_env: { CFG: { db: '$res:u/admin/pg', opts: ['res://u/admin/s3'] } }
		}
		const refs = extractFlowRefs(value)
		expect(refs).toContainEqual({ kind: 'resource', path: 'u/admin/pg' })
		expect(refs).toContainEqual({ kind: 'resource', path: 'u/admin/s3' })
		const map = new Map([
			['u/admin/pg', 'f/proj/pg'],
			['u/admin/s3', 'f/proj/s3']
		])
		const out = rewriteFlowValue(value, map)
		expect(out.flow_env.CFG.db).toBe('$res:f/proj/pg')
		expect(out.flow_env.CFG.opts[0]).toBe('$res:f/proj/s3')
	})

	it('rewriteFlowValue relocates both', () => {
		const map = new Map([
			['u/admin/preproc', 'f/proj/preproc'],
			['u/admin/slack', 'f/proj/slack']
		])
		const out = rewriteFlowValue(flowValue, map)
		expect(out.preprocessor_module.value.path).toBe('f/proj/preproc')
		expect(out.flow_env.SLACK).toBe('$res:f/proj/slack')
		expect(out.flow_env.PLAIN).toBe('not-a-ref')
	})
})

describe('projectReferencesResource', () => {
	/**
	 * A project declares one resource per `resource-<type>` input schema as well as one per
	 * `$res:` reference, so an app pinning `f/proj/google_calendar` for a script whose schema
	 * says `resource-gcal` ships both it and an unreferenced `f/proj/gcal`. Only the pinned
	 * one has to hold a credential for the project to work.
	 */
	const bundle = {
		project: { slug: 'proj', name: 'Proj', summary: '', readme: null },
		scripts: [{ path: 'f/proj/send', content: 'const c = "$res:f/proj/google_calendar"' }],
		flows: [],
		apps: [],
		resources: [
			{ path: 'f/proj/gcal', resource_type: 'gcal' },
			{ path: 'f/proj/google_calendar', resource_type: 'gcal' },
			{ path: 'f/proj/db', resource_type: 'postgresql' }
		],
		triggers: [{ path: 'f/proj/ingest', config: { postgres_resource_path: 'f/proj/db' } }]
	} as unknown as ProjectExport

	it("sees a $res: token and a trigger's bare path, and not a stub nothing points at", () => {
		expect(projectReferencesResource(bundle, 'f/proj/google_calendar')).toBe(true)
		expect(projectReferencesResource(bundle, 'f/proj/db')).toBe(true)
		// Declared only because a script's input schema names the type.
		expect(projectReferencesResource(bundle, 'f/proj/gcal')).toBe(false)
	})
})

describe('textHoldsBarePath', () => {
	// Gates three deletion decisions, and its two directions cost differently: a false yes
	// keeps a stub nobody needed, a false no deletes one something still reads.
	const P = 'f/proj/db'
	it.each([
		['a token only', `const c = "$res:${P}"`, false],
		['a path written in code', `await getResource("${P}")`, true],
		['both spellings', `"$res:${P}"; getResource("${P}")`, true],
		['a longer path that starts the same', `await getResource("${P}_prod")`, false],
		['the same path inside a longer token', `"$res:${P}_prod"`, false]
	])('%s', (_label, text, expected) => {
		expect(textHoldsBarePath(text, P)).toBe(expected)
	})
})

describe('saved AI agents', () => {
	// An `ai_agent` resource value: a provider, a script tool with a fixed resource input, an
	// inline tool reading a resource by token.
	const value = {
		provider: { kind: 'anthropic', model: 'claude', resource: '$res:u/admin/claude' },
		system_prompt: 'You make videos. Never touch u/admin/helper yourself.',
		memory: { kind: 'compaction' },
		tools: [
			{
				id: 'a',
				summary: 'make_video',
				value: {
					tool_type: 'flowmodule',
					type: 'script',
					path: 'u/admin/helper',
					input_transforms: {
						gh: { type: 'static', value: '$res:f/shared/api' },
						name: { type: 'ai' }
					}
				}
			},
			{
				id: 'b',
				summary: 'lookup',
				value: {
					tool_type: 'flowmodule',
					type: 'rawscript',
					language: 'bun',
					content: 'const db = "$res:u/admin/pg"',
					input_transforms: {}
				}
			}
		]
	}

	it('finds the scripts its tools run and every resource it reaches', () => {
		expect(extractAgentRefs(value)).toEqual(
			expect.arrayContaining([
				{ kind: 'script', path: 'u/admin/helper' },
				{ kind: 'resource', path: 'f/shared/api' },
				{ kind: 'resource', path: 'u/admin/pg' },
				{ kind: 'resource', path: 'u/admin/claude' }
			])
		)
	})

	it('relocates tools, inputs and provider, and leaves the prompt alone', () => {
		const map = new Map([
			['u/admin/helper', 'f/proj/helper'],
			['f/shared/api', 'f/proj/api'],
			['u/admin/pg', 'f/proj/pg'],
			['u/admin/claude', 'f/proj/claude']
		])
		const out = rewriteAgentValue(value, map)
		expect(out.provider.resource).toBe('$res:f/proj/claude')
		expect(out.tools[0].value.path).toBe('f/proj/helper')
		expect(out.tools[0].value.input_transforms.gh.value).toBe('$res:f/proj/api')
		expect(out.tools[1].value.content).toContain('$res:f/proj/pg')
		expect(out.system_prompt).toBe(value.system_prompt)
		expect(out.memory).toEqual(value.memory)
	})

	it('ships whole in a bundle: its script comes along, and it is not also a stub', async () => {
		const agent: FetchedItem = { kind: 'agent', path: 'u/admin/video_agent', value }
		const helper: FetchedItem = {
			kind: 'script',
			path: 'u/admin/helper',
			language: 'bun',
			content: 'export async function main(){}'
		}
		// An app naming the agent would reference it as a resource: it must stay an item.
		const flow: FetchedItem = {
			kind: 'flow',
			path: 'f/proj/uses_agent',
			value: {
				modules: [
					{
						id: 'x',
						value: {
							type: 'rawscript',
							content: 'const a = "$res:u/admin/video_agent"',
							input_transforms: {}
						}
					}
				]
			}
		}
		const bundle = await buildProjectBundle(
			[
				{ kind: 'agent', path: 'u/admin/video_agent' },
				{ kind: 'flow', path: 'f/proj/uses_agent' }
			],
			'proj',
			{
				fetchItem: async (ref: ItemRef) =>
					ref.kind === 'agent' ? agent : ref.kind === 'flow' ? flow : helper,
				resolveResourceType: async (path: string) =>
					path === 'u/admin/video_agent' ? 'ai_agent' : 'some_type'
			}
		)
		expect(bundle.items.map((i) => `${i.kind}:${i.newPath}`).sort()).toEqual([
			'agent:f/proj/video_agent',
			'flow:f/proj/uses_agent',
			'script:f/proj/helper'
		])
		expect(bundle.resourceStubs.map((r) => r.resource_type)).not.toContain('ai_agent')
		const shipped = bundle.items.find((i) => i.kind === 'agent')!
		expect(shipped.value.tools[0].value.path).toBe('f/proj/helper')
		expect(shipped.value.provider.resource).toBe('$res:f/proj/claude')
	})

	it('follows a retargeted export into another folder', () => {
		const exp: ProjectExport = {
			project: { slug: 'proj', name: 'P', summary: '', readme: null },
			scripts: [{ path: 'f/proj/helper', content: '' }],
			flows: [],
			apps: [],
			resources: [{ path: 'f/proj/claude', resource_type: 'anthropic' }],
			triggers: [],
			agents: [
				{
					path: 'f/proj/video_agent',
					value: {
						provider: { kind: 'anthropic', resource: '$res:f/proj/claude' },
						tools: [{ id: 'a', value: { type: 'script', path: 'f/proj/helper' } }]
					}
				}
			]
		}
		const out = retargetProjectExport(exp, 'proj', 'mine')
		expect(out.agents?.[0].path).toBe('f/mine/video_agent')
		expect(out.agents?.[0].value.provider.resource).toBe('$res:f/mine/claude')
		expect(out.agents?.[0].value.tools[0].value.path).toBe('f/mine/helper')
	})

	it('ships and rewrites the resource an MCP tool connects to', () => {
		const value = {
			provider: { kind: 'anthropic', resource: '$res:f/proj/claude' },
			tools: [
				{ id: 'm', value: { tool_type: 'mcp', resource_path: '$res:f/proj/mcp' } },
				{ id: 'n', value: { tool_type: 'mcp', resource_path: 'f/proj/other_mcp' } }
			]
		}
		expect(extractAgentRefs(value)).toEqual(
			expect.arrayContaining([
				{ kind: 'resource', path: 'f/proj/mcp' },
				{ kind: 'resource', path: 'f/proj/other_mcp' }
			])
		)
		const map = new Map([
			['f/proj/mcp', 'f/mine/mcp'],
			['f/proj/other_mcp', 'f/mine/other_mcp']
		])
		const out = rewriteAgentValue(value, map)
		expect(out.tools[0].value.resource_path).toBe('$res:f/mine/mcp')
		expect(out.tools[1].value.resource_path).toBe('f/mine/other_mcp')
	})

	it('relocates a variable only an agent uses', () => {
		const exp: ProjectExport = {
			project: { slug: 'proj', name: 'P', summary: '', readme: null },
			scripts: [{ path: 'f/proj/helper', content: '' }],
			flows: [],
			apps: [],
			resources: [],
			triggers: [],
			agents: [
				{
					path: 'f/proj/video_agent',
					value: {
						tools: [
							{
								id: 'a',
								value: {
									type: 'script',
									path: 'f/proj/helper',
									input_transforms: { region: { type: 'static', value: '$var:f/proj/region' } }
								}
							}
						]
					}
				}
			]
		}
		expect(collectExportVarPaths(exp)).toContain('f/proj/region')
		const out = retargetProjectExport(exp, 'proj', 'mine')
		expect(out.agents?.[0].value.tools[0].value.input_transforms.region.value).toBe(
			'$var:f/mine/region'
		)
	})

	it('ships and rewrites the MCP server of a nested agent tool', () => {
		const value = {
			tools: [
				{
					id: 'inner',
					value: {
						tool_type: 'flowmodule',
						type: 'aiagent',
						input_transforms: {},
						tools: [{ id: 'm', value: { tool_type: 'mcp', resource_path: '$res:f/proj/mcp' } }]
					}
				}
			]
		}
		expect(extractAgentRefs(value)).toContainEqual({ kind: 'resource', path: 'f/proj/mcp' })
		const out = rewriteAgentValue(value, new Map([['f/proj/mcp', 'f/mine/mcp']]))
		expect(out.tools[0].value.tools[0].value.resource_path).toBe('$res:f/mine/mcp')
	})

	it('relocates the saved agent a tool is linked to', () => {
		const exp: ProjectExport = {
			project: { slug: 'proj', name: 'P', summary: '', readme: null },
			scripts: [],
			flows: [],
			apps: [],
			resources: [],
			triggers: [],
			agents: [
				{
					path: 'f/proj/parent',
					value: {
						tools: [
							{
								id: 'child',
								value: {
									tool_type: 'flowmodule',
									type: 'aiagent',
									agent: '$res:f/proj/child',
									input_transforms: {}
								}
							}
						]
					}
				},
				{ path: 'f/proj/child', value: { tools: [] } }
			]
		}
		const out = retargetProjectExport(exp, 'proj', 'mine')
		expect(out.agents?.[0].value.tools[0].value.agent).toBe('$res:f/mine/child')
	})

	it('ships a linked agent from outside the folder whole, not as a stub', async () => {
		const parent: FetchedItem = {
			kind: 'agent',
			path: 'f/proj/parent',
			value: {
				tools: [
					{
						id: 'child',
						value: {
							tool_type: 'flowmodule',
							type: 'aiagent',
							agent: 'u/admin/child',
							input_transforms: {}
						}
					}
				]
			}
		}
		const child: FetchedItem = { kind: 'agent', path: 'u/admin/child', value: { tools: [] } }
		const bundle = await buildProjectBundle([{ kind: 'agent', path: 'f/proj/parent' }], 'proj', {
			fetchItem: async (ref: ItemRef) => (ref.path === 'u/admin/child' ? child : parent),
			resolveResourceType: async (path: string) =>
				path === 'u/admin/child' ? 'ai_agent' : 'some_type'
		})
		expect(bundle.items.map((i) => `${i.kind}:${i.newPath}`).sort()).toEqual([
			'agent:f/proj/child',
			'agent:f/proj/parent'
		])
		expect(bundle.resourceStubs).toEqual([])
		const shipped = bundle.items.find((i) => i.newPath === 'f/proj/parent')!
		expect(shipped.value.tools[0].value.agent).toBe('f/proj/child')
	})

	// A saved agent is read unvalidated, so no list of fields is kept: these pin the places
	// reviews found one by one, and one nobody has named yet.
	it('finds and rewrites a reference wherever it sits in the value', () => {
		const value = {
			provider: { kind: 'anthropic', resource: '$res:f/proj/claude' },
			output_schema: '$res:f/proj/answer_schema',
			memory: { kind: 'auto', store: { resource: 'res://f/proj/store' } },
			some_future_key: [{ deep: { ref: '$res:f/proj/later' } }],
			tools: [
				{
					id: 'child',
					value: {
						tool_type: 'flowmodule',
						type: 'aiagent',
						agent: 'f/proj/child',
						input_transforms: {},
						tool_inputs: {
							query: { db: { type: 'static', value: { conn: '$res:f/proj/db' } } }
						}
					}
				},
				{
					id: 'code',
					value: {
						type: 'rawscript',
						content: 'const k = "$res:f/proj/inline"',
						input_transforms: {}
					}
				}
			]
		}
		const resources = extractAgentRefs(value)
			.filter((r) => r.kind === 'resource')
			.map((r) => r.path)
			.sort()
		expect(resources).toEqual([
			'f/proj/answer_schema',
			'f/proj/child',
			'f/proj/claude',
			'f/proj/db',
			'f/proj/inline',
			'f/proj/later',
			'f/proj/store'
		])

		const map = new Map(resources.map((p) => [p, p.replace('f/proj/', 'f/mine/')]))
		const out = rewriteAgentValue(value, map)
		expect(JSON.stringify(out)).not.toContain('f/proj/')
		expect(out.output_schema).toBe('$res:f/mine/answer_schema')
		expect(out.memory.store.resource).toBe('res://f/mine/store')
		expect(out.some_future_key[0].deep.ref).toBe('$res:f/mine/later')
		expect(out.tools[0].value.agent).toBe('f/mine/child')
		expect(out.tools[0].value.tool_inputs.query.db.value.conn).toBe('$res:f/mine/db')
		expect(out.tools[1].value.content).toBe('const k = "$res:f/mine/inline"')
		// The source value is left as it was.
		expect(value.output_schema).toBe('$res:f/proj/answer_schema')
	})

	it('reads prose and data as what they are, not as references', () => {
		const value = {
			// The worker resolves a value that is a reference as a whole, never a token in a
			// sentence: a prompt explaining the syntax must not block the publish.
			system_prompt: 'Pass resources as $res:u/user/name strings, e.g. $res:f/team/pg.',
			memory: { messages: [{ role: 'user', content: 'use $res:u/someone/else' }] },
			tools: [
				{
					id: 't',
					value: {
						type: 'script',
						path: 'f/proj/helper',
						input_transforms: {
							// A JSON argument that looks like a tool is an argument.
							arg: { type: 'static', value: { type: 'flow', path: 'u/other/thing' } },
							// Evaluated, not resolved.
							x: { type: 'javascript', expr: '"$res:f/proj/not_a_ref"' }
						}
					}
				}
			]
		}
		expect(extractAgentRefs(value)).toEqual([{ kind: 'script', path: 'f/proj/helper' }])
		const out = rewriteAgentValue(value, new Map([['u/other/thing', 'f/mine/thing']]))
		expect(out).toEqual(value)
	})

	it('hides nothing beside a javascript or static input', () => {
		// The install check walks the same way: keys set next to `expr` or `value` to make an
		// object look like an input must still be read.
		const value = {
			type: 'javascript',
			expr: '',
			provider: { kind: 'openai', resource: '$res:u/admin/prod_openai' },
			tools: [
				{
					id: 't',
					type: 'static',
					value: { tool_type: 'flowmodule', type: 'script', path: 'u/admin/danger' }
				},
				{ id: 'm', value: { tool_type: 'mcp', resource_path: 'u/admin/mcp' } }
			]
		}
		const paths = extractAgentRefs(value).map((r) => r.path)
		expect(paths.sort()).toEqual(['u/admin/danger', 'u/admin/mcp', 'u/admin/prod_openai'])
	})

	it('keeps an MCP server enabled under its new path', () => {
		const flow = {
			modules: [
				{
					id: 'a',
					value: {
						type: 'aiagent',
						tools: [{ id: 'm', value: { tool_type: 'mcp', resource_path: '$res:u/me/mcp' } }],
						input_transforms: {
							enabled_tools: { type: 'static', value: ['u/me/mcp', 'lookup'] }
						}
					}
				}
			]
		}
		const map = new Map([['u/me/mcp', 'f/proj/mcp']])
		const out = rewriteFlowValue(flow, map)
		expect(out.modules[0].value.tools[0].value.resource_path).toBe('$res:f/proj/mcp')
		expect(out.modules[0].value.input_transforms.enabled_tools.value).toEqual([
			'f/proj/mcp',
			'lookup'
		])
		// The same on an agent used as a tool by a saved agent.
		const nested = rewriteAgentValue(
			{ tools: [{ id: 'inner', value: flow.modules[0].value }] },
			map
		)
		expect(nested.tools[0].value.input_transforms.enabled_tools.value).toEqual([
			'f/proj/mcp',
			'lookup'
		])
	})

	it('ships an agent a trigger already named whole, not as a stub', async () => {
		const flow: FetchedItem = {
			kind: 'flow',
			path: 'f/proj/uses_agent',
			value: {
				modules: [
					{ id: 'a', value: { type: 'aiagent', agent: 'u/me/agent', input_transforms: {} } }
				]
			}
		}
		const agent: FetchedItem = { kind: 'agent', path: 'u/me/agent', value: { tools: [] } }
		const bundle = await buildProjectBundle(
			[{ kind: 'flow', path: 'f/proj/uses_agent' }],
			'proj',
			{
				fetchItem: async (ref: ItemRef) => (ref.kind === 'agent' ? agent : flow),
				resolveResourceType: async () => 'ai_agent'
			},
			['u/me/agent']
		)
		expect(bundle.items.map((i) => `${i.kind}:${i.newPath}`).sort()).toEqual([
			'agent:f/proj/agent',
			'flow:f/proj/uses_agent'
		])
		expect(bundle.resourceStubs).toEqual([])
	})

	it('follows the tool wiring a linked agent step holds in a flow', () => {
		const flow = {
			modules: [
				{
					id: 'a',
					value: {
						type: 'aiagent',
						agent: '$res:f/proj/helpdesk',
						input_transforms: {},
						tools: [],
						tool_inputs: {
							lookup: { db: { type: 'static', value: '$res:f/proj/db' } }
						}
					}
				}
			]
		}
		expect(extractFlowRefs(flow)).toEqual(
			expect.arrayContaining([
				{ kind: 'resource', path: 'f/proj/helpdesk' },
				{ kind: 'resource', path: 'f/proj/db' }
			])
		)
		const out = rewriteFlowValue(
			flow,
			new Map([
				['f/proj/helpdesk', 'f/mine/helpdesk'],
				['f/proj/db', 'f/mine/db']
			])
		)
		expect(out.modules[0].value.agent).toBe('$res:f/mine/helpdesk')
		expect(out.modules[0].value.tool_inputs.lookup.db.value).toBe('$res:f/mine/db')
	})

	it('drops who an agent runs as, whoever wrote the value', () => {
		const out = withoutAgentWorkspaceKeys({
			system_prompt: 'x',
			on_behalf_of: 'u/someone',
			run_on_behalf_of: true,
			slack_anyone: true
		})
		expect(out).toEqual({ system_prompt: 'x' })
		expect(withoutAgentWorkspaceKeys(undefined)).toEqual({})
	})
})
