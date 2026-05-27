import { describe, expect, it } from 'vitest'
import { resolveGraph, type ResolveGraphInput } from './resolveGraph'
import type { PipelineAnnotations } from './parsePipelineAnnotations'
import type { AssetGraphResponse } from './types'
import type { AssetWithAltAccessType } from '$lib/components/assets/lib'

const ann = (over: Partial<PipelineAnnotations> = {}): PipelineAnnotations => ({
	inPipeline: false,
	triggerAssets: [],
	nativeTriggers: [],
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

	it('draft: adds an unsaved runnable + write edge from the static outputAsset', () => {
		const drafts = new Map([
			[
				'f/x/d',
				{ script: { content: '' }, outputAsset: { kind: 's3object' as const, path: '/out.json' } }
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

	it('draft: outputAssets snapshot wins over the static outputAsset', () => {
		const drafts = new Map([
			[
				'f/x/d',
				{
					script: { content: '' },
					outputAsset: { kind: 's3object' as const, path: '/old.json' },
					outputAssets: [{ kind: 's3object' as const, path: '/new.json' }]
				}
			]
		])
		const r = resolveGraph(input({ drafts }))
		expect(r.edges.map((e) => e.asset_path)).toContain('/new.json')
		expect(r.edges.map((e) => e.asset_path)).not.toContain('/old.json')
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
})
