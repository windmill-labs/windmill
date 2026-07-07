import { describe, expect, it } from 'vitest'
import { resolveGraph, computeMutedReadKeys, type ResolveGraphInput } from './resolveGraph'
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
	muteAssets: [],
	muteAll: false,
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

const duck = (path: string, access_type: 'r' | 'w' | 'rw'): AssetWithAltAccessType =>
	({ kind: 'ducklake', path, access_type }) as AssetWithAltAccessType

/** kind:path of the unsaved asset-trigger overlays for a runnable. */
const assetTrigKeys = (r: AssetGraphResponse, path: string): string[] =>
	r.triggers
		.filter((t) => t.trigger_kind === 'asset' && t.runnable_path === path && (t as any).unsaved)
		.map((t) => `${(t as any).asset_kind}:${(t as any).asset_path}`)

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

	it('open buffer: a ducklake/s3 read auto-derives an unsaved cascade trigger', () => {
		const liveBodyAssets = {
			scriptPath: 'f/x/open',
			assets: [duck('main.orders', 'r'), s3('raw/events', 'r'), duck('main.out', 'w')]
		}
		const liveAnnotations = { scriptPath: 'f/x/open', annotations: ann({ inPipeline: true }) }
		const r = resolveGraph(input({ liveBodyAssets, liveAnnotations }))
		// The two reads derive edges; the write (main.out) does not (self-edge).
		expect(assetTrigKeys(r, 'f/x/open').sort()).toEqual([
			'ducklake:main.orders',
			's3object:raw/events'
		])
	})

	it('open buffer: a materialize producer reading its own target does not self-cascade', () => {
		// The body `SELECT`s from the table it materializes (incremental model).
		// The write is annotation-declared, not in the body, so without excluding
		// the materialize target the read would auto-derive a self-cascade edge.
		const liveBodyAssets = { scriptPath: 'f/x/p', assets: [duck('main.orders', 'r')] }
		const liveAnnotations = {
			scriptPath: 'f/x/p',
			annotations: ann({
				inPipeline: true,
				materialize: { targetKind: 'ducklake' as const, targetPath: 'main.orders' }
			})
		}
		const r = resolveGraph(input({ liveBodyAssets, liveAnnotations }))
		expect(assetTrigKeys(r, 'f/x/p')).toEqual([])
	})

	it('open buffer: a read that is also written (rw) does not self-trigger', () => {
		const liveBodyAssets = { scriptPath: 'f/x/open', assets: [duck('main.self', 'rw')] }
		const liveAnnotations = { scriptPath: 'f/x/open', annotations: ann({ inPipeline: true }) }
		const r = resolveGraph(input({ liveBodyAssets, liveAnnotations }))
		expect(assetTrigKeys(r, 'f/x/open')).toEqual([])
	})

	it('open buffer: // mute suppresses one derived edge, // mute all suppresses all', () => {
		const liveBodyAssets = {
			scriptPath: 'f/x/open',
			assets: [duck('main.a', 'r'), duck('main.b', 'r')]
		}
		const muted = resolveGraph(
			input({
				liveBodyAssets,
				liveAnnotations: {
					scriptPath: 'f/x/open',
					annotations: ann({ inPipeline: true, muteAssets: [{ kind: 'ducklake', path: 'main.a' }] })
				}
			})
		)
		expect(assetTrigKeys(muted, 'f/x/open')).toEqual(['ducklake:main.b'])

		const all = resolveGraph(
			input({
				liveBodyAssets,
				liveAnnotations: {
					scriptPath: 'f/x/open',
					annotations: ann({ inPipeline: true, muteAll: true })
				}
			})
		)
		expect(assetTrigKeys(all, 'f/x/open')).toEqual([])
	})

	it('open buffer: an explicit // on is not double-emitted as a derived edge', () => {
		const liveBodyAssets = { scriptPath: 'f/x/open', assets: [duck('main.orders', 'r')] }
		const liveAnnotations = {
			scriptPath: 'f/x/open',
			annotations: ann({
				inPipeline: true,
				triggerAssets: [{ kind: 'ducklake', path: 'main.orders' }]
			})
		}
		const r = resolveGraph(input({ liveBodyAssets, liveAnnotations }))
		// Exactly one overlay for the table, not one explicit + one derived.
		expect(assetTrigKeys(r, 'f/x/open')).toEqual(['ducklake:main.orders'])
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

describe('computeMutedReadKeys', () => {
	const readEdge = (asset_kind: any, asset_path: string, access: any = 'r') => ({
		runnable_path: 'f/x/c',
		runnable_kind: 'script' as const,
		asset_kind,
		asset_path,
		access_type: access
	})
	const assetTrigger = (asset_kind: any, asset_path: string) => ({
		trigger_kind: 'asset' as const,
		asset_kind,
		asset_path,
		runnable_kind: 'script' as const,
		runnable_path: 'f/x/c'
	})
	// `f/x/c` (the read consumer) is an in-pipeline script.
	const pipelineRunnable = [{ path: 'f/x/c', usage_kind: 'script' as const, in_pipeline: true }]

	it('flags a ducklake/s3 read with no cascade trigger as muted', () => {
		const muted = computeMutedReadKeys(
			[readEdge('ducklake', 'main.orders'), readEdge('s3object', 'raw/events')],
			[],
			pipelineRunnable
		)
		expect([...muted].sort()).toEqual([
			'ducklake:main.orders->script:f/x/c',
			's3object:raw/events->script:f/x/c'
		])
	})

	it('does not flag a read that has a cascade trigger', () => {
		const muted = computeMutedReadKeys(
			[readEdge('ducklake', 'main.orders')],
			[assetTrigger('ducklake', 'main.orders')],
			pipelineRunnable
		)
		expect(muted.size).toBe(0)
	})

	it('ignores rw self-reads and unsupported kinds', () => {
		const muted = computeMutedReadKeys(
			[
				readEdge('ducklake', 'main.self', 'rw'), // self-read, not muted
				readEdge('resource', 'f/db'), // out of scope
				readEdge('datatable', 'main.dt') // out of scope
			],
			[],
			pipelineRunnable
		)
		expect(muted.size).toBe(0)
	})

	it('does not flag a read whose script also writes the asset', () => {
		// Live-overlay shape of a `// materialize` producer reading its own target:
		// a separate `'r'` read edge and `'w'` write edge for the same asset. It's
		// the script's own output, not a suppressed input, so it is not muted.
		const muted = computeMutedReadKeys(
			[readEdge('ducklake', 'main.orders', 'r'), readEdge('ducklake', 'main.orders', 'w')],
			[],
			pipelineRunnable
		)
		expect(muted.size).toBe(0)
	})

	it('does not flag a read by a non-pipeline script (auto-derivation never applied)', () => {
		// Same read, but the consumer is a plain script (or a flow), not a
		// `// pipeline` member — no auto trigger was ever derived to suppress.
		const plainScript = [{ path: 'f/x/c', usage_kind: 'script' as const, in_pipeline: false }]
		expect(computeMutedReadKeys([readEdge('ducklake', 'main.orders')], [], plainScript).size).toBe(
			0
		)
		const flow = [{ path: 'f/x/c', usage_kind: 'flow' as const, in_pipeline: true }]
		expect(computeMutedReadKeys([readEdge('ducklake', 'main.orders')], [], flow).size).toBe(0)
	})
})

describe('live buffer overlays (open script)', () => {
	// A deployed producer: `f/x/prod` materializes ducklake main/orders.
	const deployedProducer = () =>
		baseGraph({
			assets: [{ kind: 'ducklake', path: 'main/orders' }],
			runnables: [{ path: 'f/x/prod', usage_kind: 'script', in_pipeline: true }],
			edges: [
				{
					runnable_path: 'f/x/prod',
					runnable_kind: 'script',
					asset_kind: 'ducklake',
					asset_path: 'main/orders',
					access_type: 'w'
				}
			]
		})

	it('open draft: live annotations win over the stale draft snapshot for the materialize target', () => {
		const drafts = new Map([
			['f/x/d', { script: { content: '-- materialize ducklake://main/old_target\nselect 1' } }]
		])
		const r = resolveGraph(
			input({
				drafts,
				liveBodyAssets: { scriptPath: 'f/x/d', assets: [] },
				liveAnnotations: {
					scriptPath: 'f/x/d',
					annotations: ann({
						inPipeline: true,
						materialize: { targetKind: 'ducklake', targetPath: 'main/new_target' }
					})
				}
			})
		)
		expect(r.assets).toContainEqual({ kind: 'ducklake', path: 'main/new_target' })
		expect(r.assets).not.toContainEqual({ kind: 'ducklake', path: 'main/old_target' })
		expect(r.edges).toContainEqual({
			runnable_path: 'f/x/d',
			runnable_kind: 'script',
			asset_kind: 'ducklake',
			asset_path: 'main/new_target',
			access_type: 'w',
			unsaved: true
		})
	})

	it('open saved script: retargeting `// materialize` swaps the write edge live', () => {
		const r = resolveGraph(
			input({
				base: deployedProducer(),
				liveBodyAssets: { scriptPath: 'f/x/prod', assets: [] },
				liveAnnotations: {
					scriptPath: 'f/x/prod',
					annotations: ann({
						inPipeline: true,
						materialize: { targetKind: 'ducklake', targetPath: 'main/orders_gold' }
					})
				}
			})
		)
		// New target surfaces as an unsaved write edge…
		expect(r.assets).toContainEqual({ kind: 'ducklake', path: 'main/orders_gold' })
		expect(r.edges).toContainEqual({
			runnable_path: 'f/x/prod',
			runnable_kind: 'script',
			asset_kind: 'ducklake',
			asset_path: 'main/orders_gold',
			access_type: 'w',
			unsaved: true
		})
		// …the stale write edge is dropped, but the deployed dataset node stays.
		expect(
			r.edges.filter((e) => e.asset_path === 'main/orders' && e.runnable_path === 'f/x/prod')
		).toEqual([])
		expect(r.assets).toContainEqual({ kind: 'ducklake', path: 'main/orders' })
	})

	it('open saved script: unchanged materialize target dedups against the persisted write edge', () => {
		const r = resolveGraph(
			input({
				base: deployedProducer(),
				liveBodyAssets: { scriptPath: 'f/x/prod', assets: [] },
				liveAnnotations: {
					scriptPath: 'f/x/prod',
					annotations: ann({
						inPipeline: true,
						materialize: { targetKind: 'ducklake', targetPath: 'main/orders' }
					})
				}
			})
		)
		expect(
			r.edges.filter(
				(e) =>
					e.runnable_path === 'f/x/prod' &&
					e.asset_path === 'main/orders' &&
					(e.access_type === 'w' || e.access_type === 'rw')
			)
		).toHaveLength(1)
	})

	it('open saved script: live body reads/writes overlay as unsaved lineage', () => {
		const base = baseGraph({
			assets: [{ kind: 'ducklake', path: 'main/orders' }],
			runnables: [{ path: 'f/x/cons', usage_kind: 'script', in_pipeline: true }],
			edges: [
				{
					runnable_path: 'f/x/cons',
					runnable_kind: 'script',
					asset_kind: 'ducklake',
					asset_path: 'main/orders',
					access_type: 'r'
				}
			]
		})
		const r = resolveGraph(
			input({
				base,
				liveBodyAssets: {
					scriptPath: 'f/x/cons',
					assets: [duck('main/orders_eu', 'r'), s3('/report.parquet', 'w')]
				},
				liveAnnotations: {
					scriptPath: 'f/x/cons',
					annotations: ann({ inPipeline: true })
				}
			})
		)
		// New read + write from the buffer…
		expect(r.edges).toContainEqual({
			runnable_path: 'f/x/cons',
			runnable_kind: 'script',
			asset_kind: 'ducklake',
			asset_path: 'main/orders_eu',
			access_type: 'r',
			unsaved: true
		})
		expect(r.edges).toContainEqual({
			runnable_path: 'f/x/cons',
			runnable_kind: 'script',
			asset_kind: 's3object',
			asset_path: '/report.parquet',
			access_type: 'w',
			unsaved: true
		})
		// …the no-longer-read persisted edge is dropped.
		expect(
			r.edges.filter((e) => e.asset_path === 'main/orders' && e.runnable_path === 'f/x/cons')
		).toEqual([])
	})
})
