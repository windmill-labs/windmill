/**
 * Draft deploy/discard orchestration for the compare page's "draft" mode.
 *
 * Drafts only exist for scripts, flows and apps (the `draft_type` enum). A draft
 * is the editor's serialized state stored in the `draft` table; deploying it is
 * the same create/update call the editor makes on "Deploy", which auto-deletes
 * the matching draft server-side (unless `skip_draft_deletion`) — so we never
 * call `deleteDraft` after a successful deploy. The lock/dependency job runs
 * async, exactly as in the editor.
 *
 * Discarding branches on `draft_only`: a `draft_only` item exists only as a
 * draft, so discarding deletes the whole item (mirrors `common/table/*Row.svelte`);
 * a draft on an already-deployed item just deletes the draft row.
 */
import { ScriptService, FlowService, AppService, DraftService } from '$lib/gen'
import type { DeployResult } from '$lib/utils_workspace_deploy'
import { deployRawAppDraft } from '$lib/rawAppDeploy'
import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'

export type DraftKind = 'script' | 'flow' | 'app'

export interface DraftDiffValues {
	deployed: unknown
	draft: unknown
}

// Empty-but-valid "deployed" shapes for draft_only items (which have never been
// deployed). Using a fully-empty `{}` breaks the flow graph diff (it needs
// `value.modules`) and leaves the drawer spinning — so each kind gets a minimal
// valid shape, making the whole draft show as "all new".
const EMPTY_DEPLOYED: Record<DraftKind, (draft: any) => unknown> = {
	script: (draft) => ({ content: '', language: draft?.language, schema: {} }),
	flow: () => ({ summary: '', value: { modules: [] }, schema: {} }),
	app: () => ({ summary: '', value: {}, policy: {} })
}

/**
 * Fetch the deployed value and the draft value for an item, for the DiffDrawer
 * (`mode: 'simple'`, original = deployed, current = draft). For a `draft_only`
 * item there is no real deployed value, so the deployed side is a minimal
 * empty-but-valid shape and the draft shows as entirely new. DiffDrawer cleans
 * both sides via `cleanValueProperties`, so raw objects are fine here.
 */
export async function getDraftDiffValues(
	kind: DraftKind,
	path: string,
	workspace: string,
	draftOnly = false
): Promise<DraftDiffValues> {
	// A `draft_only` item can keep its content in the row itself with no separate
	// draft-table row (e.g. a flow created via createFlow(draft_only: true), like
	// `u/admin/new`). There `draft` is null, so the draft side must fall back to
	// the row's own value — otherwise the diff "after" is empty and nothing shows.
	if (kind === 'script') {
		const r = (await ScriptService.getScriptByPathWithDraft({ workspace, path })) as any
		const { draft, draft_created_at: _c, hash: _h, ...deployed } = r
		const draftValue = draft ?? deployed
		return { deployed: draftOnly ? EMPTY_DEPLOYED.script(draftValue) : deployed, draft: draftValue }
	} else if (kind === 'flow') {
		const r = (await FlowService.getFlowByPathWithDraft({ workspace, path })) as any
		const { draft, draft_created_at: _c, ...deployed } = r
		// Strip the staleness-tracking sidecar so it doesn't show as a spurious diff field.
		const { draft_base_version: _bv, ...draftValue } = (draft ?? deployed) as any
		return { deployed: draftOnly ? EMPTY_DEPLOYED.flow(draftValue) : deployed, draft: draftValue }
	} else {
		const r = (await AppService.getAppByPathWithDraft({ workspace, path })) as any
		const deployed = {
			summary: r.summary,
			value: r.value,
			policy: r.policy,
			path: r.path,
			custom_path: r.custom_path
		}
		const { draft_base_version: _bv, ...draftValue } = (r.draft ?? deployed) as any
		return { deployed: draftOnly ? EMPTY_DEPLOYED.app(draftValue) : deployed, draft: draftValue }
	}
}

/**
 * Whether a draft's base deployed version has been superseded by a newer deploy
 * since the draft was saved — i.e. deploying the draft would override a newer
 * version. Mirrors the editor's deploy-time "not on latest" guard, but computed
 * from the DB draft (server-readable) so it works for drafts authored elsewhere.
 *
 * The base version is read from the draft `value`: scripts carry `parent_hash`
 * (always); flows/apps carry a `draft_base_version` sidecar (only on drafts
 * saved after this was introduced — older drafts have no base and are treated
 * as not-stale to avoid false positives, consistent with `checkStaleness`).
 * `draft_only` items have never been deployed and are never stale.
 */
export type DraftStaleness =
	| { stale: false }
	| { stale: true; baseRev: string | number; deployedRev: string | number }

const NOT_STALE: DraftStaleness = { stale: false }

