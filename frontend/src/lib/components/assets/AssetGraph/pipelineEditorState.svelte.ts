import type { AssetKind, Script } from '$lib/gen'
import type { AssetWithAltAccessType } from '$lib/components/assets/lib'
import type { AssetGraphSelection } from './types'
import { parsePipelineAnnotations, type PipelineAnnotations } from './parsePipelineAnnotations'
import type { PipelineDraft } from './pipelineAiHelpers'

// ============================================================================
// Externalized pipeline-editor state — the data-pipeline analogue of the flow
// editor's `flowStore` / `flowStateStore`. It owns the in-flight draft Map, the
// live editor overlays, and the current selection: the substrate the route page
// editor and (later) the in-session preview both render through. Persistence,
// graph resolution, run dispatch, and deploy stay with the consumer; this is a
// plain reactive bag so a consumer can read/mutate it without prop plumbing.
//
// Step 1 of the FlowBuilder-style consolidation: the route page constructs one
// instance and references it in place, with no behaviour change. Subsequent
// steps move the rendering into a shared <PipelineGraphEditor>.
// ============================================================================

const EMPTY_ANNOTATIONS: PipelineAnnotations = parsePipelineAnnotations('')

export type LiveAnnotations = { scriptPath: string | undefined; annotations: PipelineAnnotations }
export type LiveBodyAssets = { scriptPath: string | undefined; assets: AssetWithAltAccessType[] }
export type LiveContent = { scriptPath: string | undefined; content: string }

export class PipelineEditorState {
	/** In-flight drafts keyed by script path (manual + AI-staged). */
	drafts = $state<Map<string, PipelineDraft>>(new Map())
	/** Draft open in the details pane (mutually exclusive with `selection`). */
	activeDraftPath = $state<string | undefined>(undefined)
	/** The persisted node/asset selected on the canvas. */
	selection = $state<AssetGraphSelection | undefined>(undefined)

	/** Live-parsed annotations of the open script (refreshed per keystroke). */
	liveAnnotations = $state<LiveAnnotations>({
		scriptPath: undefined,
		annotations: EMPTY_ANNOTATIONS
	})
	/** Live-inferred body read/write assets of the open script. */
	liveBodyAssets = $state<LiveBodyAssets>({ scriptPath: undefined, assets: [] })
	/** The open draft's live editor buffer. */
	liveContent = $state<LiveContent>({ scriptPath: undefined, content: '' })

	/** Set true once a draft bundle was restored from the DB on load — drives the
	 * route toolbar's one-shot "Loaded from draft" hint. Written by the editor's
	 * autosave hydrate when persistence is enabled. */
	loadedFromDbDraft = $state(false)

	#nextDraftLocalId = 0
	// Arrow fields so `pe.method` can be passed straight as a callback (the
	// details pane takes onDraftPersist / onAnnotationsChange / … by reference).
	newDraftLocalId = (): string => {
		this.#nextDraftLocalId += 1
		return `pe-${this.#nextDraftLocalId}`
	}

	handleAnnotationsChange = (scriptPath: string | undefined, annotations: PipelineAnnotations) => {
		this.liveAnnotations = { scriptPath, annotations }
	}
	handleAssetsChange = (scriptPath: string | undefined, assets: AssetWithAltAccessType[]) => {
		this.liveBodyAssets = { scriptPath, assets }
	}
	handleContentChange = (scriptPath: string | undefined, content: string) => {
		this.liveContent = { scriptPath, content }
	}

	clearLiveOverlays = () => {
		this.liveAnnotations = { scriptPath: undefined, annotations: EMPTY_ANNOTATIONS }
		this.liveBodyAssets = { scriptPath: undefined, assets: [] }
		this.liveContent = { scriptPath: undefined, content: '' }
	}

	/** Drop per-path editor state when a path goes away. Does NOT touch
	 * consumer-owned per-path state (e.g. the route page's save errors — the
	 * route layers that on in its own wrapper). */
	forgetPath = (path: string) => {
		if (this.activeDraftPath === path) this.activeDraftPath = undefined
		if (this.selection?.kind === 'runnable' && this.selection.path === path)
			this.selection = undefined
		if (this.liveAnnotations.scriptPath === path)
			this.liveAnnotations = { scriptPath: undefined, annotations: EMPTY_ANNOTATIONS }
		if (this.liveBodyAssets.scriptPath === path)
			this.liveBodyAssets = { scriptPath: undefined, assets: [] }
		if (this.liveContent.scriptPath === path)
			this.liveContent = { scriptPath: undefined, content: '' }
	}

	discardDraft = (path: string) => {
		if (!this.drafts.has(path)) return
		const next = new Map(this.drafts)
		next.delete(path)
		this.drafts = next
		this.forgetPath(path)
	}

	/** Commit body edits + inferred outputs back into the drafts Map on pane
	 * teardown (deferred a microtask so a same-batch discard doesn't resurrect the
	 * entry). Verbatim port of the route page's `handleDraftPersist`. */
	handleDraftPersist = (
		p: string,
		snapshot: { content: string; writes: { kind: AssetKind; path: string }[]; script?: Script }
	) => {
		queueMicrotask(() => {
			const d = this.drafts.get(p)
			if (!d) {
				if (!snapshot.script) return
				const next = new Map(this.drafts)
				next.set(p, {
					localId: this.newDraftLocalId(),
					script: snapshot.script,
					outputAssets: snapshot.writes.length > 0 ? snapshot.writes : undefined
				})
				this.drafts = next
				return
			}
			// `?? 0` is load-bearing: an undefined `outputAssets` (a no-output draft)
			// vs an empty inferred `writes` both mean "no writes". Without the
			// coalesce, `undefined === 0` is false, so this never short-circuits —
			// every persist re-writes the drafts Map with an equivalent object,
			// re-triggering the pane's emit → graph re-derive → persist, an infinite
			// microtask loop (hangs the tab without an effect-depth throw).
			const writesEqual =
				(d.outputAssets?.length ?? 0) === snapshot.writes.length &&
				(d.outputAssets ?? []).every(
					(a, i) => a.kind === snapshot.writes[i]?.kind && a.path === snapshot.writes[i]?.path
				)
			if (d.script.content === snapshot.content && writesEqual) return
			const next = new Map(this.drafts)
			next.set(p, {
				...d,
				script: { ...d.script, content: snapshot.content },
				outputAssets: snapshot.writes.length > 0 ? snapshot.writes : undefined
			})
			this.drafts = next
		})
	}

	/** The draft open in the pane, if any. */
	get activeDraft(): PipelineDraft | undefined {
		return this.activeDraftPath ? this.drafts.get(this.activeDraftPath) : undefined
	}

	/** Whichever script is open — the active draft, or a selected persisted script. */
	get openScriptPath(): string | undefined {
		if (this.activeDraftPath) return this.activeDraftPath
		if (this.selection?.kind === 'runnable' && this.selection.runnable_kind === 'script')
			return this.selection.path
		return undefined
	}

	get hasAiPending(): boolean {
		return [...this.drafts.values()].some((d) => d.aiPending)
	}
}
