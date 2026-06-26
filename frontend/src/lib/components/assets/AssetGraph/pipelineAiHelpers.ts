import { JobService, ScriptService, type AssetKind, type Script, type ScriptLang } from '$lib/gen'
import { emptySchema, sendUserToast } from '$lib/utils'
import { inferAssets } from '$lib/infer'
import { extractWrites, type AssetWithAltAccessType } from '$lib/components/assets/lib'
import { assetUri, autoOutputAsset, type PipelineOutputKind } from './pipelineTemplates'
import type { AssetGraphResponse } from './types'
import type {
	PipelineAIChatHelpers,
	PipelineContext,
	PipelineNodeSummary
} from '$lib/components/copilot/chat/pipeline/core'

// ============================================================================
// Shared data-pipeline AI helper layer.
//
// Both the full-page editor (/pipeline/[folder]) and the in-session preview
// (PipelineEditorView) drive the AI chat's pipeline tools through this factory,
// so the "stage a draft → diff/approval" logic lives in exactly one place. Each
// caller injects accessors for its own draft Map and graph; this module owns the
// AI behaviour (propose/edit/remove/accept/reject/test) and the per-turn
// snapshot bookkeeping that powers Reject.
// ============================================================================

/**
 * A staged pipeline node. `localId` is a stable per-draft id preserved across
 * renames (the page uses it to dedupe concurrent deploys). `aiPending` marks a
 * draft staged by the AI chat and awaiting Accept/Reject — it renders with an
 * accent ring, distinct from a plain unsaved draft.
 */
export type PipelineDraft = {
	localId: string
	script: Script
	outputAsset?: { kind: AssetKind; path: string }
	outputAssets?: Array<{ kind: AssetKind; path: string }>
	aiPending?: boolean
}

export type PipelineAiHelperDeps = {
	getFolder: () => string
	getWorkspace: () => string | undefined
	/** The draft-overlaid graph (resolveGraph output) the context summary reads. */
	getResolvedGraph: () => AssetGraphResponse
	getDrafts: () => Map<string, PipelineDraft>
	setDrafts: (next: Map<string, PipelineDraft>) => void
	/** Stable id for a freshly-created draft (route page tracks deploys by it). */
	newDraftLocalId: () => string
	/** Focus/select the node after it is staged (pan + open in the pane). */
	onProposeNode?: (path: string) => void
	/** Throw (or switch to edit mode) when the surface can't accept AI edits. */
	ensureEditable?: () => void
	/** Surface the draft overlay if it is hidden (the page's "show drafts" view). */
	onShowDrafts?: () => void
	/** Forget per-path state when a created proposal is reverted away. */
	onForgetPath?: (path: string) => void
	/** Notify the caller a test run started so it can light up its run UI. */
	onRunStarted?: (jobId: string, path: string) => void
}

export function makePipelineScript(
	language: ScriptLang,
	scriptPath: string,
	content: string,
	createdAt: string
): Script {
	// Cast through unknown: a local draft only needs path/language/content/schema;
	// the many readonly deployment fields on Script don't matter until createScript.
	return {
		hash: '',
		path: scriptPath,
		summary: '',
		description: '',
		content,
		schema: emptySchema(),
		is_template: false,
		extra_perms: {},
		language,
		kind: 'script',
		created_by: '',
		created_at: createdAt,
		archived: false,
		deleted: false,
		starred: false
	} as unknown as Script
}

async function inferOutputAssets(
	language: ScriptLang,
	content: string
): Promise<Array<{ kind: AssetKind; path: string }>> {
	try {
		const inferred = await inferAssets(language, content)
		if (inferred?.status === 'error') return []
		return extractWrites((inferred?.assets ?? []) as AssetWithAltAccessType[])
	} catch {
		return []
	}
}

export type PipelineAiHelpersHandle = {
	helpers: PipelineAIChatHelpers
	hasPending: () => boolean
	acceptAll: () => void
	rejectAll: () => void
}

