import { AppService, DraftService, FlowService, ScriptService } from '$lib/gen'
import type { Flow, NewScript } from '$lib/gen'
import { DEFAULT_DATA as DEFAULT_RAW_APP_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
import type { UserDraftMeta } from '$lib/userDraft.svelte'
import type { AppDraftValue } from './workspaceItems'

/**
 * Persists AI-chat drafts of `script` / `flow` / `app` items exactly the way the
 * editors' "Save draft" buttons do, against the backend `draft` table.
 *
 * The DB draft is the source of truth for AI-chat drafts. localStorage mirroring
 * for an open live editor is layered on top in `draftStore.ts` — this module
 * never touches localStorage.
 *
 * Two pieces per save:
 *  1. The draft row itself, via `DraftService.createDraft` — an upsert keyed by
 *     (workspace, path, typ). Its `value` mirrors the shape the editors store.
 *  2. For a brand-new item with no row yet in the script/flow/app table, a
 *     `draft_only` anchor row. This is required because `get*ByPathWithDraft`
 *     is a `LEFT JOIN` *from* the item table — without the anchor the draft is
 *     unreadable. Existing items (deployed or already draft_only) skip this.
 *
 * `opts.itemExists` is "a row already exists at this path in the item table
 * (deployed or draft_only)". The caller knows this from the read it already did,
 * so we never fetch it again here (keeps this module pure and avoids the
 * redundant-fetch problem).
 */

// The editors store the full item plus a `draft_triggers` array in the draft
// `value`. AI chat manages triggers as separate (localStorage-only) drafts, so
// we preserve any triggers already carried on the draft and otherwise default
// to none — never dropping the field, so the round-trip matches the editors'.
type ScriptDbDraft = NewScript & { draft_triggers?: unknown[] }
type FlowDbDraft = Flow & { draft_triggers?: unknown[] }

function scriptDraftValue(draft: ScriptDbDraft): ScriptDbDraft {
	return { ...draft, draft_triggers: draft.draft_triggers ?? [] }
}

// The anchor carries every NewScript field so that `listScripts({includeDraftOnly})`
// and a first-time deploy observe the same metadata the draft was written with.
// `draft_triggers` isn't a NewScript field, and a fresh draft_only row has no
// parent — strip both.
function scriptAnchorBody(draft: ScriptDbDraft): NewScript {
	const { draft_triggers: _triggers, parent_hash: _parent, ...rest } = draft
	return { ...rest, draft_only: true }
}

export async function saveScriptDbDraft(
	workspace: string,
	path: string,
	draft: ScriptDbDraft,
	opts: { itemExists: boolean }
): Promise<void> {
	if (!opts.itemExists) {
		await ScriptService.createScript({ workspace, requestBody: scriptAnchorBody(draft) })
	}
	await DraftService.createDraft({
		workspace,
		requestBody: { path, typ: 'script', value: scriptDraftValue(draft) }
	})
}

function flowDraftValue(draft: FlowDbDraft, path: string): FlowDbDraft {
	return { ...draft, path, draft_triggers: draft.draft_triggers ?? [] }
}

function flowAnchorBody(draft: Flow, path: string) {
	return {
		path,
		summary: draft.summary ?? '',
		description: draft.description ?? '',
		value: draft.value,
		schema: draft.schema,
		draft_only: true
	}
}

export async function saveFlowDbDraft(
	workspace: string,
	path: string,
	draft: FlowDbDraft,
	opts: { itemExists: boolean }
): Promise<void> {
	if (!opts.itemExists) {
		await FlowService.createFlow({ workspace, requestBody: flowAnchorBody(draft, path) })
	}
	await DraftService.createDraft({
		workspace,
		requestBody: { path, typ: 'flow', value: flowDraftValue(draft, path) }
	})
}

// AI chat works with raw apps. The raw-app editor stores the runtime
// `{ files, runnables, data }` nested under `value`, alongside path/summary/
// policy/custom_path — mirror that exactly so a draft round-trips between chat
// and an open raw-app editor.
function rawAppValue(value: AppDraftValue) {
	return { files: value.files, runnables: value.runnables, data: value.data }
}

function appDraftValue(value: AppDraftValue, path: string) {
	return {
		value: rawAppValue(value),
		path,
		summary: value.summary ?? '',
		policy: value.policy,
		custom_path: value.custom_path
	}
}

/**
 * Unlike scripts/flows, a raw-app anchor row carries the compiled bundle, so the
 * caller threads a `bundle` callback (used only when an anchor must be created).
 * `value.policy` must already be computed by the caller — `createAppRaw` requires it.
 */
export async function saveAppDbDraft(
	workspace: string,
	path: string,
	value: AppDraftValue,
	opts: { itemExists: boolean; bundle?: () => Promise<{ js: string; css: string }> }
): Promise<void> {
	if (!opts.itemExists) {
		if (!opts.bundle) {
			throw new Error('A bundle is required to create a new app draft anchor.')
		}
		const policy = value.policy
		if (!policy) {
			throw new Error('A policy is required to create a new app draft anchor.')
		}
		const { js, css } = await opts.bundle()
		await AppService.createAppRaw({
			workspace,
			formData: {
				app: {
					value: rawAppValue(value),
					path,
					summary: value.summary ?? '',
					policy,
					draft_only: true,
					custom_path: value.custom_path
				},
				js,
				css
			}
		})
	}
	await DraftService.createDraft({
		workspace,
		requestBody: { path, typ: 'app', value: appDraftValue(value, path) }
	})
}

/**
 * The read counterpart of the save functions above. Fetches the *latest DB
 * draft falling back to the deployed version* for a single item, surfacing the
 * flags the orchestrator needs:
 *
 *  - `itemExists`   — a row exists at this path (deployed OR draft_only). This
 *                     is exactly the `opts.itemExists` the matching `save*`
 *                     function consumes, closing the read -> write loop.
 *  - `deployedExists` — a non-`draft_only` (i.e. deployed) version exists.
 *  - `draftOnly`    — the item row is a `draft_only` anchor.
 *  - `hasDbDraft`   — a draft row exists (the `draft` field is present).
 *  - `value`        — the draft value when `hasDbDraft`, else the deployed
 *                     value, else `undefined`. Shaped exactly like what the
 *                     save path writes / the editors store.
 *  - `meta`         — `{ remoteRev, remoteDraftRev }`: the deployed version's
 *                     id/hash and the DB draft's `created_at`.
 *
 * This module never touches localStorage; read precedence (live editor ->
 * localStorage) is layered on top by the orchestrator.
 */
export type DbDraftRead<V> = {
	itemExists: boolean
	deployedExists: boolean
	draftOnly: boolean
	hasDbDraft: boolean
	value: V | undefined
	meta: UserDraftMeta
}

const NOT_FOUND: DbDraftRead<never> = {
	itemExists: false,
	deployedExists: false,
	draftOnly: false,
	hasDbDraft: false,
	value: undefined,
	meta: {}
}

// A missing item surfaces from the generated client as a thrown ApiError with
// `.status === 404` (the `get*ByPathWithDraft` handlers `not_found_if_none`).
// Catch *only* that case so genuine failures still propagate.
function isNotFound(e: unknown): boolean {
	return typeof e === 'object' && e !== null && (e as { status?: unknown }).status === 404
}

export async function readScriptDbDraft(
	workspace: string,
	path: string
): Promise<DbDraftRead<NewScript>> {
	let existing: Awaited<ReturnType<typeof ScriptService.getScriptByPathWithDraft>>
	try {
		existing = await ScriptService.getScriptByPathWithDraft({ workspace, path })
	} catch (e) {
		if (isNotFound(e)) return NOT_FOUND
		throw e
	}
	const draftOnly = !!existing.draft_only
	const hasDbDraft = existing.draft != null
	// Scripts/flows store the draft `value` as the raw item (no wrapper).
	const value = (hasDbDraft ? existing.draft : existing) as NewScript
	return {
		itemExists: true,
		deployedExists: !draftOnly,
		draftOnly,
		hasDbDraft,
		value,
		meta: { remoteRev: existing.hash, remoteDraftRev: existing.draft_created_at }
	}
}

export async function readFlowDbDraft(workspace: string, path: string): Promise<DbDraftRead<Flow>> {
	let existing: Awaited<ReturnType<typeof FlowService.getFlowByPathWithDraft>>
	try {
		existing = await FlowService.getFlowByPathWithDraft({ workspace, path })
	} catch (e) {
		if (isNotFound(e)) return NOT_FOUND
		throw e
	}
	const draftOnly = !!existing.draft_only
	const hasDbDraft = existing.draft != null
	const value = (hasDbDraft ? existing.draft : existing) as Flow

	// `remoteRev` for a flow is the latest deployed version id — a separate
	// call. A `draft_only` flow has no deployed version, so skip the lookup;
	// otherwise tolerate a 404 (deployed version raced away) by leaving
	// `remoteRev` undefined rather than failing the whole read.
	let remoteRev: number | undefined
	if (!draftOnly) {
		try {
			const latestVersion = await FlowService.getFlowLatestVersion({ workspace, path })
			remoteRev = latestVersion.id
		} catch (e) {
			if (!isNotFound(e)) throw e
		}
	}

	return {
		itemExists: true,
		deployedExists: !draftOnly,
		draftOnly,
		hasDbDraft,
		value,
		meta: { remoteRev, remoteDraftRev: existing.draft_created_at }
	}
}

// The canonical app-source codecs. `core.ts` imports these (the `core -> dbDraft`
// direction is safe; `dbDraft` never imports `core`) so the read path here, the
// chat read/write sites, and the deploy path all interpret the stored app JSON
// identically to the editors.
export function normalizeRawAppData(value: Record<string, any>): AppDraftValue['data'] {
	if (value.data?.creation) {
		return {
			tables: value.data.tables ?? [],
			datatable: value.data.creation.datatable,
			schema: value.data.creation.schema
		}
	}
	if (value.data) {
		return value.data
	}
	if (value.datatables) {
		return { ...DEFAULT_RAW_APP_DATA, tables: value.datatables }
	}
	if (value.dataTableRefs) {
		return { ...DEFAULT_RAW_APP_DATA, tables: value.dataTableRefs }
	}
	return { ...DEFAULT_RAW_APP_DATA }
}

export function appSourceToDraftValue(app: any, fallback?: any): AppDraftValue {
	const value = (app.value ?? {}) as Record<string, any>
	return {
		summary: app.summary ?? '',
		files: { ...(value.files ?? {}) },
		runnables: { ...(value.runnables ?? {}) },
		data: normalizeRawAppData(value),
		policy: app.policy ?? fallback?.policy,
		custom_path: app.custom_path ?? fallback?.custom_path
	}
}

export function appDraftMeta(app: { versions?: number[]; draft_created_at?: string }): UserDraftMeta {
	return {
		remoteRev: app.versions ? app.versions[app.versions.length - 1] : undefined,
		remoteDraftRev: app.draft_created_at
	}
}

export async function readAppDbDraft(
	workspace: string,
	path: string
): Promise<DbDraftRead<AppDraftValue>> {
	let app: Awaited<ReturnType<typeof AppService.getAppByPathWithDraft>>
	try {
		app = await AppService.getAppByPathWithDraft({ workspace, path })
	} catch (e) {
		if (isNotFound(e)) return NOT_FOUND
		throw e
	}
	const draftOnly = !!app.draft_only
	const hasDbDraft = app.draft != null
	// `app.draft` nests the raw app under `.value`; deployed apps expose it the
	// same way. `appSourceToDraftValue` reads `.value`/`.summary`/`.policy`/
	// `.custom_path`, falling back to the deployed app for policy/custom_path.
	const value = appSourceToDraftValue(hasDbDraft ? app.draft : app, app)
	return {
		itemExists: true,
		deployedExists: !draftOnly,
		draftOnly,
		hasDbDraft,
		value,
		meta: appDraftMeta(app)
	}
}
