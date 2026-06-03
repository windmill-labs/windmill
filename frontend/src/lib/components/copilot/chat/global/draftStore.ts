import type { Flow, NewScript } from '$lib/gen'
import { UserDraft, type UserDraftMeta } from '$lib/userDraft.svelte'
import {
	readScriptDbDraft,
	readFlowDbDraft,
	readAppDbDraft,
	saveScriptDbDraft,
	saveFlowDbDraft,
	saveAppDbDraft,
	type DbDraftRead
} from './dbDraft'
import {
	getGlobalDraftStoragePath,
	saveGlobalAppDraft,
	saveGlobalScriptDraft,
	saveGlobalFlowDraft,
	userDraftKindFor
} from './userDraftAdapter'
import type { AppDraftValue, WorkspaceItemType } from './workspaceItems'

/**
 * The single precedence point for AI-chat drafts of `script` / `flow` / `app`
 * (Group A). It sits ON TOP of two layers and decides, per (type, path),
 * whether to read/write localStorage vs the DB `draft` table.
 *
 * Architecture rule:
 *   The DB `draft` table is the source of truth for AI-chat drafts.
 *   localStorage (`UserDraft`) is only a *live-editor mirror*.
 *
 * Read precedence (`loadDraft`):
 *   1. a live editor is open for this (kind, path) AND has a localStorage draft
 *      -> use the localStorage value (the editor's in-flight edits)
 *   2. else the DB draft (`.draft`)
 *   3. else the deployed version
 *   4. else none
 *
 * Write fan-out (`persistDraft`):
 *   - ALWAYS write the DB draft (`saveXDbDraft`).
 *   - IFF a live editor is open for this (kind, path), ALSO mirror the value to
 *     localStorage so the open editor updates live.
 *
 * The value shapes are kept identical to the DB layer's per-type shapes:
 *   script -> NewScript, flow -> Flow, app -> AppDraftValue.
 * The localStorage layer stores those same shapes (script -> NewScript,
 * flow -> Flow, raw_app -> a flattened AppDraftValue), so reads/writes round
 * trip between chat and an open editor without lossy conversions.
 */

export type DraftType = Extract<WorkspaceItemType, 'script' | 'flow' | 'app'>

type DraftValueOf = {
	script: NewScript
	flow: Flow
	app: AppDraftValue
}

/**
 * Where the loaded value came from:
 *  - 'live'     — an open live editor's localStorage mirror (in-flight edits)
 *  - 'db'       — the DB draft row (`.draft`)
 *  - 'deployed' — the deployed version (no draft exists)
 *  - 'none'     — nothing exists at this path
 */
export type LoadedDraftSource = 'live' | 'db' | 'deployed' | 'none'

export type LoadedDraft<T extends DraftType = DraftType> = {
	source: LoadedDraftSource
	value: DraftValueOf[T] | undefined
	/**
	 * Rev metadata of the resolved value. For DB/deployed branches this is the
	 * `DbDraftRead.meta`. For the live branch it is empty (a pure read of an
	 * open editor's mirror doesn't need DB flags).
	 */
	meta: UserDraftMeta
	itemExists: boolean
	deployedExists: boolean
	draftOnly: boolean
	hasDbDraft: boolean
	/**
	 * The path stored on the resolved source when it differs from the lookup path
	 * (a draft rename). Only the app read surfaces it (its value has no `path`);
	 * scripts/flows carry `path` on `value`. Undefined on the live branch.
	 */
	draftPath?: string
}

function readDbDraft<T extends DraftType>(
	type: T,
	path: string,
	workspace: string
): Promise<DbDraftRead<DraftValueOf[T]>> {
	switch (type) {
		case 'script':
			return readScriptDbDraft(workspace, path) as Promise<DbDraftRead<DraftValueOf[T]>>
		case 'flow':
			return readFlowDbDraft(workspace, path) as Promise<DbDraftRead<DraftValueOf[T]>>
		default:
			return readAppDbDraft(workspace, path) as Promise<DbDraftRead<DraftValueOf[T]>>
	}
}

/**
 * Is a live editor currently open for this Group-A (type, path)?
 *
 * True when a live editor handle is registered for the item kind AND its
 * storage path resolves to the same path we're asking about (so an editor open
 * for a *different* path doesn't count). `getGlobalDraftStoragePath` resolves
 * `path` against the live editor (returning the live storage path when `path`
 * matches its storagePath or effectivePath), so a match means "this path".
 */
export function hasLiveEditor(
	workspace: string,
	type: WorkspaceItemType,
	path: string
): boolean {
	const itemKind = userDraftKindFor(type)
	if (!itemKind) return false
	const liveDraft = UserDraft.getLiveEditorDraft(itemKind, { workspace })
	if (!liveDraft) return false
	return getGlobalDraftStoragePath(workspace, type, path) === liveDraft.storagePath
}

/**
 * Read the raw localStorage mirror value for an OPEN live editor, in the
 * per-type shape the DB layer uses (script -> NewScript, flow -> Flow,
 * app -> AppDraftValue). Returns `undefined` when there is no localStorage
 * entry (e.g. the editor was opened but never edited) — the caller then falls
 * back to the DB.
 *
 * Reads the raw value via `UserDraft.get` rather than `getGlobalDraft`'s
 * `WorkspaceItem` (whose `.value` is lossy/variable per type).
 */
