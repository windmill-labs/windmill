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
	DraftService,
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
import { TRIGGER_RUNTIME_IGNORE } from '$lib/utils_deployable'
import { deployRawAppDraft } from '$lib/rawAppDeploy'
import { canonicalRawAppDiffValue } from '$lib/components/raw_apps/utils'
import { classicAppDraftParts } from '$lib/appDiffSides'
import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
import { invalidateWorkspaceComparison } from '$lib/workspaceComparison'
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

/** Kind → "get by path with draft overlay" call, for kinds whose draft is the
 * editor's flat config (all but script/flow/app, handled below). Feature-gated
 * trigger services 404 when the backend lacks the kind; the caller surfaces it. */
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
function splitOverlay(r: any): {
	deployed: any
	draft: any
	hasDraft: boolean
	noDeployed: boolean
} {
	const {
		draft,
		is_draft: _i,
		draft_saved_at: _c,
		no_deployed,
		other_drafts_users: _o,
		...deployed
	} = r
	return {
		deployed,
		draft: draft ?? deployed,
		hasDraft: draft != null,
		noDeployed: no_deployed === true
	}
}

export interface DraftDiffValues {
	deployed: unknown
	draft: unknown
	/** False when the overlay carried no draft row (the item's own value was used as the draft side). */
	hasDraft: boolean
	/** True when the item has never been deployed (`draft_only` overlay). */
	noDeployed: boolean
}

// Empty-but-valid "deployed" shapes for draft_only items. A bare `{}` breaks
// the flow graph diff (needs `value.modules`) and hangs the drawer, so each
// listed kind gets a minimal shape; unlisted kinds fall back to `{}`.
const EMPTY_DEPLOYED: Partial<Record<DraftKind, (draft: any) => unknown>> = {
	script: (draft) => ({ content: '', language: draft?.language, schema: {} }),
	flow: () => ({ summary: '', value: { modules: [] }, schema: {} }),
	app: () => ({ summary: '', value: {} })
}

// Server-managed script-row fields, stripped from BOTH sides of a draft diff:
// never user-edited, they are either identical noise (created_at, workspace_id)
// or spuriously different (lock is recomputed at deploy). The draft-side
// pinned-base `parent_hash` is stripped separately, like the flow `version_id`.
const SCRIPT_ROW_RUNTIME_IGNORE = new Set([
	'workspace_id',
	'hash',
	'parent_hash',
	'parent_hashes',
	'created_at',
	'created_by',
	'archived',
	'deleted',
	'extra_perms',
	'lock',
	'lock_error_logs',
	'starred',
	'has_draft',
	'draft_only',
	'assets',
	'marked'
])

function stripScriptRowRuntime(row: any): Record<string, unknown> {
	if (!row || typeof row !== 'object') return {}
	return Object.fromEntries(Object.entries(row).filter(([k]) => !SCRIPT_ROW_RUNTIME_IGNORE.has(k)))
}

/** Canonicalize a raw draft value onto the same shape `getDraftDiffValues`
 * yields for its draft side, so a value read from an in-memory editor cell
 * diffs cleanly against a deployed side (and compares equal to its own
 * persisted form instead of differing on stripped fields). */
export function canonicalDraftSideValue(kind: DraftKind, value: unknown): unknown {
	if (kind === 'script') return stripScriptRowRuntime(value)
	if (kind === 'raw_app') return canonicalRawAppDiffValue((value ?? {}) as Record<string, any>)
	if (kind === 'app') {
		const parts = classicAppDraftParts(value)
		return { summary: parts.summary ?? '', value: parts.value }
	}
	if (kind === 'flow' && value !== null && typeof value === 'object') {
		const { version_id: _v, ...rest } = value as Record<string, unknown>
		return rest
	}
	// Drawer kinds (variables/resources/schedules/triggers): the editor-state
	// shape diverges from the backend row — same canonicalization the overlay
	// diff applies.
	return canonicalizeDraftDiffValue(kind, value, true)
}

