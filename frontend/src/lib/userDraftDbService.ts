import { get } from 'svelte/store'
import { DraftService } from './gen'
import { workspaceStore } from './stores'
import type { UserDraftItemKind } from './userDraft.svelte'

/**
 * Persists `UserDraft` changes to the backend `draft` table. `UserDraft` is a
 * purely in-memory two-way sync layer; this service is the bridge that turns
 * each in-memory change into a `DraftService.createDraft` / `deleteDraft` call.
 *
 * Only the kinds that have a backing DB draft are persisted (script/flow/app,
 * and raw_app which is stored under the `app` draft type). Every other kind
 * (resource, variable, the trigger_* kinds — which are folded into their
 * parent script/flow draft via `draft_triggers`) is in-memory only and is a
 * no-op here.
 *
 * Writes are debounced per (workspace, typ, path) so a burst of edits collapses
 * into a single network call; the latest payload always wins.
 */

type DraftTyp = 'script' | 'flow' | 'app'

const ITEM_KIND_TO_DRAFT_TYP: Partial<Record<UserDraftItemKind, DraftTyp>> = {
	script: 'script',
	flow: 'flow',
	app: 'app',
	raw_app: 'app'
}

export type UserDraftDbSaveArgs = {
	path: string
	itemKind: UserDraftItemKind
	/** The draft content to persist. `undefined` deletes the DB draft. */
	content: unknown | undefined
	/** Defaults to the current `$workspaceStore`. */
	workspace?: string
}

const DEBOUNCE_MS = 600

type Pending = {
	timer: ReturnType<typeof setTimeout>
	content: unknown | undefined
	workspace: string
	typ: DraftTyp
	path: string
}

const pending = new Map<string, Pending>()

function key(workspace: string, typ: DraftTyp, path: string): string {
	return `${workspace}/${typ}/${path}`
}

async function flush(k: string): Promise<void> {
	const p = pending.get(k)
	if (!p) return
	pending.delete(k)
	try {
		if (p.content === undefined) {
			await DraftService.deleteDraft({ workspace: p.workspace, kind: p.typ, path: p.path })
		} else {
			await DraftService.createDraft({
				workspace: p.workspace,
				requestBody: { path: p.path, typ: p.typ, value: p.content }
			})
		}
	} catch (e) {
		// Drafts are best-effort autosave — never interrupt the editor with a
		// toast or modal (see the drafts UX simplification). Log and move on.
		console.error('UserDraftDbService: failed to sync draft', p.typ, p.path, e)
	}
}

export const UserDraftDbService = {
	/**
	 * Schedule a debounced persist of `content` for (workspace, itemKind, path).
	 * `content === undefined` deletes the draft. No-op for kinds without a DB
	 * draft.
	 */
	save({ path, itemKind, content, workspace }: UserDraftDbSaveArgs): void {
		const typ = ITEM_KIND_TO_DRAFT_TYP[itemKind]
		if (!typ) return
		// A brand-new item is edited under the empty storage path until it gets a
		// real path; there is nothing valid to persist a draft against yet (the
		// explicit "Save as draft" with a real path handles that).
		if (path === '') return
		const ws = workspace ?? get(workspaceStore)
		if (!ws) return
		const k = key(ws, typ, path)
		const existing = pending.get(k)
		if (existing) clearTimeout(existing.timer)
		const timer = setTimeout(() => flush(k), DEBOUNCE_MS)
		pending.set(k, { timer, content, workspace: ws, typ, path })
	},

	/** Whether `itemKind` is persisted to the DB (vs in-memory only). */
	hasDbDraft(itemKind: UserDraftItemKind): boolean {
		return ITEM_KIND_TO_DRAFT_TYP[itemKind] !== undefined
	}
}
