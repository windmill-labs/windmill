import { describe, expect, it } from 'vitest'
import { resolveGraph, type ResolveGraphInput } from './resolveGraph'
import type { PipelineAnnotations } from './parsePipelineAnnotations'
import type { AssetGraphResponse } from './types'
import type { AssetWithAltAccessType } from '$lib/components/assets/lib'

const ann = (over: Partial<PipelineAnnotations> = {}): PipelineAnnotations => ({
	inPipeline: false,
	triggerAssets: [],
	nativeTriggers: [],
	dataTests: [],
	columnLineage: [],
	macros: false,
	useLibs: [],
	...over
})

const baseGraph = (over: Partial<AssetGraphResponse> = {}): AssetGraphResponse => ({
	assets: [],
	runnables: [],
	edges: [],
	triggers: [],
	...over
})

const input = (over: Partial<ResolveGraphInput> = {}): ResolveGraphInput => ({
	base: baseGraph(),
	drafts: new Map(),
	liveBodyAssets: { scriptPath: undefined, assets: [] },
	liveAnnotations: { scriptPath: undefined, annotations: ann() },
	inferredWritesByPath: new Map(),
	inferredReadsByPath: new Map(),
	annotatedNativeKindsByPath: new Map(),
	...over
})

const s3 = (path: string, access_type: 'r' | 'w' | 'rw'): AssetWithAltAccessType =>
	({ kind: 's3object', path, access_type }) as AssetWithAltAccessType

