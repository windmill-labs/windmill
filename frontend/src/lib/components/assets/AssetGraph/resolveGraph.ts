import type { AssetGraphResponse, NativeTriggerKind } from './types'
import { parsePipelineAnnotations, type PipelineAnnotations } from './parsePipelineAnnotations'
import {
	extractWrites,
	type AssetKind,
	type AssetWithAltAccessType
} from '$lib/components/assets/lib'

/** Minimal structural shape of a pipeline draft `resolveGraph` needs. */
export type GraphDraft = {
	script: { content: string }
	outputAsset?: { kind: AssetKind; path: string }
	outputAssets?: Array<{ kind: AssetKind; path: string }>
}

export type ResolveGraphInput = {
	/** Persisted graph from the backend (`/assets/graph`). */
	base: AssetGraphResponse
	/** In-flight drafts keyed by script path. */
	drafts: Map<string, GraphDraft>
	/** Body assets inferred for the currently-open script (live keystrokes). */
	liveBodyAssets: { scriptPath: string | undefined; assets: AssetWithAltAccessType[] }
	/** Pipeline annotations parsed from the currently-open buffer. */
	liveAnnotations: { scriptPath: string | undefined; annotations: PipelineAnnotations }
	/** Sticky session caches of inferred body writes/reads per script path. */
	inferredWritesByPath: Map<string, Array<{ kind: AssetKind; path: string }>>
	inferredReadsByPath: Map<string, Array<{ kind: AssetKind; path: string }>>
	/**
	 * Sticky cache of native trigger kinds declared via `// on <kind>` in
	 * each script's deployed source. Filled by the load-time prefetch
	 * sweep. Used here to emit "missing" placeholders for scripts whose
	 * annotation has no matching trigger row in `base.triggers`.
	 */
	annotatedNativeKindsByPath: Map<string, Set<NativeTriggerKind>>
}

/**
 * Merge the persisted base graph with the draft, session-inferred and
 * open-script live overlays into one `AssetGraphResponse`.
 *
 * Precedence (lowest → highest): base < session-inferred (read/write
 * lineage) < per-draft seeded triggers/outputs < live annotations for the
 * one currently-open script (which replace that path's seeded triggers).
 * Overlay edges are tagged `unsaved` and deduped against persisted edges of
 * the matching access direction.
 *
 * Pure (no Svelte reactivity) so the precedence matrix is unit-testable in
 * isolation — extracted verbatim from the pipeline page's former
 * `graphWithDraft` `$derived`. See `resolveGraph.test.ts`.
 */
