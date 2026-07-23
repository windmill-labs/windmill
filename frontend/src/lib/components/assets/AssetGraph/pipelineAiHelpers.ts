import { JobService, ScriptService, type AssetKind, type Script, type ScriptLang } from '$lib/gen'
import { emptySchema, sendUserToast } from '$lib/utils'
import { inferAssets } from '$lib/infer'
import {
	extractReads,
	extractWrites,
	type AssetWithAltAccessType
} from '$lib/components/assets/lib'
import { assetUri, autoOutputAsset, type PipelineOutputKind } from './pipelineTemplates'
import { parsePipelineAnnotations } from './parsePipelineAnnotations'
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
// so the build/edit logic lives in exactly one place. Each caller injects
// accessors for its own draft Map and graph; this module owns the AI behaviour
// (build/edit/discard/test). AI edits apply directly as unsaved drafts — there
// is no separate approve/reject step.
// ============================================================================

/**
 * An unsaved pipeline node draft. `localId` is a stable per-draft id preserved
 * across renames (the page uses it to dedupe concurrent deploys). AI-built nodes
 * and manually-created drafts are the same thing — an unsaved node on the canvas.
 */
export type PipelineDraft = {
	localId: string
	script: Script
	outputAssets?: Array<{ kind: AssetKind; path: string }>
	/** Body-inferred reads captured with the draft, so an inactive draft keeps
	 * its input lineage on the canvas. `undefined` = not captured yet (legacy
	 * bundle / just-seeded draft) — consumers fall back to the session cache;
	 * an empty array is an authoritative "reads nothing". */
	inputAssets?: Array<{ kind: AssetKind; path: string }>
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
	/** Forget per-path state when a draft is discarded. */
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

async function inferDraftAssets(
	language: ScriptLang,
	content: string
): Promise<{
	writes: Array<{ kind: AssetKind; path: string }>
	reads: Array<{ kind: AssetKind; path: string }>
}> {
	try {
		const inferred = await inferAssets(language, content)
		if (inferred?.status === 'error') return { writes: [], reads: [] }
		const assets = (inferred?.assets ?? []) as AssetWithAltAccessType[]
		return { writes: extractWrites(assets), reads: extractReads(assets) }
	} catch {
		return { writes: [], reads: [] }
	}
}

export function createPipelineAiHelpers(deps: PipelineAiHelperDeps): PipelineAIChatHelpers {
	// A staged draft is always persisted into the OPEN folder's data_pipeline
	// bundle, so a path outside the folder would silently land an unrelated script
	// there. Both build and edit must stay scoped to the folder.
	function assertInFolder(path: string) {
		const folder = deps.getFolder()
		if (folder && !path.startsWith(`f/${folder}/`)) {
			throw new Error(
				`Pipeline nodes must be in the open folder — use a path under 'f/${folder}/' (got '${path}').`
			)
		}
	}

	// A pipeline node IS its `// pipeline` annotation (it's what makes the deployed
	// script a pipeline member). Reject content that lacks it so a staged draft
	// isn't a non-member script the model can't see is broken until deploy.
	function assertPipelineAnnotation(content: string) {
		if (!parsePipelineAnnotations(content).inPipeline) {
			throw new Error(
				`Pipeline node content must declare the pipeline annotation on its own comment line ` +
					`(\`// pipeline\`, or \`-- pipeline\` for SQL / \`# pipeline\` for Python).`
			)
		}
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
			assets: graph.assets.map((a) => assetUri({ kind: a.kind, path: a.path }))
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
			// build_pipeline_node creates a NEW node in the OPEN folder. Reject a path
			// outside the folder (it would silently stage into this folder's bundle)
			// and a path that collides with an existing node (the model should use
			// edit_pipeline_node instead of shadowing a deployed node as a draft).
			assertInFolder(path)
			assertPipelineAnnotation(content)
			const drafts = deps.getDrafts()
			if (drafts.has(path)) {
				throw new Error(
					`A draft already exists at '${path}'. Use edit_pipeline_node to change it instead.`
				)
			}
			if (deps.getResolvedGraph().runnables.some((r) => r.path === path)) {
				throw new Error(
					`A pipeline node already exists at '${path}'. Use edit_pipeline_node to change it instead.`
				)
			}
			// Authoritative new-node check: the resolved graph may not have hydrated yet
			// (the session preview can race open_preview), and it only lists pipeline
			// runnables — so probe the backend. ANY deployed script at this path means
			// "build new" would shadow it on deploy; the model should edit instead.
			const workspace = deps.getWorkspace()
			if (workspace) {
				let deployedExists = false
				try {
					await ScriptService.getScriptByPath({ workspace, path })
					deployedExists = true
				} catch {
					// 404 → no deployed script at this path, safe to create a new node.
				}
				if (deployedExists) {
					throw new Error(
						`A script already exists at '${path}'. Use edit_pipeline_node to change it instead.`
					)
				}
			}
			const inferred = await inferDraftAssets(language, content)
			// Fall back to a seeded output (from the declared output_kind) when the
			// body doesn't yet write anything inferable.
			const seeded =
				inferred.writes[0] ??
				(outputKind
					? autoOutputAsset(outputKind as PipelineOutputKind, deps.getFolder(), language)
					: undefined)
			// Effective outputs = what actually becomes an output edge on the canvas:
			// body/annotation-inferred writes, or the output_kind seed as a fallback.
			const outputAssets =
				inferred.writes.length > 0 ? inferred.writes : seeded ? [seeded] : undefined
			const next = new Map(drafts)
			next.set(path, {
				localId: deps.newDraftLocalId(),
				script: makePipelineScript(language, path, content, new Date().toISOString()),
				outputAssets,
				inputAssets: inferred.reads
			})
			deps.setDrafts(next)
			deps.onShowDrafts?.()
			deps.onProposeNode?.(path)
			// Report ONLY body/annotation-inferred lineage — the deployable truth. The
			// output_kind `seeded` value above is a random canvas placeholder that live
			// inference overwrites and deploy re-derives from the body, so it must NOT be
			// reported as a detected edge: an empty list correctly means the model's
			// write (e.g. a variable/dynamic path) produced no real edge.
			return {
				path,
				detectedReads: inferred.reads.map(assetUri),
				detectedWrites: inferred.writes.map(assetUri)
			}
		},
		editNode: async (path, content) => {
			deps.ensureEditable?.()
			assertInFolder(path)
			assertPipelineAnnotation(content)
			const drafts = deps.getDrafts()
			const existing = drafts.get(path)
			// Base the edit on the existing draft's / deployed script object and replace
			// ONLY the content — preserving hash, summary, description, tag, schema, and
			// settings. Rebuilding a fresh script would wipe that metadata: deploying
			// from the pane (auto_parent) would update the script while clearing it, and
			// the route "Save all" path (no parent_hash) could hit the path-conflict
			// branch on the occupied path.
			let baseScript: Script
			if (existing) {
				baseScript = existing.script
			} else {
				const workspace = deps.getWorkspace()
				if (!workspace) throw new Error('No workspace is selected.')
				baseScript = await ScriptService.getScriptByPath({ workspace, path })
			}
			const inferred = await inferDraftAssets(baseScript.language, content)
			const outputAssets = inferred.writes.length > 0 ? inferred.writes : existing?.outputAssets
			const next = new Map(drafts)
			next.set(path, {
				localId: existing?.localId ?? deps.newDraftLocalId(),
				script: { ...baseScript, content },
				outputAssets,
				inputAssets: inferred.reads
			})
			deps.setDrafts(next)
			deps.onShowDrafts?.()
			deps.onProposeNode?.(path)
			// Report only body/annotation-inferred lineage (the deployable truth), not a
			// carried-over seed placeholder — see the note in proposeNode.
			return {
				detectedReads: inferred.reads.map(assetUri),
				detectedWrites: inferred.writes.map(assetUri)
			}
		},
		removeProposedNode: async (path) => {
			if (!deps.getDrafts().has(path)) {
				throw new Error(`No unsaved draft at '${path}' to discard.`)
			}
			const next = new Map(deps.getDrafts())
			next.delete(path)
			deps.setDrafts(next)
			deps.onForgetPath?.(path)
		},
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
					// test_pipeline_node previews ONE node — never fan out to downstream
					// deployed subscribers via the backend asset dispatcher (which would
					// run side-effecting deployed scripts the user didn't ask for).
					jobId = await JobService.runScriptByPath({
						workspace,
						path,
						requestBody: { ...(args ?? {}), _wmill_skip_asset_dispatch: true }
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

	return helpers
}