describe('resolveGraph', () => {
	it('passes an empty graph through unchanged', () => {
		const r = resolveGraph(input())
		expect(r).toEqual({ assets: [], runnables: [], edges: [], triggers: [] })
	})

	it('preserves persisted base nodes/edges/triggers', () => {
		const base = baseGraph({
			assets: [{ kind: 's3object', path: '/a.json' }],
			runnables: [{ path: 'f/x/prod', usage_kind: 'script' }],
			edges: [
				{
					runnable_path: 'f/x/prod',
					runnable_kind: 'script',
					asset_kind: 's3object',
					asset_path: '/a.json',
					access_type: 'w'
				}
			],
			triggers: [
				{
					trigger_kind: 'asset',
					asset_kind: 's3object',
					asset_path: '/a.json',
					runnable_kind: 'script',
					runnable_path: 'f/x/cons'
				}
			]
		})
		expect(resolveGraph(input({ base }))).toEqual(base)
	})

	it('draft: adds an unsaved runnable + write edge from outputAssets', () => {
		const drafts = new Map([
			[
				'f/x/d',
				{
					script: { content: '' },
					outputAssets: [{ kind: 's3object' as const, path: '/out.json' }]
				}
			]
		])
		const r = resolveGraph(input({ drafts }))
		expect(r.runnables).toContainEqual({
			path: 'f/x/d',
			usage_kind: 'script',
			in_pipeline: true,
			partition_kind: undefined,
			freshness: undefined,
			unsaved: true
		})
		expect(r.assets).toContainEqual({ kind: 's3object', path: '/out.json' })
		expect(r.edges).toContainEqual({
			runnable_path: 'f/x/d',
			runnable_kind: 'script',
			asset_kind: 's3object',
			asset_path: '/out.json',
			access_type: 'w',
			unsaved: true
		})
	})

	it('scd2 materialize draft: writes both the base dim and its _current companion view', () => {
		const drafts = new Map([
			[
				'f/x/dim',
				{
					script: {
						content: '-- materialize ducklake://main/dim_customers key=id history\nselect 1'
					}
				}
			]
		])
		const r = resolveGraph(input({ drafts }))
		// Base dimension: a plain write output node.
		expect(r.assets).toContainEqual({ kind: 'ducklake', path: 'main/dim_customers' })
		// Companion `_current` view: same producer, marked derived from the base.
		expect(r.assets).toContainEqual({
			kind: 'ducklake',
			path: 'main/dim_customers_current',
			derived_from: 'main/dim_customers'
		})
		for (const path of ['main/dim_customers', 'main/dim_customers_current']) {
			expect(r.edges).toContainEqual({
				runnable_path: 'f/x/dim',
				runnable_kind: 'script',
				asset_kind: 'ducklake',
				asset_path: path,
				access_type: 'w',
				unsaved: true
			})
		}
	})

	it('non-scd2 materialize draft: writes only the base dim, no _current companion', () => {
		const drafts = new Map([
			[
				'f/x/dim',
				{ script: { content: '-- materialize ducklake://main/dim_customers key=id\nselect 1' } }
			]
		])
		const r = resolveGraph(input({ drafts }))
		expect(r.assets).toContainEqual({ kind: 'ducklake', path: 'main/dim_customers' })
		expect(r.assets.some((a) => a.path === 'main/dim_customers_current')).toBe(false)
	})

	it('active draft: live body writes are authoritative over the snapshot', () => {
		const drafts = new Map([
			[
				'f/x/d',
				{
					script: { content: '' },
					outputAssets: [{ kind: 's3object' as const, path: '/snapshot.json' }]
				}
			]
		])
		const r = resolveGraph(
			input({ drafts, liveBodyAssets: { scriptPath: 'f/x/d', assets: [s3('/live.json', 'w')] } })
		)
		expect(r.edges.map((e) => e.asset_path)).toContain('/live.json')
		expect(r.edges.map((e) => e.asset_path)).not.toContain('/snapshot.json')
	})

	it('session-inferred writes overlay a persisted script, deduped vs base', () => {
		const base = baseGraph({
			edges: [
				{
					runnable_path: 'f/x/prod',
					runnable_kind: 'script',
					asset_kind: 's3object',
					asset_path: '/already.json',
					access_type: 'w'
				}
			]
		})
		const inferredWritesByPath = new Map([
			[
				'f/x/prod',
				[
					{ kind: 's3object' as const, path: '/already.json' }, // deduped (in base)
					{ kind: 's3object' as const, path: '/fresh.json' } // added unsaved
				]
			]
		])
		const r = resolveGraph(input({ base, inferredWritesByPath }))
		const w = r.edges.filter((e) => e.runnable_path === 'f/x/prod' && e.access_type === 'w')
		expect(w).toHaveLength(2) // 1 persisted + 1 overlay (no dup of /already.json)
		expect(w.find((e) => e.asset_path === '/fresh.json')?.unsaved).toBe(true)
		expect(r.assets).toContainEqual({ kind: 's3object', path: '/fresh.json' })
	})

	it('session-inferred reads overlay with r/rw dedup', () => {
		const base = baseGraph({
			edges: [
				{
					runnable_path: 'f/x/prod',
					runnable_kind: 'script',
					asset_kind: 's3object',
					asset_path: '/rw.json',
					access_type: 'rw'
				}
			]
		})
		const inferredReadsByPath = new Map([
			[
				'f/x/prod',
				[
					{ kind: 's3object' as const, path: '/rw.json' }, // deduped (base has rw)
					{ kind: 's3object' as const, path: '/in.csv' } // added as 'r'
				]
			]
		])
		const r = resolveGraph(input({ base, inferredReadsByPath }))
		const reads = r.edges.filter((e) => e.access_type === 'r')
		expect(reads).toHaveLength(1)
		expect(reads[0]).toMatchObject({ asset_path: '/in.csv', unsaved: true })
	})

	it('does not overlay inferred lineage onto a path that is a draft', () => {
		const drafts = new Map([['f/x/d', { script: { content: '' } }]])
		const inferredWritesByPath = new Map([
			['f/x/d', [{ kind: 's3object' as const, path: '/should-not.json' }]]
		])
		const r = resolveGraph(input({ drafts, inferredWritesByPath }))
		expect(r.edges.find((e) => e.asset_path === '/should-not.json')).toBeUndefined()
	})

	it('open-script live annotations add unsaved triggers, deduped vs persisted', () => {
		const base = baseGraph({
			triggers: [
				{
					trigger_kind: 'asset',
					asset_kind: 's3object',
					asset_path: '/persisted.json',
					runnable_kind: 'script',
					runnable_path: 'f/x/open'
				}
			]
		})
		const liveAnnotations = {
			scriptPath: 'f/x/open',
			annotations: ann({
				triggerAssets: [
					{ kind: 's3object', path: '/persisted.json' }, // deduped
					{ kind: 's3object', path: '/new-trigger.json' } // added unsaved
				],
				nativeTriggers: [{ kind: 'schedule' }]
			})
		}
		const r = resolveGraph(input({ base, liveAnnotations }))
		const assetTrigs = r.triggers.filter(
			(t) => t.trigger_kind === 'asset' && t.runnable_path === 'f/x/open'
		)
		expect(assetTrigs).toHaveLength(2) // 1 persisted + 1 unsaved overlay
		expect(
			r.triggers.find(
				(t) => t.trigger_kind === 'asset' && (t as any).asset_path === '/new-trigger.json'
			)?.unsaved
		).toBe(true)
		// Schedule joins the native-trigger family — marker-only, missing
		// until the user creates the schedule row separately.
		expect(
			r.triggers.some(
				(t) =>
					t.trigger_kind === 'schedule' &&
					(t as any).missing === true &&
					t.runnable_path === 'f/x/open'
			)
		).toBe(true)
	})

	it('editing a saved script shows only the draft I/O, not the persisted edges', () => {
		// Saved script with persisted read/write lineage + an asset trigger.
		const base = baseGraph({
			runnables: [{ path: 'f/x/prod', usage_kind: 'script' }],
			edges: [
				{
					runnable_path: 'f/x/prod',
					runnable_kind: 'script',
					asset_kind: 's3object',
					asset_path: '/old-out.json',
					access_type: 'w'
				},
				{
					runnable_path: 'f/x/prod',
					runnable_kind: 'script',
					asset_kind: 's3object',
					asset_path: '/old-in.json',
					access_type: 'r'
				}
			],
			triggers: [
				{
					trigger_kind: 'asset',
					asset_kind: 's3object',
					asset_path: '/old-trig.json',
					runnable_kind: 'script',
					runnable_path: 'f/x/prod'
				}
			]
		})
		// The user is now editing it: a draft at the same path, open, with new
		// live-inferred reads/writes and a new live `// on` annotation.
		const drafts = new Map([['f/x/prod', { script: { content: '// on s3:///new-trig.json' } }]])
		const r = resolveGraph(
			input({
				base,
				drafts,
				liveBodyAssets: {
					scriptPath: 'f/x/prod',
					assets: [s3('/new-out.json', 'w'), s3('/new-in.json', 'r')]
				},
				liveAnnotations: {
					scriptPath: 'f/x/prod',
					annotations: ann({ triggerAssets: [{ kind: 's3object', path: '/new-trig.json' }] })
				}
			})
		)
		const paths = r.edges.filter((e) => e.runnable_path === 'f/x/prod').map((e) => e.asset_path)
		// Only the draft's I/O — the persisted edges are gone.
		expect(paths.sort()).toEqual(['/new-in.json', '/new-out.json'])
		const trigs = r.triggers.filter((t) => t.runnable_path === 'f/x/prod')
		expect(trigs).toHaveLength(1)
		expect(trigs[0]).toMatchObject({ trigger_kind: 'asset', asset_path: '/new-trig.json' })
	})

	it('editing a saved script (open, not a draft) drops the renamed input, keeps output', () => {
		// Saved DuckDB-style script: `// on s3://old.csv` + body read of old.csv
		// + datatable write. Persisted base has all three.
		const base = baseGraph({
			runnables: [{ path: 'f/x/prod', usage_kind: 'script' }],
			edges: [
				{
					runnable_path: 'f/x/prod',
					runnable_kind: 'script',
					asset_kind: 's3object',
					asset_path: 'old.csv',
					access_type: 'r'
				},
				{
					runnable_path: 'f/x/prod',
					runnable_kind: 'script',
					asset_kind: 'datatable',
					asset_path: 'main/out',
					access_type: 'w'
				}
			],
			triggers: [
				{
					trigger_kind: 'asset',
					asset_kind: 's3object',
					asset_path: 'old.csv',
					runnable_kind: 'script',
					runnable_path: 'f/x/prod'
				}
			]
		})
		// Open + edited (NOT a draft): input renamed old.csv → new.csv in both the
		// `// on` annotation and the body read; output unchanged.
		const r = resolveGraph(
			input({
				base,
				liveAnnotations: {
					scriptPath: 'f/x/prod',
					annotations: ann({ triggerAssets: [{ kind: 's3object', path: 'new.csv' }] })
				},
				liveBodyAssets: {
					scriptPath: 'f/x/prod',
					assets: [
						s3('new.csv', 'r'),
						{ kind: 'datatable', path: 'main/out', access_type: 'w' } as AssetWithAltAccessType
					]
				}
			})
		)
		// The old input is gone from both edges and triggers…
		expect(r.edges.some((e) => e.asset_path === 'old.csv')).toBe(false)
		expect(r.triggers.some((t) => (t as any).asset_path === 'old.csv')).toBe(false)
		// …the new input shows (as an unsaved trigger from the live annotation)…
		expect(
			r.triggers.some(
				(t) => t.trigger_kind === 'asset' && (t as any).asset_path === 'new.csv' && t.unsaved
			)
		).toBe(true)
		// …and the unchanged output write edge is preserved.
		expect(r.edges.some((e) => e.asset_path === 'main/out' && e.access_type === 'w')).toBe(true)
	})

	it('editing a saved scd2 producer keeps both the base and _current persisted write edges', () => {
		// Deploy persists a write to both `main/dim` and `main/dim_current`.
		// Opening the producer for editing must not judge the companion `_current`
		// write stale — otherwise a consumer of only the view orphans mid-edit.
		const base = baseGraph({
			runnables: [{ path: 'f/x/dim', usage_kind: 'script' }],
			edges: [
				{
					runnable_path: 'f/x/dim',
					runnable_kind: 'script',
					asset_kind: 'ducklake',
					asset_path: 'main/dim_customers',
					access_type: 'w'
				},
				{
					runnable_path: 'f/x/dim',
					runnable_kind: 'script',
					asset_kind: 'ducklake',
					asset_path: 'main/dim_customers_current',
					access_type: 'w'
				}
			]
		})
		const r = resolveGraph(
			input({
				base,
				liveAnnotations: {
					scriptPath: 'f/x/dim',
					annotations: ann({
						materialize: {
							targetKind: 'ducklake',
							targetPath: 'main/dim_customers',
							uniqueKey: 'id',
							scd2: true
						}
					})
				},
				liveBodyAssets: { scriptPath: 'f/x/dim', assets: [] }
			})
		)
		for (const path of ['main/dim_customers', 'main/dim_customers_current']) {
			expect(
				r.edges.some(
					(e) => e.runnable_path === 'f/x/dim' && e.asset_path === path && e.access_type === 'w'
				)
			).toBe(true)
		}
	})

	it('selecting a saved script unchanged drops nothing (no stale removal)', () => {
		const base = baseGraph({
			runnables: [{ path: 'f/x/prod', usage_kind: 'script' }],
			edges: [
				{
					runnable_path: 'f/x/prod',
					runnable_kind: 'script',
					asset_kind: 's3object',
					asset_path: 'in.csv',
					access_type: 'r'
				}
			],
			triggers: [
				{
					trigger_kind: 'asset',
					asset_kind: 's3object',
					asset_path: 'in.csv',
					runnable_kind: 'script',
					runnable_path: 'f/x/prod'
				}
			]
		})
		const r = resolveGraph(
			input({
				base,
				liveAnnotations: {
					scriptPath: 'f/x/prod',
					annotations: ann({ triggerAssets: [{ kind: 's3object', path: 'in.csv' }] })
				},
				liveBodyAssets: { scriptPath: 'f/x/prod', assets: [s3('in.csv', 'r')] }
			})
		)
		// Persisted edge + trigger untouched (not marked unsaved, not dropped).
		expect(r.edges).toEqual(base.edges)
		const assetTrigs = r.triggers.filter((t) => t.trigger_kind === 'asset')
		expect(assetTrigs).toHaveLength(1)
		expect(assetTrigs[0]).not.toHaveProperty('unsaved', true)
	})

	it('editing a saved script keeps its native (path-bound) triggers', () => {
		// A kafka trigger row points at the script by path — editing the body
		// (draft) must not drop it, since the binding survives content edits.
		const base = baseGraph({
			runnables: [{ path: 'f/x/prod', usage_kind: 'script' }],
			triggers: [
				{
					trigger_kind: 'kafka',
					path: 'f/x/kafka_trig',
					runnable_kind: 'script',
					runnable_path: 'f/x/prod'
				} as any
			]
		})
		const drafts = new Map([['f/x/prod', { script: { content: '// on kafka' } }]])
		const r = resolveGraph(input({ base, drafts }))
		expect(
			r.triggers.some(
				(t) =>
					t.trigger_kind === 'kafka' &&
					t.runnable_path === 'f/x/prod' &&
					(t as any).path === 'f/x/kafka_trig'
			)
		).toBe(true)
	})

	it('live annotations for a draft replace that draft’s seeded triggers', () => {
		// Draft body seeds a schedule annotation; the live buffer has
		// replaced it with an asset trigger — only the live one should remain.
		const drafts = new Map([['f/x/d', { script: { content: '// on schedule' } }]])
		const liveAnnotations = {
			scriptPath: 'f/x/d',
			annotations: ann({ triggerAssets: [{ kind: 's3object', path: '/live-trig.json' }] })
		}
		const r = resolveGraph(input({ drafts, liveAnnotations }))
		const forDraft = r.triggers.filter((t) => t.runnable_path === 'f/x/d')
		expect(forDraft).toHaveLength(1)
		expect(forDraft[0]).toMatchObject({ trigger_kind: 'asset', unsaved: true })
	})

	it('draft of a deployed script: no missing placeholder for a native kind with a persisted row', () => {
		// Promoted draft (unsaved edits to a deployed script) — the schedule
		// row still points at the path, so the seeded `// on schedule`
		// annotation must NOT add a red "missing" node next to the real one.
		const base = baseGraph({
			runnables: [{ path: 'f/x/prod', usage_kind: 'script' }],
			triggers: [
				{
					trigger_kind: 'schedule',
					path: 'f/x/sched',
					runnable_kind: 'script',
					runnable_path: 'f/x/prod'
				} as any
			]
		})
		const drafts = new Map([['f/x/prod', { script: { content: '// on schedule' } }]])
		const r = resolveGraph(input({ base, drafts }))
		const scheduleTriggers = r.triggers.filter(
			(t) => t.trigger_kind === 'schedule' && t.runnable_path === 'f/x/prod'
		)
		expect(scheduleTriggers).toHaveLength(1)
		expect((scheduleTriggers[0] as any).missing).toBeUndefined()
	})

	it('brand-new draft: native annotation without a persisted row still surfaces as missing', () => {
		const drafts = new Map([['f/x/new', { script: { content: '// on schedule' } }]])
		const r = resolveGraph(input({ drafts }))
		expect(r.triggers).toContainEqual({
			trigger_kind: 'schedule',
			runnable_kind: 'script',
			runnable_path: 'f/x/new',
			unsaved: true,
			missing: true
		})
	})

	it('macro edges: base passes through; live `// use` adds an unsaved via_use edge', () => {
		const base = baseGraph({
			runnables: [
				{
					path: 'f/lib/stats',
					usage_kind: 'script',
					macros: [{ name: 'safe_div', params: 'a, b', is_table: false }]
				},
				{ path: 'f/x/cons', usage_kind: 'script' }
			],
			macro_edges: [
				{
					lib_path: 'f/lib/stats',
					consumer_path: 'f/x/cons',
					macro_names: ['safe_div'],
					via_use: false
				}
			]
		})
		const r = resolveGraph(
			input({
				base,
				liveAnnotations: {
					scriptPath: 'f/x/other',
					annotations: ann({ useLibs: ['f/lib/stats'] })
				}
			})
		)
		// Detection edge preserved untouched.
		expect(r.macro_edges).toContainEqual({
			lib_path: 'f/lib/stats',
			consumer_path: 'f/x/cons',
			macro_names: ['safe_div'],
			via_use: false
		})
		// Live `// use` synthesizes an unsaved whole-lib edge with the lib's names.
		expect(r.macro_edges).toContainEqual({
			lib_path: 'f/lib/stats',
			consumer_path: 'f/x/other',
			macro_names: ['safe_div'],
			via_use: true,
			unsaved: true
		})
	})

	it('macro edges: removing the `// use` line of an overlaid consumer retires its via_use edge', () => {
		const base = baseGraph({
			macro_edges: [
				{
					lib_path: 'f/lib/stats',
					consumer_path: 'f/x/cons',
					macro_names: ['safe_div'],
					via_use: true
				}
			]
		})
		const r = resolveGraph(
			input({
				base,
				liveAnnotations: { scriptPath: 'f/x/cons', annotations: ann() }
			})
		)
		expect(r.macro_edges).toEqual([])
	})

	it('macro edges: draft `// macros` library gets the ƒ badge data from its body', () => {
		const drafts = new Map([
			[
				'f/lib/new',
				{
					script: {
						content: '// macros\nCREATE OR REPLACE MACRO dbl(a) AS a * 2;'
					}
				}
			]
		])
		const r = resolveGraph(input({ drafts }))
		const lib = r.runnables.find((x) => x.path === 'f/lib/new')
		expect(lib?.macros).toEqual([{ name: 'dbl', params: 'a', is_table: false }])
	})
})
