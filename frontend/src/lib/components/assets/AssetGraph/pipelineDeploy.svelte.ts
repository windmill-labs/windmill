import { DraftService, ScriptService, type Script } from '$lib/gen'
import { PIPELINE_DRAFT_KIND, pipelineBundlePath } from '$lib/pipelinePaths'
import type { PipelineTriggerDraft } from './types'
import type { Schema } from '$lib/common'
import { emptySchema } from '$lib/utils'
import { inferArgs, inferAssets } from '$lib/infer'
import type { AssetWithAltAccessType } from '$lib/components/assets/lib'
import type { PipelineDraft } from './pipelineAiHelpers'
import type { PipelineEditorState } from './pipelineEditorState.svelte'
import { deployTriggerDraft } from './pipelineTriggerDraftDeploy'

// Deploying a pipeline's drafts: the scripts first, then the trigger drafts whose
// script is now deployed. Shared by the pipeline page's "Save all" and the AI
// session's "Deploy pipeline", so both deploy exactly the same way.

/** Deploy one pipeline script draft. */
export async function deployPipelineScript(draft: PipelineDraft, workspace: string): Promise<void> {
	const script = structuredClone($state.snapshot(draft.script) as Script)
	script.schema = script.schema ?? emptySchema()
	try {
		const result = await inferArgs(script.language, script.content, script.schema as Schema)
		;(script as any).auto_kind = result?.auto_kind || undefined
		script.has_preprocessor = result?.has_preprocessor ?? false
	} catch {
		// Inference failures don't block deploys (the same fallback the per-pane
		// save uses): createScript is the real validation gate.
	}
	// Re-infer the lineage from the CURRENT body. `script.assets` is a snapshot
	// that isn't refreshed on edit, so a renamed/removed output would otherwise be
	// re-deployed as a phantom write edge.
	let assets: AssetWithAltAccessType[] = []
	try {
		const inferred = await inferAssets(script.language, script.content)
		if (inferred?.status !== 'error') assets = (inferred?.assets ?? []) as AssetWithAltAccessType[]
	} catch {
		// An unparsable body deploys with no lineage rather than the stale snapshot.
	}
	await ScriptService.createScript({
		workspace,
		requestBody: {
			...script,
			language: script.language,
			description: script.description ?? '',
			// A brand-new draft has an empty hash (the backend rejects an empty hex
			// string); a draft of a deployed script chains off its hash, since
			// createScript rejects a parentless deploy to an occupied path.
			parent_hash: script.hash ? String(script.hash) : undefined,
			is_template: false,
			tag: script.tag,
			kind: script.kind as Script['kind'] | undefined,
			lock: undefined,
			assets: assets as any
		}
	})
}

// Every field a deploy sends: a draft differing from the live script in any of
// them still has something to deploy.
const DEPLOYED_FIELDS = [
	'content',
	'language',
	'summary',
	'description',
	'tag',
	'kind',
	'labels'
] as const

/** Whether a draft equals the live script at its path, i.e. has nothing to deploy. */
export async function matchesDeployedScript(
	path: string,
	d: PipelineDraft,
	workspace: string
): Promise<boolean> {
	try {
		const live = await ScriptService.getScriptByPath({ workspace, path })
		const norm = (v: unknown) =>
			JSON.stringify(Array.isArray(v) && v.length === 0 ? null : (v ?? null))
		return DEPLOYED_FIELDS.every(
			(f) =>
				norm(f === 'description' ? (live[f] ?? '') : live[f]) ===
				norm(f === 'description' ? (d.script[f] ?? '') : d.script[f])
		)
	} catch {
		return false
	}
}

/**
 * A folder's draft bundle against what is deployed, one entry per node (and the
 * trigger drafts), for a diff viewer: the unit a session reviews and deploys.
 */