export async function getDraftStaleness(
	kind: DraftKind,
	path: string,
	workspace: string,
	draftOnly = false
): Promise<DraftStaleness> {
	if (draftOnly) return NOT_STALE
	try {
		if (kind === 'script') {
			const r = (await ScriptService.getScriptByPathWithDraft({ workspace, path })) as any
			const base = r.draft?.parent_hash
			if (base == null) return NOT_STALE
			return base !== r.hash ? { stale: true, baseRev: base, deployedRev: r.hash } : NOT_STALE
		} else if (kind === 'flow') {
			const r = (await FlowService.getFlowByPathWithDraft({ workspace, path })) as any
			const base = r.draft?.draft_base_version
			if (base == null) return NOT_STALE
			const current = (await FlowService.getFlowLatestVersion({ workspace, path }))?.id
			if (current == null) return NOT_STALE
			return base !== current ? { stale: true, baseRev: base, deployedRev: current } : NOT_STALE
		} else {
			const r = (await AppService.getAppByPathWithDraft({ workspace, path })) as any
			const base = r.draft?.draft_base_version
			if (base == null) return NOT_STALE
			const current = r.versions?.[r.versions.length - 1]
			if (current == null) return NOT_STALE
			return base !== current ? { stale: true, baseRev: base, deployedRev: current } : NOT_STALE
		}
	} catch (e) {
		// Detection is best-effort; never block deploy on a probe failure.
		console.error('Failed to check draft staleness', e)
		return NOT_STALE
	}
}

/**
 * Promote a draft to deployed by replaying the editor's create/update call with
 * the stored draft value. The matching draft row is deleted server-side by the
 * create/update handler. Returns the same `{ success, error? }` shape as the
 * fork-merge `deployItem`, so callers can reuse the `deploymentStatus` pattern.
 */
export async function deployDraft(
	kind: DraftKind,
	path: string,
	workspace: string,
	draftOnly = false,
	rawApp = false
): Promise<DeployResult> {
	try {
		if (kind === 'app' && rawApp) {
			// Raw apps bundle their source files to js/css and deploy via the
			// raw-app endpoints — same as the global AI chat's deploy.
			await deployRawAppDraft(workspace, path)
		} else if (kind === 'script') {
			const r = (await ScriptService.getScriptByPathWithDraft({ workspace, path })) as any
			const d = r.draft ?? r
			// Drop editor-only / server-managed keys; deploy as a real (non-draft) version.
			const { draft_triggers: _t, draft_only: _o, ...rest } = d
			await ScriptService.createScript({
				workspace,
				requestBody: { ...rest, path, parent_hash: r.hash }
			})
		} else if (kind === 'flow') {
			const r = (await FlowService.getFlowByPathWithDraft({ workspace, path })) as any
			const d = r.draft ?? r
			const requestBody = {
				path,
				summary: d.summary ?? '',
				description: d.description ?? '',
				value: d.value,
				schema: d.schema,
				tag: d.tag,
				dedicated_worker: d.dedicated_worker,
				ws_error_handler_muted: d.ws_error_handler_muted,
				visible_to_runner_only: d.visible_to_runner_only,
				on_behalf_of_email: d.on_behalf_of_email,
				labels: d.labels
			}
			// A draft (draft_only or on a deployed flow) always has a flow row, so
			// updateFlow is correct in both cases — it promotes a draft_only flow to
			// a real deployed version (clearing the flag). createFlow would 400
			// "Flow already exists".
			await FlowService.updateFlow({ workspace, path, requestBody })
		} else {
			const r = (await AppService.getAppByPathWithDraft({ workspace, path })) as any
			const d = r.draft ?? {
				value: r.value,
				summary: r.summary,
				policy: r.policy,
				path: r.path,
				custom_path: r.custom_path
			}
			const requestBody = {
				value: d.value,
				summary: d.summary ?? '',
				policy: d.policy,
				path: d.path ?? path,
				custom_path: d.custom_path
			}
			// Same as flows: a draft always has an app row, so updateApp promotes a
			// draft_only app (clearing the flag); createApp would 400 "already exists".
			await AppService.updateApp({ workspace, path, requestBody })
		}
		// Mutated the workspace's Server Drafts — refresh every mounted reader.
		invalidateWorkspaceDrafts(workspace)
		return { success: true }
	} catch (e: any) {
		return { success: false, error: e?.body ?? e?.message ?? String(e) }
	}
}

/**
 * Discard a draft. For `draft_only` items the item exists only as a draft, so
 * delete the whole item; otherwise delete just the draft row.
 */
export async function discardDraft(
	kind: DraftKind,
	path: string,
	workspace: string,
	draftOnly = false
): Promise<DeployResult> {
	try {
		if (draftOnly) {
			if (kind === 'script') {
				await ScriptService.deleteScriptByPath({ workspace, path })
			} else if (kind === 'flow') {
				await FlowService.deleteFlowByPath({ workspace, path })
			} else {
				await AppService.deleteApp({ workspace, path })
			}
		} else {
			await DraftService.deleteDraft({ workspace, path, kind })
		}
		invalidateWorkspaceDrafts(workspace)
		return { success: true }
	} catch (e: any) {
		return { success: false, error: e?.body ?? e?.message ?? String(e) }
	}
}
