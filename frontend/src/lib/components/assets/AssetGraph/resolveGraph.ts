import type { AssetGraphMacroEdge, AssetGraphResponse, NativeTriggerKind } from './types'
import {
	mergeColumnLineage,
	parsePipelineAnnotations,
	scd2CurrentTargetPath,
	type ColumnLineage,
	type PipelineAnnotations
} from './parsePipelineAnnotations'
import {
	extractWrites,
	extractReads,
	type AssetKind,
	type AssetWithAltAccessType
} from '$lib/components/assets/lib'

/** Minimal structural shape of a pipeline draft `resolveGraph` needs. */
export type GraphDraft = {
	script: { content: string }
	outputAssets?: Array<{ kind: AssetKind; path: string }>
}

export type ResolveGraphInput = {
	/** Persisted graph from the backend (`/assets/graph`). */
	base: AssetGraphResponse
	/** In-flight drafts keyed by script path. */
	drafts: Map<string, GraphDraft>
	/** Body assets inferred for the currently-open script (live keystrokes). */
	liveBodyAssets: {
		scriptPath: string | undefined
		assets: AssetWithAltAccessType[]
		/** Body-inferred column lineage (DuckDB SQL AST) for the open script. */
		columnLineage?: ColumnLineage[]
	}
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

/** Mutable bag the sequential passes accumulate the resolved graph into. */
type Accumulator = {
	runnables: AssetGraphResponse['runnables']
	assets: AssetGraphResponse['assets']
	edges: AssetGraphResponse['edges']
	extraTriggers: AssetGraphResponse['triggers']
}

/**
 * Native trigger kinds with a persisted (non-asset) trigger row pointing at
 * `path`. These bind by script path and survive content edits, so they dedup
 * against draft/live annotations to avoid showing a real source node next to a
 * red "missing" placeholder for the same kind.
 */
function persistedNativeKinds(base: AssetGraphResponse, path: string): Set<string> {
	return new Set(
		base.triggers
			.filter(
				(t) =>
					t.trigger_kind !== 'asset' && t.runnable_kind === 'script' && t.runnable_path === path
			)
			.map((t) => t.trigger_kind)
	)
}

// Read-asset kinds that auto-derive a cascade trigger edge inside a
// `// pipeline`. Mirror of the backend `is_auto_trigger_kind`
// (windmill-common assets.rs) — ducklake tables and s3 objects only.
const AUTO_TRIGGER_KINDS: ReadonlySet<AssetKind> = new Set(['ducklake', 's3object'])

/**
 * Backend-mirror of `derive_pipeline_asset_trigger_refs` (windmill-common
 * assets.rs): a pipeline script's ducklake/s3 read auto-wires a cascade
 * trigger edge from the FROM clause, so `// on <asset>` is only needed for
 * edges inference can't see. Excluded: assets the script also writes (`writes`
 * covers `w`/`rw`, so an `rw` self-read can't loop-trigger), muted assets,
 * `// mute all`, and any explicit `// on` (which already emits its own edge).
 * Returns the `{kind, path}` refs to overlay as unsaved asset triggers, deduped.
 */
function deriveAutoAssetTriggers(
	reads: Array<{ kind: AssetKind; path: string }>,
	writes: Array<{ kind: AssetKind; path: string }>,
	parsed: PipelineAnnotations
): Array<{ kind: AssetKind; path: string }> {
	// Auto-derivation is scoped to `// pipeline` scripts (backend gates on
	// `in_pipeline`); `// mute all` opts a pipeline script back out.
	if (!parsed.inPipeline || parsed.muteAll) return []
	const skip = new Set<string>([
		...writes.map((w) => `${w.kind}:${w.path}`),
		...parsed.muteAssets.map((a) => `${a.kind}:${a.path}`),
		...parsed.triggerAssets.map((a) => `${a.kind}:${a.path}`)
	])
	const out: Array<{ kind: AssetKind; path: string }> = []
	for (const r of reads) {
		const key = `${r.kind}:${r.path}`
		if (!AUTO_TRIGGER_KINDS.has(r.kind) || skip.has(key)) continue
		skip.add(key) // dedup within reads too
		out.push({ kind: r.kind, path: r.path })
	}
	return out
}

/** `kind:path` keys of persisted asset (`// on <asset>`) triggers for `path`. */
function persistedAssetKeys(base: AssetGraphResponse, path: string): Set<string> {
	return new Set(
		base.triggers
			.filter(
				(t) =>
					t.trigger_kind === 'asset' && t.runnable_kind === 'script' && t.runnable_path === path
			)
			.map((t) => (t.trigger_kind === 'asset' ? `${t.asset_kind}:${t.asset_path}` : ''))
	)
}

/**
 * Emit a red "missing" placeholder for each annotated native kind that has no
 * matching attached trigger row. `opts.unsaved` marks editor-driven overlays
 * (drafts / live buffer); deployed-but-unswept scripts pass `unsaved: false`.
 */
function pushMissingNativeTriggers(
	extraTriggers: AssetGraphResponse['triggers'],
	annotatedKinds: Iterable<NativeTriggerKind>,
	attachedKinds: Set<string>,
	path: string,
	opts: { unsaved: boolean }
) {
	for (const kind of annotatedKinds) {
		if (attachedKinds.has(kind)) continue
		extraTriggers.push({
			trigger_kind: kind,
			runnable_kind: 'script',
			runnable_path: path,
			...(opts.unsaved ? { unsaved: true } : {}),
			missing: true
		})
	}
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
	const { base } = input
	const ctx = makeContext(input)

	// Pass order mirrors the precedence comment, lowest → highest:
	const acc = seedAccumulator(input, ctx)
	seedDraftOverlays(acc, input)
	applyLiveBufferOverlay(acc, input, ctx)
	crossCheckSweptScripts(acc, input)
	overlayInferredLineage(acc, input)

	// Drop persisted ASSET triggers for drafted paths — those come from the
	// deployed `// on <asset>` annotations, which the draft's live/parsed
	// annotations now own. Native triggers (kafka/schedule/…) are kept: they
	// bind by `script_path`, which the draft shares, so the attachment is still
	// valid regardless of content edits.
	const baseTriggers = base.triggers.filter((t) => {
		if (t.trigger_kind !== 'asset') return true
		if (ctx.isDrafted(t.runnable_kind, t.runnable_path)) return false
		if (
			t.runnable_kind === 'script' &&
			t.runnable_path === ctx.openPath &&
			ctx.staleForOpen(t.asset_kind, t.asset_path)
		)
			return false
		return true
	})
	// Mirror the backend's skip-if-empty: no `macro_edges` key at all when
	// there is nothing to show (also keeps the no-macros response shape
	// byte-identical to before the feature).
	const macroEdges = resolveMacroEdges(input)
	return {
		...base,
		assets: acc.assets,
		runnables: acc.runnables,
		edges: acc.edges,
		triggers: [...baseTriggers, ...acc.extraTriggers],
		...(macroEdges.length > 0 || base.macro_edges ? { macro_edges: macroEdges } : {})
	}
}

/**
 * Macro-library → consumer edges: the deployed base edges, with `// use`
 * declarations of overlaid scripts (drafts + the open buffer) taking over
 * their consumer's `via_use` edges so adding/removing a `// use` line updates
 * the canvas live. Detection-based edges (macro calls in the deployed body)
 * are backend-owned and only refresh on redeploy.
 */
function resolveMacroEdges(input: ResolveGraphInput): AssetGraphMacroEdge[] {
	const { base, drafts, liveAnnotations } = input
	const libMacroNames = new Map<string, string[]>()
	for (const r of base.runnables) {
		if (r.usage_kind === 'script' && r.macros?.length) {
			libMacroNames.set(
				r.path,
				r.macros.map((m) => m.name)
			)
		}
	}
	const useByPath = new Map<string, string[]>()
	for (const [path, d] of drafts) {
		useByPath.set(path, parsePipelineAnnotations(d.script.content).useLibs)
	}
	if (liveAnnotations.scriptPath) {
		// `?? []` — callers may hand a minimal annotations object (tests, older
		// call sites) that predates the field.
		useByPath.set(liveAnnotations.scriptPath, liveAnnotations.annotations.useLibs ?? [])
	}
	const out: AssetGraphMacroEdge[] = []
	for (const e of base.macro_edges ?? []) {
		if (e.via_use && useByPath.has(e.consumer_path)) continue
		out.push({ ...e })
	}
	for (const [path, libs] of useByPath) {
		for (const lib of libs) {
			const existing = out.find((e) => e.lib_path === lib && e.consumer_path === path)
			if (existing) {
				// Upgrade the detection edge in place: `// use` pulls in the whole
				// library, so the edge covers every macro the lib defines.
				existing.via_use = true
				existing.unsaved = true
				existing.macro_names = [
					...new Set([...existing.macro_names, ...(libMacroNames.get(lib) ?? [])])
				]
			} else {
				out.push({
					lib_path: lib,
					consumer_path: path,
					macro_names: libMacroNames.get(lib) ?? [],
					via_use: true,
					unsaved: true
				})
			}
		}
	}
	return out
}

// Light regex extraction of a draft macro library's definitions for the live
// node badge. The strict grammar lives in the Rust `parse_macro_library` at
// deploy; this only needs name/params/table-ness for display (nested parens
// in a default value may truncate the shown signature, never the deploy).
const MACRO_DEF_RE =
	/create\s+(?:or\s+replace\s+)?(?:temp(?:orary)?\s+)?(?:macro|function)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(([^)]*)\)\s*as\s+(table\b)?/gi