export async function loadPipelineDraftDiff(
	workspace: string,
	folder: string
): Promise<{ before: unknown; after: unknown }> {
	const row = await DraftService.getOwnDraft({
		workspace,
		kind: PIPELINE_DRAFT_KIND,
		path: pipelineBundlePath(folder)
	}).catch(() => undefined)
	const bundle = (row?.value ?? {}) as {
		drafts?: Array<[string, PipelineDraft]>
		triggerDrafts?: PipelineTriggerDraft[]
	}
	const drafts = Array.isArray(bundle.drafts) ? bundle.drafts : []
	const before: Record<string, string> = {}
	const after: Record<string, string> = {}
	await Promise.all(
		drafts.map(async ([path, d]) => {
			after[path] = d.script?.content ?? ''
			const live = await ScriptService.getScriptByPath({ workspace, path }).catch(() => undefined)
			if (live) before[path] = live.content
		})
	)
	const triggers = (bundle.triggerDrafts ?? []).map((t) => ({
		kind: t.kind,
		path: t.config.path,
		script_path: t.config.script_path
	}))
	return {
		before: { nodes: before },
		after: { nodes: after, ...(triggers.length > 0 ? { new_triggers: triggers } : {}) }
	}
}

export type PipelineDeployOutcome = {
	savedPaths: string[]
	/** Keys of the trigger drafts that deployed. */
	savedTriggers: string[]
	/** Failures by script or trigger path. */
	errors: Map<string, string>
}

/**
 * Deploy every draft of `editor` into `workspace`, dropping from it what deployed:
 * failed drafts stay so they can be fixed and retried. The open pane's unsaved
 * keystrokes are deployed with their draft.
 */
export async function deployPipelineDrafts(
	editor: PipelineEditorState,
	workspace: string
): Promise<PipelineDeployOutcome> {
	// An open deployed script's edits become its draft when its pane closes: close
	// it first, and keep the user on that script, now as its draft.
	if (editor.liveEditPath != undefined) {
		const path = await editor.closePane()
		if (path && editor.drafts.has(path)) editor.activeDraftPath = path
	}
	const live = editor.liveContent
	const liveContentPath =
		live.scriptPath != undefined && editor.drafts.has(live.scriptPath) ? live.scriptPath : undefined
	const entries = [...editor.drafts.entries()].map(([path, d]): [string, PipelineDraft] =>
		path !== liveContentPath || d.script.content === live.content
			? [path, d]
			: [path, { ...d, script: { ...d.script, content: live.content } }]
	)
	const errors = new Map<string, string>()
	const savedPaths: string[] = []
	// Parallel: every createScript is independent, and one bad body must not block
	// the others.
	const results = await Promise.allSettled(
		entries.map(([, d]) => deployPipelineScript(d, workspace))
	)
	for (let i = 0; i < results.length; i++) {
		const r = results[i]
		const [path, d] = entries[i]
		if (r.status === 'fulfilled') {
			savedPaths.push(path)
			continue
		}
		// A refused draft that equals the deployed script has nothing left to deploy.
		// A duplicate refusal alone does not prove it: it is checked against every
		// past version, not just the live one.
		if (await matchesDeployedScript(path, d, workspace)) {
			savedPaths.push(path)
			continue
		}
		const msg = String((r.reason as any)?.body ?? (r.reason as any)?.message ?? r.reason)
		errors.set(
			path,
			/same hash/i.test(msg)
				? 'This is the content of an earlier version, which cannot be deployed again as is. Change anything in it (a comment will do) to deploy it.'
				: msg
		)
	}

	// A trigger's `script_path` must already exist, so triggers whose script is
	// still an undeployed draft stay drafts.
	const ready = [...editor.triggerDrafts].filter(
		([, d]) => !editor.drafts.has(d.config.script_path) || savedPaths.includes(d.config.script_path)
	)
	const triggerResults = await Promise.all(ready.map(([, d]) => deployTriggerDraft(d, workspace)))
	const savedTriggers: string[] = []
	triggerResults.forEach((ok, i) => {
		const [key, d] = ready[i]
		if (ok) savedTriggers.push(key)
		else errors.set(d.config.path, `Could not create the ${d.kind} trigger, see the notification`)
	})
	for (const key of savedTriggers) editor.discardTriggerDraft(key)

	if (savedPaths.length > 0) {
		editor.drafts = new Map([...editor.drafts].filter(([k]) => !savedPaths.includes(k)))
		// Keep the pane on the script the user was editing, now deployed.
		if (editor.activeDraftPath && savedPaths.includes(editor.activeDraftPath)) {
			editor.selection = { kind: 'runnable', runnable_kind: 'script', path: editor.activeDraftPath }
			editor.activeDraftPath = undefined
		}
	}
	return { savedPaths, savedTriggers, errors }
}
