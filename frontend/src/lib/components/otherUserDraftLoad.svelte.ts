import { SvelteMap, SvelteSet } from 'svelte/reactivity'
import { goto } from '$lib/navigation'
import { base } from '$app/paths'
import { UserDraft, type UserDraftItemKind } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'

/**
 * Coordinates "Load another user's draft into the editor as if it were ours".
 *
 * Two cases, decided by the route once it knows whether WE already have a draft
 * at this path (`is_draft` for us):
 *  - No own draft  → the loaded value just becomes our draft (normal autosave).
 *  - Own draft     → "overlay" mode: the loaded value is shown but NEVER saved
 *    (so our own draft on the server is untouched). The first edit prompts
 *    "overwrite your current draft?". Confirm persists the edited value as our
 *    draft; Reset restores our own draft.
 *
 * The save block is a hard per-key lock in the syncer (see `lockSync`) — it
 * covers the reactive mirror AND the navigation / tab-death flush paths, so the
 * foreign value can't leak onto the server through any route.
 */

function keyOf(workspace: string, itemKind: UserDraftItemKind, path: string): string {
	return `${workspace}/${itemKind}/${path}`
}

export type PendingOtherUserDraftLoad = {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	value: unknown
	ownerLabel: string
}

type ActiveSession = {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	ownerLabel: string
	/** Reload our own draft into the editor (AutosaveIndicator's "Reset to draft"). */
	onResetToOwnDraft: () => void | Promise<void>
}

// One-shot handoff: set by the Load action, consumed by the editor's loader.
const pending = new SvelteMap<string, PendingOtherUserDraftLoad>()
// Live overlay sessions, keyed by (workspace, itemKind, path).
const active = new SvelteMap<string, ActiveSession>()
// Keys whose overwrite-confirmation modal is currently open.
const overwriteOpen = new SvelteSet<string>()
// Keys not yet armed for edit-detection: the programmatic load cascade
// (seed + editor `setCode`) can momentarily look like an edit; ignore blocked
// saves until the cascade settles.
const armed = new SvelteSet<string>()

function editRouteFor(itemKind: UserDraftItemKind, path: string): string {
	switch (itemKind) {
		case 'script':
			return `${base}/scripts/edit/${path}`
		case 'flow':
			return `${base}/flows/edit/${path}`
		case 'app':
			return `${base}/apps/edit/${path}`
		case 'raw_app':
			return `${base}/apps_raw/edit/${path}`
		default:
			throw new Error(`Cannot load drafts of kind ${itemKind}`)
	}
}

export const OtherUserDraftLoad = {
	/**
	 * Stage a load. The editor's loader picks it up via `takePending`. When
	 * `navigate`, route to the item's edit page (home-page entry point); the
	 * in-editor entry point reloads in place instead.
	 */
	stage(
		workspace: string,
		itemKind: UserDraftItemKind,
		value: unknown,
		path: string,
		ownerLabel: string,
		opts: { navigate: boolean }
	): void {
		pending.set(keyOf(workspace, itemKind, path), { workspace, itemKind, path, value, ownerLabel })
		if (opts.navigate) goto(editRouteFor(itemKind, path))
	},

	/** Consume the staged load for this key (returns + removes it). */
	takePending(
		workspace: string,
		itemKind: UserDraftItemKind,
		path: string
	): PendingOtherUserDraftLoad | undefined {
		const k = keyOf(workspace, itemKind, path)
		const v = pending.get(k)
		if (v) pending.delete(k)
		return v
	},

	/**
	 * Enter overlay mode: lock all server saves for this key and remember how
	 * to restore our own draft. The first blocked save (= first edit) opens the
	 * overwrite modal. Arms edit-detection after a short delay so the load
	 * cascade doesn't trip it.
	 */
	beginOverlay(session: ActiveSession): void {
		const k = keyOf(session.workspace, session.itemKind, session.path)
		active.set(k, session)
		armed.delete(k)
		UserDraftDbSyncer.lockSync(
			{ workspace: session.workspace, itemKind: session.itemKind, path: session.path },
			() => {
				if (armed.has(k))
					this.requestOverwriteModal(session.workspace, session.itemKind, session.path)
			}
		)
		setTimeout(() => armed.add(k), 700)
	},

	isActive(workspace: string, itemKind: UserDraftItemKind, path: string): boolean {
		return active.has(keyOf(workspace, itemKind, path))
	},

	getSession(
		workspace: string,
		itemKind: UserDraftItemKind,
		path: string
	): ActiveSession | undefined {
		return active.get(keyOf(workspace, itemKind, path))
	},

	requestOverwriteModal(workspace: string, itemKind: UserDraftItemKind, path: string): void {
		const k = keyOf(workspace, itemKind, path)
		if (active.has(k)) overwriteOpen.add(k)
	},

	isOverwriteModalOpen(workspace: string, itemKind: UserDraftItemKind, path: string): boolean {
		return overwriteOpen.has(keyOf(workspace, itemKind, path))
	},

	/** Cancel: keep editing the loaded value, stay paused; re-prompt on the next edit. */
	dismissOverwriteModal(workspace: string, itemKind: UserDraftItemKind, path: string): void {
		overwriteOpen.delete(keyOf(workspace, itemKind, path))
	},

	/** Confirm: adopt the current (edited) value as our own draft and resume saving. */
	confirmOverwrite(workspace: string, itemKind: UserDraftItemKind, path: string): void {
		const current = UserDraft.get(itemKind, path, { workspace })
		this.clear(workspace, itemKind, path)
		if (current !== undefined) {
			void UserDraftDbSyncer.save({ workspace, itemKind, path, value: current, immediate: true })
		}
	},

	/** Discard the loaded view and restore our own draft. */
	async resetToOwnDraft(
		workspace: string,
		itemKind: UserDraftItemKind,
		path: string
	): Promise<void> {
		const session = active.get(keyOf(workspace, itemKind, path))
		this.clear(workspace, itemKind, path)
		await session?.onResetToOwnDraft()
	},

	/** Exit overlay mode: unlock saves, drop the session, close the modal. */
	clear(workspace: string, itemKind: UserDraftItemKind, path: string): void {
		const k = keyOf(workspace, itemKind, path)
		active.delete(k)
		overwriteOpen.delete(k)
		armed.delete(k)
		UserDraftDbSyncer.unlockSync({ workspace, itemKind, path })
	}
}