export function extractDraftMacros(
	content: string
): { name: string; params: string; is_table: boolean }[] {
	const out: { name: string; params: string; is_table: boolean }[] = []
	for (const m of content.matchAll(MACRO_DEF_RE)) {
		out.push({ name: m[1].toLowerCase(), params: m[2].trim(), is_table: m[3] !== undefined })
	}
	return out
}

type ResolveContext = {
	draftedPaths: Set<string>
	isDrafted: (kind: string, p: string) => boolean
	openPath: string | undefined
	staleForOpen: (kind: AssetKind, path: string) => boolean
}

/** Derive the shared "what's drafted / open / stale" predicates once. */
function makeContext(input: ResolveGraphInput): ResolveContext {
	const { drafts, liveBodyAssets, liveAnnotations } = input

	// A drafted path's content-derived lineage is owned entirely by the draft
	// overlay (live inference for the active draft, parsed annotations /
	// snapshot for inactive ones). Drop persisted base asset edges for those
	// paths so it shows only the draft's I/O, not the union of the saved version
	// and the in-flight edits. (Base asset triggers are dropped at the return;
	// native triggers are kept — they bind by path.)
	const draftedPaths = new Set(drafts.keys())
	const isDrafted = (kind: string, p: string) => kind === 'script' && draftedPaths.has(p)

	const openPath = liveBodyAssets.scriptPath
	const openIsSavedEdit = openPath !== undefined && !draftedPaths.has(openPath)
	const liveRefKeys = new Set<string>()
	if (openIsSavedEdit) {
		if (liveAnnotations.scriptPath === openPath) {
			for (const a of liveAnnotations.annotations.triggerAssets)
				liveRefKeys.add(`${a.kind}:${a.path}`)
			// The `// materialize <asset>` target is a declared *output*, but it
			// lives in an annotation (not the SQL body), so neither triggerAssets
			// (inputs) nor the body-inferred assets cover it. Without this its
			// persisted write-edge is judged stale and dropped the moment the
			// script is selected/edited — leaving the output asset unlinked. A
			// managed scd2 producer persists a second write to the `<dim>_current`
			// companion view, so keep that too or a consumer of only the view
			// orphans while the producer is open for editing.
			const m = liveAnnotations.annotations.materialize
			if (m) {
				liveRefKeys.add(`${m.targetKind}:${m.targetPath}`)
				const currentPath = scd2CurrentTargetPath(m)
				if (currentPath) liveRefKeys.add(`${m.targetKind}:${currentPath}`)
			}
		}
		for (const a of liveBodyAssets.assets) liveRefKeys.add(`${a.kind}:${a.path}`)
	}
	const staleForOpen = (kind: AssetKind, path: string) =>
		openIsSavedEdit && !liveRefKeys.has(`${kind}:${path}`)

	return { draftedPaths, isDrafted, openPath, staleForOpen }
}

