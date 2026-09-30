import { untrack } from 'svelte'
import { ScriptService, type AssetKind, type ScriptLang } from '$lib/gen'
import { inferAssets } from '$lib/infer'
import {
	extractReads,
	extractWrites,
	type AssetWithAltAccessType
} from '$lib/components/assets/lib'
import { parsePipelineAnnotations } from './parsePipelineAnnotations'
import type { AssetGraphResponse, NativeTriggerKind } from './types'
import type { PipelineEditorState } from './pipelineEditorState.svelte'

type AssetRef = { kind: AssetKind; path: string }

/**
 * Reads the body of every deployed script of a pipeline's graph and infers its
 * asset reads/writes and `// on <native trigger>` annotations, so the canvas
 * draws the edges of scripts the user never opened. The open script's live
 * buffer wins over its cached body.
 *
 * The caches are only-add and keyed by path: the derived maps iterate the
 * current graph's scripts, so a renamed or deleted script drops out when the
 * graph refetches. A stale value under a live path does not, so a save through
 * the editor must `forget` its path for the sweep to read it again.
 */
export function usePipelineAssetPrefetch(deps: {
	getWorkspace: () => string | undefined
	getGraph: () => AssetGraphResponse | undefined
	editor: PipelineEditorState
}) {
	const pe = deps.editor
	let bodies = $state<Map<string, string>>(new Map())
	// Deployed summary + language from the same fetch, for node titles and icons.
	let scriptMeta = $state<Map<string, { summary?: string; language: ScriptLang }>>(new Map())
	let inferredAssets = $state<Map<string, AssetWithAltAccessType[]>>(new Map())
	let prefetching = $state(false)
	// Bumped by every sweep so one started for an older graph stops writing.
	let gen = 0

	$effect(() => {
		const ws = deps.getWorkspace()
		const g = deps.getGraph()
		if (!ws || !g) return
		const sweep = ++gen
		const targets = untrack(() =>
			g.runnables
				.filter((r) => r.usage_kind === 'script')
				.map((r) => r.path)
				.filter((p) => !pe.drafts.has(p) && !bodies.has(p))
		)
		if (targets.length === 0) return
		let i = 0
		const POOL = 6
		const worker = async () => {
			while (i < targets.length && sweep === gen) {
				const path = targets[i++]
				try {
					const s = await ScriptService.getScriptByPath({ workspace: ws, path })
					if (sweep !== gen) return
					const content = s.content ?? ''
					const res = await inferAssets(s.language, content)
					if (sweep !== gen) return
					const inferred = (res?.assets ?? []) as AssetWithAltAccessType[]
					untrack(() => {
						if (!bodies.has(path)) bodies = new Map(bodies).set(path, content)
						if (!scriptMeta.has(path)) {
							scriptMeta = new Map(scriptMeta).set(path, {
								summary: s.summary || undefined,
								language: s.language
							})
						}
						if (!inferredAssets.has(path)) {
							inferredAssets = new Map(inferredAssets).set(path, inferred)
						}
					})
				} catch {
					// Skip — that node just falls back to base-graph edges.
				}
			}
		}
		prefetching = true
		const pool = Array.from({ length: Math.min(POOL, targets.length) }, () => worker())
		void Promise.all(pool).then(() => {
			if (sweep === gen) prefetching = false
		})
	})

	// Both edge maps in one pass over the graph's scripts; they only differ by
	// extractWrites vs extractReads over the same source.
	const edges = $derived.by(() => {
		const writes = new Map<string, AssetRef[]>()
		const reads = new Map<string, AssetRef[]>()
		const g = deps.getGraph()
		if (!g) return { writes, reads }
		for (const r of g.runnables) {
			if (r.usage_kind !== 'script') continue
			const assets =
				pe.liveBodyAssets.scriptPath === r.path
					? pe.liveBodyAssets.assets
					: inferredAssets.get(r.path)
			if (!assets) continue
			const w = extractWrites(assets)
			if (w.length > 0) writes.set(r.path, w)
			const rd = extractReads(assets)
			if (rd.length > 0) reads.set(r.path, rd)
		}
		return { writes, reads }
	})

	const annotatedNativeKinds = $derived.by(() => {
		const out = new Map<string, Set<NativeTriggerKind>>()
		const g = deps.getGraph()
		if (!g) return out
		const livePath = pe.liveAnnotations.scriptPath
		for (const r of g.runnables) {
			if (r.usage_kind !== 'script') continue
			let kinds: Set<NativeTriggerKind>
			if (r.path === livePath) {
				kinds = new Set(pe.liveAnnotations.annotations.nativeTriggers.map((n) => n.kind))
			} else {
				const body = bodies.get(r.path)
				if (!body) continue
				kinds = new Set(parsePipelineAnnotations(body).nativeTriggers.map((n) => n.kind))
			}
			if (kinds.size > 0) out.set(r.path, kinds)
		}
		return out
	})

	return {
		get bodies(): ReadonlyMap<string, string> {
			return bodies
		},
		get scriptMeta(): ReadonlyMap<string, { summary?: string; language: ScriptLang }> {
			return scriptMeta
		},
		get inferredWritesByPath() {
			return edges.writes
		},
		get inferredReadsByPath() {
			return edges.reads
		},
		get annotatedNativeKindsByPath() {
			return annotatedNativeKinds
		},
		get prefetching() {
			return prefetching
		},
		/** Drop cached paths so the next sweep re-reads them (after a save). */
		forget(...paths: string[]) {
			const nextBodies = new Map(bodies)
			const nextMeta = new Map(scriptMeta)
			const nextInferred = new Map(inferredAssets)
			for (const p of paths) {
				nextBodies.delete(p)
				nextMeta.delete(p)
				nextInferred.delete(p)
			}
			bodies = nextBodies
			scriptMeta = nextMeta
			inferredAssets = nextInferred
		}
	}
}
