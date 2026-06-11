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
import { get, writable } from 'svelte/store'
import {
	ScriptService,
	FlowService,
	AppService,
	VariableService,
	ResourceService,
	ScheduleService,
	HttpTriggerService,
	WebsocketTriggerService,
	PostgresTriggerService,
	KafkaTriggerService,
	NatsTriggerService,
	MqttTriggerService,
	SqsTriggerService,
	GcpTriggerService,
	AzureTriggerService,
	EmailTriggerService,
	type UserDraftItemKind
} from '$lib/gen'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import type { DeployResult } from '$lib/utils_workspace_deploy'
import { deployRawAppDraft } from '$lib/rawAppDeploy'
import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
import { setLocalDraftHint } from '$lib/localDraftHints.svelte'
import { userStore } from '$lib/stores'
import { deployTriggers, type Trigger } from '$lib/components/triggers/utils'
import { saveScheduleFromCfg } from '$lib/components/flows/scheduleUtils'
import { saveHttpRouteFromCfg } from '$lib/components/triggers/http/utils'
import { saveWebsocketTriggerFromCfg } from '$lib/components/triggers/websocket/utils'
import { savePostgresTriggerFromCfg } from '$lib/components/triggers/postgres/utils'
import { saveKafkaTriggerFromCfg } from '$lib/components/triggers/kafka/utils'
import { saveNatsTriggerFromCfg } from '$lib/components/triggers/nats/utils'
import { saveMqttTriggerFromCfg } from '$lib/components/triggers/mqtt/utils'
import { saveSqsTriggerFromCfg } from '$lib/components/triggers/sqs/utils'
import { saveGcpTriggerFromCfg } from '$lib/components/triggers/gcp/utils'
import { saveAzureTriggerFromCfg } from '$lib/components/triggers/azure/utils'
import { saveEmailTriggerFromCfg } from '$lib/components/triggers/email/utils'

export type DraftKind = UserDraftItemKind

/** Kind → "get by path with draft overlay" call, for the kinds whose draft is
 * the editor's flat config object (everything except script/flow/app, which
 * have bespoke handling below). Feature-gated trigger services 404 when the
 * backend wasn't compiled with the kind — the caller surfaces the error. */
const OVERLAY_GETTERS: Partial<
	Record<DraftKind, (workspace: string, path: string) => Promise<any>>
> = {
	variable: (workspace, path) =>
		VariableService.getVariable({ workspace, path, decryptSecret: false, getDraft: true }),
	resource: (workspace, path) => ResourceService.getResource({ workspace, path, getDraft: true }),
	trigger_schedule: (workspace, path) =>
		ScheduleService.getSchedule({ workspace, path, getDraft: true }),
	trigger_http: (workspace, path) =>
		HttpTriggerService.getHttpTrigger({ workspace, path, getDraft: true }),
	trigger_websocket: (workspace, path) =>
		WebsocketTriggerService.getWebsocketTrigger({ workspace, path, getDraft: true }),
	trigger_postgres: (workspace, path) =>
		PostgresTriggerService.getPostgresTrigger({ workspace, path, getDraft: true }),
	trigger_kafka: (workspace, path) =>
		KafkaTriggerService.getKafkaTrigger({ workspace, path, getDraft: true }),
	trigger_nats: (workspace, path) =>
		NatsTriggerService.getNatsTrigger({ workspace, path, getDraft: true }),
	trigger_mqtt: (workspace, path) =>
		MqttTriggerService.getMqttTrigger({ workspace, path, getDraft: true }),
	trigger_sqs: (workspace, path) =>
		SqsTriggerService.getSqsTrigger({ workspace, path, getDraft: true }),
	trigger_gcp: (workspace, path) =>
		GcpTriggerService.getGcpTrigger({ workspace, path, getDraft: true }),
	trigger_azure: (workspace, path) =>
		AzureTriggerService.getAzureTrigger({ workspace, path, getDraft: true }),
	trigger_email: (workspace, path) =>
		EmailTriggerService.getEmailTrigger({ workspace, path, getDraft: true })
}

/** Strip the per-user draft-overlay metadata, returning `{deployed, draft}`. */
function splitOverlay(r: any): { deployed: any; draft: any } {
	const {
		draft,
		is_draft: _i,
		draft_saved_at: _c,
		no_deployed: _n,
		other_drafts_users: _o,
		...deployed
	} = r
	return { deployed, draft: draft ?? deployed }
}

export interface DraftDiffValues {
	deployed: unknown
	draft: unknown
}