/** Base graph plus the persisted edges that survive the draft/open filter. */
function seedAccumulator(input: ResolveGraphInput, ctx: ResolveContext): Accumulator {
	const { base } = input
	const edges = base.edges.filter((e) => {
		if (ctx.isDrafted(e.runnable_kind, e.runnable_path)) return false
		if (
			e.runnable_kind === 'script' &&
			e.runnable_path === ctx.openPath &&
			ctx.staleForOpen(e.asset_kind, e.asset_path)
		)
			return false
		return true
	})
	return {
		runnables: [...base.runnables],
		assets: [...base.assets],
		edges,
		extraTriggers: []
	}
}

/**
 * Every draft contributes: a runnable, output asset(s), a write edge, live
 * read lineage for the active draft, plus its seeded asset/native triggers
 * (template includes `// on schedule …` by default). Iterates the whole
 * `drafts` map so multiple concurrent drafts all render at once.
 */
function seedDraftOverlays(acc: Accumulator, input: ResolveGraphInput) {
	const { base, drafts, liveBodyAssets, inferredReadsByPath, inferredWritesByPath } = input
	const { runnables, assets, edges, extraTriggers } = acc

	for (const [path, d] of drafts) {
		const parsed = parsePipelineAnnotations(d.script.content)
		// For the open script, fold in the WASM-inferred column lineage (DuckDB
		// SQL AST) under the same annotation-wins precedence the backend applies
		// on deploy, so the live preview matches what deploys. Only the open
		// script carries live inference (`liveBodyAssets`); other drafts stay
		// annotation-only until they deploy (the backend infers then).
		const inferredCL =
			path === liveBodyAssets.scriptPath ? (liveBodyAssets.columnLineage ?? []) : []
		const mergedCL = mergeColumnLineage(inferredCL, parsed.columnLineage)
		// The `// materialize` target this draft's column lineage describes, so
		// the column graph anchors to it rather than guessing a write-edge.
		const materializeTarget = parsed.materialize
			? { kind: parsed.materialize.targetKind, path: parsed.materialize.targetPath }
			: undefined
		// A draft can coexist with a base entry — during save the refetch
		// lands before drafts cleanup, and a user re-editing a deployed
		// script also produces both. In that case we mutate the existing
		// base runnable to carry `unsaved: true` instead of pushing a
		// duplicate (which would crash svelte-flow's keyed each), so the
		// canvas + trigger-node labels reflect that there's pending body
		// editing for this path.
		// `// macros` library draft: extract the definitions for the live node
		// badge (regex-light; the strict parse happens at deploy).
		const draftMacros = parsed.macros ? extractDraftMacros(d.script.content) : []
		const baseIdx = runnables.findIndex((r) => r.usage_kind === 'script' && r.path === path)
		if (baseIdx === -1) {
			runnables.push({
				path,
				usage_kind: 'script',
				in_pipeline: true,
				partition_kind: parsed.partition?.kind,
				freshness: parsed.freshness?.duration,
				tag: parsed.tag,
				retry: parsed.retry,
				data_tests: parsed.dataTests.length > 0 ? parsed.dataTests : undefined,
				column_lineage: mergedCL.length > 0 ? mergedCL : undefined,
				materialize_target: materializeTarget,
				macros: draftMacros.length > 0 ? draftMacros : undefined,
				unsaved: true
			})
		} else {
			// Refresh annotation-derived badges from the live parse too, so
			// adding/removing `// data_test` / `// column` lines on an
			// already-deployed script updates the badge immediately (not only
			// after redeploy/refetch).
			runnables[baseIdx] = {
				...runnables[baseIdx],
				data_tests: parsed.dataTests.length > 0 ? parsed.dataTests : undefined,
				column_lineage: mergedCL.length > 0 ? mergedCL : undefined,
				materialize_target: materializeTarget,
				macros: draftMacros.length > 0 ? draftMacros : undefined,
				unsaved: true
			}
		}
		// Output asset(s): two-tier resolution.
		//   1. Active draft (the body the user is editing right now):
		//      live body inference is authoritative — renaming a
		//      CREATE TABLE target or writeS3File path retires the
		//      old output node and surfaces the new one as the user
		//      types.
		//   2. Inactive draft: its captured `outputAssets` (inferred at
		//      creation/last edit, or the seeded output for a fresh draft
		//      whose body doesn't yet write anything inferable).
		const liveForThisDraft = liveBodyAssets.scriptPath === path
		const writeOuts: Array<{ kind: AssetKind; path: string; derivedFrom?: string }> = []
		if (liveForThisDraft) {
			writeOuts.push(...extractWrites(liveBodyAssets.assets))
		} else if (d.outputAssets) {
			writeOuts.push(...d.outputAssets)
		}
		// `// materialize <asset>` declares a write output via annotation, not
		// the SQL body, so the body-inference tiers above miss it. Add it from
		// the live-parsed annotations so an edited materialize script keeps its
		// output edge (the loop below dedups against existing assets/edges). A
		// managed scd2 materialize also produces the `<dim>_current` companion
		// view — add it too (mirrors the deploy path) so a draft consuming only
		// the view links back to this producer instead of orphaning.
		if (parsed.materialize) {
			writeOuts.push({
				kind: parsed.materialize.targetKind,
				path: parsed.materialize.targetPath
			})
			const currentPath = scd2CurrentTargetPath(parsed.materialize)
			if (currentPath) {
				writeOuts.push({
					kind: parsed.materialize.targetKind,
					path: currentPath,
					derivedFrom: parsed.materialize.targetPath
				})
			}
		}
		for (const out of writeOuts) {
			const existing = assets.find((a) => a.kind === out.kind && a.path === out.path)
			if (!existing) {
				assets.push({
					kind: out.kind,
					path: out.path,
					...(out.derivedFrom ? { derived_from: out.derivedFrom } : {})
				})
			} else if (out.derivedFrom && existing.derived_from == undefined) {
				existing.derived_from = out.derivedFrom
			}
			// Dedup against edges already in the overlay (this draft's base
			// edges were dropped above, so this only guards against duplicate
			// writeOuts entries — not against the persisted version).
			const hasWriteEdge = edges.some(
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
		// Live read lineage for the active draft (body reads like loadS3File /
		// SELECT). Only the open draft has live-inferred assets; inactive drafts
		// fall back to their `// on <asset>` annotations below for inputs. Without
		// this, dropping the persisted base edges above would lose the input
		// edges of a saved script the moment the user starts editing it.
		if (liveForThisDraft) {
			for (const inp of extractReads(liveBodyAssets.assets)) {
				if (!assets.some((a) => a.kind === inp.kind && a.path === inp.path)) {
					assets.push({ kind: inp.kind, path: inp.path })
				}
				const hasReadEdge = edges.some(
					(e) =>
						e.runnable_kind === 'script' &&
						e.runnable_path === path &&
						e.asset_kind === inp.kind &&
						e.asset_path === inp.path &&
						(e.access_type === 'r' || e.access_type === 'rw')
				)
				if (hasReadEdge) continue
				edges.push({
					runnable_path: path,
					runnable_kind: 'script',
					asset_kind: inp.kind,
					asset_path: inp.path,
					access_type: 'r',
					unsaved: true
				})
			}
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
		// Auto-derived cascade edges (backend parity): a ducklake/s3 read wires
		// the edge from the body alone. Reads/writes come from the active draft's
		// live inference, else the sticky session cache. The open buffer's derived
		// edges are re-computed authoritatively in applyLiveBufferOverlay (which
		// strips this path's seeded triggers first), same as the explicit `// on`
		// triggers above.
		const draftReads = liveForThisDraft
			? extractReads(liveBodyAssets.assets)
			: (inferredReadsByPath.get(path) ?? [])
		const draftWrites = liveForThisDraft
			? extractWrites(liveBodyAssets.assets)
			: (inferredWritesByPath.get(path) ?? [])
		for (const a of deriveAutoAssetTriggers(draftReads, draftWrites, parsed)) {
			extraTriggers.push({
				trigger_kind: 'asset',
				asset_kind: a.kind,
				asset_path: a.path,
				runnable_kind: 'script',
				runnable_path: path,
				unsaved: true
			})
			if (!assets.some((x) => x.kind === a.kind && x.path === a.path)) {
				assets.push({ kind: a.kind, path: a.path })
			}
		}
		// Native trigger annotations on a draft are "missing" until a
		// matching trigger row exists. A brand-new draft never has one (the
		// script isn't deployed yet), but a draft promoted from unsaved
		// edits to a *deployed* script keeps its persisted trigger rows —
		// they bind by path and are not dropped above. Dedup against those
		// so the canvas doesn't show the real schedule node next to a red
		// "missing" placeholder for the same kind.
		pushMissingNativeTriggers(
			extraTriggers,
			parsed.nativeTriggers.map((n) => n.kind),
			persistedNativeKinds(base, path),
			path,
			{ unsaved: true }
		)
	}
}

/**
 * Live-parsed overlay for the currently-open script — takes precedence over
 * the seeded-template triggers for the same path by swapping them out. Scoped
 * to one path (only one pane is open at a time).
 */
function applyLiveBufferOverlay(acc: Accumulator, input: ResolveGraphInput, ctx: ResolveContext) {
	const { base, liveAnnotations } = input
	const { assets, extraTriggers } = acc
	const livePath = liveAnnotations.scriptPath
	if (!livePath) return

	// When the open buffer is a draft, its base asset triggers were dropped
	// above (draft-owned), so don't dedup live asset triggers against them —
	// every live `// on <asset>` is emitted as unsaved. For a non-draft open
	// script (viewing a saved one), keep deduping so live re-parse doesn't
	// double the persisted triggers.
	const assetKeys = ctx.draftedPaths.has(livePath)
		? new Set<string>()
		: persistedAssetKeys(base, livePath)
	// Strip seeded triggers we computed above for the active draft;
	// live annotations are authoritative for the open buffer.
	for (let i = extraTriggers.length - 1; i >= 0; i--) {
		if (extraTriggers[i].runnable_path === livePath) extraTriggers.splice(i, 1)
	}
	for (const a of liveAnnotations.annotations.triggerAssets) {
		const key = `${a.kind}:${a.path}`
		if (assetKeys.has(key)) continue
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
	// Auto-derived cascade edges (backend parity): a ducklake/s3 read wires
	// the edge from the FROM clause alone, keystroke-live. The open buffer's
	// live body inference is authoritative for its reads/writes. `// mute` /
	// `// mute all` and explicit `// on` (above) suppress a derived edge; a
	// derived edge already persisted for a non-draft open script is deduped
	// via `assetKeys`.
	if (input.liveBodyAssets.scriptPath === livePath) {
		const reads = extractReads(input.liveBodyAssets.assets)
		const writes = extractWrites(input.liveBodyAssets.assets)
		for (const a of deriveAutoAssetTriggers(reads, writes, liveAnnotations.annotations)) {
			if (assetKeys.has(`${a.kind}:${a.path}`)) continue
			extraTriggers.push({
				trigger_kind: 'asset',
				asset_kind: a.kind,
				asset_path: a.path,
				runnable_kind: 'script',
				runnable_path: livePath,
				unsaved: true
			})
			if (!assets.some((x) => x.kind === a.kind && x.path === a.path)) {
				assets.push({ kind: a.kind, path: a.path })
			}
		}
	}
	// Native trigger annotations: kinds for which a matching trigger
	// row was found in the backend response. If the live buffer
	// declares `// on kafka` and at least one kafka_trigger row points
	// at this script, the source node is already on the canvas — no
	// overlay needed. Otherwise emit a "missing" placeholder so the
	// user can either create the trigger row or remove the annotation.
	pushMissingNativeTriggers(
		extraTriggers,
		liveAnnotations.annotations.nativeTriggers.map((n) => n.kind),
		persistedNativeKinds(base, livePath),
		livePath,
		{ unsaved: true }
	)
}

/**
 * Cross-check for already-deployed scripts (not the open buffer): if a
 * script's persisted body declares `// on kafka` but no matching kafka_trigger
 * row points at it, surface a red placeholder. The annotated-kinds map is
 * filled by the page-level prefetch sweep (one read per script in the folder);
 * drafts and the active editor are handled by the earlier passes. Scripts not
 * yet swept contribute nothing — they surface on the next refetch.
 */
function crossCheckSweptScripts(acc: Accumulator, input: ResolveGraphInput) {
	const { base, drafts, liveAnnotations, annotatedNativeKindsByPath } = input
	const livePath = liveAnnotations.scriptPath
	for (const [scriptPath, kinds] of annotatedNativeKindsByPath) {
		if (drafts.has(scriptPath)) continue
		if (scriptPath === livePath) continue
		pushMissingNativeTriggers(
			acc.extraTriggers,
			kinds,
			persistedNativeKinds(base, scriptPath),
			scriptPath,
			{ unsaved: false }
		)
	}
}

/**
 * Live body-asset lineage for any persisted script inferred at least once this
 * session (maps filled by `handleAssetsChange` + the load prefetch). Drafts
 * are handled by `seedDraftOverlays`. For scripts whose deploy didn't persist
 * their body assets (e.g. older WASM at save time, or object-form
 * writeS3File), this keeps the lineage edge on the canvas across selection
 * changes — not just while selected.
 */
function overlayInferredLineage(acc: Accumulator, input: ResolveGraphInput) {
	const { base, drafts, inferredWritesByPath, inferredReadsByPath } = input
	const { assets, edges } = acc

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
}
