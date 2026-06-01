import { DraftService, type UserDraftItemKind } from './gen'

export type UserDraftDbSyncerSaveOpts = {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	/** `null` signals a delete — the server removes the row under the same
	 *  conflict rules as an upsert. */
	value: unknown | null
}

/**
 * Server-side persistence for `UserDraft`. `UserDraft` owns the localStorage
 * cache; this module forwards each write through to `POST /drafts/save_draft`
 * so the per-user `draft` table on the server stays in sync.
 *
 * Kept as a separate module so the two halves stay decoupled — `UserDraft`
 * just calls `UserDraftDbSyncer.save(...)` and doesn't reach into the
 * generated client. Adding conflict handling (`last_sync` + a reject UI)
 * later means changing this file, not every editor.
 *
 * NOTE: every save currently uses `force: true`, so the server copy is
 * unconditionally overwritten. This is intentional for the first cut —
 * we'll thread `last_sync` through once the client side is settled.
 */
export const UserDraftDbSyncer = {
	async save(opts: UserDraftDbSyncerSaveOpts): Promise<void> {
		try {
			await DraftService.saveDraft({
				workspace: opts.workspace,
				kind: opts.itemKind,
				path: opts.path,
				requestBody: {
					value: opts.value as any,
					force: true
				}
			})
		} catch (e) {
			console.error('UserDraftDbSyncer.save failed', e)
		}
	}
}