// Empty-but-valid "deployed" shapes for draft_only items (which have never been
// deployed). Using a fully-empty `{}` breaks the flow graph diff (it needs
// `value.modules`) and leaves the drawer spinning — so each kind gets a minimal
// valid shape, making the whole draft show as "all new". Kinds not listed fall
// back to `{}` (their diffs are plain object diffs with no shape requirements).
const EMPTY_DEPLOYED: Partial<Record<DraftKind, (draft: any) => unknown>> = {
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
	// Overlay metadata (is_draft / draft_saved_at / no_deployed /
	// other_drafts_users) is stripped from the "deployed" side so the
	// DiffDrawer's `cleanValueProperties` doesn't render the per-user
	// markers as noise. Same shape as the script/flow editor's loader.
	if (kind === 'script') {
		const r = (await ScriptService.getScriptByPath({ workspace, path, getDraft: true })) as any
		const {
			draft,
			is_draft: _i,
			draft_saved_at: _c,
			no_deployed: _n,
			other_drafts_users: _o,
			hash: _h,
			...deployed
		} = r
		const draftValue = draft ?? deployed
		return {
			deployed: draftOnly ? EMPTY_DEPLOYED.script!(draftValue) : deployed,
			draft: draftValue
		}
	} else if (kind === 'flow') {
		const r = (await FlowService.getFlowByPath({ workspace, path, getDraft: true })) as any
		const {
			draft,
			is_draft: _i,
			draft_saved_at: _c,
			no_deployed: _n,
			other_drafts_users: _o,
			...deployed
		} = r
		const draftValue = draft ?? deployed
		return { deployed: draftOnly ? EMPTY_DEPLOYED.flow!(draftValue) : deployed, draft: draftValue }
	} else if (kind === 'app' || kind === 'raw_app') {
		// A never-deployed raw app has no `app` row; the backend resolves the
		// draft kind from `rawApp`, so it MUST be set or the lookup 404s.
		const r = (await AppService.getAppByPath({
			workspace,
			path,
			getDraft: true,
			rawApp: kind === 'raw_app'
		})) as any
		const deployed = {
			summary: r.summary,
			value: r.value,
			policy: r.policy,
			path: r.path,
			custom_path: r.custom_path
		}
		const draftValue = r.draft ?? deployed
		return { deployed: draftOnly ? EMPTY_DEPLOYED.app!(draftValue) : deployed, draft: draftValue }
	} else {
		// Variables / resources / schedules / triggers: the draft is the
		// editor's flat config object and the deployed shape diffs cleanly
		// as a plain object — one overlay GET covers both sides.
		const getter = OVERLAY_GETTERS[kind]
		if (!getter) {
			throw new Error(`Draft diff not supported for kind ${kind}`)
		}
		const { deployed, draft } = splitOverlay(await getter(workspace, path))
		return { deployed: draftOnly ? {} : deployed, draft }
	}
}

/**
 * Deploy a script/flow draft's trigger changes the same way the editors do.
 * Scripts and flows can carry `draft_triggers`; the create/update call below
 * deletes the draft row, so without this the saved trigger edits would be
 * silently lost. Uses the shared `deployTriggers` (a throwaway `usedTriggerKinds`
 * store is fine — it only tracks kinds for the editor UI). `isNew` forces each
 * trigger's `script_path` to the deployed path (matches the editors' new path).
 */