export function createPipelineAiHelpers(deps: PipelineAiHelperDeps): PipelineAiHelpersHandle {
	// Pre-proposal snapshot of every path the AI touched this review cycle, so
	// Reject restores exactly what was there before (a prior draft, or absence).
	// `undefined` = the path had no draft before the AI staged it → Reject removes
	// it entirely. Plain Map, not reactive: pure revert bookkeeping.
	const aiSnapshots = new Map<string, PipelineDraft | undefined>()

	function snapshotPath(path: string) {
		// Capture only the first time a path enters the cycle so a multi-edit turn
		// still reverts to the pre-AI baseline, not an interim one.
		if (!aiSnapshots.has(path)) aiSnapshots.set(path, deps.getDrafts().get(path))
	}

	function revertPath(path: string) {
		const snap = aiSnapshots.get(path)
		const next = new Map(deps.getDrafts())
		if (snap === undefined) {
			next.delete(path)
			deps.onForgetPath?.(path)
		} else {
			next.set(path, snap)
		}
		deps.setDrafts(next)
		aiSnapshots.delete(path)
	}

	function hasPending(): boolean {
		return [...deps.getDrafts().values()].some((d) => d.aiPending)
	}

	function acceptAll() {
		const next = new Map(deps.getDrafts())
		for (const [path, d] of next) if (d.aiPending) next.set(path, { ...d, aiPending: false })
		deps.setDrafts(next)
		aiSnapshots.clear()
	}

	function rejectAll() {
		// Iterate the snapshot set: it covers every path the AI created or edited
		// this cycle (a created path's snapshot is `undefined` → removed).
		for (const path of [...aiSnapshots.keys()]) revertPath(path)
		aiSnapshots.clear()
	}

	function buildContext(): PipelineContext {
		const graph = deps.getResolvedGraph()
		const drafts = deps.getDrafts()
		const nodes: PipelineNodeSummary[] = graph.runnables
			.filter((r) => r.usage_kind === 'script')
			.map((r) => {
				const draft = drafts.get(r.path)
				const writes = graph.edges
					.filter(
						(e) =>
							e.runnable_kind === 'script' &&
							e.runnable_path === r.path &&
							(e.access_type === 'w' || e.access_type === 'rw')
					)
					.map((e) => assetUri({ kind: e.asset_kind, path: e.asset_path }))
				const reads = graph.edges
					.filter(
						(e) =>
							e.runnable_kind === 'script' &&
							e.runnable_path === r.path &&
							(e.access_type === 'r' || e.access_type === 'rw')
					)
					.map((e) => assetUri({ kind: e.asset_kind, path: e.asset_path }))
				const triggers = graph.triggers
					.filter((t) => t.runnable_kind === 'script' && t.runnable_path === r.path)
					.map((t) =>
						t.trigger_kind === 'asset'
							? assetUri({ kind: t.asset_kind, path: t.asset_path })
							: t.trigger_kind
					)
				return {
					path: r.path,
					language: draft?.script.language,
					unsaved: r.unsaved ?? false,
					aiPending: draft?.aiPending ?? false,
					summary: draft?.script.summary || undefined,
					writes: [...new Set(writes)],
					reads: [...new Set(reads)],
					triggers: [...new Set(triggers)]
				}
			})
		return {
			folder: deps.getFolder(),
			mode: 'edit',
			nodes,
			assets: graph.assets.map((a) => assetUri({ kind: a.kind, path: a.path })),
			pendingProposals: nodes.filter((n) => n.aiPending).length
		}
	}

	const helpers: PipelineAIChatHelpers = {
		getPipelineContext: buildContext,
		getNodeBody: async (path) => {
			const draft = deps.getDrafts().get(path)
			if (draft) return { language: draft.script.language, content: draft.script.content }
			const workspace = deps.getWorkspace()
			if (!workspace) return undefined
			try {
				const deployed = await ScriptService.getScriptByPath({ workspace, path })
				return { language: deployed.language, content: deployed.content }
			} catch {
				return undefined
			}
		},
		proposeNode: async ({ path, language, content, outputKind }) => {
			deps.ensureEditable?.()
			const drafts = deps.getDrafts()
			if (drafts.has(path) && !drafts.get(path)?.aiPending) {
				throw new Error(
					`A draft already exists at '${path}'. Use edit_pipeline_node to change it instead.`
				)
			}
			snapshotPath(path)
			const outputAssets = await inferOutputAssets(language, content)
			const seededOutput =
				outputAssets[0] ??
				(outputKind
					? autoOutputAsset(outputKind as PipelineOutputKind, deps.getFolder(), language)
					: undefined)
			const prev = drafts.get(path)
			const next = new Map(drafts)
			next.set(path, {
				localId: prev?.localId ?? deps.newDraftLocalId(),
				script: makePipelineScript(language, path, content, isoNow()),
				outputAsset: seededOutput,
				outputAssets: outputAssets.length > 0 ? outputAssets : undefined,
				aiPending: true
			})
			deps.setDrafts(next)
			deps.onShowDrafts?.()
			deps.onProposeNode?.(path)
			return { path }
		},
		editNode: async (path, content) => {
			deps.ensureEditable?.()
			const drafts = deps.getDrafts()
			const existing = drafts.get(path)
			let language: ScriptLang
			if (existing) {
				language = existing.script.language
			} else {
				const workspace = deps.getWorkspace()
				if (!workspace) throw new Error('No workspace is selected.')
				const deployed = await ScriptService.getScriptByPath({ workspace, path })
				language = deployed.language
			}
			snapshotPath(path)
			const outputAssets = await inferOutputAssets(language, content)
			const next = new Map(drafts)
			next.set(path, {
				localId: existing?.localId ?? deps.newDraftLocalId(),
				script: makePipelineScript(language, path, content, isoNow()),
				outputAsset: outputAssets[0] ?? existing?.outputAsset,
				outputAssets: outputAssets.length > 0 ? outputAssets : existing?.outputAssets,
				aiPending: true
			})
			deps.setDrafts(next)
			deps.onShowDrafts?.()
			deps.onProposeNode?.(path)
		},
		removeProposedNode: async (path) => {
			if (!deps.getDrafts().get(path)?.aiPending) {
				throw new Error(`'${path}' is not an AI-pending node, so it cannot be removed here.`)
			}
			revertPath(path)
		},
		hasPendingProposals: hasPending,
		acceptAllProposals: acceptAll,
		rejectAllProposals: rejectAll,
		testNode: async (path, args) => {
			const workspace = deps.getWorkspace()
			if (!workspace) return undefined
			const draft = deps.getDrafts().get(path)
			try {
				let jobId: string
				if (draft) {
					// Un-deployed/edited body: preview-run the draft content so it can be
					// tested before deploying.
					jobId = await JobService.runScriptPreview({
						workspace,
						requestBody: {
							path,
							content: draft.script.content,
							language: draft.script.language,
							args: args ?? {}
						}
					})
				} else {
					jobId = await JobService.runScriptByPath({
						workspace,
						path,
						requestBody: { ...(args ?? {}) }
					})
				}
				deps.onRunStarted?.(jobId, path)
				return jobId
			} catch (e: any) {
				sendUserToast(`Run failed: ${e?.body ?? e?.message ?? e}`, true)
				return undefined
			}
		}
	}

	return { helpers, hasPending, acceptAll, rejectAll }
}

// `new Date()` is unavailable in some sandboxes but fine in the browser, where
// both editors run. Isolated here so the construction sites stay declarative.
function isoNow(): string {
	return new Date().toISOString()
}
