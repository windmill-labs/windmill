import { describe, it, expect } from 'vitest'
import {
	classifyPath,
	extractScriptRefs,
	extractFlowRefs,
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
	extractTriggerConfigResourceRefs,
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