async function deployDraftTriggers(
	draftTriggers: Trigger[] | undefined,
	workspace: string,
	path: string,
	isNew: boolean
): Promise<void> {
	const triggers = (draftTriggers ?? []).filter((t) => t?.draftConfig)
	if (triggers.length === 0) return
	const isAdmin = !!(get(userStore)?.is_admin || get(userStore)?.is_super_admin)
	await deployTriggers(triggers, workspace, isAdmin, writable<string[]>([]), path, isNew)
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
		if (kind === 'raw_app' || (kind === 'app' && rawApp)) {
			// Raw apps bundle their source files to js/css and deploy via the
			// raw-app endpoints — same as the global AI chat's deploy. The
			// Review & Deploy page passes `kind === 'raw_app'` (raw apps are
			// their own DRAFT_KIND); the editor path passes `kind === 'app'`
			// + `rawApp`. Either MUST route here — falling into the visual-app
			// branch below would do a partial `updateApp` with no `value`
			// (a RawAppDraft has no `value` field), silently discarding the
			// draft's files without ever bundling/deploying them.
			await deployRawAppDraft(workspace, path)
		} else if (kind === 'script') {
			const r = (await ScriptService.getScriptByPath({ workspace, path, getDraft: true })) as any
			const d = r.draft ?? r
			// Drop editor-only / server-managed keys; deploy as a real (non-draft) version.
			const { draft_triggers: draftTriggers, draft_only: _o, ...rest } = d
			const scriptPath = d.path ?? path
			// Deploy at the draft's path so a rename in the draft is honored (same as
			// the editor: createScript at the new path with parent_hash links lineage).
			await ScriptService.createScript({
				workspace,
				requestBody: { ...rest, path: scriptPath, parent_hash: r.hash }
			})
			// Then deploy any draft trigger edits, so they aren't dropped with the draft.
			await deployDraftTriggers(draftTriggers, workspace, scriptPath, true)
		} else if (kind === 'flow') {
			const r = (await FlowService.getFlowByPath({ workspace, path, getDraft: true })) as any
			const d = r.draft ?? r
			const requestBody = {
				// Honor a renamed draft path; the URL `path` stays the existing item key.
				path: d.path ?? path,
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
			// Draft-only flows have NO flow row (they live solely in the
			// draft table), so they deploy via createFlow; a draft on a
			// deployed flow updates it.
			if (draftOnly) {
				await FlowService.createFlow({ workspace, requestBody })
			} else {
				await FlowService.updateFlow({ workspace, path, requestBody })
			}
			// Then deploy any draft trigger edits, so they aren't dropped with the draft.
			await deployDraftTriggers(d.draft_triggers, workspace, d.path ?? path, draftOnly)
		} else if (kind === 'app') {
			// `raw_app` is handled above; only visual apps reach here.
			const r = (await AppService.getAppByPath({ workspace, path, getDraft: true })) as any
			const d = r.draft ?? {
				value: r.value,
				summary: r.summary,
				policy: r.policy,
				path: r.path,
				custom_path: r.custom_path
			}
			// custom_path requires admin on app update. Non-admins send undefined so
			// the backend preserves the existing route (no RequireAdmin 403). For
			// admins, fall back to the *deployed* route (`r.custom_path`) when the
			// draft doesn't carry one — the visual-app draft value usually omits
			// custom_path, and sending `''` would clear the existing route. An
			// explicit '' in the draft still clears (`'' ?? x === ''`).
			const isAdmin = !!(get(userStore)?.is_admin || get(userStore)?.is_super_admin)
			const requestBody = {
				value: d.value,
				summary: d.summary ?? '',
				policy: d.policy ?? { execution_mode: 'publisher' },
				path: d.path ?? path,
				custom_path: isAdmin ? (d.custom_path ?? r.custom_path) : undefined
			}
			// Same as flows: draft-only apps have no app row → create;
			// drafts on a deployed app update it.
			if (draftOnly) {
				await AppService.createApp({ workspace, requestBody })
			} else {
				await AppService.updateApp({ workspace, path, requestBody })
			}
		} else if (kind === 'variable') {
			const { deployed, draft: d } = splitOverlay(await OVERLAY_GETTERS.variable!(workspace, path))
			// VariableEditor's `VariableState` draft shape:
			// { path, variable: { value, is_secret, description }, labels?, wsSpecific }
			if (draftOnly) {
				await VariableService.createVariable({
					workspace,
					requestBody: {
						path: d.path ?? path,
						value: d.variable?.value ?? '',
						is_secret: !!d.variable?.is_secret,
						description: d.variable?.description ?? '',
						labels: d.labels,
						ws_specific: d.wsSpecific
					}
				})
			} else {
				await VariableService.updateVariable({
					workspace,
					path,
					requestBody: {
						path: d.path !== path ? d.path : undefined,
						// '' = untouched secret value; sending it would blank the secret.
						value: d.variable?.value === '' ? undefined : d.variable?.value,
						is_secret: d.variable?.is_secret,
						description: d.variable?.description,
						labels: d.labels,
						ws_specific: d.wsSpecific
					}
				})
			}
			void deployed
		} else if (kind === 'resource') {
			const { deployed, draft: d } = splitOverlay(await OVERLAY_GETTERS.resource!(workspace, path))
			// ResourceEditor's `ResourceState` draft shape:
			// { path, description, args, resource_type?, labels?, wsSpecific }
			if (draftOnly) {
				await ResourceService.createResource({
					workspace,
					requestBody: {
						path: d.path ?? path,
						value: d.args ?? {},
						description: d.description ?? '',
						resource_type: d.resource_type ?? deployed.resource_type,
						labels: d.labels,
						ws_specific: d.wsSpecific
					}
				})
			} else {
				await ResourceService.updateResource({
					workspace,
					path,
					requestBody: {
						path: d.path ?? path,
						value: d.args ?? {},
						description: d.description ?? '',
						labels: d.labels,
						ws_specific: d.wsSpecific
					}
				})
			}
		} else if (kind === 'trigger_schedule') {
			const { draft: d } = splitOverlay(await OVERLAY_GETTERS.trigger_schedule!(workspace, path))
			// The schedule editor's draft IS the cfg shape `saveScheduleFromCfg`
			// consumes — same save the editor's Deploy button runs.
			const ok = await saveScheduleFromCfg({ ...d, path: d.path ?? path }, !draftOnly, workspace)
			if (!ok) {
				return { success: false, error: 'Schedule save failed' }
			}
		} else if (kind in TRIGGER_SAVERS) {
			const getter = OVERLAY_GETTERS[kind]!
			const { draft: d } = splitOverlay(await getter(workspace, path))
			const isAdmin = !!(get(userStore)?.is_admin || get(userStore)?.is_super_admin)
			const ok = await TRIGGER_SAVERS[kind]!(
				path,
				{ ...d, path: d.path ?? path },
				!draftOnly,
				workspace,
				isAdmin
			)
			if (!ok) {
				return { success: false, error: 'Trigger save failed' }
			}
		} else {
			return { success: false, error: `Deploy not supported for draft kind ${kind}` }
		}
		// Mutated the workspace's Server Drafts — refresh every mounted reader.
		invalidateWorkspaceDrafts(workspace)
		// Deploy deletes the draft server-side (for script/flow/app) without
		// going through UserDraftDbSyncer, so the syncer-owned hint won't
		// auto-clear here — clear it explicitly. (The drawer kinds reload
		// their own hint state when reopened.)
		setLocalDraftHint(workspace, kind, path, false)
		return { success: true }
	} catch (e: any) {
		return { success: false, error: e?.body ?? e?.message ?? String(e) }
	}
}