// Schedule & trigger rows drop the same runtime/server-managed fields as the
// fork/compare path so the diff shows only config changes — reuse that set
// (the authoritative mirror of the backend `TRIGGER_COMPARE_IGNORE`).
function stripScheduleTriggerRuntime(row: any): Record<string, unknown> {
	if (!row || typeof row !== 'object') return {}
	return Object.fromEntries(Object.entries(row).filter(([k]) => !TRIGGER_RUNTIME_IGNORE.has(k)))
}

/**
 * Project a variable/resource/schedule/trigger value — given in either its
 * deployed backend-row shape or its draft editor-state shape — onto one
 * canonical field set, so the deployed and draft sides of a diff are comparable
 * and read as labeled rows instead of structural noise (variable `variable.value`
 * vs `value`, resource `args` vs `value`, schedule/trigger runtime fields). This
 * matches the shaping the compare page applies via `getItemValue`. `isDraft`
 * selects the editor-state field names; secret variable values are masked.
 */
function canonicalizeDraftDiffValue(kind: DraftKind, raw: any, isDraft: boolean): unknown {
	if (!raw || typeof raw !== 'object') return raw ?? {}
	if (kind === 'variable') {
		// draft: { variable: { value, is_secret, description }, labels, wsSpecific }
		// deployed row: { value, is_secret, description, labels, ws_specific }
		const v = isDraft ? (raw.variable ?? {}) : raw
		const is_secret = !!v.is_secret
		return {
			value: is_secret ? '<secret>' : (v.value ?? ''),
			is_secret,
			description: v.description ?? '',
			labels: raw.labels ?? undefined,
			ws_specific: (isDraft ? raw.wsSpecific : raw.ws_specific) ?? undefined
		}
	}
	if (kind === 'resource') {
		// draft: { args, description, resource_type, labels, wsSpecific }
		// deployed row: { value, description, resource_type, labels, ws_specific }
		return {
			value: (isDraft ? raw.args : raw.value) ?? {},
			description: raw.description ?? '',
			resource_type: raw.resource_type ?? undefined,
			labels: raw.labels ?? undefined,
			ws_specific: (isDraft ? raw.wsSpecific : raw.ws_specific) ?? undefined
		}
	}
	// schedule + triggers: same field names on both sides — drop runtime noise.
	return stripScheduleTriggerRuntime(raw)
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
	// Strip overlay metadata (is_draft / draft_saved_at / no_deployed /
	// other_drafts_users) from the deployed side so the diff doesn't show the
	// per-user markers as noise.
	if (kind === 'script') {
		const r = (await ScriptService.getScriptByPath({ workspace, path, getDraft: true })) as any
		const {
			draft,
			is_draft: _i,
			draft_saved_at: _c,
			no_deployed,
			other_drafts_users: _o,
			hash: _h,
			...deployed
		} = r
		const draftValue = stripScriptRowRuntime(draft ?? deployed)
		return {
			deployed: draftOnly ? EMPTY_DEPLOYED.script!(draftValue) : stripScriptRowRuntime(deployed),
			draft: draftValue,
			hasDraft: draft != null,
			noDeployed: no_deployed === true
		}
	} else if (kind === 'flow') {
		const r = (await FlowService.getFlowByPath({ workspace, path, getDraft: true })) as any
		const {
			draft,
			is_draft: _i,
			draft_saved_at: _c,
			no_deployed,
			other_drafts_users: _o,
			version_id: _v,
			...deployed
		} = r
		// Strip the draft's pinned base `version_id` (which differs from the deployed
		// head for a stale draft) so it never renders as a spurious diff line.
		const { version_id: _dv, ...draftValue } = (draft ?? deployed) as any
		return {
			deployed: draftOnly ? EMPTY_DEPLOYED.flow!(draftValue) : deployed,
			draft: draftValue,
			hasDraft: draft != null,
			noDeployed: no_deployed === true
		}
	} else if (kind === 'app' || kind === 'raw_app') {
		// A never-deployed raw app has no `app` row; the backend resolves the
		// draft kind from `rawApp`, so it MUST be set or the lookup 404s.
		const r = (await AppService.getAppByPath({
			workspace,
			path,
			getDraft: true,
			rawApp: kind === 'raw_app'
		})) as any
		if (kind === 'raw_app' || r.raw_app === true) {
			// Raw-app drafts are stored flat (files/runnables/data top-level) while the
			// deployed row nests them under `value`, and deployed inline scripts carry
			// server-recomputed locks. Canonicalize both onto the same shape with the
			// post-deploy noise stripped — the same module the editor's Diff button uses.
			// A staged rename (`draft_path`) changes where deploy lands the app —
			// compare it as `path` on both sides so a rename-only draft diffs.
			const rawDraftPath = (r.draft?.draft_path as string | undefined) ?? r.path
			return {
				deployed: draftOnly
					? canonicalRawAppDiffValue({})
					: { ...canonicalRawAppDiffValue(r), path: r.path },
				draft: { ...canonicalRawAppDiffValue(r.draft ?? r), path: rawDraftPath },
				hasDraft: r.draft != null,
				noDeployed: r.no_deployed === true
			}
		}
		// Classic app: the editor drafts the bare grid with summary/draft_path
		// mirrored into it, while the row keeps summary as a column beside
		// `value`. Both sides reduce to `{ summary, value }` with the metadata
		// extracted from the grid, so a summary edit diffs as a summary edit and
		// the grid never diffs against draft-only markers.
		const deployedParts = classicAppDraftParts(r.value)
		const draftParts = r.draft != null ? classicAppDraftParts(r.draft) : deployedParts
		const deployed = { summary: r.summary ?? '', value: deployedParts.value, path: r.path }
		return {
			deployed: draftOnly ? EMPTY_DEPLOYED.app!(undefined) : deployed,
			draft: {
				summary: draftParts.summary ?? r.summary ?? '',
				value: draftParts.value,
				path: draftParts.draftPath ?? r.path
			},
			hasDraft: r.draft != null,
			noDeployed: r.no_deployed === true
		}
	} else {
		// Variables / resources / schedules / triggers: one overlay GET yields
		// both sides, but the draft side is the editor's state shape while the
		// deployed side is the backend row — they diverge enough to make a raw
		// diff pure noise. Canonicalize both onto a shared field set (same shaping
		// the compare page's `getItemValue` applies) so only real changes show.
		const getter = OVERLAY_GETTERS[kind]
		if (!getter) {
			throw new Error(`Draft diff not supported for kind ${kind}`)
		}
		const { deployed, draft, hasDraft, noDeployed } = splitOverlay(await getter(workspace, path))
		return {
			deployed: draftOnly ? {} : canonicalizeDraftDiffValue(kind, deployed, false),
			draft: canonicalizeDraftDiffValue(kind, draft, true),
			hasDraft,
			noDeployed
		}
	}
}