function readLiveValue<T extends DraftType>(
	type: T,
	path: string,
	workspace: string
): DraftValueOf[T] | undefined {
	const itemKind = userDraftKindFor(type)
	if (!itemKind) return undefined
	const storagePath = getGlobalDraftStoragePath(workspace, type, path)
	return UserDraft.get<DraftValueOf[T]>(itemKind, storagePath, { workspace })
}

/**
 * THE read precedence. See the module header for the rules.
 *
 * For the 'live' branch it skips the DB call entirely — a pure read of an open
 * editor's mirror doesn't need the DB flags. For 'db'/'deployed'/'none' it
 * returns the `DbDraftRead` flags so callers (and the write path) can reuse
 * `itemExists` etc. without re-fetching.
 */
export async function loadDraft<T extends DraftType>(
	type: T,
	path: string,
	workspace: string
): Promise<LoadedDraft<T>> {
	if (hasLiveEditor(workspace, type, path)) {
		const liveValue = readLiveValue(type, path, workspace)
		if (liveValue !== undefined) {
			return {
				source: 'live',
				value: liveValue,
				meta: {},
				// A live editor implies the item participates; precise DB flags
				// aren't needed for a pure read, so report the optimistic shape.
				itemExists: true,
				deployedExists: false,
				draftOnly: false,
				hasDbDraft: false
			}
		}
	}

	const db = await readDbDraft(type, path, workspace)
	const source: LoadedDraftSource = !db.itemExists ? 'none' : db.hasDbDraft ? 'db' : 'deployed'
	return {
		source,
		value: db.value,
		meta: db.meta,
		itemExists: db.itemExists,
		deployedExists: db.deployedExists,
		draftOnly: db.draftOnly,
		hasDbDraft: db.hasDbDraft,
		draftPath: db.draftPath
	}
}

async function saveDbDraft<T extends DraftType>(
	type: T,
	path: string,
	workspace: string,
	value: DraftValueOf[T],
	itemExists: boolean,
	bundle?: () => Promise<{ js: string; css: string }>
): Promise<void> {
	switch (type) {
		case 'script':
			await saveScriptDbDraft(workspace, path, value as NewScript, { itemExists })
			return
		case 'flow':
			await saveFlowDbDraft(workspace, path, value as Flow, { itemExists })
			return
		default:
			await saveAppDbDraft(workspace, path, value as AppDraftValue, { itemExists, bundle })
			return
	}
}

function mirrorToLocalStorage<T extends DraftType>(
	type: T,
	path: string,
	workspace: string,
	value: DraftValueOf[T],
	meta: UserDraftMeta
): void {
	switch (type) {
		case 'script':
			saveGlobalScriptDraft(workspace, path, value as NewScript, meta)
			return
		case 'flow':
			saveGlobalFlowDraft(workspace, path, value as Flow, meta)
			return
		default:
			saveGlobalAppDraft(workspace, path, value as AppDraftValue, meta)
			return
	}
}

export type PersistDraftOptions = {
	/**
	 * Whether a row already exists at this path in the item table (deployed or
	 * draft_only). Pass it when the caller already read it (e.g. from a prior
	 * `loadDraft`) to avoid the redundant fetch. Omit it and this function reads
	 * it once.
	 */
	itemExists?: boolean
	/**
	 * App-only: compiled bundle, used solely when a new draft_only anchor must
	 * be created (see `saveAppDbDraft`).
	 */
	bundle?: () => Promise<{ js: string; css: string }>
}

/**
 * THE write. Always persists the DB draft, then mirrors to localStorage IFF a
 * live editor is open for this (type, path).
 *
 * Staleness (chosen option A — see module note below): after the DB write, when
 * a live editor is open, do one `readXDbDraft` to fetch the fresh
 * `meta.remoteDraftRev` (the new draft row's `created_at`) and stamp it on the
 * mirror. `DraftService.createDraft` doesn't return the new `created_at`, and
 * the mirror's content now EQUALS the new DB draft — so without the fresh rev
 * the editor's `checkStaleness` banner would false-positive. The extra read
 * happens on writes only (rare) and on the live branch only, so the cost is
 * negligible relative to keeping the editor banner correct.
 */
export async function persistDraft<T extends DraftType>(
	type: T,
	path: string,
	workspace: string,
	value: DraftValueOf[T],
	opts: PersistDraftOptions = {}
): Promise<void> {
	let itemExists = opts.itemExists
	if (itemExists === undefined) {
		const db = await readDbDraft(type, path, workspace)
		itemExists = db.itemExists
	}

	await saveDbDraft(type, path, workspace, value, itemExists, opts.bundle)

	// Mirror to localStorage only when a live editor is open for this path.
	if (!hasLiveEditor(workspace, type, path)) return

	// Re-read the DB draft for the fresh `remoteDraftRev` so the mirror's
	// staleness check sees it as in-sync with the DB draft we just wrote.
	const fresh = await readDbDraft(type, path, workspace)
	mirrorToLocalStorage(type, path, workspace, value, fresh.meta)
}