/** Kind → editor save helper for the standalone trigger kinds. All share the
 * `(initialPath, cfg, edit, workspace, ...)` shape; the two that take an
 * `isAdmin` slot get it threaded. A throwaway `usedTriggerKinds` store is fine
 * — it only feeds the editors' kind-usage UI. */
const TRIGGER_SAVERS: Partial<
	Record<
		DraftKind,
		(
			initialPath: string,
			cfg: Record<string, any>,
			edit: boolean,
			workspace: string,
			isAdmin: boolean
		) => Promise<boolean>
	>
> = {
	trigger_http: (p, cfg, edit, ws, isAdmin) =>
		saveHttpRouteFromCfg(p, cfg, edit, ws, isAdmin, writable<string[]>([])),
	trigger_websocket: (p, cfg, edit, ws) =>
		saveWebsocketTriggerFromCfg(p, cfg, edit, ws, writable<string[]>([])),
	trigger_postgres: (p, cfg, edit, ws) =>
		savePostgresTriggerFromCfg(p, cfg, edit, ws, writable<string[]>([])),
	trigger_kafka: (p, cfg, edit, ws) =>
		saveKafkaTriggerFromCfg(p, cfg, edit, ws, writable<string[]>([])),
	trigger_nats: (p, cfg, edit, ws) =>
		saveNatsTriggerFromCfg(p, cfg, edit, ws, writable<string[]>([])),
	trigger_mqtt: (p, cfg, edit, ws) =>
		saveMqttTriggerFromCfg(p, cfg, edit, ws, writable<string[]>([])),
	trigger_sqs: (p, cfg, edit, ws) =>
		saveSqsTriggerFromCfg(p, cfg, edit, ws, writable<string[]>([])),
	trigger_gcp: (p, cfg, edit, ws) =>
		saveGcpTriggerFromCfg(p, cfg, edit, ws, writable<string[]>([])),
	trigger_azure: (p, cfg, edit, ws) =>
		saveAzureTriggerFromCfg(p, cfg, edit, ws, writable<string[]>([])),
	trigger_email: (p, cfg, edit, ws, isAdmin) =>
		saveEmailTriggerFromCfg(p, cfg, edit, ws, isAdmin, writable<string[]>([]))
}

/**
 * Discard a draft. Draft-only items exist ONLY as a draft-table row (the
 * deployed tables hold nothing for them since the draft_only column was
 * dropped), so deleting the draft row is the whole discard in every case —
 * `save({ value: null })` is the canonical "drop my draft for this path"
 * POST, the same call the editor makes on Reset to deployed.
 */
export async function discardDraft(
	kind: DraftKind,
	path: string,
	workspace: string,
	_draftOnly = false
): Promise<DeployResult> {
	try {
		// Routes through UserDraftDbSyncer.postSave, which clears the
		// syncer-owned `*` hint on a `value: null` delete — no explicit
		// clear needed here. `immediate: true` so this await resolves only
		// after the POST lands — without it the save resolves at enqueue
		// time and the invalidate below refetches ~1.5s ahead of the
		// delete, re-listing the just-discarded draft.
		await UserDraftDbSyncer.save({ workspace, itemKind: kind, path, value: null, immediate: true })
		invalidateWorkspaceDrafts(workspace)
		return { success: true }
	} catch (e: any) {
		return { success: false, error: e?.body ?? e?.message ?? String(e) }
	}
}
