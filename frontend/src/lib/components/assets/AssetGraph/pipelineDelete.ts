import { DraftService, OpenAPI, ScriptService, type RunnableItem } from '$lib/gen'
import { discardDraft } from '$lib/utils_draft_deploy'
import { PIPELINE_DRAFT_KIND, pipelineBundlePath, pipelineLocalMirrorKey } from '$lib/pipelinePaths'
import { deleteTriggerRow } from './pipelineTriggerDraftDeploy'
import type { AssetGraphResponse, NativeTriggerKind } from './types'

export type PipelineDeleteResult = { removed: number; failures: string[] }

/** The folder's pipeline-member scripts, deployed and draft-only alike: every one of
 * them makes the home list show the pipeline, so one left behind brings it back. */
async function listPipelineScripts(workspace: string, folder: string): Promise<RunnableItem[]> {
	const out: RunnableItem[] = []
	let cursor: string | undefined
	do {
		const res = await ScriptService.listRunnables({
			workspace,
			kinds: 'script',
			pathStart: `f/${folder}/`,
			includeDraftOnly: true,
			perPage: 100,
			cursor
		})
		out.push(...res.items.filter((it) => it.auto_kind === 'pipeline'))
		cursor = res.next_cursor ?? undefined
	} while (cursor)
	return out
}

/** Trigger rows starting one of `scripts`. The folder graph also carries the
 * triggers of the folder's ordinary scripts and flows, which are not the pipeline's. */
async function listNativeTriggers(workspace: string, folder: string, scripts: ReadonlySet<string>) {
	const res = await fetch(
		`${OpenAPI.BASE ?? ''}/w/${workspace}/assets/graph?${new URLSearchParams({ folder })}`,
		{ credentials: 'include' }
	)
	if (!res.ok) throw new Error(`GET /assets/graph → ${res.status}`)
	const graph = (await res.json()) as AssetGraphResponse
	return graph.triggers.flatMap((t) =>
		t.trigger_kind !== 'asset' &&
			t.runnable_kind === 'script' &&
			scripts.has(t.runnable_path) &&
			t.trigger_kind !== 'webhook' &&
			t.trigger_kind !== 'data_upload' &&
			t.path &&
			!t.unsaved
			? [{ kind: t.trigger_kind, path: t.path }]
			: []
	)
}

export type PipelineDeletePlan = {
	/** `path` is what the API addresses (a draft-only node's storage key);
	 * `displayPath` is the path the user typed for it. */
	scripts: { path: string; displayPath: string; draftOnly: boolean }[]
	triggers: { kind: NativeTriggerKind; path: string }[]
	/** Whether the caller has an unsaved pipeline draft for this folder. */
	hasDraft: boolean
}

/** Everything `deletePipeline` would remove, so it can be listed before it is. */
export async function planPipelineDelete(
	workspace: string,
	folder: string
): Promise<PipelineDeletePlan> {
	const scripts = await listPipelineScripts(workspace, folder)
	const deployed = new Set(scripts.filter((s) => !s.draft_only).map((s) => s.path))
	const [triggers, draft] = await Promise.all([
		listNativeTriggers(workspace, folder, deployed),
		DraftService.getOwnDraft({
			workspace,
			kind: PIPELINE_DRAFT_KIND,
			path: pipelineBundlePath(folder)
		}).catch(() => undefined)
	])
	return {
		scripts: scripts.map((s) => ({
			path: s.path,
			displayPath: (s.draft_only && s.draft_path) || s.path,
			draftOnly: !!s.draft_only
		})),
		triggers,
		hasDraft: !!draft
	}
}

/** Removes what `plan` lists: the trigger rows, the scripts (deleted when
 * `hardDelete`, which the backend allows admins only, archived otherwise; a
 * draft-only node is discarded) and the caller's pipeline draft. The data the
 * nodes wrote (DuckLake tables, S3 objects) is left alone. Triggers go first: a
 * script refused afterwards loses its trigger, but no trigger outlives the script
 * it starts. */
export async function deletePipeline(
	workspace: string,
	folder: string,
	plan: PipelineDeletePlan,
	hardDelete: boolean
): Promise<PipelineDeleteResult> {
	const failures: string[] = []
	let removed = 0
	const fail = (what: string, e: any) =>
		failures.push(`${what}: ${e?.body ?? e?.message ?? String(e)}`)

	for (const t of plan.triggers) {
		try {
			if (await deleteTriggerRow(t.kind, t.path, workspace)) removed++
		} catch (e) {
			fail(`${t.kind} trigger ${t.path}`, e)
		}
	}
	for (const s of plan.scripts) {
		try {
			if (s.draftOnly) {
				const r = await discardDraft('script', s.path, workspace, true, false, false)
				if (!r.success) throw new Error(r.error)
			} else if (hardDelete) {
				await ScriptService.deleteScriptByPath({ workspace, path: s.path })
			} else {
				await ScriptService.archiveScriptByPath({ workspace, path: s.path })
			}
			removed++
		} catch (e) {
			fail(s.displayPath, e)
		}
	}
	if (plan.hasDraft) {
		const bundle = await discardDraft(PIPELINE_DRAFT_KIND, pipelineBundlePath(folder), workspace, true)
		if (!bundle.success) failures.push(`pipeline draft: ${bundle.error}`)
	}
	try {
		localStorage.removeItem(pipelineLocalMirrorKey(folder))
	} catch {
		// Storage unavailable: nothing was mirrored either.
	}
	return { removed, failures }
}