/**
 * Whether a draft's base is stale: the deployed version the draft forked from
 * no longer matches the current deployed head — a newer version was deployed
 * after the draft began, so deploying the draft would silently revert it.
 * Scripts compare the draft's `parent_hash` vs the deployed `hash`; flows the
 * pinned `version_id` vs the deployed head `version_id`; apps (incl. raw) the
 * pinned `parent_version` vs the head of `versions`. `r` is the item fetched
 * with `get_draft=true`; only script/flow/app kinds carry a base pointer.
 */
export function draftBaseIsStale(draftKind: UserDraftItemKind, r: any): boolean {
	const draft = r?.draft
	if (!draft) return false
	if (draftKind === 'script') {
		return !!r.hash && !!draft.parent_hash && draft.parent_hash !== r.hash
	}
	if (draftKind === 'flow') {
		return r.version_id != null && draft.version_id != null && draft.version_id !== r.version_id
	}
	const head = Array.isArray(r.versions) ? r.versions[r.versions.length - 1] : undefined
	return head != null && draft.parent_version != null && draft.parent_version !== head
}

/** Fetch-and-test wrapper over `draftBaseIsStale` for one draft item. Returns
 *  false for kinds without a base pointer and on fetch errors (warn, not block). */
export async function fetchDraftBaseStale(
	draftKind: UserDraftItemKind,
	path: string,
	workspace: string
): Promise<boolean> {
	try {
		if (draftKind === 'script') {
			const r = await ScriptService.getScriptByPath({ workspace, path, getDraft: true })
			return draftBaseIsStale(draftKind, r)
		}
		if (draftKind === 'flow') {
			const r = await FlowService.getFlowByPath({ workspace, path, getDraft: true })
			return draftBaseIsStale(draftKind, r)
		}
		if (draftKind === 'app' || draftKind === 'raw_app') {
			// The apps endpoint auto-detects a raw app and overlays its draft.
			const r = await AppService.getAppByPath({ workspace, path, getDraft: true })
			return draftBaseIsStale(draftKind, r)
		}
		return false
	} catch (e) {
		console.error(`Stale-draft check failed for ${draftKind}:${path}`, e)
		return false
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
	opts: { draftOnly?: boolean; rawApp?: boolean; deploymentMessage?: string } = {}
): Promise<DeployResult> {
	const { draftOnly = false, rawApp = false, deploymentMessage } = opts
	try {
		if (kind === 'raw_app' || (kind === 'app' && rawApp)) {
			// Raw apps bundle their source files and deploy via the raw-app
			// endpoints. Reached as `kind === 'raw_app'` (Review & Deploy) or
			// `kind === 'app'` + `rawApp` (editor). Must route here: the
			// visual-app branch would `updateApp` with no `value` (RawAppDraft
			// has none) and silently drop the draft's files.
			await deployRawAppDraft(workspace, path, deploymentMessage)
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
				requestBody: {
					...rest,
					path: scriptPath,
					parent_hash: r.hash,
					deployment_message: deploymentMessage
				}
			})
			// Then deploy any draft trigger edits, so they aren't dropped with the draft.
			await deployDraftTriggers(draftTriggers, workspace, scriptPath, true)
		} else if (kind === 'flow') {
			const r = (await FlowService.getFlowByPath({ workspace, path, getDraft: true })) as any
			const d = r.draft ?? r
			const requestBody = {
				// Deploy at the draft's intended path: flow/app/raw-app drafts keep the
				// user-typed path in `draft_path` (a never-deployed item is parked at a
				// synthetic `u/{user}/draft_{uuid}` storage key). The URL `path` stays
				// that storage key.
				path: d.draft_path ?? d.path ?? path,
				summary: d.summary ?? '',
				description: d.description ?? '',
				value: d.value,
				schema: d.schema,
				tag: d.tag,
				dedicated_worker: d.dedicated_worker,
				ws_error_handler_muted: d.ws_error_handler_muted,
				visible_to_runner_only: d.visible_to_runner_only,
				on_behalf_of_email: d.on_behalf_of_email,
				labels: d.labels,
				deployment_message: deploymentMessage
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
			await deployDraftTriggers(
				d.draft_triggers,
				workspace,
				d.draft_path ?? d.path ?? path,
				draftOnly
			)
		} else if (kind === 'app') {
			// `raw_app` is handled above; only visual apps reach here.
			const r = (await AppService.getAppByPath({ workspace, path, getDraft: true })) as any
			// A visual-app draft is stored as the *bare* app value (grid/theme/...,
			// plus a `draft_path` when the path was renamed) — NOT wrapped in
			// { value, summary, policy } like script/flow drafts. So the deploy value
			// is the draft object itself; fall back to the deployed value when there's
			// no draft. `draft_path` and `summary` are draft-only fields mirrored onto
			// the App value (the editor drops them on deploy), so strip them from the
			// value and apply them as the deploy path / summary column.
			const draft = r.draft as Record<string, any> | undefined
			const { draft_path: draftPath, summary: draftSummary, ...appValue } = draft ?? r.value ?? {}
			// Policy isn't carried in the app draft, so it comes from the deployed app
			// (or a default). custom_path requires admin on update; non-admins send
			// undefined so the backend preserves the existing route. The draft has no
			// custom_path, so admins fall back to the deployed route (`''` when none).
			const isAdmin = !!(get(userStore)?.is_admin || get(userStore)?.is_super_admin)
			const policy = r.policy ?? { execution_mode: 'publisher' }
			const requestBody = {
				value: appValue,
				summary: draftSummary ?? r.summary ?? '',
				policy,
				// Honor the draft's intended path; `draft_path` holds the user-typed path
				// for a never-deployed app parked at a `u/{user}/draft_{uuid}` storage key.
				path: draftPath ?? r.path ?? path,
				custom_path: isAdmin ? (r.custom_path ?? '') : undefined,
				deployment_message: deploymentMessage,
				// The draft carries no on-behalf-of selector — the policy comes straight
				// from the deployed app. Preserve its on_behalf_of (the backend resets it
				// to the deploying user without this flag, gated by can_preserve_on_behalf_of).
				preserve_on_behalf_of: policy?.on_behalf_of ? true : undefined
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
		// Delete the draft at its STORAGE path (the row key, = the `path` arg).
		// Two reasons it must happen here for every kind, mirroring the editors'
		// post-deploy `discardDraftAfterDeploy(draftPath)`:
		//  - Drawer kinds (variable / resource / triggers) aren't deleted by
		//    their create/update endpoints at all.
		//  - script/flow/app/raw_app DO delete server-side, but only the draft at
		//    the *deployed* path (`d.path`). A renamed draft_only item lives at a
		//    synthetic `u/{user}/draft_{uuid}` storage path ≠ `d.path`, so its
		//    draft row survives the deploy and keeps listing. Deleting the
		//    storage-path draft removes it (a no-op when the server already did).
		await UserDraftDbSyncer.save({
			workspace,
			itemKind: kind,
			path,
			value: null,
			immediate: true
		})
		// Mutated the workspace's Server Drafts — refresh every mounted reader.
		invalidateWorkspaceDrafts(workspace)
		// The DEPLOYED state moved: cached fork comparisons involving this
		// workspace (as fork or as parent) are no longer trustworthy. Draft-only
		// mutations skip this — they never move the deployed tally.
		invalidateWorkspaceComparison(workspace)
		// For script/flow/app the server-side delete bypasses UserDraftDbSyncer,
		// so the syncer-owned hint won't auto-clear — clear it explicitly.
		// (Idempotent: the drawer-kind delete above already cleared it.)
		setLocalDraftHint(workspace, kind, path, false)
		return { success: true }
	} catch (e: any) {
		return { success: false, error: e?.body ?? e?.message ?? String(e) }
	}
}

/** Kind → editor save helper for the standalone trigger kinds, all sharing the
 * `(initialPath, cfg, edit, workspace, isAdmin?)` shape. The throwaway
 * `usedTriggerKinds` store only feeds the editors' kind-usage UI. */
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
 * Discard a draft. Draft-only items exist only as a draft-table row, so
 * deleting that row is the whole discard in every case. `save({ value: null })`
 * is the canonical "drop my draft for this path" POST.
 *
 * A legacy draft (workspace-level, `email IS NULL`) isn't owned by the authed
 * user, so the email-scoped syncer delete can't reach it. Discard it via a
 * direct `legacy` delete instead — then clear the local hint + invalidate as
 * the syncer path would.
 */
export async function discardDraft(
	kind: DraftKind,
	path: string,
	workspace: string,
	_draftOnly = false,
	legacy = false
): Promise<DeployResult> {
	try {
		if (legacy) {
			await DraftService.updateDraft({
				workspace,
				kind,
				path,
				requestBody: { value: null, legacy: true }
			})
			setLocalDraftHint(workspace, kind, path, false)
			invalidateWorkspaceDrafts(workspace)
			return { success: true }
		}
		// postSave clears the syncer-owned `*` hint on the delete. `immediate`
		// so the await resolves after the POST lands — else it resolves at
		// enqueue time and the invalidate below refetches before the delete,
		// re-listing the just-discarded draft.
		await UserDraftDbSyncer.save({ workspace, itemKind: kind, path, value: null, immediate: true })
		invalidateWorkspaceDrafts(workspace)
		return { success: true }
	} catch (e: any) {
		return { success: false, error: e?.body ?? e?.message ?? String(e) }
	}
}