export function resolveGraph(input: ResolveGraphInput): AssetGraphResponse {
	const {
		base,
		drafts,
		liveBodyAssets,
		liveAnnotations,
		inferredWritesByPath,
		inferredReadsByPath,
		annotatedNativeKindsByPath
	} = input

	// Every draft contributes: a runnable, an output asset, a write edge,
	// plus its own seeded schedule trigger (template includes `// on
	// schedule "0 * * * *"` by default, picked up through live parse).
	// We iterate the whole `drafts` map so multiple concurrent drafts
	// all render as their own subgraph at once.
	const runnables = [...base.runnables]
	const assets = [...base.assets]
	const edges = [...base.edges]
	const extraTriggers: AssetGraphResponse['triggers'] = []

	for (const [path, d] of drafts) {
		const parsed = parsePipelineAnnotations(d.script.content)
		// A draft can coexist with a base entry — during save the refetch
		// lands before drafts cleanup, and a user re-editing a deployed
		// script also produces both. In that case we mutate the existing
		// base runnable to carry `unsaved: true` instead of pushing a
		// duplicate (which would crash svelte-flow's keyed each), so the
		// canvas + trigger-node labels reflect that there's pending body
		// editing for this path.
		const baseIdx = runnables.findIndex(
			(r) => r.usage_kind === 'script' && r.path === path
		)
		if (baseIdx === -1) {
			runnables.push({
				path,
				usage_kind: 'script',
				in_pipeline: true,
				partition_kind: parsed.partition?.kind,
				freshness: parsed.freshness?.duration,
				tag: parsed.tag,
				retry: parsed.retry,
				unsaved: true
			})
		} else {
			runnables[baseIdx] = { ...runnables[baseIdx], unsaved: true }
		}
		// Output asset(s): three-tier resolution.
		//   1. Active draft (the body the user is editing right now):
		//      live body inference is authoritative — renaming a
		//      CREATE TABLE target or writeS3File path retires the
		//      old output node and surfaces the new one as the user
		//      types.
		//   2. Inactive draft with a captured `outputAssets` snapshot
		//      (taken on the last pane transition): use those, so a
		//      draft the user already edited keeps its renamed outputs
		//      after they've clicked elsewhere.
		//   3. Fallback to the static `outputAsset` seeded at draft
		//      creation — covers fresh drafts and parser misses (e.g.
		//      WIN-1943: wmill.writeS3File({s3, storage}) object form
		//      not yet detected by the TS parser).
		const liveForThisDraft = liveBodyAssets.scriptPath === path
		const writeOuts: Array<{ kind: AssetKind; path: string }> = []
		if (liveForThisDraft) {
			writeOuts.push(...extractWrites(liveBodyAssets.assets))
		} else if (d.outputAssets) {
			writeOuts.push(...d.outputAssets)
		}
		if (writeOuts.length === 0 && d.outputAsset) {
			writeOuts.push(d.outputAsset)
		}
		for (const out of writeOuts) {
			const hasAsset = assets.some((a) => a.kind === out.kind && a.path === out.path)
			if (!hasAsset) assets.push({ kind: out.kind, path: out.path })
			const hasWriteEdge = base.edges.some(
				(e) =>
					e.runnable_kind === 'script' &&
					e.runnable_path === path &&
					e.asset_kind === out.kind &&
					e.asset_path === out.path &&
					(e.access_type === 'w' || e.access_type === 'rw')
			)
			if (hasWriteEdge) continue
			edges.push({
				runnable_path: path,
				runnable_kind: 'script',
				asset_kind: out.kind,
				asset_path: out.path,
				access_type: 'w',
				unsaved: true
			})
		}
		// Seed trigger edges from the draft's template so the graph stays
		// stable when the user clicks off this draft. Live annotations
		// (below) take over for the currently-open draft so keystroke
		// edits still update in real time.
		for (const a of parsed.triggerAssets) {
			extraTriggers.push({
				trigger_kind: 'asset',
				asset_kind: a.kind,
				asset_path: a.path,
				runnable_kind: 'script',
				runnable_path: path,
				unsaved: true
			})
			// Also synthesize the asset node so the trigger edge has a
			// target even if the upstream asset isn't in base (e.g. the
			// producer script is in another folder we haven't fetched
			// or also a draft).
			const hasTriggerAsset = assets.some((x) => x.kind === a.kind && x.path === a.path)
			if (!hasTriggerAsset) assets.push({ kind: a.kind, path: a.path })
		}
		// Native trigger annotations on a draft are always "missing" until
		// the user creates the matching trigger row — drafts can't carry a
		// real trigger row since the script isn't deployed yet. Surface a
		// red placeholder so the user knows to wire it up.
		for (const n of parsed.nativeTriggers) {
			extraTriggers.push({
				trigger_kind: n.kind,
				runnable_kind: 'script',
				runnable_path: path,
				unsaved: true,
				missing: true
			})
		}
	}

	// Live-parsed overlay for the currently-open script — takes precedence
	// over the seeded-template triggers for the same path by swapping
	// them out. Scoped to one path (only one pane is open at a time).
	const livePath = liveAnnotations.scriptPath
	if (livePath) {
		const persistedAssetKeys = new Set(
			base.triggers
				.filter(
					(t) =>
						t.trigger_kind === 'asset' &&
						t.runnable_kind === 'script' &&
						t.runnable_path === livePath
				)
				.map((t) => (t.trigger_kind === 'asset' ? `${t.asset_kind}:${t.asset_path}` : ''))
		)
		// Strip seeded triggers we computed above for the active draft;
		// live annotations are authoritative for the open buffer.
		for (let i = extraTriggers.length - 1; i >= 0; i--) {
			if (extraTriggers[i].runnable_path === livePath) extraTriggers.splice(i, 1)
		}
		for (const a of liveAnnotations.annotations.triggerAssets) {
			const key = `${a.kind}:${a.path}`
			if (persistedAssetKeys.has(key)) continue
			extraTriggers.push({
				trigger_kind: 'asset',
				asset_kind: a.kind,
				asset_path: a.path,
				runnable_kind: 'script',
				runnable_path: livePath,
				unsaved: true
			})
			// Synthesize the asset node so the new trigger edge has a
			// target — without this, typing `// on s3:///...` adds an
			// edge to a node that doesn't exist and the canvas silently
			// drops it. Mirrors the draft branch above.
			if (!assets.some((x) => x.kind === a.kind && x.path === a.path)) {
				assets.push({ kind: a.kind, path: a.path })
			}
		}
		// Native trigger annotations: kinds for which a matching trigger
		// row was found in the backend response. If the live buffer
		// declares `// on kafka` and at least one kafka_trigger row points
		// at this script, the source node is already on the canvas — no
		// overlay needed. Otherwise emit a "missing" placeholder so the
		// user can either create the trigger row or remove the annotation.
		const persistedNativeKinds = new Set(
			base.triggers
				.filter(
					(t) =>
						t.trigger_kind !== 'asset' &&
						t.runnable_kind === 'script' &&
						t.runnable_path === livePath
				)
				.map((t) => t.trigger_kind)
		)
		for (const n of liveAnnotations.annotations.nativeTriggers) {
			if (persistedNativeKinds.has(n.kind)) continue
			extraTriggers.push({
				trigger_kind: n.kind,
				runnable_kind: 'script',
				runnable_path: livePath,
				unsaved: true,
				missing: true
			})
		}
	}

	// Cross-check for already-deployed scripts (not the open buffer): if a
	// script's persisted body declares `// on kafka` but no matching
	// kafka_trigger row points at it, surface a red placeholder. The
	// annotated-kinds map is filled by the page-level prefetch sweep
	// (one read per script in the folder); drafts and the active editor
	// are handled by the loops above. Scripts that haven't been swept
	// yet contribute nothing here — they'll surface on the next refetch.
	const livePathExcl = livePath
	for (const [scriptPath, kinds] of annotatedNativeKindsByPath) {
		if (drafts.has(scriptPath)) continue
		if (scriptPath === livePathExcl) continue
		const attachedKinds = new Set(
			base.triggers
				.filter(
					(t) =>
						t.trigger_kind !== 'asset' &&
						t.runnable_kind === 'script' &&
						t.runnable_path === scriptPath
				)
				.map((t) => t.trigger_kind)
		)
		for (const kind of kinds) {
			if (attachedKinds.has(kind)) continue
			extraTriggers.push({
				trigger_kind: kind,
				runnable_kind: 'script',
				runnable_path: scriptPath,
				missing: true
			})
		}
	}

	// Live body-asset lineage for any persisted script inferred at least
	// once this session (maps filled by `handleAssetsChange` + the load
	// prefetch). Drafts are handled by the loop above. For scripts whose
	// deploy didn't persist their body assets (e.g. older WASM at save
	// time, or object-form writeS3File), this keeps the lineage edge on
	// the canvas across selection changes — not just while selected.
	const overlayLineage = (
		byPath: Map<string, Array<{ kind: AssetKind; path: string }>>,
		access: 'w' | 'r'
	) => {
		for (const [scriptPath, refs] of byPath) {
			if (drafts.has(scriptPath)) continue
			const persisted = new Set(
				base.edges
					.filter(
						(e) =>
							e.runnable_path === scriptPath &&
							e.runnable_kind === 'script' &&
							(e.access_type === access || e.access_type === 'rw')
					)
					.map((e) => `${e.asset_kind}:${e.asset_path}`)
			)
			for (const a of refs) {
				const key = `${a.kind}:${a.path}`
				if (persisted.has(key)) continue
				if (!assets.some((x) => x.kind === a.kind && x.path === a.path)) {
					assets.push({ kind: a.kind, path: a.path })
				}
				edges.push({
					runnable_path: scriptPath,
					runnable_kind: 'script',
					asset_kind: a.kind,
					asset_path: a.path,
					access_type: access,
					unsaved: true
				})
			}
		}
	}
	overlayLineage(inferredWritesByPath, 'w')
	overlayLineage(inferredReadsByPath, 'r')

	return { ...base, assets, runnables, edges, triggers: [...base.triggers, ...extraTriggers] }
}
