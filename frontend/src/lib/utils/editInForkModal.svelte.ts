import { createState } from '$lib/svelte5Utils.svelte'
import type { StateStore } from '$lib/utils'
import type { ItemType } from './editInFork'

/**
 * An "Edit in <dev workspace>" click that landed on an item the dev workspace
 * doesn't have yet. Held globally so the confirmation renders once in the logged
 * layout instead of per item row.
 */
export type UpdateDevWorkspaceModalState = {
	itemType: ItemType
	itemPath: string
	devWorkspaceId: string
	devWorkspaceName: string
	prodWorkspaceId: string
	/**
	 * The click that raised this came from an editor's dropdown, which opens a tab rather than
	 * navigating away from work in progress. Answering it has to keep that promise: without this
	 * the "item is present" branch opens a tab while the "item is missing" branch — this prompt —
	 * would leave the editor the user was told would be preserved.
	 */
	openInNewTab?: boolean
}

export let updateDevWorkspaceModal: StateStore<UpdateDevWorkspaceModalState | undefined> =
	createState({
		val: undefined
	})
