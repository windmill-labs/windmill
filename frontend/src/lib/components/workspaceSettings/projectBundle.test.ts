import { describe, it, expect } from 'vitest'
import {
	classifyPath,
	extractScriptRefs,
	extractFlowRefs,
	buildPathMap,
	rewriteContent,
	rewriteFlowValue,
	buildProjectBundle,
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
